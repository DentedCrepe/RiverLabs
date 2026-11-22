"""Game, stack and rake configuration (ADR-0004).

`config` columns hold the engine-validated JSON document; scalar columns duplicate the
fields used for filtering and are kept consistent by the service layer (and by CHECK
constraints where the database can enforce them).
"""

import uuid
from typing import Any

from sqlalchemy import CheckConstraint, ForeignKey, Index, Integer, String, UniqueConstraint
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, Timestamps, UUIDPk

STACK_PRESET_LABELS = ("20bb", "40bb", "50bb", "75bb", "100bb", "150bb", "200bb")


class Position(Base):
    """Reference data for six-max positions (seeded by migration 0001 from the engine)."""

    __tablename__ = "positions"

    code: Mapped[str] = mapped_column(String(3), primary_key=True)
    name: Mapped[str] = mapped_column(String(32), nullable=False)
    preflop_order: Mapped[int] = mapped_column(Integer, nullable=False, unique=True)
    postflop_order: Mapped[int] = mapped_column(Integer, nullable=False, unique=True)
    is_blind: Mapped[bool] = mapped_column(nullable=False)


class StackConfig(UUIDPk, Timestamps, Base):
    __tablename__ = "stack_configs"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    effective_stack_mbb: Mapped[int] = mapped_column(Integer, nullable=False)
    preset: Mapped[str | None] = mapped_column(String(8), nullable=True)
    schema_version: Mapped[int] = mapped_column(Integer, nullable=False)
    config: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (
        CheckConstraint("effective_stack_mbb BETWEEN 2000 AND 1000000", name="effective_stack_range"),
        CheckConstraint(
            "preset IS NULL OR preset IN (" + ",".join(f"'{p}'" for p in STACK_PRESET_LABELS) + ")",
            name="preset_label",
        ),
        UniqueConstraint("owner_id", "effective_stack_mbb", postgresql_nulls_not_distinct=True),
    )


class RakeConfig(UUIDPk, Timestamps, Base):
    __tablename__ = "rake_configs"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    percentage_bps: Mapped[int] = mapped_column(Integer, nullable=False)
    no_flop_no_drop: Mapped[bool] = mapped_column(nullable=False)
    schema_version: Mapped[int] = mapped_column(Integer, nullable=False)
    config: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (
        CheckConstraint("percentage_bps BETWEEN 0 AND 10000", name="percentage_range"),
        UniqueConstraint("owner_id", "name", postgresql_nulls_not_distinct=True),
    )


class GameConfig(UUIDPk, Timestamps, Base):
    __tablename__ = "game_configs"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    players: Mapped[int] = mapped_column(Integer, nullable=False)
    small_blind_mbb: Mapped[int] = mapped_column(Integer, nullable=False)
    ante_mbb: Mapped[int] = mapped_column(Integer, nullable=False)
    stack_config_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("stack_configs.id", ondelete="RESTRICT"), nullable=False
    )
    rake_config_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("rake_configs.id", ondelete="RESTRICT"), nullable=False
    )
    stake_currency: Mapped[str | None] = mapped_column(String(3), nullable=True)
    stake_big_blind_minor_units: Mapped[int | None] = mapped_column(Integer, nullable=True)
    schema_version: Mapped[int] = mapped_column(Integer, nullable=False)
    # Full resolved engine GameConfig document (stack + rake embedded).
    config: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    stack_config: Mapped[StackConfig] = relationship(lazy="joined")
    rake_config: Mapped[RakeConfig] = relationship(lazy="joined")

    __table_args__ = (
        CheckConstraint("players BETWEEN 3 AND 6", name="players_range"),
        CheckConstraint("small_blind_mbb BETWEEN 1 AND 1000", name="small_blind_range"),
        CheckConstraint("ante_mbb BETWEEN 0 AND 1000", name="ante_range"),
        CheckConstraint(
            "(stake_currency IS NULL) = (stake_big_blind_minor_units IS NULL)", name="stake_complete"
        ),
        UniqueConstraint("owner_id", "name", postgresql_nulls_not_distinct=True),
        Index("ix_game_configs_stack_rake", "stack_config_id", "rake_config_id"),
    )
