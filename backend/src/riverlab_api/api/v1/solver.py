import uuid
from typing import Annotated

from fastapi import APIRouter, Depends, Query, Response
from sqlalchemy.orm import Session

from riverlab_api.db.models import SolverJob
from riverlab_api.db.session import get_db
from riverlab_api.schemas.solver import (
    ClassStrategy,
    IndependentCheck,
    NodeViewOut,
    RootStrategy,
    SolverBackendInfo,
    SolverCapabilities,
    SolverJobCreate,
    SolverJobOut,
    SolverResultOut,
)
from riverlab_api.services import compute, engine, trees

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
        status="available" if backends else "unavailable",
        message=(
            "Heads-up postflop solving with GTOpen (pinned, audited). Every result's exploitability is "
            "re-checked by RiverLab's independent validator whenever it fits in the memory limit."
            if backends
            else "No solver backend is enabled. Job submission is validated and then rejected (503)."
        ),
        available_backends=backends,
        engines=[SolverBackendInfo(**b) for b in engine.solver_backends()] if backends else [],
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


@router.get("/jobs/{job_id}/result", response_model=SolverResultOut)
def get_result(job_id: uuid.UUID, db: DbSession) -> SolverResultOut:
    job, result, root = compute.JobService(db).result(job_id)
    summary = result.summary
    return SolverResultOut(
        job_id=job.id,
        engine_version=result.engine_version,
        solver_backend=result.solver_backend,
        solver_version=result.solver_version,
        algorithm=result.assumptions.get("algorithm", ""),
        iterations=result.iterations,
        stop_reason=summary["stop_reason"],
        exploitability_mbb=result.exploitability or 0.0,
        exploitability_pct_pot=summary["exploitability_pct_pot"],
        values_mbb=summary["values_mbb"],
        elapsed_ms=summary["elapsed_ms"],
        memory_bytes=summary["memory_bytes"],
        approximations=result.assumptions.get("approximations", []),
        independent_check=IndependentCheck(**summary["independent_check"]),
        root=RootStrategy(
            player=root.player,
            street=root.street,
            board=root.board,
            actions=root.actions,
            by_class=[ClassStrategy(**c) for c in root.strategy["by_class"]],
        )
        if root
        else None,
        tree_available=result.storage_uri is not None and trees.path_from_uri(result.storage_uri).exists(),
    )


@router.get("/jobs/{job_id}/node", response_model=NodeViewOut)
def get_node(
    job_id: uuid.UUID, db: DbSession, path: Annotated[str, Query(max_length=400)] = ""
) -> NodeViewOut:
    """A node of the saved tree. `path`: dot-separated steps, `a<action index>` or `c<card>`."""
    _, result, _ = compute.JobService(db).result(job_id)
    return NodeViewOut(**trees.node(result, path), path=path)


@router.delete("/jobs/{job_id}", status_code=204)
def delete_job(job_id: uuid.UUID, db: DbSession) -> Response:
    compute.JobService(db).delete(job_id)
    return Response(status_code=204)
