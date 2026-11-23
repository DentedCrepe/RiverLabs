"""The `local` compute backend: GTOpen solves in a background thread of the API process.

One solve runs at a time (a single worker thread); further jobs wait in the executor's FIFO
queue. Jobs are tracked in the database through JobService, so status, progress, cancellation
and failures follow the documented lifecycle. A job interrupted by a server restart cannot
resume; `fail_interrupted_jobs` marks it failed on startup.
"""

import logging
import os
import threading
import uuid
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import JobStatus, SolverJob, SolverResult, StrategyNode
from riverlab_api.services import engine

logger = logging.getLogger(__name__)
SessionFactory = Callable[[], Session]

BACKEND_NAME = "local"
STREET = {3: "flop", 4: "turn", 5: "river"}


class LocalSolverBackend:
    name = BACKEND_NAME

    def __init__(
        self,
        session_factory: SessionFactory,
        max_memory_mb: int,
        threads: int,
        storage_dir: str | None = None,
        equity_cache_dir: str | None = None,
    ) -> None:
        self._sessions = session_factory
        self._storage = Path(storage_dir).resolve() if storage_dir else None
        if self._storage:
            self._storage.mkdir(parents=True, exist_ok=True)
        # GTOpen's preflop class-equity table (~30 s to build) is cached here; defaults to storage.
        self._equity_cache = Path(equity_cache_dir).resolve() if equity_cache_dir else self._storage
        self._max_bytes = max_memory_mb * 1024 * 1024
        self._threads = threads or (os.cpu_count() or 1)
        self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="riverlab-solver")
        self._tokens: dict[uuid.UUID, Any] = {}
        self._lock = threading.Lock()

    def can_run(self, spec: dict[str, Any]) -> bool:
        return True

    @property
    def equity_cache_dir(self) -> Path | None:
        return self._equity_cache

    def submit(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str = "postflop") -> None:
        with self._lock:
            self._tokens[job_id] = engine.CancelToken()
        self._executor.submit(self._run_guarded, job_id, spec, kind)

    def request_cancel(self, job_id: uuid.UUID) -> None:
        with self._lock:
            token = self._tokens.get(job_id)
        if token is not None:
            token.cancel()

    def shutdown(self) -> None:
        with self._lock:
            for token in self._tokens.values():
                token.cancel()
        self._executor.shutdown(wait=True, cancel_futures=True)

    # ---- worker ----

    def _run_guarded(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str) -> None:
        try:
            self._run(job_id, spec, kind)
        except Exception:  # pragma: no cover - last-resort reporting, never silent
            logger.exception("solver job %s crashed", job_id)
            self._finish(
                job_id, JobStatus.FAILED, ("internal_error", "The solver worker crashed; see server logs")
            )
        finally:
            with self._lock:
                self._tokens.pop(job_id, None)

    def _service(self, session: Session) -> Any:
        from riverlab_api.services.compute import BackendRegistry, JobService

        return JobService(session, BackendRegistry())

    def _run(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str) -> None:
        with self._sessions() as session:
            job = session.get(SolverJob, job_id)
            if job is None or job.status != JobStatus.QUEUED:
                return  # cancelled while queued, or gone
            self._service(session).mark_running(job_id)
        with self._lock:
            token = self._tokens[job_id]

        def on_progress(p: dict[str, Any]) -> None:
            with self._sessions() as s:
                current = s.get(SolverJob, job_id)
                if current is not None and current.status == JobStatus.RUNNING:
                    self._service(s).report_progress(job_id, int(p["iterations"]), p)

        suffix = "gtp" if kind == "preflop" else "gto"
        save_path = str(self._storage / f"{job_id}.{suffix}") if self._storage else None
        try:
            if kind == "preflop":
                outcome = engine.solve_preflop(
                    spec,
                    token,
                    on_progress,
                    self._max_bytes,
                    self._threads,
                    save_path,
                    str(self._equity_cache) if self._equity_cache else None,
                )
            else:
                outcome = engine.solve_postflop(
                    spec, token, on_progress, self._max_bytes, self._threads, save_path
                )
        except engine.SolveRejectedError as exc:
            self._finish(job_id, JobStatus.FAILED, ("solve_rejected", str(exc)))
            return
        except engine.SolveFailedError as exc:
            self._finish(job_id, JobStatus.FAILED, ("solver_error", str(exc)))
            return
        stop_reason = outcome["outcome"]["stop_reason"] if kind == "preflop" else outcome["stop_reason"]
        if stop_reason == "cancelled":
            self._finish(job_id, JobStatus.CANCELLED)
            return
        with self._sessions() as session:
            if kind == "preflop":
                self._store_preflop_result(session, job_id, spec, outcome, save_path)
            else:
                self._store_result(session, job_id, spec, outcome, save_path)
            self._service(session).mark_finished(job_id, JobStatus.SUCCEEDED)

    def _finish(self, job_id: uuid.UUID, status: JobStatus, error: tuple[str, str] | None = None) -> None:
        with self._sessions() as session:
            job = session.get(SolverJob, job_id)
            if job is None or job.status.is_terminal:
                return
            self._service(session).mark_finished(job_id, status, error=error)

    @staticmethod
    def _store_result(
        session: Session,
        job_id: uuid.UUID,
        spec: dict[str, Any],
        outcome: dict[str, Any],
        save_path: str | None,
    ) -> None:
        progress = outcome["progress"]
        result = SolverResult(
            job_id=job_id,
            engine_version=engine.engine_version(),
            solver_backend=outcome["backend"],
            solver_version=outcome["backend_commit"],
            config_schema_version=spec["solver"]["schema_version"],
            assumptions={
                "spec": spec,
                "approximations": outcome["approximations"],
                "backend_repository": outcome["backend_repository"],
                "algorithm": outcome["algorithm"],
                "units": "mbb",
                "values_convention": "net of each player's half of the starting pot",
            },
            iterations=progress["iterations"],
            exploitability=progress["exploitability_mbb"],
            exploitability_unit="mbb",
            storage_uri=Path(save_path).as_uri() if save_path else None,
            summary={
                "stop_reason": outcome["stop_reason"],
                "values_mbb": outcome["values_mbb"],
                "exploitability_pct_pot": progress["exploitability_pct_pot"],
                "elapsed_ms": progress["elapsed_ms"],
                "memory_bytes": outcome["memory_bytes"],
                "independent_check": outcome["independent_check"],
            },
        )
        session.add(result)
        session.flush()
        root = outcome["root"]
        session.add(
            StrategyNode(
                result_id=result.id,
                node_path="root",
                street=STREET[len(spec["board"])],
                board="".join(spec["board"]),
                player=root["player"],
                actions=root["actions"],
                strategy={
                    "combos": root["combos"],
                    "frequencies": root["frequencies"],
                    "by_class": outcome["root_by_class"],
                },
            )
        )
        session.commit()

    @staticmethod
    def _store_preflop_result(
        session: Session,
        job_id: uuid.UUID,
        spec: dict[str, Any],
        result: dict[str, Any],
        save_path: str | None,
    ) -> None:
        outcome = result["outcome"]
        progress = outcome["progress"]
        session.add(
            SolverResult(
                job_id=job_id,
                engine_version=engine.engine_version(),
                solver_backend=outcome["backend"],
                solver_version=outcome["backend_commit"],
                config_schema_version=spec["game"]["schema_version"],
                assumptions={
                    "spec": spec,
                    "model": "GTOpen preflop model (approximate; flop pots are modelled, not solved)",
                    "validated_scope": outcome["validated_scope"],
                    "model_notes": outcome["model_notes"],
                    "backend_repository": outcome["backend_repository"],
                    "units": "mbb",
                },
                iterations=progress["iterations"],
                # Not an exploitability of the real game: the model's own best-response gap is in
                # the summary instead, labelled as such.
                exploitability=None,
                exploitability_unit=None,
                storage_uri=Path(save_path).as_uri() if save_path else None,
                summary={
                    "stop_reason": outcome["stop_reason"],
                    "positions": outcome["positions"],
                    "model_gap_mbb": progress["model_gap_mbb"],
                    "gaps_mbb": outcome["gaps_mbb"],
                    "values_mbb": outcome["values_mbb"],
                    "elapsed_ms": progress["elapsed_ms"],
                    "memory_bytes": outcome["memory_bytes"],
                    "nodes": outcome["nodes"],
                    "root": result["root"],
                },
            )
        )
        session.commit()


def fail_interrupted_jobs(session: Session) -> int:
    """Local jobs cannot survive a restart: mark queued/running ones failed."""
    from riverlab_api.services.compute import BackendRegistry, JobService

    svc = JobService(session, BackendRegistry())
    jobs = session.scalars(
        select(SolverJob).where(
            SolverJob.backend == BACKEND_NAME,
            SolverJob.status.in_([JobStatus.QUEUED, JobStatus.RUNNING]),
        )
    ).all()
    for job in jobs:
        svc.mark_finished(
            job.id, JobStatus.FAILED, error=("interrupted", "The server restarted before the solve finished")
        )
    return len(jobs)
