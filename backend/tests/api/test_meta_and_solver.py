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
            "bet_sizes_pct_pot": [[33, 75], [75], [75]],
            "raise_sizes_pct_pot": [[100], [100], [100]],
            "max_raises_per_street": 3,
            "all_in_threshold_pct_pot": 67,
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


def test_meta_never_claims_a_solver(client: TestClient) -> None:
    caps = {c["key"]: c["status"] for c in client.get("/api/v1/meta").json()["capabilities"]}
    assert caps["hand_evaluator"] == "available"
    assert caps["solver"] == "interface_only"


def test_solver_capabilities_report_not_implemented(client: TestClient) -> None:
    body = client.get("/api/v1/solver/capabilities").json()
    assert body["status"] == "not_implemented"
    assert body["available_backends"] == []


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
