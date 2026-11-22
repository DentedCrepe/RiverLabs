"""Transport shapes mirroring the engine's serde documents. Pydantic validates the *shape*;
the engine validates the *semantics* (ADR-0001)."""

from typing import Annotated, Literal
from uuid import UUID

from pydantic import Field

from riverlab_api.schemas.common import StrictModel

# Upper bound keeps every amount far inside the engine's u64 arithmetic (10^12 mbb = 10^9 bb).
MAX_MBB = 10**12
Mbb = Annotated[int, Field(ge=0, le=MAX_MBB, description="Milli-big-blinds (1bb = 1000)")]


class RakeAmountBigBlinds(StrictModel):
    unit: Literal["big_blinds"]
    mbb: Mbb


class RakeAmountCurrency(StrictModel):
    unit: Literal["currency"]
    minor_units: int = Field(ge=0, le=MAX_MBB, description="Currency minor units, e.g. cents")


RakeAmount = Annotated[RakeAmountBigBlinds | RakeAmountCurrency, Field(discriminator="unit")]


class RakeRounding(StrictModel):
    increment: RakeAmount
    mode: Literal["down", "nearest", "up"]


class RakeConfigDoc(StrictModel):
    percentage_bps: int = Field(ge=0, le=10_000, description="500 = 5%")
    cap: RakeAmount | None
    min_pot: RakeAmount | None
    rounding: RakeRounding
    no_flop_no_drop: bool
    allocation: Literal["proportional", "main_pot_first"]


class StackConfigDoc(StrictModel):
    effective_stack: Mbb


class StakeDoc(StrictModel):
    currency: str = Field(min_length=3, max_length=3)
    big_blind_minor_units: int = Field(gt=0, le=MAX_MBB)


class GameConfigDoc(StrictModel):
    schema_version: int = 1
    players: int = Field(ge=3, le=6)
    small_blind: Mbb
    ante: Mbb
    stack: StackConfigDoc
    rake: RakeConfigDoc
    stake: StakeDoc | None


class StackPreset(StrictModel):
    label: str
    big_blinds: int
    mbb: int


class StackPresets(StrictModel):
    presets: list[StackPreset]
    default: str
    min_mbb: int
    max_mbb: int


class PositionOut(StrictModel):
    code: str
    name: str
    preflop_order: int
    postflop_order: int
    is_blind: bool


class GameConfigCreate(StrictModel):
    name: str = Field(min_length=1, max_length=100)
    config: GameConfigDoc


class GameConfigOut(StrictModel):
    id: UUID
    name: str
    builtin: bool
    stack_preset: str | None
    config: GameConfigDoc


class RakeRequest(StrictModel):
    config: GameConfigDoc | None = Field(default=None, description="Defaults to the engine default config")
    pot_mbb: Mbb
    saw_flop: bool


class RakeResult(StrictModel):
    rake: int
    capped: bool
    reason: Literal["no_flop_no_drop", "below_minimum_pot", "zero_rake", "percentage", "capped"]
