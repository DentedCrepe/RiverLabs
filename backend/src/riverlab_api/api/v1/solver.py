import uuid
from typing import Annotated

from fastapi import APIRouter, Depends, Query, Response
from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import SolverJob, SolverResult
from riverlab_api.db.session import get_db
from riverlab_api.errors import NotFoundError
from riverlab_api.schemas.solver import (
    ClassStrategy,
    IndependentCheck,
    LibraryEntry,
    NodeViewOut,
    PreflopNodeOut,
    PreflopResultOut,
    RootStrategy,
    SolverBackendInfo,
    SolverCapabilities,
    SolverJobCreate,
    SolverJobOut,
    SolverResultOut,
)
from riverlab_api.services import compute, engine, hand_review, trees

router = APIRouter(prefix="/solver", tags=["solver"])
DbSession = Annotated[Session, Depends(get_db)]


def _out(job: SolverJob) -> SolverJobOut:
    return SolverJobOut(
        id=job.id,
        kind=job.kind,
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
            "re-checked by RiverLab's independent validator whenever it fits in the memory limit. "
            "Preflop solving is an approximate model (flop pots are modelled, not solved); only "
            "heads-up push/fold is validated."
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


@router.post("/preflop/jobs", response_model=SolverJobOut, status_code=202)
def submit_preflop_job(body: SolverJobCreate, db: DbSession) -> SolverJobOut:
    """Queues a preflop solve. Results are an approximate model (flop pots are not solved)."""
    return _out(compute.JobService(db).submit(body.spec, kind="preflop"))


@router.get("/jobs", response_model=list[SolverJobOut])
def list_jobs(
    db: DbSession,
    limit: Annotated[int, Query(ge=1, le=200)] = 50,
    kind: Annotated[str | None, Query(pattern="^(postflop|preflop)$")] = None,
) -> list[SolverJobOut]:
    return [_out(j) for j in compute.JobService(db).list(limit, kind)]


@router.get("/library", response_model=list[LibraryEntry])
def library(
    db: DbSession,
    limit: Annotated[int, Query(ge=1, le=500)] = 100,
    kind: Annotated[str | None, Query(pattern="^(postflop|preflop)$")] = None,
) -> list[LibraryEntry]:
    """Every saved result, newest first, with its configuration and quality measures."""
    q = (
        select(SolverJob, SolverResult)
        .join(SolverResult, SolverResult.job_id == SolverJob.id)
        .order_by(SolverJob.finished_at.desc().nulls_last(), SolverJob.id)
        .limit(limit)
    )
    if kind is not None:
        q = q.where(SolverJob.kind == kind)
    hands = hand_review.hand_of_jobs(db)
    out = []
    for job, result in db.execute(q).all():
        summary, spec = result.summary, job.spec
        entry = LibraryEntry(
            job_id=job.id,
            kind=job.kind,
            finished_at=job.finished_at,
            engine_version=result.engine_version,
            solver_backend=result.solver_backend,
            solver_version=result.solver_version,
            iterations=result.iterations,
            stop_reason=summary.get("stop_reason"),
            tree_available=_tree_available(result),
            hand=hands.get(str(job.id)),
        )
        if job.kind == "postflop":
            entry.board = spec.get("board")
            entry.ranges = spec.get("ranges")
            entry.starting_pot_mbb = spec.get("starting_pot_mbb")
            entry.effective_stack_mbb = spec.get("effective_stack_mbb")
            entry.exploitability_mbb = result.exploitability
            entry.exploitability_pct_pot = summary.get("exploitability_pct_pot")
            entry.independent_check = bool(summary.get("independent_check", {}).get("performed"))
            entry.approximations = len(result.assumptions.get("approximations", []))
        else:
            entry.players = spec.get("game", {}).get("players")
            entry.positions = summary.get("positions")
            entry.validated_scope = result.assumptions.get("validated_scope")
            entry.model_gap_mbb = summary.get("model_gap_mbb")
            entry.effective_stack_mbb = spec.get("game", {}).get("stack", {}).get("effective_stack")
        out.append(entry)
    return out


@router.get("/jobs/{job_id}", response_model=SolverJobOut)
def get_job(job_id: uuid.UUID, db: DbSession) -> SolverJobOut:
    return _out(compute.JobService(db).get(job_id))


@router.post("/jobs/{job_id}/cancel", response_model=SolverJobOut)
def cancel_job(job_id: uuid.UUID, db: DbSession) -> SolverJobOut:
    return _out(compute.JobService(db).cancel(job_id))


@router.get("/jobs/{job_id}/result", response_model=SolverResultOut)
def get_result(job_id: uuid.UUID, db: DbSession) -> SolverResultOut:
    job, result, root = compute.JobService(db).result(job_id)
    _require_kind(job, "postflop")
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
        tree_available=_tree_available(result),
        locks=result.assumptions["spec"].get("locks", []),
        unlocked=summary.get("unlocked"),
        spec=result.assumptions["spec"],
    )


