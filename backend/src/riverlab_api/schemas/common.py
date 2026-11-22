from pydantic import BaseModel, ConfigDict


class StrictModel(BaseModel):
    """Request/response base: unknown fields are rejected, never ignored."""

    model_config = ConfigDict(extra="forbid")
