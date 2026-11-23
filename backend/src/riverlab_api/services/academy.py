"""RiverLab Academy: syncing content into the database, progress, grading and the dashboard."""

import logging
from datetime import UTC, date, datetime, timedelta
from pathlib import Path
from typing import Any

from sqlalchemy import func, select
from sqlalchemy.dialects.postgresql import insert
from sqlalchemy.orm import Session

from riverlab_api.db.models import (
    AcademyActivity,
    AcademyAttempt,
    AcademyBookmark,
    AcademyLesson,
    AcademyLessonVersion,
    AcademyProgress,
    AcademyQuestion,
)
from riverlab_api.errors import ConflictError, EngineValidationError, NotFoundError
from riverlab_api.services.academy_content import ContentReport, read_content

logger = logging.getLogger(__name__)

# The curriculum outline from the last sync (curriculum.toml), served with lesson status.
_curriculum: list[dict[str, Any]] = []

# Spaced revision: after n consecutive correct answers, a question is due again in this many days.
REVIEW_INTERVALS_DAYS = (1, 3, 7, 21, 60)


def sync(db: Session, root: Path) -> ContentReport:
    """Validates the content directory and upserts every valid lesson.

    A published lesson whose content changed must also change its version: otherwise the
    change is rejected (the old content stays live) so learners never see silent edits.
    """
    global _curriculum
    report = read_content(root)
    if report.curriculum:
        _curriculum = report.curriculum
    for parsed in report.lessons:
        row = db.scalar(select(AcademyLesson).where(AcademyLesson.slug == parsed.slug))
        if row is not None and row.content_hash == parsed.content_hash:
            continue
        if row is not None and row.version == parsed.version and parsed.status == "published":
            report.errors.append(
                f"{parsed.source_path}: content changed but version is still {parsed.version}; "
                "bump it to publish"
            )
            continue
        if row is None:
            row = AcademyLesson(slug=parsed.slug)
            db.add(row)
        row.level, row.position, row.title, row.summary = (
            parsed.level,
            parsed.position,
            parsed.title,
            parsed.summary,
        )
        row.status, row.version, row.content_hash = parsed.status, parsed.version, parsed.content_hash
        row.meta, row.sections, row.source_path = parsed.meta, parsed.sections, parsed.source_path
        row.synced_at = datetime.now(UTC)
        db.flush()
        exists = db.scalar(
            select(AcademyLessonVersion).where(
                AcademyLessonVersion.lesson_id == row.id, AcademyLessonVersion.version == parsed.version
            )
        )
        if exists is None:
            db.add(
                AcademyLessonVersion(
                    lesson_id=row.id,
                    version=parsed.version,
                    content_hash=parsed.content_hash,
                    status=parsed.status,
                    meta=parsed.meta,
                    sections=parsed.sections,
                )
            )
        else:  # a draft edited in place
            exists.content_hash, exists.status = parsed.content_hash, parsed.status
            exists.meta, exists.sections = parsed.meta, parsed.sections
        current = {f"{parsed.slug}:{q['id']}" for q in parsed.questions}
        for q in db.scalars(select(AcademyQuestion).where(AcademyQuestion.lesson_id == row.id)):
            q.active = q.id in current
        for pq in parsed.questions:
            qid = f"{parsed.slug}:{pq['id']}"
            existing = db.get(AcademyQuestion, qid)
            if existing is None:
                existing = AcademyQuestion(id=qid, lesson_id=row.id)
                db.add(existing)
            existing.kind, existing.prompt, existing.data = pq["kind"], pq["prompt"], pq["data"]
            existing.concepts, existing.active = pq["concepts"], True
    db.commit()
    for e in report.errors:
        logger.warning("academy content: %s", e)
    return report


def _visible(include_drafts: bool) -> Any:
    return True if include_drafts else AcademyLesson.status == "published"


def lessons(db: Session, include_drafts: bool = False) -> list[AcademyLesson]:
    q = (
        select(AcademyLesson)
        .where(_visible(include_drafts))
        .order_by(AcademyLesson.level, AcademyLesson.position)
    )
    return list(db.scalars(q))


def lesson(db: Session, slug: str, include_drafts: bool = False) -> AcademyLesson:
    row = db.scalar(select(AcademyLesson).where(AcademyLesson.slug == slug, _visible(include_drafts)))
    if row is None:
        raise NotFoundError(f"Lesson {slug!r} not found")
    return row


def public_question(q: AcademyQuestion) -> dict[str, Any]:
    """A question without its answer."""
    d = q.data
    return {
        "id": q.id,
        "kind": q.kind,
        "prompt": q.prompt,
        "options": d.get("options"),
        "unit": d.get("unit"),
        "scene": d.get("scene"),
        "concepts": q.concepts,
    }


def questions_for(db: Session, lesson_id: Any) -> list[AcademyQuestion]:
    q = select(AcademyQuestion).where(AcademyQuestion.lesson_id == lesson_id, AcademyQuestion.active)
    return list(db.scalars(q.order_by(AcademyQuestion.id)))


