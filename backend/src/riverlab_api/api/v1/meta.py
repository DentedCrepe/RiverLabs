from fastapi import APIRouter

from riverlab_api import __version__
from riverlab_api.schemas.meta import Capability, MetaResponse
from riverlab_api.services import engine

router = APIRouter(tags=["meta"])

CAPABILITIES = [
    Capability(
        key="hand_evaluator",
        name="Seven-card hand evaluator",
        status="available",
        detail="Exhaustively validated on all 133,784,560 seven-card hands.",
    ),
    Capability(
        key="ranges",
        name="Range engine",
        status="available",
        detail="169 hand classes, 1,326 combos, weighted ranges, notation, card removal.",
    ),
    Capability(
        key="equity",
        name="Equity calculator",
        status="available",
        detail="Exact enumeration within a work budget; seeded Monte Carlo with standard error.",
    ),
    Capability(
        key="game_rules",
        name="No-limit betting engine",
        status="available",
        detail="Betting rules, side pots, uncalled bets, rake and settlement.",
    ),
    Capability(
        key="solver",
        name="CFR solver",
        status="interface_only",
        detail="Interfaces and configuration exist. No solver is implemented; no GTO results exist.",
    ),
    Capability(key="trainer", name="Trainer", status="planned", detail="Requires validated solver results."),
    Capability(key="hand_analysis", name="Hand analysis", status="planned", detail="Hand-history parsing."),
    Capability(key="analytics", name="Analytics", status="planned", detail="Built from your own hands."),
]


@router.get("/meta", response_model=MetaResponse)
def meta() -> MetaResponse:
    return MetaResponse(
        api_version=__version__, engine_version=engine.engine_version(), capabilities=CAPABILITIES
    )
