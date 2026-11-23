from datetime import datetime
from typing import Any
from uuid import UUID

from riverlab_api.schemas.common import StrictModel


class SolverBackendInfo(StrictModel):
    name: str
    repository: str
    commit: str
    scope: str


class SolverCapabilities(StrictModel):
    status: str
    message: str
    available_backends: list[str]
    engines: list[SolverBackendInfo]
    planned_algorithms: list[str]


class SolverJobCreate(StrictModel):
    spec: dict[str, Any]


class SolverJobOut(StrictModel):
    id: UUID
    status: str
    backend: str
    iterations: int
    progress: dict[str, Any]
    cancel_requested: bool
    error_code: str | None
    error_message: str | None
    created_at: datetime
    started_at: datetime | None
    finished_at: datetime | None


class IndependentCheck(StrictModel):
    performed: bool
    values_mbb: list[float] | None
    exploitability_mbb: float | None
    note: str


class ClassStrategy(StrictModel):
    label: str
    row: int
    col: int
    weight: float
    frequencies: list[float]


class RootStrategy(StrictModel):
    player: int
    street: str
    board: str
    actions: list[str]
    by_class: list[ClassStrategy]


class SolverResultOut(StrictModel):
    job_id: UUID
    engine_version: str
    solver_backend: str
    solver_version: str
    algorithm: str
    iterations: int
    stop_reason: str
    exploitability_mbb: float
    exploitability_pct_pot: float
    values_mbb: list[float]
    elapsed_ms: int
    memory_bytes: int
    approximations: list[str]
    independent_check: IndependentCheck
    root: RootStrategy | None
