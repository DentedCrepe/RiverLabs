"""Push/fold trainer: drills graded against the exact heads-up push/fold equilibrium.

The exact all-in equity table (7 MB) is needed first. It is loaded from
`<result_storage_dir>/hu_combo_equity.bin` (the Android setup downloads a pre-built copy) and
verified on load: structure, symmetry and entries re-computed exactly. If it is missing, it can
be built here, which takes tens of minutes. Nothing is loaded until the trainer is used.
"""

import json
import logging
import threading
import uuid
from pathlib import Path
from typing import Any, Literal

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.config import get_settings
from riverlab_api.db.models import PlayHand, PostflopAnswer, SolverJob, SolverResult
from riverlab_api.errors import ConflictError
from riverlab_api.services import activity, compute, engine, hand_review, trees

logger = logging.getLogger(__name__)

TABLE_NAME = "hu_combo_equity.bin"
# Entries re-computed exactly when a table is loaded (about 1 s of work).
VERIFY_SAMPLES = 200

State = Literal["missing", "loading", "building", "ready", "error"]


class PushFoldService:
    def __init__(self, table_path: Path) -> None:
        self.table_path = table_path
        self._lock = threading.Lock()
        self._state: State = "missing"
        self._progress: float | None = None
        self._message: str | None = None
        self._trainer: Any = None
        self._thread: threading.Thread | None = None

    def status(self) -> dict[str, Any]:
        """Also starts loading a table that is present but not loaded yet."""
        with self._lock:
            if self._state == "missing" and self.table_path.exists():
                self._start_locked(build=False)
            return {
                "state": self._state,
                "progress": self._progress,
                "message": self._message,
                "table_present": self.table_path.exists(),
            }

    def build(self) -> dict[str, Any]:
        """Builds the table (or loads it if present)."""
        with self._lock:
            if self._state in ("loading", "building", "ready"):
                raise ConflictError(f"The equity table is already {self._state}")
            self._start_locked(build=not self.table_path.exists())
        return self.status()

    def _start_locked(self, build: bool) -> None:
        if self._thread is not None and self._thread.is_alive():
            return
        self._state = "building" if build else "loading"
        self._progress = 0.0 if build else None
        self._message = None
        self._thread = threading.Thread(target=self._run, args=(build,), name="push-fold-table", daemon=True)
        self._thread.start()

    def _run(self, build: bool) -> None:
        with activity.working():
            try:
                if build:
                    self.table_path.parent.mkdir(parents=True, exist_ok=True)

                    def progress(done: int, total: int) -> None:
                        with self._lock:
                            self._progress = done / total

                    engine.push_fold_build_table(str(self.table_path), progress)
                    with self._lock:
                        self._state = "loading"
                        self._progress = None
                trainer = engine.push_fold_trainer(str(self.table_path), VERIFY_SAMPLES)
                with self._lock:
                    self._trainer = trainer
                    self._state = "ready"
            except Exception as exc:
                logger.exception("push/fold equity table failed")
                with self._lock:
                    self._state = "error"
                    self._message = str(exc)

    def _ready(self) -> Any:
        with self._lock:
            trainer = self._trainer
        if trainer is None:
            raise ConflictError("The push/fold trainer is not ready: its equity table is not loaded yet")
        return trainer

    def drill(self, min_mbb: int, max_mbb: int, seat: str | None, seed: int) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().drill, min_mbb, max_mbb, seat, seed)
        return result

    def grade(self, drill: dict[str, Any], choice: str) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().grade, json.dumps(drill), choice)
        return result

    def chart(self, stack_mbb: int) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().chart, stack_mbb)
        return result


_service: PushFoldService | None = None
_service_lock = threading.Lock()


def service() -> PushFoldService:
    global _service
    with _service_lock:
        if _service is None:
            _service = PushFoldService(Path(get_settings().result_storage_dir) / TABLE_NAME)
        return _service


# ---- postflop drills from saved solves ----


def postflop_sources(db: Session) -> list[dict[str, Any]]:
    """Saved postflop solves whose trees are on disk, newest first."""
    rows = db.execute(
        select(SolverJob, SolverResult)
        .join(SolverResult, SolverResult.job_id == SolverJob.id)
        .where(SolverJob.kind == "postflop", SolverResult.storage_uri.is_not(None))
        .order_by(SolverJob.finished_at.desc().nulls_last(), SolverJob.id)
    ).all()
    out = []
    for job, result in rows:
        if not trees.path_from_uri(result.storage_uri or "").exists():
            continue
        spec = result.assumptions["spec"]
        out.append(
            {
                "job_id": job.id,
                "board": spec["board"],
                "starting_pot_mbb": spec["starting_pot_mbb"],
                "effective_stack_mbb": spec["effective_stack_mbb"],
                "exploitability_pct_pot": result.summary.get("exploitability_pct_pot"),
                "finished_at": job.finished_at,
            }
        )
    return out


def _solve(db: Session, job_id: uuid.UUID) -> tuple[Any, dict[str, Any], SolverResult]:
    _, result, _ = compute.JobService(db).result(job_id)
    if result.job.kind != "postflop":
        raise ConflictError("Postflop drills need a postflop solve")
    return trees.game(result), result.assumptions["spec"], result


def postflop_drill(db: Session, job_id: uuid.UUID | None, seed: int, hero: int | None) -> dict[str, Any]:
    if job_id is None:
        sources = postflop_sources(db)
        if not sources:
            raise ConflictError("No saved postflop solves yet: solve a spot first (Solver, or a hand review)")
        job_id = sources[seed % len(sources)]["job_id"]
    game, spec, result = _solve(db, job_id)
    drill: dict[str, Any] = engine.play_call(game.drill, json.dumps(spec), seed, hero)
    # A solve from a hand you played: send that hand, so the drill can show what came before.
    hand = None
    origin = hand_review.hand_of_jobs(db).get(str(job_id))
    if origin is not None:
        row = db.scalar(
            select(PlayHand).where(
                PlayHand.session_id == origin["session_id"], PlayHand.hand_no == origin["hand_no"]
            )
        )
        if row is not None:
            hand = {**origin, "record": row.record}
    return {
        "job_id": job_id,
        "drill": drill,
        "starting_pot_mbb": spec["starting_pot_mbb"],
        "exploitability_pct_pot": result.summary.get("exploitability_pct_pot"),
        "hand": hand,
    }


def postflop_answer(
    db: Session, job_id: uuid.UUID, drill: dict[str, Any], choice: int, store: bool = True
) -> dict[str, Any]:
    game, spec, _ = _solve(db, job_id)
    grade: dict[str, Any] = engine.play_call(game.grade, json.dumps(spec), json.dumps(drill), choice)
    if not store:
        return grade
    db.add(
        PostflopAnswer(
            job_id=job_id,
            street=drill["street"],
            hero_player=drill["hero_player"],
            cards=drill["hero_combo"],
            hand_class=drill["hero_class"],
            choice=grade["actions"][choice][:40],
            verdict=grade["verdict"],
            ev_loss_bb=grade["ev_loss_bb"],
            ev_loss_pct_pot=grade["ev_loss_pct_pot"],
            grade=grade,
        )
    )
    db.commit()
    return grade
