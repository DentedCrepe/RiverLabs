"""hand_reviews: solver reviews of played hands, one chained postflop solve per street

Downgrading drops every hand review and requires `-x allow_destructive=true` (the solves
themselves stay in solver_jobs).

Revision ID: 0007
Revises: 0006
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0007"
down_revision: str | None = "0006"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "hand_reviews",
        sa.Column("session_id", sa.UUID(), nullable=False),
        sa.Column("hand_no", sa.Integer(), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("streets", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("error_message", sa.String(length=1000), nullable=True),
        sa.Column("finished_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "status IN ('queued', 'running', 'done', 'failed', 'interrupted')",
            name=op.f("ck_hand_reviews_review_status"),
        ),
        sa.ForeignKeyConstraint(
            ["session_id"],
            ["play_sessions.id"],
            name=op.f("fk_hand_reviews_session_id_play_sessions"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_hand_reviews")),
        sa.UniqueConstraint("session_id", "hand_no", name=op.f("uq_hand_reviews_session_id_hand_no")),
    )
    op.create_index(op.f("ix_hand_reviews_session_id"), "hand_reviews", ["session_id"], unique=False)


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop hand_reviews. Re-run with "
            "`alembic -x allow_destructive=true downgrade 0006` if you really mean it."
        )
    op.drop_index(op.f("ix_hand_reviews_session_id"), table_name="hand_reviews")
    op.drop_table("hand_reviews")
