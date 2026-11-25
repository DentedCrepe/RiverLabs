"""Unified computation API (architecture §8).

Callers submit work to `JobService`; a `ComputeBackend` (local worker, cloud queue, ...)
executes it. The frontend never learns which backend ran a job.

The app registers the `local` GTOpen backend at startup when `RIVERLAB_SOLVER_ENABLED` is true
(see `services/local_solver.py`). With no backend registered, submitting a solve raises
`SolverUnavailableError` (HTTP 503) instead of creating a job that would never run.
"""

import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from typing import Any, Protocol

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import JobStatus, SolverJob, SolverResult, StrategyNode
from riverlab_api.errors import ConflictError, NotFoundError, SolverUnavailableError
from riverlab_api.services import engine

# Legal status transitions. Terminal states have no outgoing transitions.
TRANSITIONS: dict[JobStatus, frozenset[JobStatus]] = {
    JobStatus.QUEUED: frozenset({JobStatus.RUNNING, JobStatus.CANCELLED, JobStatus.FAILED}),
    # RUNNING -> QUEUED: the worker running it stopped (restart, crash); it will run again.
    JobStatus.RUNNING: frozenset(
        {JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED, JobStatus.QUEUED}
    ),
    JobStatus.SUCCEEDED: frozenset(),
    JobStatus.FAILED: frozenset(),
    JobStatus.CANCELLED: frozenset(),
}


class ComputeBackend(Protocol):
    """Executes solver jobs. Implementations: local in-process worker, cloud queue (planned)."""

    name: str

    def can_run(self, spec: dict[str, Any]) -> bool: ...
    def submit(self, job_id: uuid.UUID, spec: dict[str, Any], kind: str) -> None:
        """Called after the job is committed as queued (the queue is the database)."""
        ...

    def request_cancel(self, job_id: uuid.UUID) -> None:
        """A fast path: the database flag `cancel_requested` is what every worker obeys."""
        ...


@dataclass
class BackendRegistry:
    backends: list[ComputeBackend] = field(default_factory=list)

    def register(self, backend: ComputeBackend) -> None:
        self.backends.append(backend)

    def select(self, spec: dict[str, Any]) -> ComputeBackend | None:
        return next((b for b in self.backends if b.can_run(spec)), None)


# Populated at app startup (main.py).
registry = BackendRegistry()


class JobService:
    def __init__(self, db: Session, backends: BackendRegistry | None = None) -> None:
        self.db = db
        self.backends = backends if backends is not None else registry

    def submit(
        self, spec: dict[str, Any], owner_id: uuid.UUID | None = None, kind: str = "postflop"
    ) -> SolverJob:
        if kind == "preflop":
            canonical = engine.validate_preflop_spec(spec)
        elif kind == "postflop":
            canonical = engine.validate_solve_spec(spec)
        else:
            raise ValueError(f"unknown job kind {kind!r}")
        backend = self.backends.select(canonical)
        if backend is None:
            raise SolverUnavailableError(
                "No solver backend is available. RiverLab's solver is not implemented yet; "
                "the request was validated but not queued."
            )
        job = SolverJob(
            owner_id=owner_id, status=JobStatus.QUEUED, kind=kind, spec=canonical, backend=backend.name
        )
        self.db.add(job)
        self.db.commit()
        try:
            backend.submit(job.id, canonical, kind)
        except Exception as exc:
            self._transition(job, JobStatus.FAILED, error=("backend_submit_failed", str(exc)))
            raise
        return job

    def get(self, job_id: uuid.UUID) -> SolverJob:
        job = self.db.get(SolverJob, job_id)
        if job is None:
            raise NotFoundError(f"Solver job {job_id} not found")
        return job

    def result(self, job_id: uuid.UUID) -> tuple[SolverJob, SolverResult, StrategyNode | None]:
        job = self.get(job_id)
        result = self.db.scalar(select(SolverResult).where(SolverResult.job_id == job_id))
        if result is None:
            raise NotFoundError(f"Job {job_id} has no result (status: {job.status.value})")
        root = self.db.scalar(
            select(StrategyNode).where(StrategyNode.result_id == result.id, StrategyNode.node_path == "root")
        )
        return job, result, root

    def delete(self, job_id: uuid.UUID) -> None:
        """Deletes a finished job, its result and any saved tree file."""
        job = self.get(job_id)
        if not job.status.is_terminal:
            raise ConflictError("Cancel the job before deleting it")
        result = self.db.scalar(select(SolverResult).where(SolverResult.job_id == job_id))
        if result is not None and result.storage_uri:
            from riverlab_api.services.trees import forget, path_from_uri

            forget(result.id)
            path_from_uri(result.storage_uri).unlink(missing_ok=True)
        self.db.delete(job)
        self.db.commit()

    def list(self, limit: int = 50, kind: str | None = None) -> list[SolverJob]:
        query = select(SolverJob).order_by(SolverJob.created_at.desc(), SolverJob.id).limit(limit)
        if kind is not None:
            query = query.where(SolverJob.kind == kind)
        return list(self.db.scalars(query))

    def cancel(self, job_id: uuid.UUID) -> SolverJob:
        job = self.get(job_id)
        if job.status.is_terminal:
            raise ConflictError(f"Job is already {job.status.value}")
        if job.status == JobStatus.QUEUED:
            self._transition(job, JobStatus.CANCELLED)
        else:
            # Running jobs stop cooperatively; the backend reports the final transition.
            job.cancel_requested = True
            self.db.commit()
        for b in self.backends.backends:
            if b.name == job.backend:
                b.request_cancel(job.id)
        return job

    # ---- backend callbacks ----

    def mark_running(self, job_id: uuid.UUID) -> SolverJob:
        job = self.get(job_id)
        self._transition(job, JobStatus.RUNNING)
        return job

    def report_progress(self, job_id: uuid.UUID, iterations: int, progress: dict[str, Any]) -> SolverJob:
        job = self.get(job_id)
        if job.status != JobStatus.RUNNING:
            raise ConflictError(f"Cannot report progress for a {job.status.value} job")
        if iterations < job.iterations:
            raise ConflictError("Iteration count cannot decrease")
        job.iterations = iterations
        job.progress = progress
        self.db.commit()
        return job

    def mark_finished(
        self, job_id: uuid.UUID, status: JobStatus, error: tuple[str, str] | None = None
    ) -> SolverJob:
        if not status.is_terminal:
            raise ConflictError(f"{status.value} is not a terminal status")
        job = self.get(job_id)
        self._transition(job, status, error=error)
        return job

    def requeue(self, job: SolverJob) -> None:
        """Puts a running job whose worker stopped back in the queue, from the start."""
        if job.status != JobStatus.RUNNING:
            raise ConflictError(f"Only a running job can be requeued, not a {job.status.value} one")
        job.status = JobStatus.QUEUED
        job.started_at = None
        job.worker_id = None
        job.heartbeat_at = None
        job.iterations = 0
        job.progress = {}
        self.db.commit()

    def _transition(self, job: SolverJob, to: JobStatus, error: tuple[str, str] | None = None) -> None:
        if to not in TRANSITIONS[job.status]:
            raise ConflictError(f"Illegal job transition {job.status.value} -> {to.value}")
        if to == JobStatus.FAILED and error is None:
            raise ConflictError("A failed job must carry an error code")
        now = datetime.now(UTC)
        job.status = to
        if to == JobStatus.RUNNING:
            job.started_at = now
        if to.is_terminal:
            job.finished_at = now
        if error is not None:
            job.error_code, job.error_message = error
        self.db.commit()
