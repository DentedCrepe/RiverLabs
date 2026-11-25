"""RiverLab Academy: curriculum, lessons, progress, assessments and revision (docs/academy.md)."""

import uuid
from pathlib import Path
from typing import Annotated, Any

from fastapi import APIRouter, Depends, Response
from sqlalchemy.orm import Session

from riverlab_api.api.guest import Guest, GuestForbiddenError
from riverlab_api.config import get_settings
from riverlab_api.db.models import AcademyLesson, AcademyProgress
from riverlab_api.db.session import get_db
from riverlab_api.schemas.academy import AttemptIn, BookmarkIn, ProgressUpdate, ReviewIn, TutorIn
from riverlab_api.services import academy, academy_review, tutor

router = APIRouter(prefix="/academy", tags=["academy"])
DbSession = Annotated[Session, Depends(get_db)]


def _summary(row: AcademyLesson, progress: AcademyProgress | None, db: Session) -> dict[str, Any]:
    history = academy_review.reviews_for(db, [row])[row.slug]
    return {
        "slug": row.slug,
        "title": row.title,
        "summary": row.summary,
        "level": row.level,
        "status": row.status,
        "version": row.version,
        "review": academy_review.review_of(row, history),
        "estimated_minutes": row.meta.get("estimated_minutes"),
        "tags": row.meta.get("tags", []),
        "progress": None
        if progress is None
        else {"status": progress.status, "saved": progress.saved, "last_section": progress.last_section},
    }


@router.get("/dashboard")
def dashboard(db: DbSession) -> dict[str, Any]:
    return academy.dashboard(db)


@router.get("/curriculum")
def curriculum(db: DbSession) -> list[dict[str, Any]]:
    return academy.curriculum(db)


@router.get("/lessons")
def list_lessons(db: DbSession, include_drafts: bool = False) -> list[dict[str, Any]]:
    return [_summary(r, db.get(AcademyProgress, r.slug), db) for r in academy.lessons(db, include_drafts)]


@router.get("/lessons/{slug}")
def get_lesson(slug: str, db: DbSession, include_drafts: bool = False) -> dict[str, Any]:
    row = academy.lesson(db, slug, include_drafts)
    p = db.get(AcademyProgress, slug)
    return {
        **_summary(row, p, db),
        "explanation_level": p.explanation_level if p else "simple",
        "meta": row.meta,
        "sections": row.sections,
        "questions": [academy.public_question(q) for q in academy.questions_for(db, row.id)],
        "bookmarks": [b for b in academy.dashboard(db)["bookmarks"] if b["lesson"] == slug],
    }


@router.post("/lessons/{slug}/progress")
def update_progress(slug: str, body: ProgressUpdate, db: DbSession, guest: Guest) -> dict[str, Any]:
    """Saves your place, level and completion (in guest mode nothing is saved)."""
    return academy.update_progress(db, slug, body.model_dump(exclude_none=True), store=not guest)


@router.post("/questions/{question_id}/attempts")
def attempt(question_id: str, body: AttemptIn, db: DbSession, guest: Guest) -> dict[str, Any]:
    """Grades an answer; recorded for revision and your streak, except in guest mode."""
    return academy.attempt(db, question_id, body.answer, body.confidence, body.time_ms, store=not guest)


@router.get("/revision")
def revision(db: DbSession) -> dict[str, Any]:
    return academy.revision(db)


@router.post("/bookmarks", status_code=201)
def add_bookmark(body: BookmarkIn, db: DbSession, guest: Guest) -> dict[str, Any]:
    if guest:
        raise GuestForbiddenError("Guests cannot change the owner's bookmarks")
    b = academy.add_bookmark(db, body.lesson, body.section, body.label, body.note)
    return {
        "id": str(b.id),
        "lesson": b.lesson_slug,
        "section": b.section_id,
        "label": b.label,
        "note": b.note,
    }


@router.delete("/bookmarks/{bookmark_id}", status_code=204)
def delete_bookmark(bookmark_id: uuid.UUID, db: DbSession, guest: Guest) -> Response:
    if guest:
        raise GuestForbiddenError("Guests cannot change the owner's bookmarks")
    academy.delete_bookmark(db, bookmark_id)
    return Response(status_code=204)


@router.post("/sync")
def sync(db: DbSession) -> dict[str, Any]:
    """Re-reads the content directory (for authors: edit a lesson file, then sync)."""
    report = academy.sync(db, Path(get_settings().academy_content_dir))
    return {
        "lessons": [lesson.slug for lesson in report.lessons],
        "errors": report.errors,
        "warnings": report.warnings,
    }


@router.get("/lessons/{slug}/versions")
def versions(slug: str, db: DbSession, include_drafts: bool = True) -> list[dict[str, Any]]:
    from sqlalchemy import select

    from riverlab_api.db.models import AcademyLessonVersion

    row = academy.lesson(db, slug, include_drafts)
    q = select(AcademyLessonVersion).where(AcademyLessonVersion.lesson_id == row.id)
    return [
        {"version": v.version, "status": v.status, "content_hash": v.content_hash, "created_at": v.created_at}
        for v in db.scalars(q.order_by(AcademyLessonVersion.version))
    ]


@router.get("/reviews")
def reviews(db: DbSession) -> dict[str, Any]:
    """Lessons to review, with their state and every review made in the app."""
    return academy_review.queue(db)


@router.post("/lessons/{slug}/reviews", status_code=201)
def add_review(slug: str, body: ReviewIn, db: DbSession, guest: Guest) -> dict[str, Any]:
    """A person's review of the lesson's current version: "reviewed", or "changes_requested"
    with notes on what to fix."""
    if guest:
        raise GuestForbiddenError("Guests cannot review the owner's lessons")
    return academy_review.add(db, slug, body.verdict, body.reviewer, body.notes)


@router.get("/tutor")
def tutor_status() -> dict[str, Any]:
    """Whether the AI tutor can answer (an Anthropic API key is set and it is turned on)."""
    return tutor.status()


@router.post("/lessons/{slug}/tutor")
def ask_tutor(slug: str, body: TutorIn, db: DbSession) -> dict[str, Any]:
    """An AI tutor answer about a lesson (docs/academy-ai-tutor.md): numbers come only from
    RiverLab's engine, the lesson or the conversation, and untraced ones are flagged."""
    return tutor.ask(
        db, slug, body.level, body.question, [t.model_dump() for t in body.history], body.consent
    )
