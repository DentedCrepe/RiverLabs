import uuid
from datetime import date, datetime
from typing import Any

from sqlalchemy import (
    CheckConstraint,
    Date,
    DateTime,
    Float,
    ForeignKey,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column

from riverlab_api.db.base import Base, CreatedAt, UUIDPk


class HandHistory(UUIDPk, CreatedAt, Base):
    """Imported hand histories. Parsing is planned; `parsed` stays NULL until a validated
    parser exists."""

    __tablename__ = "hand_histories"

    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    source: Mapped[str] = mapped_column(String(32), nullable=False)
    external_hand_id: Mapped[str] = mapped_column(String(64), nullable=False)
    played_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    raw_text: Mapped[str] = mapped_column(Text, nullable=False)
    content_sha256: Mapped[str] = mapped_column(String(64), nullable=False)
    parsed: Mapped[dict[str, Any] | None] = mapped_column(JSONB, nullable=True)
    game_config_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("game_configs.id", ondelete="SET NULL"), nullable=True
    )

    __table_args__ = (
        UniqueConstraint("user_id", "source", "external_hand_id"),
        Index("ix_hand_histories_user_played", "user_id", "played_at"),
    )


class PerformanceStat(UUIDPk, CreatedAt, Base):
    """Aggregated metrics computed from a user's own hands/decisions — never fabricated."""

    __tablename__ = "performance_stats"

    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    metric: Mapped[str] = mapped_column(String(64), nullable=False)
    # Canonical scope key, e.g. "position=BTN;stack=100bb" ("" for all hands).
    scope: Mapped[str] = mapped_column(String(128), nullable=False, default="")
    period_start: Mapped[date] = mapped_column(Date, nullable=False)
    period_end: Mapped[date] = mapped_column(Date, nullable=False)
    value: Mapped[float] = mapped_column(Float, nullable=False)
    sample_size: Mapped[int] = mapped_column(Integer, nullable=False)

    __table_args__ = (
        UniqueConstraint("user_id", "metric", "scope", "period_start", "period_end"),
        CheckConstraint("period_end >= period_start", name="period_order"),
        CheckConstraint("sample_size >= 0", name="sample_size_non_negative"),
    )
