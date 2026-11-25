"""icm_push_fold_answers: ICM push/fold drill answers (3 to 6 players, a modelled approximation)

Downgrading drops every ICM push/fold answer and requires `-x allow_destructive=true`.

Revision ID: 0013
Revises: 0012
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0013"
down_revision: str | None = "0012"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "icm_push_fold_answers",
        sa.Column("players", sa.SmallInteger(), nullable=False),
        sa.Column("position", sa.String(length=8), nullable=False),
        sa.Column("facing", sa.String(length=8), nullable=True),
        sa.Column("cards", sa.String(length=4), nullable=False),
        sa.Column("hand_class", sa.String(length=3), nullable=False),
        sa.Column("choice", sa.String(length=4), nullable=False),
        sa.Column("verdict", sa.String(length=8), nullable=False),
        sa.Column("loss_pct", sa.Float(), nullable=False),
        sa.Column("grade", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", postgresql.UUID(as_uuid=True), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint("choice IN ('fold', 'play')", name=op.f("ck_icm_push_fold_answers_choice_kind")),
        sa.CheckConstraint("loss_pct >= 0", name=op.f("ck_icm_push_fold_answers_loss_non_negative")),
        sa.CheckConstraint("players BETWEEN 2 AND 6", name=op.f("ck_icm_push_fold_answers_players_range")),
        sa.CheckConstraint(
            "verdict IN ('correct', 'close', 'mistake')", name=op.f("ck_icm_push_fold_answers_verdict_kind")
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_icm_push_fold_answers")),
    )
    op.create_index(
        op.f("ix_icm_push_fold_answers_hand_class"), "icm_push_fold_answers", ["hand_class"], unique=False
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop icm_push_fold_answers (every ICM push/fold answer would be lost). Re-run "
            "with `alembic -x allow_destructive=true downgrade 0012` if you really mean it."
        )
    op.drop_index(op.f("ix_icm_push_fold_answers_hand_class"), table_name="icm_push_fold_answers")
    op.drop_table("icm_push_fold_answers")
