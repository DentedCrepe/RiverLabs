from datetime import datetime
from typing import Any
from uuid import UUID

from riverlab_api.schemas.common import StrictModel


class PlaySessionCreate(StrictModel):
    # play_engine::TableConfig; the engine validates its contents.
    config: dict[str, Any]


class PlayAction(StrictModel):
    # poker_core::game::Action, e.g. {"type": "raise", "to": 2500}.
    action: dict[str, Any]


class RevealIn(StrictModel):
    on: bool


class PlaySessionOut(StrictModel):
    id: UUID
    format: str
    status: str
    error_message: str | None
    hands: int
    hero_net: int
    created_at: datetime
    finished_at: datetime | None
    config: dict[str, Any]
    # False for a guest game, which never counts towards your statistics.
    tracked: bool


class PlayState(StrictModel):
    session: PlaySessionOut
    # play_engine::view::TableView, or null when the game is no longer in memory.
    table: dict[str, Any] | None
