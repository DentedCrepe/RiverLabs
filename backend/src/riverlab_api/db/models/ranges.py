import uuid

from sqlalchemy import CheckConstraint, Float, ForeignKey, SmallInteger, String, Text
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, Timestamps, UUIDPk


class Range(UUIDPk, Timestamps, Base):
    """A stored range. `source` records where the weights came from so user-entered ranges
    are never confused with solver output."""

    __tablename__ = "ranges"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True, index=True
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    notation: Mapped[str] = mapped_column(Text, nullable=False)
    combo_count: Mapped[float] = mapped_column(Float, nullable=False)
    source: Mapped[str] = mapped_column(String(16), nullable=False)
    position_code: Mapped[str | None] = mapped_column(
        String(3), ForeignKey("positions.code", ondelete="RESTRICT"), nullable=True
    )
    solver_result_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_results.id", ondelete="SET NULL"), nullable=True
    )

    combos: Mapped[list["RangeCombo"]] = relationship(
        back_populates="range", cascade="all, delete-orphan", order_by="RangeCombo.combo_index"
    )

    __table_args__ = (
        CheckConstraint("source IN ('user', 'imported', 'solver')", name="source_kind"),
        CheckConstraint("combo_count >= 0 AND combo_count <= 1326", name="combo_count_range"),
        CheckConstraint(
            "source <> 'solver' OR solver_result_id IS NOT NULL", name="solver_source_has_result"
        ),
    )


class RangeCombo(Base):
    """Non-zero combo weights. combo_index follows the engine's 0..1325 combo indexing."""

    __tablename__ = "range_combos"

    range_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("ranges.id", ondelete="CASCADE"), primary_key=True
    )
    combo_index: Mapped[int] = mapped_column(SmallInteger, primary_key=True)
    weight: Mapped[float] = mapped_column(Float, nullable=False)

    range: Mapped[Range] = relationship(back_populates="combos")

    __table_args__ = (
        CheckConstraint("combo_index BETWEEN 0 AND 1325", name="combo_index_range"),
        CheckConstraint("weight > 0 AND weight <= 1", name="weight_range"),
    )
