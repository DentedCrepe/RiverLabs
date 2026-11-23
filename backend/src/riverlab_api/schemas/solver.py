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
    kind: str
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
    tree_available: bool


class NodeViewOut(StrictModel):
    kind: str
    street: str
    board: list[str]
    put_mbb: list[float]
    player: int | None
    actions: list[str]
    cards: list[str]
    by_class: list[ClassStrategy]
    reach_fraction: list[float]
    path: str


class PreflopAction(StrictModel):
    label: str
    kind: str
    to_mbb: float
    frequency: float


class PreflopStep(StrictModel):
    actor: str
    actions: list[str]
    chosen: int | None


class SeatRange(StrictModel):
    position: str
    classes: list[ClassStrategy]


class PreflopNodeOut(StrictModel):
    kind: str
    actor: str | None
    positions: list[str]
    pot_mbb: float
    invested_mbb: list[float]
    live: list[bool]
    actions: list[PreflopAction]
    by_class: list[ClassStrategy]
    ranges: list[SeatRange]
    history: list[PreflopStep]
    strategy_note: str | None
    reach_probability: float
    stack_to_pot: float | None
    path: str = ""


class PreflopResultOut(StrictModel):
    job_id: UUID
    engine_version: str
    solver_backend: str
    solver_version: str
    model: str
    validated_scope: str
    model_notes: list[str]
    iterations: int
    stop_reason: str
    positions: list[str]
    model_gap_mbb: float
    gaps_mbb: list[float]
    values_mbb: list[float]
    elapsed_ms: int
    memory_bytes: int
    nodes: int
    root: PreflopNodeOut
    tree_available: bool


class LibraryEntry(StrictModel):
    """One saved result, with what defines it and how good it is."""

    job_id: UUID
    kind: str
    finished_at: datetime | None
    engine_version: str
    solver_backend: str
    solver_version: str
    iterations: int
    stop_reason: str | None
    tree_available: bool
    # Postflop
    board: list[str] | None = None
    ranges: list[str] | None = None
    starting_pot_mbb: int | None = None
    effective_stack_mbb: int | None = None
    exploitability_mbb: float | None = None
    exploitability_pct_pot: float | None = None
    independent_check: bool | None = None
    approximations: int = 0
    # Preflop (an approximate model unless validated_scope says otherwise)
    players: int | None = None
    positions: list[str] | None = None
    validated_scope: str | None = None
    model_gap_mbb: float | None = None
