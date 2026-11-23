"""Games against bots: sessions and their hand records (engine `play_engine::HandRecord`)."""

import uuid
from datetime import datetime
from typing import Any

from sqlalchemy import BigInteger, CheckConstraint, DateTime, ForeignKey, Integer, String, UniqueConstraint
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, CreatedAt, Timestamps, UUIDPk


class PlaySession(UUIDPk, Timestamps, Base):
    __tablename__ = "play_sessions"

    # "cash", "fast_fold" or "sit_and_go" (copied from the config for listing).
    format: Mapped[str] = mapped_column(String(16), nullable=False)
    # Engine-validated TableConfig.
    config: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    # preparing -> active -> finished; "interrupted" when the server restarted mid-session.
    status: Mapped[str] = mapped_column(String(16), nullable=False, default="preparing")
    error_message: Mapped[str | None] = mapped_column(String(500), nullable=True)
    hands: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
    # Your net result in chip units (mbb at cash tables, tournament chips in a sit & go).
    hero_net: Mapped[int] = mapped_column(BigInteger, nullable=False, default=0)
    finished_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)

    hand_records: Mapped[list["PlayHand"]] = relationship(
        back_populates="session",
        cascade="all, delete-orphan",
        passive_deletes=True,
        order_by="PlayHand.hand_no",
    )

    __table_args__ = (
        CheckConstraint("format IN ('cash', 'fast_fold', 'sit_and_go')", name="play_format"),
        CheckConstraint(
            "status IN ('preparing', 'active', 'finished', 'interrupted', 'failed')", name="play_status"
        ),
        CheckConstraint("hands >= 0", name="hands_non_negative"),
    )


class PlayHand(UUIDPk, CreatedAt, Base):
    __tablename__ = "play_hands"

    session_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("play_sessions.id", ondelete="CASCADE"), nullable=False, index=True
    )
    hand_no: Mapped[int] = mapped_column(Integer, nullable=False)
    record: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    session: Mapped[PlaySession] = relationship(back_populates="hand_records")

    __table_args__ = (UniqueConstraint("session_id", "hand_no"),)
