"""Academy: content sync and versioning, lessons without answers, grading, progress,
bookmarks, revision scheduling and the dashboard."""

import shutil
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import AcademyActivity, AcademyAttempt
from riverlab_api.services import academy

pytestmark = pytest.mark.db

CONTENT = Path(__file__).resolve().parents[3] / "content" / "academy"


@pytest.fixture
def synced(db: Session) -> Session:
    report = academy.sync(db, CONTENT)
    assert report.errors == []
    return db


def test_sync_is_idempotent_and_requires_a_version_bump(db: Session, tmp_path: Path) -> None:
    root = tmp_path / "academy"
    shutil.copytree(CONTENT, root)
    assert academy.sync(db, root).errors == []
    assert academy.sync(db, root).errors == []
    lesson = root / "lessons" / "pot-odds.md"
    lesson.write_text(lesson.read_text().replace("How much equity a call needs", "What equity a call needs"))
    errors = academy.sync(db, root).errors
    assert any("bump it to publish" in e for e in errors)
    assert academy.lesson(db, "pot-odds").summary.startswith("How much"), "the old content stays live"
    old = academy.lesson(db, "pot-odds").version
    lesson.write_text(lesson.read_text().replace(f"version = {old}", f"version = {old + 1}", 1))
    assert academy.sync(db, root).errors == []
    row = academy.lesson(db, "pot-odds")
    assert row.version == old + 1
    assert row.summary.startswith("What equity")


def test_lessons_hide_answers_and_grade_attempts(synced: Session, db_client: TestClient) -> None:
    lesson = db_client.get("/api/v1/academy/lessons/pot-odds").json()
    assert lesson["review"]["status"] == "pending"
    assert [s["id"] for s in lesson["sections"]][:2] == ["price", "break-even"]
    q = next(q for q in lesson["questions"] if q["id"] == "pot-odds:half-pot")
    assert "answer" not in q
    assert "explanation" not in q
    wrong = db_client.post(
        f"/api/v1/academy/questions/{q['id']}/attempts", json={"answer": 33, "confidence": 4}
    )
    assert wrong.json()["correct"] is False
    right = db_client.post(
        f"/api/v1/academy/questions/{q['id']}/attempts", json={"answer": 25.2, "time_ms": 9000}
    )
    body = right.json()
    assert body["correct"] is True
    assert body["answer"] == 25.0
    assert body["computed_by_engine"] is True
    mc = "pot-odds:which-equity"
    assert db_client.post(f"/api/v1/academy/questions/{mc}/attempts", json={"answer": 1}).json()["correct"]
    assert db_client.post(f"/api/v1/academy/questions/{mc}/attempts", json={"answer": 9}).status_code == 422
    assert (
        db_client.post(
            f"/api/v1/academy/questions/{mc}/attempts", json={"answer": 1, "confidence": 9}
        ).status_code
        == 422
    )
    assert db_client.post("/api/v1/academy/questions/nope:x/attempts", json={"answer": 1}).status_code == 404


