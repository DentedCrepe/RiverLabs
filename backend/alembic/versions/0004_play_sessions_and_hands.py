"""play_sessions, play_hands: games against bots and their hand records

Downgrading drops both tables (every recorded game) and requires `-x allow_destructive=true`.

Revision ID: 0004
Revises: 0003
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0004"
down_revision: str | None = "0003"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "play_sessions",
        sa.Column("format", sa.String(length=16), nullable=False),
        sa.Column("config", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("status", sa.String(length=16), nullable=False),
        sa.Column("error_message", sa.String(length=500), nullable=True),
        sa.Column("hands", sa.Integer(), nullable=False),
        sa.Column("hero_net", sa.BigInteger(), nullable=False),
        sa.Column("finished_at", sa.DateTime(timezone=True), nullable=True),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint(
            "format IN ('cash', 'fast_fold', 'sit_and_go')", name=op.f("ck_play_sessions_play_format")
        ),
        sa.CheckConstraint(
            "status IN ('preparing', 'active', 'finished', 'interrupted', 'failed')",
            name=op.f("ck_play_sessions_play_status"),
        ),
        sa.CheckConstraint("hands >= 0", name=op.f("ck_play_sessions_hands_non_negative")),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_play_sessions")),
    )
    op.create_table(
        "play_hands",
        sa.Column("session_id", sa.UUID(), nullable=False),
        sa.Column("hand_no", sa.Integer(), nullable=False),
        sa.Column("record", postgresql.JSONB(astext_type=sa.Text()), nullable=False),
        sa.Column("id", sa.UUID(), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.ForeignKeyConstraint(
            ["session_id"],
            ["play_sessions.id"],
            name=op.f("fk_play_hands_session_id_play_sessions"),
            ondelete="CASCADE",
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_play_hands")),
        sa.UniqueConstraint("session_id", "hand_no", name=op.f("uq_play_hands_session_id_hand_no")),
    )
    op.create_index(op.f("ix_play_hands_session_id"), "play_hands", ["session_id"], unique=False)


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop play_sessions and play_hands (all recorded games). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0003` if you really mean it."
        )
    op.drop_index(op.f("ix_play_hands_session_id"), table_name="play_hands")
    op.drop_table("play_hands")
    op.drop_table("play_sessions")