@router.get("/jobs/{job_id}/node", response_model=NodeViewOut)
def get_node(
    job_id: uuid.UUID, db: DbSession, path: Annotated[str, Query(max_length=400)] = ""
) -> NodeViewOut:
    """A node of the saved tree. `path`: dot-separated steps, `a<action index>` or `c<card>`."""
    job, result, _ = compute.JobService(db).result(job_id)
    _require_kind(job, "postflop")
    return NodeViewOut(**trees.node(result, path), path=path)


def _require_kind(job: SolverJob, kind: str) -> None:
    if job.kind != kind:
        other = "/preflop" if job.kind == "preflop" else "/result"
        raise NotFoundError(f"Job {job.id} is a {job.kind} solve; read it at /solver/jobs/{job.id}{other}")


def _tree_available(result: SolverResult) -> bool:
    return result.storage_uri is not None and trees.path_from_uri(result.storage_uri).exists()


@router.get("/jobs/{job_id}/preflop", response_model=PreflopResultOut)
def get_preflop_result(job_id: uuid.UUID, db: DbSession) -> PreflopResultOut:
    job, result, _ = compute.JobService(db).result(job_id)
    _require_kind(job, "preflop")
    summary, assumptions = result.summary, result.assumptions
    return PreflopResultOut(
        job_id=job.id,
        engine_version=result.engine_version,
        solver_backend=result.solver_backend,
        solver_version=result.solver_version,
        model=assumptions["model"],
        validated_scope=assumptions["validated_scope"],
        model_notes=assumptions["model_notes"],
        iterations=result.iterations,
        stop_reason=summary["stop_reason"],
        positions=summary["positions"],
        model_gap_mbb=summary["model_gap_mbb"],
        gaps_mbb=summary["gaps_mbb"],
        values_mbb=summary["values_mbb"],
        elapsed_ms=summary["elapsed_ms"],
        memory_bytes=summary["memory_bytes"],
        nodes=summary["nodes"],
        root=PreflopNodeOut(**summary["root"]),
        tree_available=_tree_available(result),
    )


@router.get("/jobs/{job_id}/preflop/node", response_model=PreflopNodeOut)
def get_preflop_node(
    job_id: uuid.UUID, db: DbSession, path: Annotated[str, Query(max_length=200)] = ""
) -> PreflopNodeOut:
    """A node of a saved preflop tree. `path`: dot-separated action indices from the root."""
    job, result, _ = compute.JobService(db).result(job_id)
    _require_kind(job, "preflop")
    return PreflopNodeOut(**trees.preflop_node(result, path), path=path)


@router.delete("/jobs/{job_id}", status_code=204)
def delete_job(job_id: uuid.UUID, db: DbSession) -> Response:
    compute.JobService(db).delete(job_id)
    return Response(status_code=204)
