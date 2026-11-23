"""strategy_nodes: player index for heads-up subgames; longer solver versions

Postflop subgames have an out-of-position and an in-position player rather than table seats,
so `player` (0 = OOP, 1 = IP) is added and `player_position` becomes optional.
`solver_results.solver_version` grows to 64 characters to hold git commit hashes.

Downgrading drops the `player` column and requires `-x allow_destructive=true`.

Revision ID: 0002
Revises: 0001
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op

revision: str = "0002"
down_revision: str | None = "0001"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column(
        "strategy_nodes", sa.Column("player", sa.SmallInteger(), server_default="0", nullable=False)
    )
    op.create_check_constraint("player_index", "strategy_nodes", "player IN (0, 1)")
    op.alter_column("strategy_nodes", "player_position", existing_type=sa.String(length=3), nullable=True)
    op.alter_column(
        "solver_results", "solver_version", existing_type=sa.String(length=32), type_=sa.String(length=64)
    )


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop strategy_nodes.player. Re-run with "
            "`alembic -x allow_destructive=true downgrade 0001` if you really mean it."
        )
    op.execute("DELETE FROM solver_results WHERE length(solver_version) > 32")
    op.alter_column(
        "solver_results", "solver_version", existing_type=sa.String(length=64), type_=sa.String(length=32)
    )
    op.execute("DELETE FROM strategy_nodes WHERE player_position IS NULL")
    op.alter_column("strategy_nodes", "player_position", existing_type=sa.String(length=3), nullable=False)
    op.drop_constraint(op.f("ck_strategy_nodes_player_index"), "strategy_nodes", type_="check")
    op.drop_column("strategy_nodes", "player")
