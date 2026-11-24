"""Lesson review by a person (docs/academy.md, "Review").

A lesson counts as reviewed when its front matter says so, or when a person marked the exact
current content reviewed on the review page. Any change to a lesson (which bumps its version)
leaves earlier reviews as reviews of an older version, so it goes back to the queue.
"""

from datetime import UTC, datetime
from typing import Any

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import AcademyLesson, AcademyQuestion, AcademyReview
from riverlab_api.services import academy

# Review page states, in the order the page lists them.
STATES = ("to_review", "changes_requested", "outdated", "reviewed")


def _record(r: AcademyReview, row: AcademyLesson) -> dict[str, Any]:
    return {
        "id": str(r.id),
        "verdict": r.verdict,
        "reviewer": r.reviewer,
        "notes": r.notes,
        "version": r.version,
        "current": r.content_hash == row.content_hash,
        "created_at": r.created_at,
    }


def _history(db: Session, slugs: list[str]) -> dict[str, list[AcademyReview]]:
    out: dict[str, list[AcademyReview]] = {s: [] for s in slugs}
    q = (
        select(AcademyReview)
        .where(AcademyReview.lesson_slug.in_(slugs))
        .order_by(AcademyReview.created_at.desc())
    )
    for r in db.scalars(q):
        out[r.lesson_slug].append(r)
    return out


def review_of(row: AcademyLesson, history: list[AcademyReview]) -> dict[str, Any]:
    """The `review` block shown with a lesson: the front matter's, or the app review of this
    exact content."""
    file_review: dict[str, Any] = dict(row.meta.get("review", {}))
    if file_review.get("status") == "reviewed":
        return {**file_review, "source": "file"}
    latest = next((r for r in history if r.content_hash == row.content_hash), None)
    if latest is not None and latest.verdict == "reviewed":
        return {
            "status": "reviewed",
            "reviewer": latest.reviewer,
            "reviewed_on": latest.created_at.date().isoformat(),
            "notes": latest.notes,
            "source": "app",
        }
    return {**file_review, "status": "pending"}


def state(row: AcademyLesson, history: list[AcademyReview]) -> str:
    if row.meta.get("review", {}).get("status") == "reviewed":
        return "reviewed"
    current = next((r for r in history if r.content_hash == row.content_hash), None)
    if current is not None:
        return "reviewed" if current.verdict == "reviewed" else "changes_requested"
    return "outdated" if history else "to_review"


def reviews_for(db: Session, rows: list[AcademyLesson]) -> dict[str, list[AcademyReview]]:
    return _history(db, [r.slug for r in rows])


def queue(db: Session) -> dict[str, Any]:
    """Every published or in-review lesson with its review state and review history."""
    rows = [r for r in academy.lessons(db, include_drafts=True) if r.status != "draft"]
    history = reviews_for(db, rows)
    counts: dict[str, int] = {}
    for q in db.scalars(select(AcademyQuestion).where(AcademyQuestion.active)):
        lesson_slug = q.id.split(":", 1)[0]
        counts[lesson_slug] = counts.get(lesson_slug, 0) + 1
    lessons = []
    for row in rows:
        h = history[row.slug]
        lessons.append(
            {
                "slug": row.slug,
                "title": row.title,
                "level": row.level,
                "version": row.version,
                "status": row.status,
                "state": state(row, h),
                "file_notes": row.meta.get("review", {}).get("notes", ""),
                "questions": counts.get(row.slug, 0),
                "history": [_record(r, row) for r in h],
            }
        )
    lessons.sort(key=lambda x: (STATES.index(x["state"]), x["level"]))
    totals = {s: sum(1 for x in lessons if x["state"] == s) for s in STATES}
    return {"totals": totals, "lessons": lessons}


def add(db: Session, slug: str, verdict: str, reviewer: str, notes: str) -> dict[str, Any]:
    row = academy.lesson(db, slug, include_drafts=True)
    r = AcademyReview(
        lesson_slug=row.slug,
        version=row.version,
        content_hash=row.content_hash,
        verdict=verdict,
        reviewer=reviewer,
        notes=notes.strip(),
        # Stamped here, not by now(): that is the transaction's start, so it can tie.
        created_at=datetime.now(UTC),
    )
    db.add(r)
    db.commit()
    return _record(r, row)
