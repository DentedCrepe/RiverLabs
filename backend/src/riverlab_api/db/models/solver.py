"""Solver configuration, jobs, results and strategy storage.

No solver exists yet (ADR-0005). These tables define how results will be stored: every
result carries the engine version, solver backend/version, configuration schema version and
the full assumptions document, so stored results stay interpretable as the engine evolves.
"""

import enum
import uuid
from datetime import datetime
from typing import Any

from sqlalchemy import (
    BigInteger,
    CheckConstraint,
    DateTime,
    Enum,
    Float,
    ForeignKey,
    Index,
    Integer,
    SmallInteger,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, CreatedAt, Timestamps, UUIDPk


class JobStatus(enum.StrEnum):
    QUEUED = "queued"
    RUNNING = "running"
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    CANCELLED = "cancelled"

    @property
    def is_terminal(self) -> bool:
        return self in (JobStatus.SUCCEEDED, JobStatus.FAILED, JobStatus.CANCELLED)


class SolverConfig(UUIDPk, Timestamps, Base):
    __tablename__ = "solver_configs"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    algorithm: Mapped[str] = mapped_column(String(32), nullable=False, index=True)
    schema_version: Mapped[int] = mapped_column(Integer, nullable=False)
    config: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (UniqueConstraint("owner_id", "name", postgresql_nulls_not_distinct=True),)


class SolverJob(UUIDPk, Timestamps, Base):
    __tablename__ = "solver_jobs"

    owner_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=True, index=True
    )
    status: Mapped[JobStatus] = mapped_column(
        Enum(JobStatus, name="job_status", values_callable=lambda e: [m.value for m in e]),
        nullable=False,
        default=JobStatus.QUEUED,
    )
    solver_config_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_configs.id", ondelete="SET NULL"), nullable=True
    )
    # Full engine-validated SolveSpec, copied so later config edits never change a job.
    spec: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    # Which compute backend accepted the job (e.g. "local", "cloud"); hidden from the UI.
    backend: Mapped[str] = mapped_column(String(32), nullable=False)
    iterations: Mapped[int] = mapped_column(BigInteger, nullable=False, default=0)
    progress: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False, default=dict)
    cancel_requested: Mapped[bool] = mapped_column(nullable=False, default=False)
    error_code: Mapped[str | None] = mapped_column(String(64), nullable=True)
    error_message: Mapped[str | None] = mapped_column(Text, nullable=True)
    started_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    finished_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)

    result: Mapped["SolverResult | None"] = relationship(back_populates="job", uselist=False)

    __table_args__ = (
        Index("ix_solver_jobs_status_created", "status", "created_at"),
        CheckConstraint(
            "(status IN ('succeeded', 'failed', 'cancelled')) = (finished_at IS NOT NULL)",
            name="finished_iff_terminal",
        ),
        CheckConstraint("status <> 'failed' OR error_code IS NOT NULL", name="failed_has_error"),
        CheckConstraint("iterations >= 0", name="iterations_non_negative"),
    )


class SolverResult(UUIDPk, CreatedAt, Base):
    __tablename__ = "solver_results"

    job_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_jobs.id", ondelete="CASCADE"), nullable=False, unique=True
    )
    engine_version: Mapped[str] = mapped_column(String(32), nullable=False)
    solver_backend: Mapped[str] = mapped_column(String(32), nullable=False)
    # e.g. a 40-character git commit hash for pinned third-party backends.
    solver_version: Mapped[str] = mapped_column(String(64), nullable=False)
    config_schema_version: Mapped[int] = mapped_column(Integer, nullable=False)
    # Everything needed to interpret the result: spec, solver config, abstractions, units.
    assumptions: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    iterations: Mapped[int] = mapped_column(BigInteger, nullable=False)
    exploitability: Mapped[float | None] = mapped_column(Float, nullable=True)
    exploitability_unit: Mapped[str | None] = mapped_column(String(16), nullable=True)
    # Large strategy payloads live in S3-compatible storage; only metadata is stored here.
    storage_uri: Mapped[str | None] = mapped_column(Text, nullable=True)
    summary: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False, default=dict)

    job: Mapped[SolverJob] = relationship(back_populates="result")

    __table_args__ = (
        CheckConstraint(
            "(exploitability IS NULL) = (exploitability_unit IS NULL)", name="exploitability_has_unit"
        ),
        CheckConstraint("exploitability IS NULL OR exploitability >= 0", name="exploitability_non_negative"),
    )


class StrategyNode(Base):
    """Strategy at one decision node of a solved tree (small trees inline; large ones in storage)."""

    __tablename__ = "strategy_nodes"

    id: Mapped[int] = mapped_column(BigInteger, primary_key=True, autoincrement=True)
    result_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_results.id", ondelete="CASCADE"), nullable=False
    )
    # Action path from the subgame root, e.g. "x/b33/c".
    node_path: Mapped[str] = mapped_column(Text, nullable=False)
    street: Mapped[str] = mapped_column(String(8), nullable=False)
    board: Mapped[str] = mapped_column(String(10), nullable=False)
    # 0 = out of position, 1 = in position (heads-up postflop subgames).
    player: Mapped[int] = mapped_column(SmallInteger, nullable=False, server_default="0")
    # Table position when the subgame comes from a known table context (optional).
    player_position: Mapped[str | None] = mapped_column(
        String(3), ForeignKey("positions.code", ondelete="RESTRICT"), nullable=True
    )
    actions: Mapped[list[str]] = mapped_column(JSONB, nullable=False)
    # Per-combo action frequencies (combo index -> list aligned with `actions`).
    strategy: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    ev_mbb: Mapped[dict[str, Any] | None] = mapped_column(JSONB, nullable=True)

    __table_args__ = (
        UniqueConstraint("result_id", "node_path"),
        CheckConstraint("street IN ('preflop', 'flop', 'turn', 'river')", name="street_kind"),
        CheckConstraint("player IN (0, 1)", name="player_index"),
    )


class SavedSolution(UUIDPk, CreatedAt, Base):
    __tablename__ = "saved_solutions"

    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), nullable=False
    )
    result_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("solver_results.id", ondelete="CASCADE"), nullable=False
    )
    name: Mapped[str] = mapped_column(String(100), nullable=False)
    notes: Mapped[str | None] = mapped_column(Text, nullable=True)

    __table_args__ = (UniqueConstraint("user_id", "result_id"),)
