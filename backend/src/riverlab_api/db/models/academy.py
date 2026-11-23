"""RiverLab Academy: lessons synced from content files, assessments and the learner's progress.

The app has a single local learner (no accounts yet), so progress tables have no user column.
"""

import uuid
from datetime import date, datetime
from typing import Any

from sqlalchemy import (
    Boolean,
    CheckConstraint,
    Date,
    DateTime,
    ForeignKey,
    Integer,
    SmallInteger,
    String,
    Text,
    UniqueConstraint,
    func,
)
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column

from riverlab_api.db.base import Base, CreatedAt, UUIDPk


class AcademyLesson(UUIDPk, Base):
    __tablename__ = "academy_lessons"

    slug: Mapped[str] = mapped_column(String(80), nullable=False, unique=True)
    level: Mapped[int] = mapped_column(SmallInteger, nullable=False)
    position: Mapped[int] = mapped_column(Integer, nullable=False)
    title: Mapped[str] = mapped_column(String(200), nullable=False)
    summary: Mapped[str] = mapped_column(Text, nullable=False)
    # draft -> in_review -> published (see docs/academy.md).
    status: Mapped[str] = mapped_column(String(16), nullable=False)
    version: Mapped[int] = mapped_column(Integer, nullable=False)
    content_hash: Mapped[str] = mapped_column(String(64), nullable=False)
    # Objectives, prerequisites, tags, tools, reading, review, estimated minutes.
    meta: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    # Parsed body: sections of blocks (markdown by explanation level, callouts, components).
    sections: Mapped[list[Any]] = mapped_column(JSONB, nullable=False)
    source_path: Mapped[str] = mapped_column(Text, nullable=False)
    synced_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False
    )

    __table_args__ = (
        CheckConstraint("level BETWEEN 1 AND 6", name="lesson_level"),
        CheckConstraint("status IN ('draft', 'in_review', 'published')", name="lesson_status"),
        CheckConstraint("version >= 1", name="lesson_version"),
    )


class AcademyLessonVersion(UUIDPk, CreatedAt, Base):
    """Every distinct published content of a lesson, kept for history and review."""

    __tablename__ = "academy_lesson_versions"

    lesson_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("academy_lessons.id", ondelete="CASCADE"), nullable=False, index=True
    )
    version: Mapped[int] = mapped_column(Integer, nullable=False)
    content_hash: Mapped[str] = mapped_column(String(64), nullable=False)
    status: Mapped[str] = mapped_column(String(16), nullable=False)
    meta: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    sections: Mapped[list[Any]] = mapped_column(JSONB, nullable=False)

    __table_args__ = (UniqueConstraint("lesson_id", "version"),)


class AcademyQuestion(Base):
    __tablename__ = "academy_questions"

    # "<lesson slug>:<question id>"
    id: Mapped[str] = mapped_column(String(160), primary_key=True)
    lesson_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("academy_lessons.id", ondelete="CASCADE"), nullable=False, index=True
    )
    kind: Mapped[str] = mapped_column(String(24), nullable=False)
    prompt: Mapped[str] = mapped_column(Text, nullable=False)
    # Options, answer, tolerance, unit, explanation (the answer is never sent before an attempt).
    data: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False)
    concepts: Mapped[list[str]] = mapped_column(JSONB, nullable=False)
    active: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)

    __table_args__ = (
        CheckConstraint("kind IN ('multiple_choice', 'decision', 'numeric')", name="question_kind"),
    )


class AcademyAttempt(UUIDPk, CreatedAt, Base):
    __tablename__ = "academy_attempts"

    question_id: Mapped[str] = mapped_column(
        String(160), ForeignKey("academy_questions.id", ondelete="CASCADE"), nullable=False, index=True
    )
    answer: Mapped[Any] = mapped_column(JSONB, nullable=False)
    correct: Mapped[bool] = mapped_column(Boolean, nullable=False)
    # 1 (guessing) to 5 (certain), when the learner gives it.
    confidence: Mapped[int | None] = mapped_column(SmallInteger, nullable=True)
    time_ms: Mapped[int | None] = mapped_column(Integer, nullable=True)

    __table_args__ = (
        CheckConstraint("confidence IS NULL OR confidence BETWEEN 1 AND 5", name="attempt_confidence"),
        CheckConstraint("time_ms IS NULL OR time_ms >= 0", name="attempt_time"),
    )


class AcademyProgress(Base):
    __tablename__ = "academy_progress"

    lesson_slug: Mapped[str] = mapped_column(String(80), primary_key=True)
    status: Mapped[str] = mapped_column(String(16), nullable=False, default="started")
    last_section: Mapped[str | None] = mapped_column(String(80), nullable=True)
    explanation_level: Mapped[str] = mapped_column(String(16), nullable=False, default="simple")
    saved: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    started_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), nullable=False
    )
    completed_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now(), nullable=False
    )

    __table_args__ = (
        CheckConstraint("status IN ('started', 'completed')", name="progress_status"),
        CheckConstraint(
            "explanation_level IN ('simple', 'intermediate', 'advanced', 'math')", name="progress_level"
        ),
    )


class AcademyBookmark(UUIDPk, CreatedAt, Base):
    __tablename__ = "academy_bookmarks"

    lesson_slug: Mapped[str] = mapped_column(String(80), nullable=False)
    section_id: Mapped[str] = mapped_column(String(80), nullable=False)
    label: Mapped[str] = mapped_column(String(200), nullable=False)
    note: Mapped[str | None] = mapped_column(Text, nullable=True)

    __table_args__ = (UniqueConstraint("lesson_slug", "section_id"),)


class AcademyActivity(Base):
    """One row per day with learning activity (for streaks)."""

    __tablename__ = "academy_activity"

    day: Mapped[date] = mapped_column(Date, primary_key=True)
    events: Mapped[int] = mapped_column(Integer, nullable=False, default=0)
