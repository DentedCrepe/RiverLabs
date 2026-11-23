"""Games against bots through the API, with committed database state."""

import time
import uuid
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import Engine, text
from sqlalchemy.orm import Session, sessionmaker

from riverlab_api.db.models import PlaySession
from riverlab_api.db.session import get_db
from riverlab_api.main import create_app
from riverlab_api.services import engine, play

pytestmark = pytest.mark.db

# Shared with test_local_solver: GTOpen's equity table and solved bot models (git-ignored).
CACHE = Path(__file__).resolve().parents[2] / ".cache"


def _clean(db_engine: Engine) -> None:
    with db_engine.begin() as conn:
        conn.execute(text("DELETE FROM play_hands; DELETE FROM play_sessions"))


@pytest.fixture
def live(db_engine: Engine) -> Iterator[tuple[TestClient, sessionmaker[Session]]]:
    _clean(db_engine)
    factory = sessionmaker(bind=db_engine, expire_on_commit=False)
    app = create_app()

    def real_db() -> Iterator[Session]:
        with factory() as s:
            yield s

    app.dependency_overrides[get_db] = real_db
    play.manager = play.PlayManager(factory, str(CACHE), 1024, 2)
    try:
        yield TestClient(app), factory
    finally:
        play.manager.shutdown()
        play.manager = None
        _clean(db_engine)


def cash_config(players: int = 2) -> dict[str, Any]:
    game = engine.default_game_config()
    game["players"] = players
    game["stack"]["effective_stack"] = 20_000  # 20bb: a small push/fold-sized model, fast to solve
    return {
        "format": {"type": "cash", "game": game, "auto_rebuy": True},
        "opponents": ["hard"] * (players - 1),
        "seed": 7,
    }


def wait_active(client: TestClient, sid: str, timeout: float = 300) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state: dict[str, Any] = client.get(f"/api/v1/play/sessions/{sid}").json()
        if state["session"]["status"] != "preparing":
            return state
        time.sleep(0.2)
    pytest.fail("the table was not prepared in time")


def play_hand(client: TestClient, sid: str) -> dict[str, Any]:
    state: dict[str, Any] = client.post(f"/api/v1/play/sessions/{sid}/next").json()
    while state["table"]["status"] == "awaiting_hero":
        legal = state["table"]["legal"]
        action = {"type": "check"} if legal["check"] else {"type": "call"}
        r = client.post(f"/api/v1/play/sessions/{sid}/act", json={"action": action})
        assert r.status_code == 200, r.json()
        state = r.json()
    return state


