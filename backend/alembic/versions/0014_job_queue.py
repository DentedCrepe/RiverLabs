"""solver_jobs: durable queue columns (attempts, worker_id, heartbeat_at)

Workers claim queued jobs with SELECT ... FOR UPDATE SKIP LOCKED and report a heartbeat while
they run; a running job whose heartbeat goes stale is requeued (docs/architecture.md, ADR-0007).

Downgrading drops the queue columns (the attempt counts are lost) and requires
`-x allow_destructive=true`.

Revision ID: 0014
Revises: 0013
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op

revision: str = "0014"
down_revision: str | None = "0013"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column("solver_jobs", sa.Column("attempts", sa.Integer(), server_default="0", nullable=False))
    op.add_column("solver_jobs", sa.Column("worker_id", sa.String(length=128), nullable=True))
    op.add_column("solver_jobs", sa.Column("heartbeat_at", sa.DateTime(timezone=True), nullable=True))
    op.create_check_constraint(op.f("ck_solver_jobs_attempts_non_negative"), "solver_jobs", "attempts >= 0")


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop the job queue columns (attempt counts would be lost). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0013` if you really mean it."
        )
    op.drop_constraint(op.f("ck_solver_jobs_attempts_non_negative"), "solver_jobs", type_="check")
    op.drop_column("solver_jobs", "heartbeat_at")
    op.drop_column("solver_jobs", "worker_id")
    op.drop_column("solver_jobs", "attempts")
