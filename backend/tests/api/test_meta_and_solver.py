from typing import Any

from fastapi.testclient import TestClient


def _spec(client: TestClient) -> dict[str, Any]:
    return {
        "game": client.get("/api/v1/game-configs/default").json(),
        "ranges": ["QQ+,AK", "JJ-22,AQs"],
        "board": ["Ks", "8d", "4c"],
        "starting_pot_mbb": 6_500,
        "effective_stack_mbb": 97_500,
        "tree": {
            "oop": {"bet_pct_pot": [[33, 75], [75], [75]], "raise_pct_pot": [[100], [100], [100]]},
            "ip": {"bet_pct_pot": [[33, 75], [75], [75]], "raise_pct_pot": [[100], [100], [100]]},
            "max_raises_per_street": 3,
            "all_in_threshold_pct_stack": 85,
            "add_all_in": False,
        },
        "solver": {
            "schema_version": 1,
            "variant": {"algorithm": "discounted", "alpha": 1.5, "beta": 0.0, "gamma": 2.0},
            "averaging": "quadratic",
            "stopping": {
                "max_iterations": 1000,
                "target_exploitability_pct_pot": 0.5,
                "max_seconds": None,
                "exploitability_check_interval": 10,
            },
            "threads": 1,
            "seed": None,
        },
    }


def test_meta_does_not_claim_a_solver_without_a_backend(client: TestClient) -> None:
    caps = {c["key"]: c["status"] for c in client.get("/api/v1/meta").json()["capabilities"]}
    assert caps["hand_evaluator"] == "available"
    assert caps["solver"] == "interface_only"


def test_solver_capabilities_without_a_backend(client: TestClient) -> None:
    body = client.get("/api/v1/solver/capabilities").json()
    assert body["status"] == "unavailable"
    assert body["available_backends"] == []
    assert body["engines"] == []


def test_valid_spec_is_rejected_with_503_not_queued(client: TestClient) -> None:
    r = client.post("/api/v1/solver/jobs", json={"spec": _spec(client)})
    assert r.status_code == 503
    assert r.json()["error"]["code"] == "solver_unavailable"


def test_invalid_spec_is_rejected_with_422(client: TestClient) -> None:
    spec = _spec(client)
    spec["solver"]["variant"] = {"algorithm": "monte_carlo", "sampling": {"scheme": "external"}}
    r = client.post("/api/v1/solver/jobs", json={"spec": spec})
    assert r.status_code == 422
    assert "seed" in r.json()["error"]["message"]
    spec = _spec(client)
    spec["ranges"] = ["QQ+", "ZZ"]
    assert client.post("/api/v1/solver/jobs", json={"spec": spec}).status_code == 422


def _preflop_spec(client: TestClient) -> dict[str, Any]:
    return {
        "game": client.get("/api/v1/game-configs/default").json(),
        "heads_up": False,
        "sizing": {
            "open_raise_to_mbb": [2_500],
            "reraise_pct_of_bet": [300],
            "max_raises": 3,
            "add_all_in": True,
            "all_in_threshold_pct_stack": 85,
            "allow_limp": False,
        },
        "realization": "positional",
        "max_iterations": 100,
        "max_seconds": None,
        "check_interval": 10,
        "threads": 1,
    }


def test_preflop_jobs_validate_then_need_a_backend(client: TestClient) -> None:
    caps = {c["key"]: c["status"] for c in client.get("/api/v1/meta").json()["capabilities"]}
    assert caps["preflop_solver"] == "interface_only"
    spec = _preflop_spec(client)
    assert client.post("/api/v1/solver/preflop/jobs", json={"spec": spec}).status_code == 503
    spec["sizing"]["open_raise_to_mbb"] = [1_500]
    r = client.post("/api/v1/solver/preflop/jobs", json={"spec": spec})
    assert r.status_code == 422
    assert "at least 2bb" in r.json()["error"]["message"]
    # A postflop spec is not a preflop spec.
    assert client.post("/api/v1/solver/preflop/jobs", json={"spec": {"bogus": 1}}).status_code == 422
    assert client.get("/api/v1/solver/jobs", params={"kind": "river"}).status_code == 422


def test_play_needs_the_solver_backend(client: TestClient) -> None:
    r = client.post("/api/v1/play/sessions", json={"config": {}})
    assert r.status_code == 503