def test_heads_up_cash_game_is_played_recorded_and_reported(
    live: tuple[TestClient, sessionmaker[Session]],
) -> None:
    client, factory = live
    r = client.post("/api/v1/play/sessions", json={"config": cash_config()})
    assert r.status_code == 201, r.json()
    sid = r.json()["session"]["id"]
    assert r.json()["session"]["status"] == "preparing"
    state = wait_active(client, sid)
    assert state["session"]["status"] == "active", state
    assert state["table"]["seats"][0]["name"] == "You"

    for _ in range(6):
        state = play_hand(client, sid)
        assert state["table"]["status"] == "hand_over"
    assert state["session"]["hands"] == 6
    assert len(client.get(f"/api/v1/play/sessions/{sid}/hands").json()) == 6

    # An illegal action is rejected by the engine, not corrected.
    state = client.post(f"/api/v1/play/sessions/{sid}/next").json()
    if state["table"]["status"] == "awaiting_hero":
        bad = client.post(f"/api/v1/play/sessions/{sid}/act", json={"action": {"type": "raise", "to": 1}})
        assert bad.status_code == 422

    # Feedback on each decision and a result line for the hand.
    last = client.get(f"/api/v1/play/sessions/{sid}").json()["table"]
    assert last["summary"]
    assert all(" win" in line for line in last["summary"])
    revealed = client.post(f"/api/v1/play/sessions/{sid}/reveal", json={"on": True}).json()
    assert revealed["table"]["reveal"] is True
    state = play_hand(client, sid)
    assert state["table"]["feedback"], "each of your decisions gets feedback"
    assert {f["feedback"]["verdict"] for f in state["table"]["feedback"]} <= {
        "mistake",
        "rare",
        "ok",
        "not_graded",
    }

    report = client.get(f"/api/v1/play/sessions/{sid}/report").json()
    assert report["hands"] >= 7
    assert any("revealed" in c for c in report["caveats"])
    assert report["stats"]["vpip"]["opportunities"] == report["hands"]
    assert any("small sample" in c for c in report["caveats"])
    assert all(f["measured_against"] for f in report["findings"])

    # "Solve this spot": a heads-up decision after the flop becomes a postflop solve spec.
    hand, index = None, -1
    for _ in range(40):
        play_hand(client, sid)
        last_hand = client.get(f"/api/v1/play/sessions/{sid}/hands").json()[-1]
        spots = [i for i, d in enumerate(last_hand["hero_decisions"]) if d.get("spot")]
        if spots:
            hand, index = last_hand, spots[0]
            break
    assert hand is not None, "a heads-up postflop decision within 40 hands"
    url = f"/api/v1/play/sessions/{sid}/hands/{hand['hand_no']}/spot"
    r = client.get(url, params={"decision": index})
    assert r.status_code == 200, r.json()
    spot = r.json()
    assert len(spot["spec"]["ranges"]) == 2
    assert len(spot["spec"]["board"]) >= 3
    assert spot["hero_class"]
    assert spot["notes"]
    # The spec is accepted by the solver as is.
    engine.validate_solve_spec(spot["spec"])
    preflop = next(i for i, d in enumerate(hand["hero_decisions"]) if d["street"] == "preflop")
    assert client.get(url, params={"decision": preflop}).status_code == 422
    missing = f"/api/v1/play/sessions/{sid}/hands/99999/spot"
    assert client.get(missing, params={"decision": 0}).status_code == 404

    # Analytics: the same hands, as one report across sessions.
    a = client.get("/api/v1/analytics/play", params={"scope": "cash"}).json()
    assert [x["id"] for x in a["sessions"]] == [sid]
    stored = len(client.get(f"/api/v1/play/sessions/{sid}/hands").json())
    assert a["report"]["hands"] == stored
    # Hands across sessions, newest first, filterable by the feedback's verdict.
    page = client.get("/api/v1/play/hands", params={"limit": 5}).json()
    assert page["total"] == stored
    assert len(page["hands"]) == min(5, stored)
    assert page["hands"][0]["record"]["hand_no"] == stored
    assert page["hands"][0]["session_id"] == sid
    flagged = client.get("/api/v1/play/hands", params={"verdict": "mistake", "limit": 100}).json()
    for h in flagged["hands"]:
        assert any(
            (d.get("feedback") or {}).get("verdict") == "mistake" for d in h["record"]["hero_decisions"]
        )
    assert client.get("/api/v1/play/hands", params={"verdict": "ok"}).status_code == 422
    sng = client.get("/api/v1/analytics/play", params={"scope": "sit_and_go"}).json()
    assert sng["sessions"] == []
    assert sng["report"] is None

    left = client.post(f"/api/v1/play/sessions/{sid}/leave").json()
    assert left["session"]["status"] == "finished"
    assert client.post(f"/api/v1/play/sessions/{sid}/next").status_code == 409
    # The report is still available after leaving.
    final = client.get(f"/api/v1/play/sessions/{sid}/report")
    assert final.status_code == 200
    report = final.json()
    with factory() as s:
        row = s.get(PlaySession, uuid.UUID(sid))
        assert row is not None
        assert row.hero_net == report["net"]
    assert client.delete(f"/api/v1/play/sessions/{sid}").status_code == 204
    assert client.get(f"/api/v1/play/sessions/{sid}").status_code == 404


def test_invalid_tables_and_interrupted_games(live: tuple[TestClient, sessionmaker[Session]]) -> None:
    client, factory = live
    bad = cash_config(3)
    bad["opponents"] = ["hard"]
    r = client.post("/api/v1/play/sessions", json={"config": bad})
    assert r.status_code == 422
    assert "seats" in r.json()["error"]["message"]
    with factory() as s:
        s.add(PlaySession(format="cash", config=cash_config(), status="active"))
        s.commit()
        assert play.fail_interrupted_games(s) == 1
    listed = client.get("/api/v1/play/sessions").json()
    assert listed[0]["status"] == "interrupted"
    conflict = client.post(f"/api/v1/play/sessions/{listed[0]['id']}/next")
    assert conflict.status_code == 409
    assert "report is still available" in conflict.json()["error"]["message"]
