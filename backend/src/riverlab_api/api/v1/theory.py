"""Closed-form poker theory for Academy demonstrations (engine `poker-theory`)."""

from typing import Any, Literal

from fastapi import APIRouter

from riverlab_api.schemas.academy import TheoryIn
from riverlab_api.services import engine

router = APIRouter(prefix="/theory", tags=["theory"])

Kind = Literal[
    "pot_odds",
    "bet_math",
    "ev_call",
    "ev_bluff",
    "outs",
    "regret_matching",
    "implied_odds",
    "semi_bluff",
    "zero_sum_2x2",
    "icm",
    "icm_call",
]


@router.post("/{kind}")
def theory(kind: Kind, body: TheoryIn) -> Any:
    """Each result is exact for the model stated in the engine docs, not a full-poker equilibrium."""
    return engine.theory(kind, body.params)
