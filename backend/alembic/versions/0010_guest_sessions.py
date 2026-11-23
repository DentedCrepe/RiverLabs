"""play_sessions.tracked: games played in guest mode do not count towards your statistics

Downgrading drops the flag, so guest games would start counting as yours: it deletes them and
requires `-x allow_destructive=true`.

Revision ID: 0010
Revises: 0009
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op

revision: str = "0010"
down_revision: str | None = "0009"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column(
        "play_sessions", sa.Column("tracked", sa.Boolean(), server_default=sa.text("true"), nullable=False)
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop play_sessions.tracked (guest games would be deleted). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0009` if you really mean it."
        )
    op.execute("DELETE FROM play_sessions WHERE NOT tracked")
    op.drop_column("play_sessions", "tracked")
