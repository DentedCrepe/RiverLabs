"""Solver review of a whole hand: one postflop solve per street, chained (docs/play.md).

A review runs in a background thread. For each street where you made decisions it builds the
street's solve with the engine (`play_engine::review`), queues it as an ordinary solver job (so
it also appears in the Solver and the Solution Library), waits for it, then follows the real line
through the saved tree. The next street starts from the ranges where this one ended; a street
that cannot be solved (multiway) breaks the chain and the next starts afresh.
"""

import logging
import threading
import time
import uuid
from collections import deque
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from sqlalchemy import func, select
from sqlalchemy.orm import Session

from riverlab_api.db.models import HandReview, PlayHand, PlaySession, SolverJob, SolverResult
from riverlab_api.errors import ConflictError, NotFoundError
from riverlab_api.services import compute, engine, trees
from riverlab_api.services.play import SessionFactory

logger = logging.getLogger(__name__)
POLL_SECONDS = 2.0


def _streets_out(row: HandReview, db: Session) -> list[dict[str, Any]]:
    out = []
    for st in row.streets:
        item = dict(st)
        if st.get("job_id") and st.get("status") == "solving":
            job = db.get(SolverJob, uuid.UUID(st["job_id"]))
            if job is not None:
                item["job_status"] = job.status.value
                item["progress"] = job.progress
        out.append(item)
    return out


def review_out(row: HandReview, db: Session) -> dict[str, Any]:
    return {
        "id": row.id,
        "session_id": row.session_id,
        "hand_no": row.hand_no,
        "status": row.status,
        "streets": _streets_out(row, db),
        "error_message": row.error_message,
        "created_at": row.created_at,
        "finished_at": row.finished_at,
    }


def _pot_size(h: PlayHand) -> float:
    """How much you won or lost in the hand, in big blinds."""
    r = h.record
    hero = next((i for i, s in enumerate(r.get("seats", [])) if s.get("is_hero")), 0)
    net = r.get("net", [0])
    return float(abs(net[hero] if hero < len(net) else 0)) / float(max(1, r.get("big_blind", 1)))


