from typing import Any, Literal

from pydantic import Field

from riverlab_api.schemas.common import StrictModel

ExplanationLevel = Literal["simple", "intermediate", "advanced", "math"]


class ProgressUpdate(StrictModel):
    last_section: str | None = Field(default=None, max_length=80)
    explanation_level: ExplanationLevel | None = None
    saved: bool | None = None
    completed: bool | None = None


class AttemptIn(StrictModel):
    # Option index, or a number for numeric questions; the engine-side grader checks the type.
    answer: Any
    confidence: int | None = Field(default=None, ge=1, le=5)
    time_ms: int | None = Field(default=None, ge=0, le=3_600_000)


class BookmarkIn(StrictModel):
    lesson: str = Field(max_length=80)
    section: str = Field(max_length=80)
    label: str = Field(min_length=1, max_length=200)
    note: str | None = Field(default=None, max_length=2000)


class TheoryIn(StrictModel):
    params: dict[str, Any]
