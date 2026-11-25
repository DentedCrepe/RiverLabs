"""The `local` compute backend: GTOpen solves run by RiverLab's own workers.

The queue is the `solver_jobs` table (ADR-0007). A worker claims the oldest queued job with
`SELECT ... FOR UPDATE SKIP LOCKED`, marks it running under its worker id, and while it solves a
heartbeat thread records that it is alive and checks the job's `cancel_requested` flag, so a
cancel from any process stops the solve. A running job whose heartbeat goes stale (its worker
was restarted or crashed) goes back in the queue and runs again from the start, up to
`MAX_ATTEMPTS` times.

Workers run inside the API process (the default, `RIVERLAB_WORKER_MODE=embedded`) or as separate
processes (`python -m riverlab_api.worker`, with the API set to `external`). Each worker runs
one solve at a time; the solve itself uses many threads.
"""

import logging
import os
import socket
import threading
import uuid
from collections.abc import Callable
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import JobStatus, SolverJob, SolverResult, StrategyNode
from riverlab_api.services import activity, engine

logger = logging.getLogger(__name__)
SessionFactory = Callable[[], Session]

BACKEND_NAME = "local"
STREET = {3: "flop", 4: "turn", 5: "river"}
# A running job's worker writes a heartbeat this often…
HEARTBEAT_SECONDS = 2.0
# …and a heartbeat older than this means the worker is gone.
STALE_SECONDS = 30.0
# Starts before a job that keeps losing its worker is failed instead of retried.
MAX_ATTEMPTS = 3
# How often an idle worker looks for work and for stale jobs (submissions wake it at once).
POLL_SECONDS = 2.0


def new_worker_id() -> str:
    return f"{socket.gethostname()}:{os.getpid()}:{uuid.uuid4().hex[:8]}"


def recover_stale_jobs(session: Session, stale_after: float = STALE_SECONDS) -> dict[str, int]:
    """Running jobs whose worker stopped reporting: requeued, cancelled (if a cancel was asked
    for) or failed after `MAX_ATTEMPTS` starts. Safe to call from any number of workers."""
    from riverlab_api.services.compute import BackendRegistry, JobService

    svc = JobService(session, BackendRegistry())
    cutoff = datetime.now(UTC) - timedelta(seconds=stale_after)
    counts = {"requeued": 0, "failed": 0, "cancelled": 0}
    jobs = session.scalars(
        select(SolverJob)
        .where(
            SolverJob.backend == BACKEND_NAME,
            SolverJob.status == JobStatus.RUNNING,
            (SolverJob.heartbeat_at < cutoff) | (SolverJob.heartbeat_at.is_(None)),
        )
        .with_for_update(skip_locked=True)
    ).all()
    for job in jobs:
        if job.cancel_requested:
            svc.mark_finished(job.id, JobStatus.CANCELLED)
            counts["cancelled"] += 1
        elif job.attempts >= MAX_ATTEMPTS:
            svc.mark_finished(
                job.id,
                JobStatus.FAILED,
                error=(
                    "interrupted",
                    f"The solve was interrupted {job.attempts} times (server restarts or crashes)",
                ),
            )
            counts["failed"] += 1
        else:
            svc.requeue(job)
            counts["requeued"] += 1
    session.commit()
    return counts