def _touch_activity(db: Session) -> None:
    stmt = insert(AcademyActivity).values(day=date.today(), events=1)
    db.execute(
        stmt.on_conflict_do_update(index_elements=["day"], set_={"events": AcademyActivity.events + 1})
    )


def update_progress(db: Session, slug: str, changes: dict[str, Any], store: bool = True) -> dict[str, Any]:
    """Applies `changes` to a lesson's progress. With `store=False` (guest mode) the result is
    returned but nothing is saved."""
    lesson(db, slug)
    row = db.get(AcademyProgress, slug)
    if row is None:
        row = AcademyProgress(lesson_slug=slug, status="started", explanation_level="simple", saved=False)
        db.add(row)
    if changes.get("last_section") is not None:
        row.last_section = changes["last_section"]
    if changes.get("explanation_level") is not None:
        row.explanation_level = changes["explanation_level"]
    if changes.get("saved") is not None:
        row.saved = changes["saved"]
    if changes.get("completed") is True and row.status != "completed":
        row.status, row.completed_at = "completed", datetime.now(UTC)
    elif changes.get("completed") is False:
        row.status, row.completed_at = "started", None
    out = {
        "status": row.status,
        "saved": row.saved,
        "last_section": row.last_section,
        "explanation_level": row.explanation_level,
    }
    if not store:
        db.rollback()
        return out
    _touch_activity(db)
    db.commit()
    return out


def grade(q: AcademyQuestion, answer: Any) -> bool:
    d = q.data
    if q.kind == "numeric":
        if isinstance(answer, bool) or not isinstance(answer, int | float):
            raise EngineValidationError("A numeric answer is required")
        return abs(float(answer) - float(d["answer"])) <= float(d["tolerance"]) + 1e-9
    if isinstance(answer, bool) or not isinstance(answer, int) or not 0 <= answer < len(d["options"]):
        raise EngineValidationError("Answer with the index of one option")
    return bool(answer == d["answer"])


def attempt(
    db: Session,
    question_id: str,
    answer: Any,
    confidence: int | None,
    time_ms: int | None,
    store: bool = True,
) -> dict[str, Any]:
    """Grades an answer and records it (not in guest mode, `store=False`)."""
    q = db.get(AcademyQuestion, question_id)
    if q is None or not q.active:
        raise NotFoundError(f"Question {question_id!r} not found")
    correct = grade(q, answer)
    if store:
        db.add(
            AcademyAttempt(
                question_id=q.id, answer=answer, correct=correct, confidence=confidence, time_ms=time_ms
            )
        )
        _touch_activity(db)
        db.commit()
    d = q.data
    return {
        "question_id": q.id,
        "correct": correct,
        "answer": d["answer"],
        "tolerance": d.get("tolerance"),
        "explanation": d["explanation"],
        "computed_by_engine": bool(d.get("computed")),
    }


def _question_state(db: Session) -> dict[str, dict[str, Any]]:
    """Per question: attempts in order -> streak of correct answers, last time, next due."""
    state: dict[str, dict[str, Any]] = {}
    for a in db.scalars(select(AcademyAttempt).order_by(AcademyAttempt.created_at)):
        s = state.setdefault(
            a.question_id, {"attempts": 0, "correct": 0, "streak": 0, "last": None, "ever_wrong": False}
        )
        s["attempts"] += 1
        s["correct"] += int(a.correct)
        s["streak"] = s["streak"] + 1 if a.correct else 0
        s["ever_wrong"] |= not a.correct
        s["last"] = a.created_at
        s["last_correct"] = a.correct
    for s in state.values():
        if s["streak"] == 0:
            s["due"] = s["last"]
        else:
            days = REVIEW_INTERVALS_DAYS[min(s["streak"], len(REVIEW_INTERVALS_DAYS)) - 1]
            s["due"] = s["last"] + timedelta(days=days)
    return state


def revision(db: Session, now: datetime | None = None) -> dict[str, Any]:
    now = now or datetime.now(UTC)
    state = _question_state(db)
    questions = {q.id: q for q in db.scalars(select(AcademyQuestion).where(AcademyQuestion.active))}
    due = [
        {"question": public_question(questions[qid]), "last_correct": s["last_correct"], "due": s["due"]}
        for qid, s in state.items()
        if qid in questions and s["due"] <= now
    ]
    due.sort(key=lambda d: (d["last_correct"], d["due"]))
    # Misconceptions: concepts answered wrongly more than once.
    wrong: dict[str, int] = {}
    for a in db.scalars(select(AcademyAttempt).where(AcademyAttempt.correct.is_(False))):
        q = questions.get(a.question_id)
        for c in q.concepts if q else []:
            wrong[c] = wrong.get(c, 0) + 1
    misconceptions = [
        {"concept": c, "incorrect": n} for c, n in sorted(wrong.items(), key=lambda kv: -kv[1]) if n >= 2
    ]
    # Retention: of questions answered correctly and re-asked later, how many were right again.
    reasked = [s for s in state.values() if s["attempts"] > 1]
    retention = sum(1 for s in reasked if s["last_correct"]) / len(reasked) if reasked else None
    return {"due": due, "misconceptions": misconceptions, "retention": retention}


