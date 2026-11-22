import pytest
from fastapi.testclient import TestClient


def test_health(client: TestClient) -> None:
    assert client.get("/health").json() == {"status": "ok", "version": "0.1.0"}


def test_evaluate(client: TestClient) -> None:
    r = client.post("/api/v1/hands/evaluate", json={"cards": "KsKdKhTcTd2c3d"})
    assert r.status_code == 200
    assert r.json()["description"] == "Full house, kings full of tens"


def test_validation_errors_use_the_error_envelope(client: TestClient) -> None:
    r = client.post("/api/v1/hands/evaluate", json={"cards": "Xx"})
    assert r.status_code == 422
    assert r.json()["error"]["code"] == "engine_validation_error"
    r = client.post("/api/v1/hands/evaluate", json={"cards": "AsKs", "extra": 1})
    assert r.status_code == 422
    assert r.json()["error"]["code"] == "request_validation_error"


def test_compare_detects_ties_kickers_and_conflicts(client: TestClient) -> None:
    r = client.post("/api/v1/hands/compare", json={"board": "AsKdQh7c2s", "hands": ["AcJd", "AhJs", "Ad9c"]})
    assert r.status_code == 200
    assert r.json()["winners"] == [0, 1]
    r = client.post("/api/v1/hands/compare", json={"board": "AsKdQh7c2s", "hands": ["AsJd", "AhJs"]})
    assert r.status_code == 422
    assert "more than once" in r.json()["error"]["message"]
    r = client.post("/api/v1/hands/compare", json={"board": "AsKdQh", "hands": ["AcJd", "AhJs"]})
    assert r.status_code == 422


def test_range_analysis(client: TestClient) -> None:
    r = client.post("/api/v1/ranges/analyze", json={"notation": "QQ+,AKs", "dead": "As"})
    body = r.json()
    assert r.status_code == 200
    assert body["combos"] == 22
    assert body["live_combos"] == 22 - 3 - 1
    assert len(body["matrix"]) == 169


def test_equity_exact_and_monte_carlo(client: TestClient) -> None:
    r = client.post("/api/v1/equity", json={"ranges": ["AsAh", "KsKh"], "board": "2c7d9hTs3c"})
    assert r.json()["equity"] == [1.0, 0.0]
    r = client.post(
        "/api/v1/equity",
        json={"ranges": ["QQ+", "AK"], "method": "monte_carlo", "iterations": 20_000, "seed": 3},
    )
    body = r.json()
    assert body["method"] == "monte_carlo"
    assert body["seed"] == 3
    assert len(body["std_error"]) == 2


def test_equity_refuses_unbounded_work(client: TestClient) -> None:
    r = client.post("/api/v1/equity", json={"ranges": ["22+,A2+,K2+", "22+,A2+,K2+"]})
    assert r.status_code == 422
    assert "Monte Carlo" in r.json()["error"]["message"]
    r = client.post("/api/v1/equity", json={"ranges": ["AA", "KK"], "method": "monte_carlo", "iterations": 10**9})
    assert r.status_code == 422


def test_rake_calculation(client: TestClient) -> None:
    default = client.get("/api/v1/game-configs/default").json()
    default["rake"]["cap"] = {"unit": "big_blinds", "mbb": 3000}
    r = client.post("/api/v1/rake/calculate", json={"config": default, "pot_mbb": 100_000, "saw_flop": True})
    assert r.json() == {"rake": 3000, "capped": True, "reason": "capped"}
    default["rake"]["cap"] = {"unit": "currency", "minor_units": 300}
    r = client.post("/api/v1/rake/calculate", json={"config": default, "pot_mbb": 100_000, "saw_flop": True})
    assert r.status_code == 422
    assert "stake" in r.json()["error"]["message"]


def test_replay_full_hand_to_showdown(client: TestClient) -> None:
    events = [
        {"type": "hole_cards", "seat": 3, "cards": "AsAh"},
        {"type": "hole_cards", "seat": 5, "cards": "KsKh"},
        {"type": "action", "action": {"type": "fold"}},
        {"type": "action", "action": {"type": "fold"}},
        {"type": "action", "action": {"type": "fold"}},
        {"type": "action", "action": {"type": "raise", "to": 2500}},
        {"type": "action", "action": {"type": "fold"}},
        {"type": "action", "action": {"type": "call"}},
        {"type": "board", "cards": "2c7d9h"},
        {"type": "action", "action": {"type": "check"}},
        {"type": "action", "action": {"type": "bet", "to": 2000}},
        {"type": "action", "action": {"type": "call"}},
        {"type": "board", "cards": "Ts"},
        {"type": "action", "action": {"type": "check"}},
        {"type": "action", "action": {"type": "check"}},
        {"type": "board", "cards": "3c"},
        {"type": "action", "action": {"type": "check"}},
        {"type": "action", "action": {"type": "check"}},
    ]
    r = client.post("/api/v1/hands/replay", json={"events": events})
    body = r.json()
    assert r.status_code == 200, body
    assert body["phase"] == {"phase": "showdown"}
    s = body["settlement"]
    # Pot: 2.5 + 2.5 + 0.5 (SB) + 2 + 2 = 9.5bb; 5% rake = 0.475bb.
    assert s["rake"]["rake"] == 475
    assert s["pots"][0]["winners"] == [3]
    assert s["net"][3] == 9500 - 475 - 4500
    assert sum(s["final_stacks"]) + s["rake"]["rake"] == 600_000


@pytest.mark.parametrize(
    ("events", "fragment"),
    [
        ([{"type": "action", "action": {"type": "check"}}], "cannot check"),
        ([{"type": "board", "cards": "2c7d9h"}], "cannot deal"),
        ([{"type": "action", "action": {"type": "raise", "to": 1500}}], "raise to"),
    ],
)
def test_replay_rejects_illegal_events(client: TestClient, events: list[dict[str, object]], fragment: str) -> None:
    r = client.post("/api/v1/hands/replay", json={"events": events})
    assert r.status_code == 422
    assert fragment in r.json()["error"]["message"]