class LocalSolverBackend:
    name = BACKEND_NAME

    def __init__(
        self,
        session_factory: SessionFactory,
        max_memory_mb: int,
        threads: int,
        storage_dir: str | None = None,
        equity_cache_dir: str | None = None,
        run_worker: bool = True,
        heartbeat_seconds: float = HEARTBEAT_SECONDS,
        stale_seconds: float = STALE_SECONDS,
        poll_seconds: float = POLL_SECONDS,
    ) -> None:
        self._sessions = session_factory
        self._storage = Path(storage_dir).resolve() if storage_dir else None
        if self._storage:
            self._storage.mkdir(parents=True, exist_ok=True)
        # GTOpen's preflop class-equity table (~30 s to build) is cached here; defaults to storage.
        self._equity_cache = Path(equity_cache_dir).resolve() if equity_cache_dir else self._storage
        self._max_bytes = max_memory_mb * 1024 * 1024
        self._threads = threads or (os.cpu_count() or 1)
        self._heartbeat = heartbeat_seconds
        self._stale = stale_seconds
        self._poll = poll_seconds
        self.worker_id = new_worker_id()
        self._tokens: dict[uuid.UUID, Any] = {}
        self._lock = threading.Lock()
        self._wake = threading.Event()
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        if run_worker:
            self._thread = threading.Thread(target=self._loop, name="riverlab-solver", daemon=True)
            self._thread.start()

    def can_run(self, spec: dict[str, Any]) -> bool:
        return True

    @property
    def equity_cache_dir(self) -> Path | None:
        return self._equity_cache

    def submit(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str = "postflop") -> None:
        # The job is already committed as queued; wake this process's worker (if any) at once.
        self._wake.set()

    def request_cancel(self, job_id: uuid.UUID) -> None:
        with self._lock:
            token = self._tokens.get(job_id)
        if token is not None:
            token.cancel()

    def shutdown(self) -> None:
        """Stops the worker. A solve in progress is cancelled locally but stays `running` in the
        database, so after a restart it is requeued rather than lost."""
        self._stop.set()
        self._wake.set()
        with self._lock:
            for token in self._tokens.values():
                token.cancel()
        if self._thread is not None:
            self._thread.join(timeout=60)

    def run_forever(self) -> None:
        """The worker loop in the calling thread (for `python -m riverlab_api.worker`)."""
        self._loop()

    # ---- worker ----

    def _loop(self) -> None:
        logger.info("solver worker %s started", self.worker_id)
        while not self._stop.is_set():
            try:
                with self._sessions() as session:
                    counts = recover_stale_jobs(session, self._stale)
                if any(counts.values()):
                    logger.warning("stale solver jobs: %s", counts)
                claimed = self._claim()
            except Exception:
                logger.exception("solver worker %s could not reach the database", self.worker_id)
                claimed = None
            if claimed is None:
                self._wake.wait(self._poll)
                self._wake.clear()
                continue
            self._run_guarded(*claimed)
        logger.info("solver worker %s stopped", self.worker_id)

    def _claim(self) -> tuple[uuid.UUID, dict[str, Any], str] | None:
        """Takes the oldest queued job, so no two workers ever run the same one."""
        with self._sessions() as session:
            job = session.scalars(
                select(SolverJob)
                .where(SolverJob.backend == BACKEND_NAME, SolverJob.status == JobStatus.QUEUED)
                .order_by(SolverJob.created_at, SolverJob.id)
                .limit(1)
                .with_for_update(skip_locked=True)
            ).first()
            if job is None:
                return None
            now = datetime.now(UTC)
            job.status = JobStatus.RUNNING
            job.started_at = now
            job.heartbeat_at = now
            job.worker_id = self.worker_id
            job.attempts += 1
            session.commit()
            return job.id, dict(job.spec), job.kind

    def _beat(self, job_id: uuid.UUID, token: Any, done: threading.Event) -> None:
        """While the solve runs: record that this worker is alive, and stop the solve when a
        cancel is requested (from any process) or the job was taken away from this worker."""
        while not done.wait(self._heartbeat):
            try:
                with self._sessions() as session:
                    job = session.get(SolverJob, job_id)
                    if job is None or job.status != JobStatus.RUNNING or job.worker_id != self.worker_id:
                        token.cancel()
                        return
                    if job.cancel_requested:
                        token.cancel()
                    job.heartbeat_at = datetime.now(UTC)
                    session.commit()
            except Exception:
                logger.exception("heartbeat for solver job %s failed", job_id)

    def _run_guarded(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str) -> None:
        token = engine.CancelToken()
        with self._lock:
            self._tokens[job_id] = token
        done = threading.Event()
        beat = threading.Thread(target=self._beat, args=(job_id, token, done), daemon=True)
        # Busy while solving: keeps the device awake and RiverLab running.
        activity.begin()
        beat.start()
        try:
            self._run(job_id, spec, kind, token)
        except Exception:  # pragma: no cover - last-resort reporting, never silent
            logger.exception("solver job %s crashed", job_id)
            self._finish(
                job_id, JobStatus.FAILED, ("internal_error", "The solver worker crashed; see server logs")
            )
        finally:
            done.set()
            beat.join(timeout=10)
            with self._lock:
                self._tokens.pop(job_id, None)
            activity.end()

    def _service(self, session: Session) -> Any:
        from riverlab_api.services.compute import BackendRegistry, JobService

        return JobService(session, BackendRegistry())

    def _still_mine(self, session: Session, job_id: uuid.UUID) -> SolverJob | None:
        job = session.get(SolverJob, job_id)
        if job is None or job.status != JobStatus.RUNNING or job.worker_id != self.worker_id:
            return None
        return job

    def _run(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str, token: Any) -> None:
        with self._sessions() as session:
            job = self._still_mine(session, job_id)
            if job is None:
                return
            if job.cancel_requested:
                self._service(session).mark_finished(job_id, JobStatus.CANCELLED)
                return

        def on_progress(p: dict[str, Any]) -> None:
            with self._sessions() as s:
                if self._still_mine(s, job_id) is not None:
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
            if self._stop.is_set():
                # Shutting down, not cancelled by anyone: leave it running for the next worker.
                return
            self._finish(job_id, JobStatus.CANCELLED)
            return
        with self._sessions() as session:
            if self._still_mine(session, job_id) is None:
                return  # requeued or finished elsewhere meanwhile; never store a second result
            if kind == "preflop":
                self._store_preflop_result(session, job_id, spec, outcome, save_path)
            else:
                self._store_result(session, job_id, spec, outcome, save_path)
            self._service(session).mark_finished(job_id, JobStatus.SUCCEEDED)

    def _finish(self, job_id: uuid.UUID, status: JobStatus, error: tuple[str, str] | None = None) -> None:
        with self._sessions() as session:
            if self._still_mine(session, job_id) is None:
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
                # Node-locked solves: the unlocked solve the locks were applied to.
                **({"unlocked": outcome["unlocked"]} if outcome.get("unlocked") else {}),
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