def _streak(db: Session, today: date) -> int:
    days = set(
        db.scalars(select(AcademyActivity.day).where(AcademyActivity.day > today - timedelta(days=400)))
    )
    start = today if today in days else today - timedelta(days=1)
    n = 0
    while start - timedelta(days=n) in days:
        n += 1
    return n


def curriculum(db: Session) -> list[dict[str, Any]]:
    by_slug = {row.slug: row for row in lessons(db)}
    progress = {p.lesson_slug: p for p in db.scalars(select(AcademyProgress))}
    out = []
    for lv in _curriculum:
        topics = []
        for t in lv.get("topics", []):
            slug = t.get("lesson")
            row = by_slug.get(slug) if slug else None
            p = progress.get(slug) if slug else None
            topics.append(
                {
                    "slug": t["slug"],
                    "title": t["title"],
                    "lesson": slug if row else None,
                    "status": (
                        "completed" if p and p.status == "completed" else "started" if p else "available"
                    )
                    if row
                    else "planned",
                }
            )
        out.append(
            {"number": lv["number"], "title": lv["title"], "summary": lv.get("summary", ""), "topics": topics}
        )
    return out


def dashboard(db: Session) -> dict[str, Any]:
    published = lessons(db)
    progress = {p.lesson_slug: p for p in db.scalars(select(AcademyProgress))}
    completed = [
        row for row in published if progress.get(row.slug) and progress[row.slug].status == "completed"
    ]
    done = {row.slug for row in completed}
    # Topics covered by completed lessons count as met prerequisites; topics without a
    # lesson yet never block a recommendation.
    covered = {t["slug"] for lv in _curriculum for t in lv.get("topics", []) if t.get("lesson") in done}
    lesson_topics = {t["slug"] for lv in _curriculum for t in lv.get("topics", []) if t.get("lesson")}
    recommended = next(
        (
            row
            for row in published
            if row.slug not in done
            and all(p in covered or p not in lesson_topics for p in row.meta.get("prerequisites", []))
        ),
        None,
    )
    current_level = min((row.level for row in published if row.slug not in done), default=None)
    per_level = []
    for n in range(1, 7):
        lv = [row for row in published if row.level == n]
        per_level.append(
            {"level": n, "lessons": len(lv), "completed": sum(1 for row in lv if row.slug in done)}
        )
    # Assessment results per lesson: latest attempt per question.
    latest: dict[str, bool] = {}
    for a in db.scalars(select(AcademyAttempt).order_by(AcademyAttempt.created_at)):
        latest[a.question_id] = a.correct
    results = []
    for row in published:
        qs = [q.id for q in questions_for(db, row.id)]
        answered = [latest[q] for q in qs if q in latest]
        if answered:
            results.append(
                {
                    "lesson": row.slug,
                    "title": row.title,
                    "questions": len(qs),
                    "answered": len(answered),
                    "correct": sum(answered),
                }
            )
    totals = db.execute(
        select(func.count(), func.count().filter(AcademyAttempt.correct), func.avg(AcademyAttempt.time_ms))
    ).one()
    rev = revision(db)
    title = {row.slug: row.title for row in published}
    return {
        "lessons_published": len(published),
        "lessons_completed": len(completed),
        "current_level": current_level,
        "per_level": per_level,
        "recommended": {"slug": recommended.slug, "title": recommended.title, "level": recommended.level}
        if recommended
        else None,
        "completed": [{"slug": r.slug, "title": r.title, "level": r.level} for r in completed],
        "assessments": results,
        "answers": {
            "total": totals[0],
            "correct": totals[1],
            "avg_time_ms": float(totals[2]) if totals[2] else None,
        },
        "streak_days": _streak(db, date.today()),
        "saved": [{"slug": s, "title": title[s]} for s, p in progress.items() if p.saved and s in title],
        "bookmarks": [
            {
                "id": str(b.id),
                "lesson": b.lesson_slug,
                "section": b.section_id,
                "label": b.label,
                "note": b.note,
            }
            for b in db.scalars(select(AcademyBookmark).order_by(AcademyBookmark.created_at.desc()))
        ],
        "revision": {
            "due": len(rev["due"]),
            "misconceptions": rev["misconceptions"],
            "retention": rev["retention"],
        },
    }


def add_bookmark(db: Session, slug: str, section: str, label: str, note: str | None) -> AcademyBookmark:
    row = lesson(db, slug)
    if not any(s["id"] == section for s in row.sections):
        raise NotFoundError(f"Section {section!r} not found in {slug!r}")
    existing = db.scalar(
        select(AcademyBookmark).where(
            AcademyBookmark.lesson_slug == slug, AcademyBookmark.section_id == section
        )
    )
    if existing is not None:
        raise ConflictError("This section is already bookmarked")
    b = AcademyBookmark(lesson_slug=slug, section_id=section, label=label, note=note)
    db.add(b)
    db.commit()
    return b


def delete_bookmark(db: Session, bookmark_id: Any) -> None:
    b = db.get(AcademyBookmark, bookmark_id)
    if b is None:
        raise NotFoundError("Bookmark not found")
    db.delete(b)
    db.commit()
