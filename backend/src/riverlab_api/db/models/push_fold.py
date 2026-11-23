"""Push/fold trainer answers, each graded by the engine against the exact equilibrium."""

from typing import Any

from sqlalchemy import BigInteger, CheckConstraint, Float, String
from sqlalchemy.dialects.postgresql import JSONB
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
