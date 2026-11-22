"""Trainer persistence. Grading is planned; recommendation/EV-loss columns stay NULL until
decisions can be graded against stored, validated solver results."""

import uuid
from datetime import datetime
from typing import Any

from sqlalchemy import CheckConstraint, DateTime, Float, ForeignKey, String, Text, func
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, UUIDPk


class TrainingSession(UUIDPk, Base):
    __tablename__ = "training_sessions"

    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False, index=True
    )
    game_config_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("game_configs.id", ondelete="RESTRICT"), nullable=False
    )
    mode: Mapped[str] = mapped_column(String(32), nullable=False)
    status: Mapped[str] = mapped_column(String(16), nullable=False, default="active")
    started_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())
    ended_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)

    decisions: Mapped[list["TrainingDecision"]] = relationship(
        back_populates="session", cascade="all, delete-orphan"
    )

    __table_args__ = (
        CheckConstraint("status IN ('active', 'completed', 'abandoned')", name="status_kind"),
        CheckConstraint("ended_at IS NULL OR ended_at >= started_at", name="ended_after_start"),
    )


class TrainingDecision(UUIDPk, Base):
    __tablename__ = "training_decisions"

    session_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("training_sessions.id", ondelete="CASCADE"), nullable=False, index=True
    )
    solver_result_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_results.id", ondelete="SET NULL"), nullable=True
    )
    node_path: Mapped[str] = mapped_column(Text, nullable=False)
    hole_cards: Mapped[str] = mapped_column(String(4), nullable=False)
    chosen_action: Mapped[str] = mapped_column(String(32), nullable=False)
    recommended: Mapped[dict[str, Any] | None] = mapped_column(JSONB, nullable=True)
    ev_loss_mbb: Mapped[float | None] = mapped_column(Float, nullable=True)
    decided_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    session: Mapped[TrainingSession] = relationship(back_populates="decisions")

    __table_args__ = (
        CheckConstraint("ev_loss_mbb IS NULL OR ev_loss_mbb >= 0", name="ev_loss_non_negative"),
        CheckConstraint("ev_loss_mbb IS NULL OR solver_result_id IS NOT NULL", name="graded_against_result"),
    )
