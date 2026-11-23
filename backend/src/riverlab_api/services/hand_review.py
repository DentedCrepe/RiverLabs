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
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from sqlalchemy import select, update
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


class HandReviewer:
    def __init__(self, session_factory: SessionFactory, max_bytes: int) -> None:
        self._sessions = session_factory
        self._max_bytes = max_bytes
        self._stop = threading.Event()

    def shutdown(self) -> None:
        self._stop.set()

    def get(self, db: Session, session_id: uuid.UUID, hand_no: int) -> HandReview | None:
        return db.scalar(
            select(HandReview).where(HandReview.session_id == session_id, HandReview.hand_no == hand_no)
        )

    def start(self, db: Session, session_id: uuid.UUID, hand_no: int) -> HandReview:
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
        threading.Thread(target=self._run, args=(row.id,), name=f"hand-review-{row.id}", daemon=True).start()
        return row

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
            spec = engine.review_street_spec(record, st["street"], config, self._max_bytes, carried)
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


def fail_interrupted_reviews(db: Session) -> int:
    """Reviews cannot survive a restart (their solves are failed too): mark unfinished ones."""
    result = db.execute(
        update(HandReview)
        .where(HandReview.status.in_(["queued", "running"]))
        .values(status="interrupted", finished_at=datetime.now(UTC))
    )
    db.commit()
    return int(result.rowcount or 0)  # type: ignore[attr-defined]


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
