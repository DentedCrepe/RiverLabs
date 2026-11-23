"""Guest mode (`X-RiverLab-Guest: 1`): answers are graded and games played, but nothing counts
towards the owner's progress, streak, goals, analytics or leaks."""

from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import func, select
from sqlalchemy.orm import Session, sessionmaker

from riverlab_api.db.models import (
    AcademyActivity,
    AcademyAttempt,
    AcademyProgress,
    PlayHand,
    PlaySession,
    PushFoldAnswer,
)
from riverlab_api.services import academy, play, trainer

pytestmark = pytest.mark.db

CONTENT = Path(__file__).resolve().parents[3] / "content" / "academy"
GUEST = {"X-RiverLab-Guest": "1"}


def _count(db: Session, model: Any) -> int:
    return db.scalar(select(func.count()).select_from(model)) or 0


def test_guest_academy_answers_are_graded_but_not_recorded(db: Session, db_client: TestClient) -> None:
    assert academy.sync(db, CONTENT).errors == []
    url = "/api/v1/academy/questions/pot-odds:half-pot/attempts"
    r = db_client.post(url, json={"answer": 25}, headers=GUEST)
    assert r.status_code == 200
    assert r.json()["correct"] is True
    assert _count(db, AcademyAttempt) == 0
    assert _count(db, AcademyActivity) == 0
    p = db_client.post("/api/v1/academy/lessons/pot-odds/progress", json={"completed": True}, headers=GUEST)
    assert p.json()["status"] == "completed", "the guest sees the result of their action"
    assert db.get(AcademyProgress, "pot-odds") is None, "but nothing is saved"
    assert db_client.get("/api/v1/academy/dashboard").json()["lessons_completed"] == 0
    b = db_client.post(
        "/api/v1/academy/bookmarks",
        json={"lesson": "pot-odds", "section": "break-even", "label": "x"},
        headers=GUEST,
    )
    assert b.status_code == 403
    assert b.json()["error"]["code"] == "guest_mode"
    # Without the header, the owner's answers are recorded as before.
    db_client.post(url, json={"answer": 25})
    assert _count(db, AcademyAttempt) == 1


@pytest.fixture
def manager(db: Session, tmp_path: Path) -> Iterator[Any]:
    play.manager = play.PlayManager(sessionmaker(bind=db.get_bind()), str(tmp_path), 1024, 1)
    yield play.manager
    play.manager.shutdown()
    play.manager = None


def test_guest_games_are_hidden_from_the_owners_statistics(
    db: Session, db_client: TestClient, manager: Any
) -> None:
    mine = PlaySession(format="cash", config={}, status="finished", hands=1)
    theirs = PlaySession(format="cash", config={}, status="finished", hands=2, tracked=False)
    db.add_all([mine, theirs])
    db.flush()
    db.add(PlayHand(session_id=mine.id, hand_no=1, record={"tournament": True}))
    for n in (1, 2):
        db.add(PlayHand(session_id=theirs.id, hand_no=n, record={"tournament": True}))
    db.flush()

    assert db_client.get("/api/v1/goals/today").json()["today"]["hands"] == 1
    owner_list = db_client.get("/api/v1/play/sessions").json()
    assert {(s["id"], s["tracked"]) for s in owner_list} == {(str(mine.id), True), (str(theirs.id), False)}
    guest_list = db_client.get("/api/v1/play/sessions", headers=GUEST).json()
    assert [(s["id"], s["tracked"]) for s in guest_list] == [(str(theirs.id), False)]
    assert db_client.get("/api/v1/play/hands").json()["total"] == 3, "the owner can browse every game"
    assert db_client.get("/api/v1/play/hands", headers=GUEST).json()["total"] == 2
    # Analytics: a guest sit & go (no hand records needed for the listing) is not yours.
    db.add(PlaySession(format="sit_and_go", config={}, status="finished", hands=3, tracked=False))
    db.flush()
    sng = db_client.get("/api/v1/analytics/play", params={"scope": "sit_and_go"}).json()
    assert sng["sessions"] == []
    assert sng["report"] is None

    # A guest cannot change the owner's goals, start batch reviews of the owner's hands, or
    # delete the owner's games.
    r = db_client.put("/api/v1/goals", json={"hands_target": 5, "study_target": 5}, headers=GUEST)
    assert r.status_code == 403
    assert db_client.post("/api/v1/play/reviews/batch", json={}, headers=GUEST).status_code == 403
    assert db_client.delete(f"/api/v1/play/sessions/{mine.id}", headers=GUEST).status_code == 403


def test_guest_push_fold_answers_are_not_stored(
    db: Session, db_client: TestClient, monkeypatch: pytest.MonkeyPatch
) -> None:
    graded = {
        "drill": {"stack_mbb": 10000, "seat": "sb", "cards": ["As", "Kd"], "class": "AKo"},
        "choice": "play",
        "verdict": "correct",
        "ev_loss_bb": 0.0,
    }

    class Fake:
        def grade(self, drill: dict[str, Any], choice: str) -> dict[str, Any]:
            return graded

    monkeypatch.setattr(trainer, "service", lambda: Fake())
    body = {"drill": {}, "choice": "play"}
    assert db_client.post("/api/v1/trainer/push-fold/answer", json=body, headers=GUEST).json() == graded
    assert _count(db, PushFoldAnswer) == 0
    db_client.post("/api/v1/trainer/push-fold/answer", json=body)
    assert _count(db, PushFoldAnswer) == 1


def test_a_table_can_be_started_untracked() -> None:
    from riverlab_api.schemas.play import PlaySessionCreate

    assert PlaySessionCreate(config={}).tracked is True
    assert PlaySessionCreate(config={}, tracked=False).tracked is False
