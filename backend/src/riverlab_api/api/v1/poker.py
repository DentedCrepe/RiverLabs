from typing import Annotated, Any

from fastapi import APIRouter, Depends

from riverlab_api.config import Settings, get_settings
from riverlab_api.errors import EngineValidationError
from riverlab_api.schemas.poker import (
    CompareRequest,
    CompareResponse,
    EquityRequest,
    EquityResponse,
    EvaluateRequest,
    EvaluateResponse,
    RangeRequest,
    RangeResponse,
    ReplayRequest,
    ReplayResponse,
)
from riverlab_api.services import engine

router = APIRouter(tags=["poker"])


@router.post("/hands/evaluate", response_model=EvaluateResponse)
def evaluate(body: EvaluateRequest) -> dict[str, Any]:
    return engine.evaluate_hand(body.cards)


@router.post("/hands/compare", response_model=CompareResponse)
def compare(body: CompareRequest) -> dict[str, Any]:
    """Showdown on a complete board. Card conflicts between players are rejected by the engine."""
    return engine.compare_hands(body.board, body.hands)


@router.post("/ranges/analyze", response_model=RangeResponse)
def analyze_range(body: RangeRequest) -> dict[str, Any]:
    return engine.analyze_range(body.notation, body.dead)


@router.post("/equity", response_model=EquityResponse)
def equity(body: EquityRequest, settings: Annotated[Settings, Depends(get_settings)]) -> dict[str, Any]:
    if body.method == "monte_carlo" and body.iterations > settings.equity_max_iterations:
        raise EngineValidationError(f"iterations must be at most {settings.equity_max_iterations}")
    return engine.equity(
        body.ranges,
        board=body.board,
        dead=body.dead,
        method=body.method,
        iterations=body.iterations,
        seed=body.seed,
        exact_budget=settings.equity_exact_budget,
    )


@router.post("/hands/replay", response_model=ReplayResponse)
def replay(body: ReplayRequest) -> dict[str, Any]:
    """Replays a hand event by event through the betting engine (rules: docs/poker-rules.md)."""
    config = body.config.model_dump() if body.config else engine.default_game_config()
    return engine.replay_hand(config, body.events, body.stacks_mbb)
