"""Solver review of a whole hand through the API: chained street solves, run by the background
reviewer against the real local solver (committed database state)."""

import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import Engine, text
from sqlalchemy.orm import Session, sessionmaker

from riverlab_api.db.session import get_db
from riverlab_api.main import create_app
from riverlab_api.services import compute, engine, hand_review, play
from riverlab_api.services.local_solver import LocalSolverBackend

pytestmark = pytest.mark.db
CACHE = Path(__file__).resolve().parents[2] / ".cache"
MAX_MB = 300  # turn and river trees fit; a full-range flop does not, so its solve fails


def _clean(db_engine: Engine) -> None:
    with db_engine.begin() as conn:
        conn.execute(
            text(
                "DELETE FROM hand_reviews; DELETE FROM play_hands; DELETE FROM play_sessions; "
                "DELETE FROM strategy_nodes; DELETE FROM solver_results; DELETE FROM solver_jobs"
            )
        )


@pytest.fixture
def live(db_engine: Engine, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[TestClient]:
    _clean(db_engine)
    factory = sessionmaker(bind=db_engine, expire_on_commit=False)
    app = create_app()

    def real_db() -> Iterator[Session]:
        with factory() as s:
            yield s

    app.dependency_overrides[get_db] = real_db
    backend = LocalSolverBackend(
        factory,
        max_memory_mb=MAX_MB,
        threads=2,
        storage_dir=str(tmp_path / "results"),
        equity_cache_dir=str(CACHE),
    )
    compute.registry.register(backend)
    play.manager = play.PlayManager(factory, str(CACHE), 1024, 2)
    hand_review.reviewer = hand_review.HandReviewer(factory, MAX_MB * 1024 * 1024)
    # Few iterations per street keep the test quick; the chain is what is under test.
    real_spec = engine.review_street_spec

    def quick(*args: Any, **kwargs: Any) -> dict[str, Any]:
        spec = real_spec(*args, **kwargs)
        spec["spec"]["solver"]["stopping"]["max_iterations"] = 30
        spec["spec"]["solver"]["stopping"]["exploitability_check_interval"] = 30
        return spec

    monkeypatch.setattr(engine, "review_street_spec", quick)
    try:
        yield TestClient(app)
    finally:
        hand_review.reviewer.shutdown()
        hand_review.reviewer = None
        play.manager.shutdown()
        play.manager = None
        compute.registry.backends.remove(backend)
        backend.shutdown()
        _clean(db_engine)


def test_a_hand_is_reviewed_street_by_street(live: TestClient) -> None:
    client = live
    g = engine.default_game_config()
    g["players"] = 2
    cfg = {"format": {"type": "cash", "game": g, "auto_rebuy": True}, "opponents": ["medium"], "seed": 23}
    sid = client.post("/api/v1/play/sessions", json={"config": cfg}).json()["session"]["id"]
    deadline = time.monotonic() + 300
    while client.get(f"/api/v1/play/sessions/{sid}").json()["session"]["status"] == "preparing":
        assert time.monotonic() < deadline
        time.sleep(0.2)
    hand_no = None
    for _ in range(300):
        state = client.post(f"/api/v1/play/sessions/{sid}/next").json()
        while state["table"]["status"] == "awaiting_hero":
            t = state["table"]
            legal = t["legal"]
            if len(t["board"]) == 4 and legal["bet"]:
                action = {"type": "bet", "to": max(legal["bet"][0], t["pot"] // 2)}
            else:
                action = {"type": "check"} if legal["check"] else {"type": "call"}
            state = client.post(f"/api/v1/play/sessions/{sid}/act", json={"action": action}).json()
        record = client.get(f"/api/v1/play/sessions/{sid}/hands").json()[-1]
        streets = {s["street"]: s["solvable"] for s in engine.review_streets(record)}
        if streets.get("turn") and streets.get("river"):
            hand_no = record["hand_no"]
            break
    assert hand_no is not None, "a heads-up hand that reached the river with decisions on it"

    url = f"/api/v1/play/sessions/{sid}/hands/{hand_no}/review"
    assert client.get(url).status_code == 404
    _check_resume_after_restart(client, url)
    r = client.post(url)
    assert r.status_code == 202, r.json()
    review: dict[str, Any] = r.json()
    assert [s["street"] for s in review["streets"]][-2:] == ["turn", "river"]
    # Starting again while it runs returns the same review.
    assert client.post(url).json()["id"] == review["id"]
    deadline = time.monotonic() + 600
    while review["status"] in ("queued", "running"):
        assert time.monotonic() < deadline, review
        time.sleep(0.5)
        review = client.get(url).json()
    assert review["status"] == "done", review
    by = {s["street"]: s for s in review["streets"]}
    # The full-range flop does not fit in 300 MB: that street fails and the turn starts afresh.
    if "flop" in by and by["flop"]["solvable"]:
        assert by["flop"]["status"] == "failed"
        assert any("tracked exactly" in n for n in by["turn"]["notes"])
    for st in ("turn", "river"):
        assert by[st]["status"] == "done", by[st]
        rv = by[st]["review"]
        assert rv["followed"], rv
        assert [d["decision"] for d in rv["decisions"]] == by[st]["decisions"]
        for d in rv["decisions"]:
            assert d["ev_loss_bb"] >= 0
            assert abs(sum(d["strategy"]) - 1) < 1e-3
    assert by["turn"]["review"]["next_ranges"] is not None
    assert any("subgame re-solving" in n for n in by["river"]["notes"])
    # Each street is an ordinary solve, listed in the library.
    lib = {e["job_id"]: e for e in client.get("/api/v1/solver/library").json()}
    assert by["river"]["job_id"] in lib
    # ...tagged with the hand it came from, so the library can show the whole hand.
    assert lib[by["river"]["job_id"]]["hand"] == {"session_id": sid, "hand_no": hand_no, "street": "river"}
    assert [s["decision"] for s in by["river"]["line"] if s["decision"] is not None] == by["river"][
        "decisions"
    ]
    assert client.get(f"/api/v1/play/sessions/{sid}/hands/{hand_no}").json()["hand_no"] == hand_no
    # The leak summary adds up the engine's per-decision EV losses from finished reviews.
    leaks = client.get("/api/v1/analytics/leaks").json()
    decisions = [d for st in review["streets"] if st.get("review") for d in st["review"]["decisions"]]
    assert leaks["reviews"] == 1
    assert leaks["total"]["decisions"] == len(decisions)
    assert abs(leaks["total"]["ev_lost_bb"] - sum(d["ev_loss_bb"] for d in decisions)) < 1e-9
    assert sum(s["decisions"] for s in leaks["by_street"]) == len(decisions)
    for c in leaks["costliest"]:
        assert c["ev_loss_pct_pot"] > leaks["threshold_pct_pot"]
        assert c["session_id"] == sid
    # A batch queues the other hands that have something to solve, one after another.
    batch = client.post("/api/v1/play/reviews/batch", json={"scope": "recent", "limit": 2}).json()
    assert len(batch["queued"]) <= 2
    assert all(q["hand_no"] != hand_no for q in batch["queued"]), "already reviewed hands are skipped"
    assert client.post("/api/v1/play/reviews/batch", json={"scope": "nope"}).status_code == 422
    status = client.get("/api/v1/play/reviews/status").json()["reviews"]
    assert status["done"] >= 1
    # A drill dealt from one of these solves carries the real hand.
    dealt = client.post(
        "/api/v1/trainer/postflop/drill", json={"job_id": by["river"]["job_id"], "seed": 3}
    ).json()
    assert dealt["hand"]["hand_no"] == hand_no
    assert dealt["hand"]["record"]["board"] == dealt["drill"]["board"]


def _check_resume_after_restart(client: TestClient, url: str) -> None:
    """A review queued before a restart picks up where it was: the reviewer of the new process
    resumes it from the database (its solves are in the durable job queue)."""
    assert hand_review.reviewer is not None
    old = hand_review.reviewer
    old.shutdown()  # the old process's reviewer is gone: nothing runs the review
    r = client.post(url)
    assert r.status_code == 202, r.json()
    time.sleep(1)
    assert client.get(url).json()["status"] == "queued", "nobody is working on it"
    hand_review.reviewer = hand_review.HandReviewer(old._sessions, old._max_bytes)
    assert hand_review.reviewer.resume_unfinished() == 1
    deadline = time.monotonic() + 600
    review: dict[str, Any] = client.get(url).json()
    while review["status"] in ("queued", "running"):
        assert time.monotonic() < deadline, review
        time.sleep(0.5)
        review = client.get(url).json()
    assert review["status"] == "done", review
