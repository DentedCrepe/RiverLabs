"""End to end: the local GTOpen backend running real solves through the API, with committed
database state (the worker thread uses its own sessions)."""

import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import Engine, text
from sqlalchemy.orm import Session, sessionmaker

from riverlab_api.db.models import JobStatus, SolverJob
from riverlab_api.db.session import get_db
from riverlab_api.main import create_app
from riverlab_api.services import compute, engine
from riverlab_api.services.local_solver import LocalSolverBackend, fail_interrupted_jobs

pytestmark = pytest.mark.db

NONE = {"bet_pct_pot": [[], [], []], "raise_pct_pot": [[], [], []]}


def _clean(db_engine: Engine) -> None:
    with db_engine.begin() as conn:
        conn.execute(text("DELETE FROM strategy_nodes; DELETE FROM solver_results; DELETE FROM solver_jobs"))


def _start(
    db_engine: Engine, max_memory_mb: int = 512, storage_dir: str | None = None
) -> tuple[TestClient, LocalSolverBackend]:
    factory = sessionmaker(bind=db_engine, expire_on_commit=False)
    app = create_app()

    def real_db() -> Iterator[Session]:
        with factory() as s:
            yield s

    app.dependency_overrides[get_db] = real_db
    backend = LocalSolverBackend(factory, max_memory_mb=max_memory_mb, threads=2, storage_dir=storage_dir)
    compute.registry.register(backend)
    return TestClient(app), backend


@pytest.fixture
def live(db_engine: Engine, tmp_path: Path) -> Iterator[TestClient]:
    _clean(db_engine)
    client, backend = _start(db_engine, storage_dir=str(tmp_path / "results"))
    try:
        yield client
    finally:
        compute.registry.backends.remove(backend)
        backend.shutdown()
        _clean(db_engine)


def toy_spec(iterations: int = 2000, rake_bps: int = 0) -> dict[str, Any]:
    cfg = engine.default_game_config()
    cfg["rake"]["percentage_bps"] = rake_bps
    return {
        "game": cfg,
        "ranges": ["AQ", "KK,43o"],
        "board": ["Ks", "Qd", "7h", "7c", "2s"],
        "starting_pot_mbb": 10_000,
        "effective_stack_mbb": 10_000,
        "tree": {
            "oop": NONE,
            "ip": {"bet_pct_pot": [[], [], [100]], "raise_pct_pot": [[], [], []]},
            "max_raises_per_street": 0,
            "all_in_threshold_pct_stack": 100,
            "add_all_in": False,
        },
        "solver": {
            "schema_version": 1,
            "variant": {"algorithm": "discounted", "alpha": 1.5, "beta": 0.0, "gamma": 2.0},
            "averaging": "quadratic",
            "stopping": {
                "max_iterations": iterations,
                "target_exploitability_pct_pot": None,
                "max_seconds": None,
                "exploitability_check_interval": 250,
            },
            "threads": 1,
            "seed": None,
        },
    }


def wide_turn_spec(iterations: int) -> dict[str, Any]:
    spec = toy_spec(iterations)
    spec["ranges"] = [
        "22+,A2+,K2+,Q2+,J5+,T6+,96+,85+,75+,64+,54",
        "22+,A2+,K2+,Q2+,J2+,T2+,93+,84+,74+,63+,53+",
    ]
    spec["board"] = ["Ks", "7d", "2c", "8h"]
    spec["starting_pot_mbb"] = 100_000
    spec["effective_stack_mbb"] = 1_000_000
    sizes = {"bet_pct_pot": [[], [33, 75, 150], [33, 75, 150]], "raise_pct_pot": [[], [100], [100]]}
    spec["tree"].update({"oop": sizes, "ip": sizes, "max_raises_per_street": 2})
    return spec


def wait(client: TestClient, job_id: str, statuses: set[str], timeout: float = 120) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        job: dict[str, Any] = client.get(f"/api/v1/solver/jobs/{job_id}").json()
        if job["status"] in statuses:
            return job
        time.sleep(0.1)
    pytest.fail(f"job {job_id} did not reach {statuses}")


def test_solve_end_to_end_matches_the_hand_solution(live: TestClient) -> None:
    caps = live.get("/api/v1/solver/capabilities").json()
    assert caps["status"] == "available"
    assert caps["engines"][0]["name"] == "gtopen"
    meta = {c["key"]: c["status"] for c in live.get("/api/v1/meta").json()["capabilities"]}
    assert meta["solver"] == "available"

    r = live.post("/api/v1/solver/jobs", json={"spec": toy_spec()})
    assert r.status_code == 202, r.json()
    job = wait(live, r.json()["id"], {"succeeded", "failed"})
    assert job["status"] == "succeeded", (job["error_code"], job["error_message"])
    assert job["iterations"] == 2000
    assert job["progress"]["exploitability_mbb"] >= 0

    res = live.get(f"/api/v1/solver/jobs/{job['id']}/result").json()
    # Hand solution: IP pot share 3bb of 10 => net -2bb.
    assert res["values_mbb"][0] == pytest.approx(2000, abs=1)
    assert res["values_mbb"][1] == pytest.approx(-2000, abs=1)
    check = res["independent_check"]
    assert check["performed"] is True
    assert check["exploitability_mbb"] == pytest.approx(res["exploitability_mbb"], abs=0.5)
    assert res["solver_backend"] == "gtopen"
    assert len(res["solver_version"]) == 40
    assert res["root"]["actions"] == ["check"]
    assert {c["label"] for c in res["root"]["by_class"]} == {"AQs", "AQo"}


