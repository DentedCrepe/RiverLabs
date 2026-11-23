"""solver_jobs.kind: postflop solves or preflop (approximate model) solves

Existing jobs are postflop solves, so the column defaults to 'postflop'.

Downgrading deletes preflop jobs (and their results) and requires `-x allow_destructive=true`.

Revision ID: 0003
Revises: 0002
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op

revision: str = "0003"
down_revision: str | None = "0002"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column(
        "solver_jobs", sa.Column("kind", sa.String(length=16), server_default="postflop", nullable=False)
    )
    op.create_check_constraint("job_kind", "solver_jobs", "kind IN ('postflop', 'preflop')")


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop solver_jobs.kind (deleting preflop jobs). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0002` if you really mean it."
        )
    op.execute("DELETE FROM solver_jobs WHERE kind = 'preflop'")
    op.drop_constraint(op.f("ck_solver_jobs_job_kind"), "solver_jobs", type_="check")
    op.drop_column("solver_jobs", "kind")
