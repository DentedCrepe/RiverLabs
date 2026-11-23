"""push_fold_answers: push/fold trainer answers, graded by the engine (Analytics reads them)

Downgrading drops your trainer history and requires `-x allow_destructive=true`.

Revision ID: 0006
Revises: 0005
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0006"
down_revision: str | None = "0005"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "push_fold_answers",
        sa.Column("stack_mbb", sa.BigInteger(), nullable=False),
        sa.Column("seat", sa.String(length=2), nullable=False),
        sa.Column("cards", sa.String(length=4), nullable=False),
        sa.Column("hand_class", sa.String(length=3), nullable=False),
        sa.Column("choice", sa.String(length=4), nullable=False),
        sa.Column("verdict", sa.String(length=8), nullable=False),
        sa.Column("ev_loss_bb", sa.Float(), nullable=False),
        sa.Column("grade", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint("choice IN ('fold', 'play')", name=op.f("ck_push_fold_answers_choice_kind")),
        sa.CheckConstraint("seat IN ('sb', 'bb')", name=op.f("ck_push_fold_answers_seat_kind")),
        sa.CheckConstraint(
            "verdict IN ('correct', 'close', 'mistake')", name=op.f("ck_push_fold_answers_verdict_kind")
        ),
        sa.CheckConstraint("ev_loss_bb >= 0", name=op.f("ck_push_fold_answers_ev_loss_non_negative")),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_push_fold_answers")),
    )
    op.create_index(
        op.f("ix_push_fold_answers_hand_class"), "push_fold_answers", ["hand_class"], unique=False
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop push_fold_answers (your trainer history). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0005` if you really mean it."
        )
    op.drop_index(op.f("ix_push_fold_answers_hand_class"), table_name="push_fold_answers")
    op.drop_table("push_fold_answers")
