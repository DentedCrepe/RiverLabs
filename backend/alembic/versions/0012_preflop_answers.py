"""preflop_answers: preflop drill answers, graded against GTOpen's preflop model (approximate)

Downgrading drops every preflop drill answer and requires `-x allow_destructive=true`.

Revision ID: 0012
Revises: 0011
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0012"
down_revision: str | None = "0011"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "preflop_answers",
        sa.Column("job_id", postgresql.UUID(as_uuid=True), nullable=True),
        sa.Column("spot", sa.String(length=16), nullable=False),
        sa.Column("position", sa.String(length=8), nullable=False),
        sa.Column("cards", sa.String(length=4), nullable=False),
        sa.Column("hand_class", sa.String(length=3), nullable=False),
        sa.Column("choice", sa.String(length=40), nullable=False),
        sa.Column("verdict", sa.String(length=8), nullable=False),
        sa.Column("model_frequency", sa.Float(), nullable=False),
        sa.Column("grade", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", postgresql.UUID(as_uuid=True), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "model_frequency BETWEEN 0 AND 1", name=op.f("ck_preflop_answers_frequency_range")
        ),
        sa.CheckConstraint(
            "spot IN ('open', 'facing_raise', 'any')", name=op.f("ck_preflop_answers_spot_kind")
        ),
        sa.CheckConstraint(
            "verdict IN ('matches', 'mixed', 'differs')", name=op.f("ck_preflop_answers_verdict_kind")
        ),
        sa.ForeignKeyConstraint(
            ["job_id"],
            ["solver_jobs.id"],
            name=op.f("fk_preflop_answers_job_id_solver_jobs"),
            ondelete="SET NULL",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_preflop_answers")),
    )
    op.create_index(op.f("ix_preflop_answers_job_id"), "preflop_answers", ["job_id"], unique=False)


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop preflop_answers (every preflop drill answer would be lost). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0011` if you really mean it."
        )
    op.drop_index(op.f("ix_preflop_answers_job_id"), table_name="preflop_answers")
    op.drop_table("preflop_answers")
