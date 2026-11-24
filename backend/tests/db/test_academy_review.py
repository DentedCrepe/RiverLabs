"""Lesson review by a person: the review page's queue, states and the lesson's review label."""

import shutil
from pathlib import Path

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from riverlab_api.services import academy

pytestmark = pytest.mark.db

CONTENT = Path(__file__).resolve().parents[3] / "content" / "academy"


def _lesson_state(client: TestClient, slug: str) -> dict[str, object]:
    q = client.get("/api/v1/academy/reviews").json()
    return next(x for x in q["lessons"] if x["slug"] == slug)


def test_review_queue_marks_and_outdates_reviews(db: Session, db_client: TestClient, tmp_path: Path) -> None:
    root = tmp_path / "academy"
    shutil.copytree(CONTENT, root)
    assert academy.sync(db, root).errors == []

    q = db_client.get("/api/v1/academy/reviews").json()
    assert q["totals"]["to_review"] == len(q["lessons"]) > 0
    assert q["totals"]["reviewed"] == 0
    assert _lesson_state(db_client, "pot-odds")["questions"] > 0

    url = "/api/v1/academy/lessons/pot-odds/reviews"
    assert db_client.post(url, json={"verdict": "changes_requested", "reviewer": "Sam"}).status_code == 422
    assert db_client.post(url, json={"verdict": "reviewed", "reviewer": "   "}).status_code == 422
    assert (
        db_client.post(
            url, json={"verdict": "reviewed", "reviewer": "Sam"}, headers={"X-RiverLab-Guest": "1"}
        )
    ).status_code == 403
    r = db_client.post(url, json={"verdict": "changes_requested", "reviewer": "Sam", "notes": "Reword q2"})
    assert r.status_code == 201
    assert _lesson_state(db_client, "pot-odds")["state"] == "changes_requested"
    assert db_client.get("/api/v1/academy/lessons/pot-odds").json()["review"]["status"] == "pending"

    db_client.post(url, json={"verdict": "reviewed", "reviewer": "Sam", "notes": "Fine now"})
    s = _lesson_state(db_client, "pot-odds")
    assert s["state"] == "reviewed"
    assert [h["verdict"] for h in s["history"]] == ["reviewed", "changes_requested"]
    review = db_client.get("/api/v1/academy/lessons/pot-odds").json()["review"]
    assert review["status"] == "reviewed"
    assert review["reviewer"] == "Sam"
    assert review["source"] == "app"

    # Editing the lesson (with its version bump) sends it back to the queue as outdated.
    f = root / "lessons" / "pot-odds.md"
    text = f.read_text()
    version = int(text.split("version = ", 1)[1].split("\n", 1)[0])
    f.write_text(
        text.replace(f"version = {version}", f"version = {version + 1}", 1).replace(
            "## ", "Edited for the test.\n\n## ", 1
        )
    )
    assert academy.sync(db, root).errors == []
    s = _lesson_state(db_client, "pot-odds")
    assert s["state"] == "outdated"
    assert all(not h["current"] for h in s["history"])
    assert db_client.get("/api/v1/academy/lessons/pot-odds").json()["review"]["status"] == "pending"
    assert (
        db_client.post(
            "/api/v1/academy/lessons/nope/reviews", json={"verdict": "reviewed", "reviewer": "S"}
        ).status_code
        == 404
    )
