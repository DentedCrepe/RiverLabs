"""postflop_answers: postflop drill answers, graded by EV from saved solves (Analytics reads them)

Downgrading drops your postflop trainer history and requires `-x allow_destructive=true`.

Revision ID: 0008
Revises: 0007
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0008"
down_revision: str | None = "0007"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "postflop_answers",
        sa.Column("job_id", sa.UUID(), nullable=True),
        sa.Column("street", sa.String(length=5), nullable=False),
        sa.Column("hero_player", sa.SmallInteger(), nullable=False),
        sa.Column("cards", sa.String(length=4), nullable=False),
        sa.Column("hand_class", sa.String(length=3), nullable=False),
        sa.Column("choice", sa.String(length=40), nullable=False),
        sa.Column("verdict", sa.String(length=8), nullable=False),
        sa.Column("ev_loss_bb", sa.Float(), nullable=False),
        sa.Column("ev_loss_pct_pot", sa.Float(), nullable=False),
        sa.Column("grade", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "street IN ('flop', 'turn', 'river')", name=op.f("ck_postflop_answers_street_kind")
        ),
        sa.CheckConstraint(
            "verdict IN ('correct', 'close', 'mistake')", name=op.f("ck_postflop_answers_verdict_kind")
        ),
        sa.CheckConstraint("ev_loss_bb >= 0", name=op.f("ck_postflop_answers_ev_loss_non_negative")),
        sa.CheckConstraint("hero_player IN (0, 1)", name=op.f("ck_postflop_answers_hero_player_kind")),
        sa.ForeignKeyConstraint(
            ["job_id"],
            ["solver_jobs.id"],
            name=op.f("fk_postflop_answers_job_id_solver_jobs"),
            ondelete="SET NULL",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_postflop_answers")),
    )
    op.create_index(op.f("ix_postflop_answers_job_id"), "postflop_answers", ["job_id"], unique=False)


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop postflop_answers (your trainer history). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0007` if you really mean it."
        )
    op.drop_index(op.f("ix_postflop_answers_job_id"), table_name="postflop_answers")
    op.drop_table("postflop_answers")
