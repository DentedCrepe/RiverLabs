"""academy_reviews: a person's reviews of lesson versions, made on the review page

Downgrading drops every review (the lessons go back to "not yet reviewed") and requires
`-x allow_destructive=true`.

Revision ID: 0011
Revises: 0010
"""

from collections.abc import Sequence

import sqlalchemy as sa
from alembic import context, op
from sqlalchemy.dialects import postgresql

revision: str = "0011"
down_revision: str | None = "0010"
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "academy_reviews",
        sa.Column("lesson_slug", sa.String(length=80), nullable=False),
        sa.Column("version", sa.Integer(), nullable=False),
        sa.Column("content_hash", sa.String(length=64), nullable=False),
        sa.Column("verdict", sa.String(length=24), nullable=False),
        sa.Column("reviewer", sa.String(length=100), nullable=False),
        sa.Column("notes", sa.Text(), nullable=False),
        sa.Column("id", postgresql.UUID(as_uuid=True), nullable=False),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.text("now()"), nullable=False),
        sa.CheckConstraint("length(reviewer) > 0", name=op.f("ck_academy_reviews_reviewer_named")),
        sa.CheckConstraint(
            "verdict IN ('reviewed', 'changes_requested')", name=op.f("ck_academy_reviews_review_verdict")
        ),
        sa.PrimaryKeyConstraint("id", name=op.f("pk_academy_reviews")),
    )
    op.create_index(op.f("ix_academy_reviews_lesson_slug"), "academy_reviews", ["lesson_slug"], unique=False)


def downgrade() -> None:
    if context.get_x_argument(as_dictionary=True).get("allow_destructive") != "true":
        raise RuntimeError(
            "Refusing to drop academy_reviews (every lesson review would be lost). Re-run with "
            "`alembic -x allow_destructive=true downgrade 0010` if you really mean it."
        )
    op.drop_index(op.f("ix_academy_reviews_lesson_slug"), table_name="academy_reviews")
    op.drop_table("academy_reviews")
