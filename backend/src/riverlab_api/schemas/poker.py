from typing import Any, Literal

from pydantic import Field

from riverlab_api.schemas.common import StrictModel
from riverlab_api.schemas.config import GameConfigDoc

CardString = Field(max_length=32, description="Concatenated cards, e.g. 'AsKd7h'", examples=["AsKdQhJcTs"])


class EvaluateRequest(StrictModel):
    cards: str = CardString


class EvaluateResponse(StrictModel):
    value: int
    category: str
    category_name: str
    description: str
    best_five: list[str]


class CompareRequest(StrictModel):
    board: str = Field(max_length=10, description="Five community cards")
    hands: list[str] = Field(min_length=2, max_length=10, description="Hole cards per player, e.g. 'AsKd'")


class CompareResponse(StrictModel):
    results: list[EvaluateResponse]
    winners: list[int]


class RangeRequest(StrictModel):
    notation: str = Field(max_length=4000, examples=["TT+,AQs+,AKo,76s:0.5"])
    dead: str = Field(default="", max_length=14)


class MatrixCell(StrictModel):
    label: str
    row: int
    col: int
    kind: Literal["pair", "suited", "offsuit"]
    combos: int
    live_combos: int
    weight_sum: float


class RangeResponse(StrictModel):
    notation: str
    combos: float
    live_combos: float
    fraction_of_all_hands: float
    blocked_fraction: float
    matrix: list[MatrixCell]


class EquityRequest(StrictModel):
    ranges: list[str] = Field(
        min_length=2, max_length=6, description="Range notation or exact hands (e.g. 'AsKd')"
    )
    board: str = Field(default="", max_length=10)
    dead: str = Field(default="", max_length=20)
    method: Literal["exact", "monte_carlo"] = "exact"
    iterations: int = Field(default=200_000, ge=1)
    seed: int = Field(default=0, ge=0, le=2**63 - 1)


class EquityResponse(StrictModel):
    method: Literal["exact", "monte_carlo"]
    equity: list[float]
    win: list[float]
    tie: list[float]
    samples: int
    std_error: list[float] | None
    seed: int | None


class ReplayRequest(StrictModel):
    config: GameConfigDoc | None = None
    stacks_mbb: list[int] | None = Field(default=None, min_length=3, max_length=6)
    events: list[dict[str, Any]] = Field(max_length=500)


class ReplayResponse(StrictModel):
    phase: dict[str, Any]
    street: str
    board: list[str]
    pot_mbb: int
    current_bet_mbb: int
    min_raise_increment_mbb: int
    players: list[dict[str, Any]]
    pots: list[dict[str, Any]]
    legal_actions: dict[str, Any] | None
    history: list[dict[str, Any]]
    settlement: dict[str, Any] | None
