"""daily_goals: your daily hands and study targets (one row)

Downgrading drops your targets (they return to the defaults) and requires
`-x allow_destructive=true`.

Revision ID: 0009
Revises: 0008
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op

revision: str = "0009"
down_revision: str | None = "0008"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "daily_goals",
        sa.Column("id", sa.Integer(), nullable=False),
        sa.Column("hands_target", sa.Integer(), nullable=False),
        sa.Column("study_target", sa.Integer(), nullable=False),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint("hands_target BETWEEN 1 AND 1000", name=op.f("ck_daily_goals_hands_target_range")),
        sa.CheckConstraint("id = 1", name=op.f("ck_daily_goals_single_row")),
        sa.CheckConstraint("study_target BETWEEN 1 AND 1000", name=op.f("ck_daily_goals_study_target_range")),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_daily_goals")),
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop daily_goals. Re-run with "
            "`alembic -x allow_destructive=true downgrade 0008` if you really mean it."
        )
    op.drop_table("daily_goals")
