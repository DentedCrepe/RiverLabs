import pytest
from alembic.autogenerate import compare_metadata
from alembic.migration import MigrationContext
from sqlalchemy import Engine, select
from sqlalchemy.orm import Session

import riverlab_api.db.models  # noqa: F401
from riverlab_api.db.base import Base
from riverlab_api.db.models import GameConfig, Position, StackConfig
from riverlab_api.services import engine

pytestmark = pytest.mark.db


def test_models_match_migrations(db_engine: Engine) -> None:
    with db_engine.connect() as conn:
        diff = compare_metadata(MigrationContext.configure(conn), Base.metadata)
    assert diff == [], f"models and migrations disagree: {diff}"


def test_seeded_positions_match_engine(db: Session) -> None:
    rows = db.scalars(select(Position).order_by(Position.preflop_order)).all()
    assert [
        {
            "code": r.code,
            "name": r.name,
            "preflop_order": r.preflop_order,
            "postflop_order": r.postflop_order,
            "is_blind": r.is_blind,
        }
        for r in rows
    ] == engine.positions(6)


def test_seeded_default_config_is_the_engine_default(db: Session) -> None:
    row = db.scalars(select(GameConfig).where(GameConfig.name == "6-max 100bb (default)")).one()
    assert row.config == engine.default_game_config()
    assert row.rake_config.config == engine.default_game_config()["rake"]
    presets = sorted(
        db.scalars(select(StackConfig.effective_stack_mbb).where(StackConfig.preset.is_not(None)))
    )
    assert presets == [p["mbb"] for p in engine.stack_presets()["presets"]]


def test_destructive_downgrade_requires_confirmation(db_engine: Engine) -> None:
    from pathlib import Path

    from alembic import command
    from alembic.config import Config

    cfg = Config(str(Path(__file__).resolve().parents[2] / "alembic.ini"))
    cfg.cmd_opts = type(
        "Opts", (), {"x": [f"db_url={db_engine.url.render_as_string(hide_password=False)}"]}
    )()  # type: ignore[assignment]
    with pytest.raises(RuntimeError, match="Refusing to drop"):
        command.downgrade(cfg, "base")
