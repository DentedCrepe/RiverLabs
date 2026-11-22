from datetime import datetime
from typing import Any
from uuid import UUID

from riverlab_api.schemas.common import StrictModel


class SolverCapabilities(StrictModel):
    status: str
    message: str
    available_backends: list[str]
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