def test_raked_solve_lists_its_approximation(live: TestClient) -> None:
    r = live.post("/api/v1/solver/jobs", json={"spec": toy_spec(rake_bps=500)})
    job = wait(live, r.json()["id"], {"succeeded", "failed"})
    assert job["status"] == "succeeded", job
    res = live.get(f"/api/v1/solver/jobs/{job['id']}/result").json()
    assert res["values_mbb"][1] == pytest.approx(-2223.1, abs=1)
    assert len(res["approximations"]) == 1


def test_jobs_queue_and_running_jobs_cancel(live: TestClient) -> None:
    long_job = live.post("/api/v1/solver/jobs", json={"spec": wide_turn_spec(1_000_000)}).json()
    queued = live.post("/api/v1/solver/jobs", json={"spec": toy_spec()}).json()
    wait(live, long_job["id"], {"running"})
    assert live.get(f"/api/v1/solver/jobs/{queued['id']}").json()["status"] == "queued"
    # Cancel the queued one first: it must never run.
    assert live.post(f"/api/v1/solver/jobs/{queued['id']}/cancel").json()["status"] == "cancelled"
    r = live.post(f"/api/v1/solver/jobs/{long_job['id']}/cancel").json()
    assert r["cancel_requested"] is True
    done = wait(live, long_job["id"], {"cancelled", "failed", "succeeded"})
    assert done["status"] == "cancelled"
    assert live.get(f"/api/v1/solver/jobs/{long_job['id']}/result").status_code == 404
    assert live.get(f"/api/v1/solver/jobs/{queued['id']}").json()["started_at"] is None


def test_oversized_spots_fail_with_a_clear_reason(db_engine: Engine) -> None:
    _clean(db_engine)
    client, backend = _start(db_engine, max_memory_mb=16)
    try:
        r = client.post("/api/v1/solver/jobs", json={"spec": wide_turn_spec(10)})
        job = wait(client, r.json()["id"], {"failed", "succeeded"})
        assert job["status"] == "failed"
        assert job["error_code"] == "solve_rejected"
        assert "too large" in job["error_message"]
    finally:
        compute.registry.backends.remove(backend)
        backend.shutdown()
        _clean(db_engine)


def test_interrupted_jobs_are_failed_on_startup(db_engine: Engine) -> None:
    _clean(db_engine)
    factory = sessionmaker(bind=db_engine, expire_on_commit=False)
    try:
        with factory() as s:
            s.add_all(
                [
                    SolverJob(spec={}, backend="local", status=JobStatus.QUEUED),
                    SolverJob(spec={}, backend="local", status=JobStatus.RUNNING),
                    SolverJob(spec={}, backend="elsewhere", status=JobStatus.QUEUED),
                ]
            )
            s.commit()
        with factory() as s:
            assert fail_interrupted_jobs(s) == 2
        with factory() as s:
            statuses = sorted((j.backend, j.status.value, j.error_code) for j in s.query(SolverJob))
        assert statuses == [
            ("elsewhere", "queued", None),
            ("local", "failed", "interrupted"),
            ("local", "failed", "interrupted"),
        ]
    finally:
        _clean(db_engine)


def test_saved_tree_can_be_browsed_and_deleted(live: TestClient, tmp_path: Path) -> None:
    job = wait(
        live,
        live.post("/api/v1/solver/jobs", json={"spec": toy_spec()}).json()["id"],
        {"succeeded", "failed"},
    )
    assert job["status"] == "succeeded", job
    res = live.get(f"/api/v1/solver/jobs/{job['id']}/result").json()
    assert res["tree_available"] is True
    saved = list((tmp_path / "results").glob("*.gto"))
    assert len(saved) == 1

    base = f"/api/v1/solver/jobs/{job['id']}/node"
    root = live.get(base).json()
    assert root["kind"] == "decision"
    assert root["player"] == 0
    assert root["by_class"] == res["root"]["by_class"]
    ip = live.get(base, params={"path": "a0"}).json()
    assert ip["player"] == 1
    assert ip["actions"][0] == "check"
    assert ip["actions"][1].startswith("bet")
    # IP bets: OOP faces fold/call with the whole AQ range (it never folded or bet before).
    facing = live.get(base, params={"path": "a0.a1"}).json()
    assert facing["player"] == 0
    assert facing["actions"][0] == "fold"
    assert facing["actions"][1].startswith("call")
    call_freq = sum(c["weight"] * c["frequencies"][1] for c in facing["by_class"]) / sum(
        c["weight"] for c in facing["by_class"]
    )
    assert call_freq == pytest.approx(0.5, abs=0.01)  # the hand-derived equilibrium
    # IP's reach after betting: all 3 KK + 1.5 of 12 air combos => 4.5 / 15.
    assert facing["reach_fraction"][1] == pytest.approx(4.5 / 15, abs=0.01)
    assert live.get(f"{base}", params={"path": "a7"}).status_code == 422
    assert live.get(f"{base}", params={"path": "x1"}).status_code == 422

    assert live.delete(f"/api/v1/solver/jobs/{job['id']}").status_code == 204
    assert live.get(f"/api/v1/solver/jobs/{job['id']}").status_code == 404
    assert not saved[0].exists()
