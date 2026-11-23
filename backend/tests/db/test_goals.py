"""Daily goals: today's progress, the streak and the study plan, from your own data only."""

from datetime import date, datetime, timedelta

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from riverlab_api.db.models import PlayHand, PlaySession, PushFoldAnswer
from riverlab_api.services import daily

pytestmark = pytest.mark.db


def test_streaks_count_consecutive_days_and_today_does_not_break_them() -> None:
    today = date(2026, 11, 23)
    d = lambda n: today - timedelta(days=n)  # noqa: E731
    assert daily._streaks(set(), today) == (0, 0)
    # Yesterday and the day before met; today not yet: the streak is still 2.
    assert daily._streaks({d(1), d(2)}, today) == (2, 2)
    assert daily._streaks({d(0), d(1), d(2)}, today) == (3, 3)
    # A gap ends it; the best streak is remembered.
    assert daily._streaks({d(0), d(2), d(3), d(4), d(5)}, today) == (1, 4)
    assert daily._streaks({d(2), d(3)}, today) == (0, 2)


def _local_noon(days_ago: int) -> datetime:
    now = datetime.now().astimezone()
    return (now - timedelta(days=days_ago)).replace(hour=12, minute=0, second=0, microsecond=0)


def test_plan_counts_today_and_the_streak(db_client: TestClient, db: Session) -> None:
    empty = db_client.get("/api/v1/goals/today").json()
    assert empty["targets"] == {"hands": 30, "study": 10}
    assert empty["today"] == {**empty["today"], "hands": 0, "study": 0, "done": False}
    assert empty["streak"] == 0
    assert len(empty["days"]) == 14
    assert [r["kind"] for r in empty["recommended"]][-1] == "trainer", "with no data, only a generic drill"

    r = db_client.put("/api/v1/goals", json={"hands_target": 2, "study_target": 1})
    assert r.json()["targets"] == {"hands": 2, "study": 1}
    assert db_client.put("/api/v1/goals", json={"hands_target": 0, "study_target": 1}).status_code == 422

    game = PlaySession(format="cash", config={}, status="finished")
    db.add(game)
    db.flush()
    n = 0
    for days_ago, hands, drills in [(0, 2, 1), (1, 3, 1), (2, 1, 1)]:  # two days ago: too few hands
        for _ in range(hands):
            n += 1
            db.add(
                PlayHand(
                    session_id=game.id,
                    hand_no=n,
                    record={"tournament": True},
                    created_at=_local_noon(days_ago),
                )
            )
        for _ in range(drills):
            db.add(
                PushFoldAnswer(
                    stack_mbb=10000,
                    seat="sb",
                    cards="AsKd",
                    hand_class="AKo",
                    choice="play",
                    verdict="correct",
                    ev_loss_bb=0.0,
                    grade={},
                    created_at=_local_noon(days_ago),
                )
            )
    db.flush()
    p = db_client.get("/api/v1/goals/today").json()
    assert p["today"]["hands"] == 2
    assert p["today"]["study"] == 1
    assert p["today"]["done"] is True
    assert p["streak"] == 2
    assert p["best_streak"] == 2
    assert [d["met"] for d in p["days"][-3:]] == [False, True, True]
