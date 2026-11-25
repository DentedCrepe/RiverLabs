from typing import Any, Literal

from pydantic import Field, model_validator

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


class ReviewIn(StrictModel):
    verdict: Literal["reviewed", "changes_requested"]
    reviewer: str = Field(min_length=1, max_length=100)
    notes: str = Field(default="", max_length=5000)

    @model_validator(mode="after")
    def _notes_for_changes(self) -> "ReviewIn":
        self.reviewer = self.reviewer.strip()
        if not self.reviewer:
            raise ValueError("reviewer must name the person who reviewed the lesson")
        if self.verdict == "changes_requested" and not self.notes.strip():
            raise ValueError("say what should change in notes")
        return self


class TutorTurn(StrictModel):
    role: Literal["learner", "tutor"]
    text: str = Field(max_length=20_000)


class TutorIn(StrictModel):
    """One question to the AI tutor about a lesson. `consent`: the learner agreed to send the
    lesson, their Academy progress and the conversation to Anthropic."""

    level: Literal["simple", "intermediate", "advanced", "math"]
    question: str = Field(min_length=1, max_length=4_000)
    history: list[TutorTurn] = Field(default_factory=list, max_length=40)
    consent: bool
