"""Daily goals: how many hands to play and how much to study each day (one row)."""

from sqlalchemy import CheckConstraint, Integer
from sqlalchemy.orm import Mapped, mapped_column

from riverlab_api.db.base import Base, Timestamps


class DailyGoals(Timestamps, Base):
    __tablename__ = "daily_goals"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, default=1)
    hands_target: Mapped[int] = mapped_column(Integer, nullable=False, default=30)
    study_target: Mapped[int] = mapped_column(Integer, nullable=False, default=10)

    __table_args__ = (
        CheckConstraint("id = 1", name="single_row"),
        CheckConstraint("hands_target BETWEEN 1 AND 1000", name="hands_target_range"),
        CheckConstraint("study_target BETWEEN 1 AND 1000", name="study_target_range"),
    )
