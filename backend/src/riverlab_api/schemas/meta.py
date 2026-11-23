from typing import Literal

from riverlab_api.schemas.common import StrictModel

CapabilityStatus = Literal["available", "approximate", "interface_only", "planned"]


class Capability(StrictModel):
    key: str
    name: str
    status: CapabilityStatus
    detail: str


class MetaResponse(StrictModel):
    api_version: str
    engine_version: str
    capabilities: list[Capability]


class HealthResponse(StrictModel):
    status: Literal["ok"]
    version: str


class ReadinessResponse(StrictModel):
    status: Literal["ok", "degraded"]
    database: Literal["ok", "unavailable"]
    engine: str
