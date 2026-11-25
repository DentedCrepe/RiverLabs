"""Push/fold trainer: drills graded against the exact heads-up push/fold equilibrium.

The exact all-in equity table (7 MB) is needed first. It is loaded from
`<result_storage_dir>/hu_combo_equity.bin` (the Android setup downloads a pre-built copy) and
verified on load: structure, symmetry and entries re-computed exactly. If it is missing, it can
be built here, which takes tens of minutes. Nothing is loaded until the trainer is used.
"""

import json
import logging
import secrets
import threading
import uuid
from pathlib import Path
from typing import Any, Literal

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.config import get_settings
from riverlab_api.db.models import PlayHand, PostflopAnswer, PreflopAnswer, SolverJob, SolverResult
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

    def icm_drill(self, spec: dict[str, Any], seed: int) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().icm_drill, json.dumps(spec), seed)
        return result

    def icm_grade(self, drill: dict[str, Any], choice: str) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().icm_grade, json.dumps(drill), choice)
        return result

    def icm_check(self, record: dict[str, Any], decision: int, config: dict[str, Any]) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(
            self._ready().icm_check, json.dumps(record), decision, json.dumps(config)
        )
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
        if spec.get("locks"):
            continue  # a node-locked solve is an exploit, not an equilibrium: never a drill answer key
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
    if result.assumptions["spec"].get("locks"):
        raise ConflictError("Drills are not dealt from node-locked solves: they are exploits, not equilibria")
    return trees.game(result), result.assumptions["spec"], result


# "Any saved solve": the solve the next drill will use, chosen at random when a drill is dealt
# and loaded in the background while the player answers. Loading a flop tree reads hundreds of MB
# from storage, so this keeps dealing fast without making the choice any less random.
_next_any: uuid.UUID | None = None
_next_any_lock = threading.Lock()


def _preload(job_id: uuid.UUID) -> None:
    from riverlab_api.db.session import get_sessionmaker

    try:
        with get_sessionmaker()() as db:
            _, result, _ = compute.JobService(db).result(job_id)
            trees.game(result)
    except Exception:  # only a warm-up: a failure here is reported when the drill is dealt
        logger.debug("could not preload postflop solve %s", job_id, exc_info=True)


def postflop_drill(db: Session, job_id: uuid.UUID | None, seed: int, hero: int | None) -> dict[str, Any]:
    global _next_any
    if job_id is None:
        sources = postflop_sources(db)
        if not sources:
            raise ConflictError("No saved postflop solves yet: solve a spot first (Solver, or a hand review)")
        ids = [s["job_id"] for s in sources]
        with _next_any_lock:
            job_id = _next_any if _next_any is not None and _next_any in ids else ids[seed % len(ids)]
            _next_any = ids[secrets.randbelow(len(ids))]
            upcoming = _next_any
        if upcoming != job_id:
            threading.Thread(target=_preload, args=(upcoming,), daemon=True).start()
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


# ---- preflop drills from saved preflop solves (GTOpen's preflop model: an approximate model) ----


def _model_label(result: SolverResult) -> dict[str, Any]:
    """What a preflop drill is graded against, stated with every drill and grade."""
    return {
        "model": result.assumptions["model"],
        "validated_scope": result.assumptions["validated_scope"],
        "model_notes": result.assumptions["model_notes"],
        "model_gap_mbb": result.summary.get("model_gap_mbb"),
        "iterations": result.iterations,
    }


def preflop_sources(db: Session) -> list[dict[str, Any]]:
    """Saved preflop solves whose trees are on disk, newest first."""
    rows = db.execute(
        select(SolverJob, SolverResult)
        .join(SolverResult, SolverResult.job_id == SolverJob.id)
        .where(SolverJob.kind == "preflop", SolverResult.storage_uri.is_not(None))
        .order_by(SolverJob.finished_at.desc().nulls_last(), SolverJob.id)
    ).all()
    out = []
    for job, result in rows:
        if not trees.path_from_uri(result.storage_uri or "").exists():
            continue
        game = result.assumptions["spec"]["game"]
        out.append(
            {
                "job_id": job.id,
                "positions": result.summary.get("positions", []),
                "effective_stack_mbb": game["stack"]["effective_stack"],
                "rake_bps": game["rake"]["percentage_bps"],
                "finished_at": job.finished_at,
                **_model_label(result),
            }
        )
    return out


def _preflop_solve(db: Session, job_id: uuid.UUID) -> tuple[Any, SolverResult]:
    _, result, _ = compute.JobService(db).result(job_id)
    if result.job.kind != "preflop":
        raise ConflictError("Preflop drills need a preflop solve")
    return trees.game(result), result


def preflop_drill(db: Session, job_id: uuid.UUID | None, spot: str, seed: int) -> dict[str, Any]:
    if job_id is None:
        sources = preflop_sources(db)
        if not sources:
            raise ConflictError("No saved preflop solves yet: run one on the Preflop page first")
        job_id = sources[seed % len(sources)]["job_id"]
    game, result = _preflop_solve(db, job_id)
    drill: dict[str, Any] = engine.play_call(game.drill, spot, seed)
    return {
        "job_id": job_id,
        "drill": drill,
        "effective_stack_mbb": result.assumptions["spec"]["game"]["stack"]["effective_stack"],
        **_model_label(result),
    }


def preflop_answer(
    db: Session, job_id: uuid.UUID, drill: dict[str, Any], choice: int, store: bool = True
) -> dict[str, Any]:
    game, result = _preflop_solve(db, job_id)
    grade: dict[str, Any] = engine.play_call(game.grade, json.dumps(drill), choice)
    out = {**grade, **_model_label(result)}
    if not store:
        return out
    d = grade["drill"]
    db.add(
        PreflopAnswer(
            job_id=job_id,
            spot=d["spot"],
            position=d["position"],
            cards="".join(d["cards"]),
            hand_class=d["class"],
            choice=grade["choice_label"][:40],
            verdict=grade["verdict"],
            model_frequency=min(max(float(grade["your_frequency"]), 0.0), 1.0),
            grade=grade,
        )
    )
    db.commit()
    return out
