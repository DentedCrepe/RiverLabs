import uuid
from typing import Annotated

from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session

from riverlab_api.db.models import SolverJob
from riverlab_api.db.session import get_db
from riverlab_api.schemas.solver import SolverCapabilities, SolverJobCreate, SolverJobOut
from riverlab_api.services import compute

router = APIRouter(prefix="/solver", tags=["solver"])
DbSession = Annotated[Session, Depends(get_db)]


def _out(job: SolverJob) -> SolverJobOut:
    return SolverJobOut(
        id=job.id,
        status=job.status.value,
        backend=job.backend,
        iterations=job.iterations,
        progress=job.progress,
        cancel_requested=job.cancel_requested,
        error_code=job.error_code,
        error_message=job.error_message,
        created_at=job.created_at,
        started_at=job.started_at,
        finished_at=job.finished_at,
    )


@router.get("/capabilities", response_model=SolverCapabilities)
def capabilities() -> SolverCapabilities:
    backends = [b.name for b in compute.registry.backends]
    return SolverCapabilities(
        status="available" if backends else "not_implemented",
        message=(
            "Solver backends are available."
            if backends
            else "No solver is implemented yet. Job submission is validated and then rejected (503)."
        ),
        available_backends=backends,
        planned_algorithms=["cfr", "cfr_plus", "discounted_cfr", "monte_carlo_cfr"],
    )


@router.post("/jobs", response_model=SolverJobOut, status_code=202)
def submit_job(body: SolverJobCreate, db: DbSession) -> SolverJobOut:
    return _out(compute.JobService(db).submit(body.spec))


@router.get("/jobs", response_model=list[SolverJobOut])
def list_jobs(db: DbSession, limit: Annotated[int, Query(ge=1, le=200)] = 50) -> list[SolverJobOut]:
    return [_out(j) for j in compute.JobService(db).list(limit)]


@router.get("/jobs/{job_id}", response_model=SolverJobOut)
def get_job(job_id: uuid.UUID, db: DbSession) -> SolverJobOut:
    return _out(compute.JobService(db).get(job_id))


@router.post("/jobs/{job_id}/cancel", response_model=SolverJobOut)
def cancel_job(job_id: uuid.UUID, db: DbSession) -> SolverJobOut:
    return _out(compute.JobService(db).cancel(job_id))
