"""Games against bots (engine `play_engine`).

A game lives in memory in the API process (one `riverlab_engine.PlaySession` per game) while
it is being played; every finished hand is written to `play_hands` straight away, so the
report survives a restart even though the game itself does not (it is marked "interrupted").
Preparing a new table solves the bots' preflop model in a background thread.
"""

import logging
import threading
import uuid
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from datetime import UTC, datetime
from typing import Any

from sqlalchemy import func, select, update
from sqlalchemy.orm import Session

from riverlab_api.db.models import PlayHand, PlaySession
from riverlab_api.errors import ConflictError, NotFoundError, SolverUnavailableError
from riverlab_api.services import activity, engine

logger = logging.getLogger(__name__)
SessionFactory = Callable[[], Session]


class PlayManager:
    def __init__(
        self, session_factory: SessionFactory, model_cache_dir: str | None, max_memory_mb: int, threads: int
    ) -> None:
        self._sessions = session_factory
        self._cache_dir = model_cache_dir
        self._max_bytes = max_memory_mb * 1024 * 1024
        self._threads = max(1, threads)
        self._games: dict[uuid.UUID, Any] = {}
        self._locks: dict[uuid.UUID, threading.Lock] = {}
        self._lock = threading.Lock()
        self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="riverlab-play-prepare")

    def shutdown(self) -> None:
        self._executor.shutdown(wait=False, cancel_futures=True)

    # ---- lifecycle ----

    def create(self, db: Session, config: dict[str, Any]) -> PlaySession:
        canonical = engine.validate_table_config(config)
        game = engine.new_play_session(canonical, self._cache_dir, self._max_bytes, self._threads)
        row = PlaySession(format=canonical["format"]["type"], config=canonical, status="preparing")
        db.add(row)
        db.commit()
        with self._lock:
            self._games[row.id] = game
            self._locks[row.id] = threading.Lock()
        activity.begin()
        self._executor.submit(self._prepare, row.id, game)
        return row

    def _prepare(self, session_id: uuid.UUID, game: Any) -> None:
        try:
            try:
                game.prepare()
                status, error = "active", None
            except Exception as exc:  # reported on the session, never swallowed
                logger.exception("preparing play session %s failed", session_id)
                status, error = "failed", str(exc)[:500]
            with self._sessions() as s:
                s.execute(
                    update(PlaySession)
                    .where(PlaySession.id == session_id, PlaySession.status == "preparing")
                    .values(status=status, error_message=error)
                )
                s.commit()
        finally:
            activity.end()

    def get(self, db: Session, session_id: uuid.UUID) -> PlaySession:
        row = db.get(PlaySession, session_id)
        if row is None:
            raise NotFoundError(f"Game {session_id} not found")
        return row

    def _game(self, db: Session, session_id: uuid.UUID) -> tuple[PlaySession, Any, threading.Lock]:
        row = self.get(db, session_id)
        with self._lock:
            game, lock = self._games.get(session_id), self._locks.get(session_id)
        if game is None or lock is None:
            raise ConflictError(
                f"This game is {row.status}; it can no longer be played (its report is still available)"
            )
        if row.status == "preparing":
            raise ConflictError("The table is still being prepared (the bots' preflop model is being solved)")
        if row.status != "active":
            raise ConflictError(f"This game is {row.status}")
        return row, game, lock

    def view(self, db: Session, session_id: uuid.UUID) -> dict[str, Any] | None:
        with self._lock:
            game = self._games.get(session_id)
        if game is None:
            self.get(db, session_id)
            return None
        result: dict[str, Any] = engine.play_call(game.view)
        return result

    def next_hand(self, db: Session, session_id: uuid.UUID) -> dict[str, Any]:
        row, game, lock = self._game(db, session_id)
        with lock, activity.working():
            view: dict[str, Any] = engine.play_call(game.next_hand)
            self._sync(db, row, game, view)
        return view

    def act(self, db: Session, session_id: uuid.UUID, action: dict[str, Any]) -> dict[str, Any]:
        import json

        row, game, lock = self._game(db, session_id)
        with lock:
            view: dict[str, Any] = engine.play_call(game.act, json.dumps(action))
            self._sync(db, row, game, view)
        return view

    def set_reveal(self, db: Session, session_id: uuid.UUID, on: bool) -> dict[str, Any]:
        _, game, lock = self._game(db, session_id)
        with lock:
            view: dict[str, Any] = engine.play_call(game.set_reveal, on)
        return view

    def leave(self, db: Session, session_id: uuid.UUID) -> dict[str, Any] | None:
        row = self.get(db, session_id)
        with self._lock:
            game = self._games.pop(session_id, None)
            self._locks.pop(session_id, None)
        view: dict[str, Any] | None = None
        if game is not None:
            view = engine.play_call(game.leave)
            self._sync(db, row, game, view)
        if row.status in ("preparing", "active"):
            row.status = "finished"
            row.finished_at = datetime.now(UTC)
            db.commit()
        return view

    def delete(self, db: Session, session_id: uuid.UUID) -> None:
        row = self.get(db, session_id)
        with self._lock:
            self._games.pop(session_id, None)
            self._locks.pop(session_id, None)
        db.delete(row)
        db.commit()

    def _sync(self, db: Session, row: PlaySession, game: Any, view: dict[str, Any]) -> None:
        """Stores hands finished since the last sync and updates the session row."""
        records: list[dict[str, Any]] = engine.play_call(game.records, row.hands)
        for r in records:
            db.add(PlayHand(session_id=row.id, hand_no=r["hand_no"], record=r))
            hero = next(i for i, s in enumerate(r["seats"]) if s["is_hero"])
            row.hero_net += r["net"][hero]
        row.hands += len(records)
        if view["status"] == "session_over" and row.status == "active":
            row.status = "finished"
            row.finished_at = datetime.now(UTC)
            with self._lock:
                self._games.pop(row.id, None)
                self._locks.pop(row.id, None)
        db.commit()

    # ---- reading ----

    def records(self, db: Session, session_id: uuid.UUID) -> list[dict[str, Any]]:
        self.get(db, session_id)
        rows = db.scalars(
            select(PlayHand).where(PlayHand.session_id == session_id).order_by(PlayHand.hand_no)
        )
        return [r.record for r in rows]

    def report(self, db: Session, session_id: uuid.UUID) -> dict[str, Any]:
        row = self.get(db, session_id)
        return engine.play_report(self.records(db, session_id), row.format == "sit_and_go")

    def spot(self, db: Session, session_id: uuid.UUID, hand_no: int, decision: int) -> dict[str, Any]:
        """The solve for one of your decisions in a stored hand (heads-up, after the flop)."""
        row = self.get(db, session_id)
        hand = db.scalar(
            select(PlayHand).where(PlayHand.session_id == session_id, PlayHand.hand_no == hand_no)
        )
        if hand is None:
            raise NotFoundError(f"Hand {hand_no} not found in this game")
        return engine.play_spot(hand.record, decision, row.config, self._max_bytes)

    def all_hands(self, db: Session, verdict: str | None, limit: int, offset: int) -> dict[str, Any]:
        """Hands across sessions, newest first, optionally only those with a decision of `verdict`."""
        q = select(PlayHand, PlaySession).join(PlaySession, PlaySession.id == PlayHand.session_id)
        if verdict is not None:
            q = q.where(PlayHand.record["hero_decisions"].contains([{"feedback": {"verdict": verdict}}]))
        total = db.scalar(select(func.count()).select_from(q.subquery())) or 0
        rows = db.execute(
            q.order_by(PlaySession.created_at.desc(), PlayHand.hand_no.desc()).limit(limit).offset(offset)
        ).all()
        return {
            "total": total,
            "hands": [
                {
                    "session_id": s.id,
                    "format": s.format,
                    "session_created_at": s.created_at,
                    "record": h.record,
                }
                for h, s in rows
            ],
        }

    def list(self, db: Session, limit: int = 50) -> list[PlaySession]:
        q = select(PlaySession).order_by(PlaySession.created_at.desc(), PlaySession.id).limit(limit)
        return list(db.scalars(q))


def fail_interrupted_games(db: Session) -> int:
    """Games cannot survive a restart: mark unfinished ones interrupted (their hands are kept)."""
    result = db.execute(
        update(PlaySession)
        .where(PlaySession.status.in_(["preparing", "active"]))
        .values(status="interrupted", finished_at=datetime.now(UTC))
    )
    db.commit()
    return int(result.rowcount or 0)  # type: ignore[attr-defined]


# Set at startup (main.py) when the solver is enabled.
manager: PlayManager | None = None


def require() -> PlayManager:
    if manager is None:
        raise SolverUnavailableError(
            "Playing against bots needs the solver backend (RIVERLAB_SOLVER_ENABLED=true)"
        )
    return manager
