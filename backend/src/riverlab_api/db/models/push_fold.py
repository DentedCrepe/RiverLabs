"""Trainer answers: push/fold (graded against the exact equilibrium) and postflop drills
(graded by EV from a saved solve)."""

import uuid
from typing import Any

from sqlalchemy import BigInteger, CheckConstraint, Float, ForeignKey, SmallInteger, String
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column

from riverlab_api.db.base import Base, CreatedAt, UUIDPk


class PushFoldAnswer(UUIDPk, CreatedAt, Base):
    __tablename__ = "push_fold_answers"

    stack_mbb: Mapped[int] = mapped_column(BigInteger, nullable=False)
    seat: Mapped[str] = mapped_column(String(2), nullable=False)
    cards: Mapped[str] = mapped_column(String(4), nullable=False)
    hand_class: Mapped[str] = mapped_column(String(3), nullable=False, index=True)
    choice: Mapped[str] = mapped_column(String(4), nullable=False)
    verdict: Mapped[str] = mapped_column(String(8), nullable=False)
    # Expected values in bb (an engine measure, not a chip amount): best action minus yours.
    ev_loss_bb: Mapped[float] = mapped_column(Float, nullable=False)
    # The engine's full grade (EVs, equilibrium frequencies, exploitability of the reference).
    grade: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (
        CheckConstraint("seat IN ('sb', 'bb')", name="seat_kind"),
        CheckConstraint("choice IN ('fold', 'play')", name="choice_kind"),
        CheckConstraint("verdict IN ('correct', 'close', 'mistake')", name="verdict_kind"),
        CheckConstraint("ev_loss_bb >= 0", name="ev_loss_non_negative"),
    )


class PostflopAnswer(UUIDPk, CreatedAt, Base):
    """Postflop drill answers, graded by the EV the engine computes from a saved solve."""

    __tablename__ = "postflop_answers"

    job_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_jobs.id", ondelete="SET NULL"), nullable=True, index=True
    )
    street: Mapped[str] = mapped_column(String(5), nullable=False)
    # 0 = out of position, 1 = in position.
    hero_player: Mapped[int] = mapped_column(SmallInteger, nullable=False)
    cards: Mapped[str] = mapped_column(String(4), nullable=False)
    hand_class: Mapped[str] = mapped_column(String(3), nullable=False)
    choice: Mapped[str] = mapped_column(String(40), nullable=False)
    verdict: Mapped[str] = mapped_column(String(8), nullable=False)
    ev_loss_bb: Mapped[float] = mapped_column(Float, nullable=False)
    ev_loss_pct_pot: Mapped[float] = mapped_column(Float, nullable=False)
    grade: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (
        CheckConstraint("street IN ('flop', 'turn', 'river')", name="street_kind"),
        CheckConstraint("hero_player IN (0, 1)", name="hero_player_kind"),
        CheckConstraint("verdict IN ('correct', 'close', 'mistake')", name="verdict_kind"),
        CheckConstraint("ev_loss_bb >= 0", name="ev_loss_non_negative"),
    )


class PreflopAnswer(UUIDPk, CreatedAt, Base):
    """Preflop drill answers, graded against GTOpen's preflop model (an approximate model):
    by how often the model takes the chosen action with the hand, not by EV (docs/trainer.md)."""

    __tablename__ = "preflop_answers"

    job_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_jobs.id", ondelete="SET NULL"), nullable=True, index=True
    )
    spot: Mapped[str] = mapped_column(String(16), nullable=False)
    position: Mapped[str] = mapped_column(String(8), nullable=False)
    cards: Mapped[str] = mapped_column(String(4), nullable=False)
    hand_class: Mapped[str] = mapped_column(String(3), nullable=False)
    choice: Mapped[str] = mapped_column(String(40), nullable=False)
    verdict: Mapped[str] = mapped_column(String(8), nullable=False)
    # How often the model takes the chosen action with this hand class (0..1); not chips.
    model_frequency: Mapped[float] = mapped_column(Float, nullable=False)
    grade: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (
        CheckConstraint("spot IN ('open', 'facing_raise', 'any')", name="spot_kind"),
        CheckConstraint("verdict IN ('matches', 'mixed', 'differs')", name="verdict_kind"),
        CheckConstraint("model_frequency BETWEEN 0 AND 1", name="frequency_range"),
    )
