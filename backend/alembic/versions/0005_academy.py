"""academy_*: RiverLab Academy lessons, versions, questions, attempts, progress, bookmarks

Downgrading drops every Academy table, including the learner's progress and answers, and
requires `-x allow_destructive=true`.

Revision ID: 0005
Revises: 0004
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0005"
down_revision: str | None = "0004"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "academy_activity",
        sa.Column("day", sa.Date(), nullable=False),
        sa.Column("events", sa.Integer(), nullable=False),
        sa.PrimaryKeyConstraint("day", name=op.f("pk_academy_activity")),
    )
    op.create_table(
        "academy_bookmarks",
        sa.Column("lesson_slug", sa.String(length=80), nullable=False),
        sa.Column("section_id", sa.String(length=80), nullable=False),
        sa.Column("label", sa.String(length=200), nullable=False),
        sa.Column("note", sa.Text(), nullable=True),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_bookmarks")),
        sa.UniqueConstraint(
            "lesson_slug", "section_id", name=op.f("uq_academy_bookmarks_lesson_slug_section_id")
        ),
    )
    op.create_table(
        "academy_lessons",
        sa.Column("slug", sa.String(length=80), nullable=False),
        sa.Column("level", sa.SmallInteger(), nullable=False),
        sa.Column("position", sa.Integer(), nullable=False),
        sa.Column("title", sa.String(length=200), nullable=False),
        sa.Column("summary", sa.Text(), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("version", sa.Integer(), nullable=False),
        sa.Column("content_hash", sa.String(length=64), nullable=False),
        sa.Column("meta", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("sections", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("source_path", sa.Text(), nullable=False),
        sa.Column("synced_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.CheckConstraint(
            "status IN ('draft', 'in_review', 'published')", name=op.f("ck_academy_lessons_lesson_status")
        ),
        sa.CheckConstraint("level BETWEEN 1 AND 6", name=op.f("ck_academy_lessons_lesson_level")),
        sa.CheckConstraint("version >= 1", name=op.f("ck_academy_lessons_lesson_version")),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_lessons")),
        sa.UniqueConstraint("slug", name=op.f("uq_academy_lessons_slug")),
    )
    op.create_table(
        "academy_progress",
        sa.Column("lesson_slug", sa.String(length=80), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("last_section", sa.String(length=80), nullable=True),
        sa.Column("explanation_level", sa.String(length=16), nullable=False),
        sa.Column("saved", sa.Boolean(), nullable=False),
        sa.Column("started_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.Column("completed_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "explanation_level IN ('simple', 'intermediate', 'advanced', 'math')",
            name=op.f("ck_academy_progress_progress_level"),
        ),
        sa.CheckConstraint(
            "status IN ('started', 'completed')", name=op.f("ck_academy_progress_progress_status")
        ),
        sa.PrimaryKeyConstraint("lesson_slug", name=op.f("pk_academy_progress")),
    )
    op.create_table(
        "academy_lesson_versions",
        sa.Column("lesson_id", sa.UUID(), nullable=False),
        sa.Column("version", sa.Integer(), nullable=False),
        sa.Column("content_hash", sa.String(length=64), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("meta", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("sections", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.ForeignKeyConstraint(
            ["lesson_id"],
            ["academy_lessons.id"],
            name=op.f("fk_academy_lesson_versions_lesson_id_academy_lessons"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_lesson_versions")),
        sa.UniqueConstraint(
            "lesson_id", "version", name=op.f("uq_academy_lesson_versions_lesson_id_version")
        ),
    )
    op.create_index(
        op.f("ix_academy_lesson_versions_lesson_id"), "academy_lesson_versions", ["lesson_id"], unique=False
    )
    op.create_table(
        "academy_questions",
        sa.Column("id", sa.String(length=160), nullable=False),
        sa.Column("lesson_id", sa.UUID(), nullable=False),
        sa.Column("kind", sa.String(length=24), nullable=False),
        sa.Column("prompt", sa.Text(), nullable=False),
        sa.Column("data", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("concepts", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("active", sa.Boolean(), nullable=False),
        sa.CheckConstraint(
            "kind IN ('multiple_choice', 'decision', 'numeric')",
            name=op.f("ck_academy_questions_question_kind"),
        ),
        sa.ForeignKeyConstraint(
            ["lesson_id"],
            ["academy_lessons.id"],
            name=op.f("fk_academy_questions_lesson_id_academy_lessons"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_questions")),
    )
    op.create_index(op.f("ix_academy_questions_lesson_id"), "academy_questions", ["lesson_id"], unique=False)
    op.create_table(
        "academy_attempts",
        sa.Column("question_id", sa.String(length=160), nullable=False),
        sa.Column("answer", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("correct", sa.Boolean(), nullable=False),
        sa.Column("confidence", sa.SmallInteger(), nullable=True),
        sa.Column("time_ms", sa.Integer(), nullable=True),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "confidence IS NULL OR confidence BETWEEN 1 AND 5",
            name=op.f("ck_academy_attempts_attempt_confidence"),
        ),
        sa.CheckConstraint("time_ms IS NULL OR time_ms >= 0", name=op.f("ck_academy_attempts_attempt_time")),
        sa.ForeignKeyConstraint(
            ["question_id"],
            ["academy_questions.id"],
            name=op.f("fk_academy_attempts_question_id_academy_questions"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_attempts")),
    )
    op.create_index(
        op.f("ix_academy_attempts_question_id"), "academy_attempts", ["question_id"], unique=False
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop the Academy tables (progress and answers). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0004` if you really mean it."
        )
    op.drop_index(op.f("ix_academy_attempts_question_id"), table_name="academy_attempts")
    op.drop_table("academy_attempts")
    op.drop_index(op.f("ix_academy_questions_lesson_id"), table_name="academy_questions")
    op.drop_table("academy_questions")
    op.drop_index(op.f("ix_academy_lesson_versions_lesson_id"), table_name="academy_lesson_versions")
    op.drop_table("academy_lesson_versions")
    op.drop_table("academy_progress")
    op.drop_table("academy_lessons")
    op.drop_table("academy_bookmarks")
    op.drop_table("academy_activity")