class HandReviewer:
    def __init__(self, session_factory: SessionFactory, max_bytes: int) -> None:
        self._sessions = session_factory
        self._max_bytes = max_bytes
        self._stop = threading.Event()
        # One review runs at a time, in order; reviews you start yourself go to the front.
        self._queue: deque[uuid.UUID] = deque()
        self._wake = threading.Condition()
        self._worker = threading.Thread(target=self._loop, name="hand-reviews", daemon=True)
        self._worker.start()

    def shutdown(self) -> None:
        self._stop.set()
        with self._wake:
            self._wake.notify_all()

    def _loop(self) -> None:
        while not self._stop.is_set():
            with self._wake:
                while not self._queue and not self._stop.is_set():
                    self._wake.wait(timeout=5)
                if self._stop.is_set():
                    return
                review_id = self._queue.popleft()
            self._run(review_id)

    def _enqueue(self, review_id: uuid.UUID, front: bool) -> None:
        with self._wake:
            if front:
                self._queue.appendleft(review_id)
            else:
                self._queue.append(review_id)
            self._wake.notify()

    def resume_unfinished(self) -> int:
        """After a restart: queues every review that was queued or running, oldest first. Their
        solves are in the durable job queue, so finished streets are kept and a street whose
        solve was in progress waits for that same solve."""
        with self._sessions() as db:
            rows = db.scalars(
                select(HandReview)
                .where(HandReview.status.in_(["queued", "running"]))
                .order_by(HandReview.created_at)
            ).all()
            ids = [r.id for r in rows]
            for r in rows:
                r.status = "queued"
            db.commit()
        for review_id in ids:
            self._enqueue(review_id, front=False)
        return len(ids)

    def get(self, db: Session, session_id: uuid.UUID, hand_no: int) -> HandReview | None:
        return db.scalar(
            select(HandReview).where(HandReview.session_id == session_id, HandReview.hand_no == hand_no)
        )

    def start(self, db: Session, session_id: uuid.UUID, hand_no: int, front: bool = True) -> HandReview:
        hand = db.scalar(
            select(PlayHand).where(PlayHand.session_id == session_id, PlayHand.hand_no == hand_no)
        )
        if hand is None:
            raise NotFoundError(f"Hand {hand_no} not found in this game")
        existing = self.get(db, session_id, hand_no)
        if existing is not None:
            if existing.status in ("queued", "running", "done"):
                return existing
            db.delete(existing)  # failed or interrupted: start again
            db.flush()
        streets = engine.review_streets(hand.record)
        if not any(s["solvable"] for s in streets):
            raise ConflictError(
                "Nothing in this hand can be solved: RiverLab solves heads-up decisions after the flop, "
                "and this hand has none."
            )
        row = HandReview(
            session_id=session_id,
            hand_no=hand_no,
            status="queued",
            streets=[{**s, "status": "pending" if s["solvable"] else "skipped"} for s in streets],
        )
        db.add(row)
        db.commit()
        self._enqueue(row.id, front)
        return row

    def batch(self, db: Session, scope: str, limit: int) -> dict[str, Any]:
        """Queues reviews for up to `limit` of your hands that have something to solve and no
        review yet: the most recent, those with a flagged or unusual decision, or the biggest pots."""
        rows = list(
            db.scalars(
                select(PlayHand)
                .join(PlaySession, PlaySession.id == PlayHand.session_id)
                .where(PlaySession.tracked)
                .order_by(PlayHand.created_at.desc())
                .limit(2000)
            )
        )
        reviewed = {
            (r.session_id, r.hand_no)
            for r in db.scalars(
                select(HandReview).where(HandReview.status.in_(["queued", "running", "done"]))
            )
        }

        def flagged(h: PlayHand) -> bool:
            return any(
                (d.get("feedback") or {}).get("verdict") in ("mistake", "rare")
                for d in h.record.get("hero_decisions", [])
            )

        if scope == "flagged":
            rows = [h for h in rows if flagged(h)]
        elif scope == "biggest":
            rows.sort(key=_pot_size, reverse=True)
        queued: list[dict[str, Any]] = []
        for h in rows:
            if len(queued) >= limit:
                break
            if (h.session_id, h.hand_no) in reviewed:
                continue
            if not any(s["solvable"] for s in engine.review_streets(h.record)):
                continue
            self.start(db, h.session_id, h.hand_no, front=False)
            queued.append({"session_id": h.session_id, "hand_no": h.hand_no})
        return {"queued": queued, **self.status(db)}

    def status(self, db: Session) -> dict[str, Any]:
        counts = dict(
            db.execute(select(HandReview.status, func.count(HandReview.id)).group_by(HandReview.status)).all()
        )
        return {
            "reviews": {
                k: int(counts.get(k, 0)) for k in ("queued", "running", "done", "failed", "interrupted")
            }
        }

    # ---- background ----

    def _save(self, review_id: uuid.UUID, **changes: Any) -> None:
        with self._sessions() as db:
            row = db.get(HandReview, review_id)
            if row is None:
                return
            for k, v in changes.items():
                setattr(row, k, v)
            db.commit()

    def _run(self, review_id: uuid.UUID) -> None:
        try:
            self._chain(review_id)
        except Exception as exc:
            logger.exception("hand review %s failed", review_id)
            self._save(
                review_id, status="failed", error_message=str(exc)[:1000], finished_at=datetime.now(UTC)
            )

    def _chain(self, review_id: uuid.UUID) -> None:
        with self._sessions() as db:
            row = db.get(HandReview, review_id)
            assert row is not None
            hand = db.scalar(
                select(PlayHand).where(PlayHand.session_id == row.session_id, PlayHand.hand_no == row.hand_no)
            )
            session = db.get(PlaySession, row.session_id)
            if hand is None or session is None:
                raise NotFoundError("the hand or its game was deleted")
            record, config, streets = hand.record, session.config, [dict(s) for s in row.streets]
        self._save(review_id, status="running")
        carried: list[str] | None = None
        for i, st in enumerate(streets):
            if self._stop.is_set():
                return
            if not st["solvable"]:
                carried = None  # the chain breaks; the next solvable street starts afresh
                continue
            if st.get("status") == "done":
                # Resumed after a restart: this street is finished; carry its ranges on.
                carried = (st.get("review") or {}).get("next_ranges")
                continue
            spec = engine.review_street_spec(record, st["street"], config, self._max_bytes, carried)
            if st.get("status") == "solving" and st.get("job_id"):
                # Resumed: the solve is still in the durable queue (or finished meanwhile).
                job_id = uuid.UUID(st["job_id"])
            else:
                with self._sessions() as db:
                    job = compute.JobService(db).submit(spec["spec"])
                    job_id = job.id
            st.update(
                status="solving",
                job_id=str(job_id),
                notes=spec["notes"],
                # The real line and your hand, so the solve can be opened at each of your decisions.
                line=spec["line"],
                hero_player=spec["hero_player"],
                hero_combo=spec["hero_combo"],
                hero_class=spec["hero_class"],
            )
            self._save(review_id, streets=[dict(s) for s in streets])
            job_status, path, canonical = self._wait(job_id)
            if job_status != "succeeded" or path is None:
                st.update(
                    status="failed", error=f"The solve {job_status}" + ("" if path else " (no saved tree)")
                )
                carried = None
            else:
                result = engine.review_analyze({**spec, "spec": canonical}, path)
                st.update(status="done", review=result)
                carried = result.get("next_ranges")
            self._save(review_id, streets=[dict(s) for s in streets])
            streets[i] = st
        self._save(review_id, status="done", finished_at=datetime.now(UTC))

    def _wait(self, job_id: uuid.UUID) -> tuple[str, str | None, dict[str, Any]]:
        while not self._stop.is_set():
            with self._sessions() as db:
                job = db.get(SolverJob, job_id)
                assert job is not None
                if job.status.is_terminal:
                    result = db.scalar(select(SolverResult).where(SolverResult.job_id == job_id))
                    path = None
                    if result is not None and result.storage_uri:
                        p = trees.path_from_uri(result.storage_uri)
                        path = str(p) if Path(p).exists() else None
                    return job.status.value, path, job.spec
            time.sleep(POLL_SECONDS)
        return "cancelled", None, {}


# Set at startup (main.py) when the solver is enabled.
reviewer: HandReviewer | None = None


def require() -> HandReviewer:
    from riverlab_api.errors import SolverUnavailableError

    if reviewer is None:
        raise SolverUnavailableError("Hand reviews need the solver, which is not enabled on this server.")
    return reviewer


def hand_of_jobs(db: Session) -> dict[str, dict[str, Any]]:
    """job id -> {session_id, hand_no, street} for every street solve of a hand review."""
    out: dict[str, dict[str, Any]] = {}
    for row in db.scalars(select(HandReview)):
        for st in row.streets:
            if st.get("job_id"):
                out[st["job_id"]] = {
                    "session_id": row.session_id,
                    "hand_no": row.hand_no,
                    "street": st["street"],
                }
    return out