def test_progress_bookmarks_and_dashboard(synced: Session, db_client: TestClient) -> None:
    dash = db_client.get("/api/v1/academy/dashboard").json()
    assert dash["lessons_published"] == len(list((CONTENT / "lessons").glob("*.md")))
    assert dash["recommended"]["slug"] == "hand-rankings", (
        "level 1 in order; its prerequisite topic (terminology) has no lesson, so it doesn't block"
    )
    assert dash["current_level"] == 1
    r = db_client.post(
        "/api/v1/academy/lessons/pot-odds/progress",
        json={"explanation_level": "math", "last_section": "assumptions"},
    )
    assert r.json()["explanation_level"] == "math"
    db_client.post("/api/v1/academy/lessons/pot-odds/progress", json={"completed": True, "saved": True})
    b = db_client.post(
        "/api/v1/academy/bookmarks",
        json={"lesson": "pot-odds", "section": "break-even", "label": "Break-even formula"},
    )
    assert b.status_code == 201
    assert (
        db_client.post(
            "/api/v1/academy/bookmarks", json={"lesson": "pot-odds", "section": "nope", "label": "x"}
        ).status_code
        == 404
    )
    dash = db_client.get("/api/v1/academy/dashboard").json()
    assert dash["lessons_completed"] == 1
    assert dash["recommended"]["slug"] == "hand-rankings", "still the first unfinished lesson in order"
    db_client.post("/api/v1/academy/lessons/hand-rankings/progress", json={"completed": True})
    assert db_client.get("/api/v1/academy/dashboard").json()["recommended"]["slug"] == "positions"
    assert dash["saved"] == [{"slug": "pot-odds", "title": "Pot odds and break-even equity"}]
    assert dash["bookmarks"][0]["section"] == "break-even"
    assert dash["streak_days"] >= 1
    curriculum = db_client.get("/api/v1/academy/curriculum").json()
    topic = next(t for t in curriculum[0]["topics"] if t["slug"] == "pot-odds")
    assert topic["status"] == "completed"
    assert next(t for t in curriculum[0]["topics"] if t["slug"] == "starting-hands")["status"] == "planned"
    lesson = db_client.get("/api/v1/academy/lessons/pot-odds").json()
    assert lesson["explanation_level"] == "math"
    assert lesson["progress"]["last_section"] == "assumptions"
    db_client.delete(f"/api/v1/academy/bookmarks/{b.json()['id']}")
    assert db_client.get("/api/v1/academy/dashboard").json()["bookmarks"] == []


def test_revision_schedule_and_misconceptions(synced: Session, db_client: TestClient) -> None:
    qid = "mdf-and-bluff-ratios:mdf-half"
    for _ in range(2):
        db_client.post(f"/api/v1/academy/questions/{qid}/attempts", json={"answer": 50})
    rev = db_client.get("/api/v1/academy/revision").json()
    assert [d["question"]["id"] for d in rev["due"]] == [qid], "wrong answers are due immediately"
    assert {"concept": "minimum-defence-frequency", "incorrect": 2} in rev["misconceptions"]
    db_client.post(f"/api/v1/academy/questions/{qid}/attempts", json={"answer": 66.7})
    assert db_client.get("/api/v1/academy/revision").json()["due"] == [], (
        "a correct answer is due again in a day"
    )
    later = academy.revision(synced, now=datetime.now(UTC) + timedelta(days=2))
    assert [d["question"]["id"] for d in later["due"]] == [qid]
    assert synced.scalar(select(AcademyActivity)) is not None
    assert len(list(synced.scalars(select(AcademyAttempt)))) == 3


def test_drafts_are_hidden_unless_requested(db: Session, db_client: TestClient, tmp_path: Path) -> None:
    root = tmp_path / "academy"
    shutil.copytree(CONTENT, root)
    lesson = root / "lessons" / "regret-matching.md"
    lesson.write_text(lesson.read_text().replace('status = "published"', 'status = "draft"', 1))
    assert academy.sync(db, root).errors == []
    assert db_client.get("/api/v1/academy/lessons/regret-matching").status_code == 404
    assert (
        db_client.get("/api/v1/academy/lessons/regret-matching", params={"include_drafts": True}).status_code
        == 200
    )


def test_theory_endpoint(client: TestClient) -> None:
    r = client.post("/api/v1/theory/bet_math", json={"params": {"pot_mbb": 10000, "bet_mbb": 10000}})
    assert r.status_code == 200
    assert r.json()["mdf"] == 0.5
    rm = client.post(
        "/api/v1/theory/regret_matching", json={"params": {"iterations": 50, "start_regrets": [1, 0, 0]}}
    ).json()
    assert len(rm) == 50
    assert client.post("/api/v1/theory/pot_odds", json={"params": {"pot_mbb": 1}}).status_code == 422
    assert client.post("/api/v1/theory/nope", json={"params": {}}).status_code == 422
