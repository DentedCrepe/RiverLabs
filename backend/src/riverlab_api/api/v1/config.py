import uuid
from typing import Annotated, Any

from fastapi import APIRouter, Depends, Query, Response
from sqlalchemy.orm import Session

from riverlab_api.db.models import GameConfig
from riverlab_api.db.session import get_db
from riverlab_api.schemas.config import (
    GameConfigCreate,
    GameConfigDoc,
    GameConfigOut,
    PositionOut,
    RakeRequest,
    RakeResult,
    StackPresets,
)
from riverlab_api.services import engine, game_configs

router = APIRouter(tags=["configuration"])
DbSession = Annotated[Session, Depends(get_db)]


def _out(row: GameConfig) -> GameConfigOut:
    return GameConfigOut(
        id=row.id,
        name=row.name,
        builtin=row.is_builtin,
        stack_preset=row.stack_config.preset,
        config=GameConfigDoc.model_validate(row.config),
    )


@router.get("/positions", response_model=list[PositionOut])
def positions(players: Annotated[int, Query(ge=2, le=6)] = 6) -> list[dict[str, Any]]:
    return engine.positions(players)


@router.get("/stack-presets", response_model=StackPresets)
def stack_presets() -> dict[str, Any]:
    return engine.stack_presets()


@router.get("/game-configs/default", response_model=GameConfigDoc)
def default_game_config() -> dict[str, Any]:
    return engine.default_game_config()


@router.post("/game-configs/validate", response_model=GameConfigDoc)
def validate_game_config(body: GameConfigDoc) -> dict[str, Any]:
    """Validates a configuration with the engine and returns its canonical form."""
    return engine.validate_game_config(body.model_dump())


@router.get("/game-configs", response_model=list[GameConfigOut])
def list_game_configs(db: DbSession) -> list[GameConfigOut]:
    return [_out(r) for r in game_configs.list_game_configs(db)]


@router.post("/game-configs", response_model=GameConfigOut, status_code=201)
def create_game_config(body: GameConfigCreate, db: DbSession) -> GameConfigOut:
    return _out(game_configs.create_game_config(db, body.name, body.config.model_dump()))


@router.get("/game-configs/{config_id}", response_model=GameConfigOut)
def get_game_config(config_id: uuid.UUID, db: DbSession) -> GameConfigOut:
    return _out(game_configs.get_game_config(db, config_id))


@router.delete("/game-configs/{config_id}", status_code=204)
def delete_game_config(config_id: uuid.UUID, db: DbSession) -> Response:
    game_configs.delete_game_config(db, config_id)
    return Response(status_code=204)


@router.post("/rake/calculate", response_model=RakeResult)
def calculate_rake(body: RakeRequest) -> dict[str, Any]:
    config = body.config.model_dump() if body.config else engine.default_game_config()
    return engine.compute_rake(config, body.pot_mbb, body.saw_flop)
