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

# GTOpen's preflop equity table takes ~30 s to build; keep it between test runs (git-ignored).
EQUITY_CACHE = Path(__file__).resolve().parents[2] / ".cache"

NONE: dict[str, Any] = {"bet_pct_pot": [[], [], []], "raise_pct_pot": [[], [], []]}


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
    backend = LocalSolverBackend(
        factory,
        max_memory_mb=max_memory_mb,
        threads=2,
        storage_dir=storage_dir,
        equity_cache_dir=str(EQUITY_CACHE),
    )
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
    assert caps["engines"][0]["patches"] == ["arenabuf-provenance"]
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
    assert res["solver_version"] == "eb5ccd671223293527f36d219cfe33baf0b0475c+arenabuf-provenance"
    assert res["root"]["actions"] == ["check"]
    assert {c["label"] for c in res["root"]["by_class"]} == {"AQs", "AQo"}


def test_raked_solve_lists_its_approximation(live: TestClient) -> None:
    r = live.post("/api/v1/solver/jobs", json={"spec": toy_spec(rake_bps=500)})
    job = wait(live, r.json()["id"], {"succeeded", "failed"})
    assert job["status"] == "succeeded", job
    res = live.get(f"/api/v1/solver/jobs/{job['id']}/result").json()
    assert res["values_mbb"][1] == pytest.approx(-2223.1, abs=1)
    assert len(res["approximations"]) == 1
    # The library lists it with its configuration and quality measures.
    lib = live.get("/api/v1/solver/library", params={"kind": "postflop"}).json()
    entry = next(e for e in lib if e["job_id"] == job["id"])
    assert entry["board"] == toy_spec()["board"]
    assert entry["exploitability_mbb"] == pytest.approx(res["exploitability_mbb"])
    assert entry["approximations"] == 1
    assert entry["solver_version"] == res["solver_version"]


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


def preflop_spec(
    heads_up: bool = True, push_fold: bool = True, iterations: int = 400, rake_bps: int = 0
) -> dict[str, Any]:
    cfg = engine.default_game_config()
    cfg["rake"]["percentage_bps"] = rake_bps
    cfg["players"] = 3
    cfg["stack"]["effective_stack"] = 10_000
    return {
        "game": cfg,
        "heads_up": heads_up,
        "sizing": {
            "open_raise_to_mbb": [] if push_fold else [2_500],
            "reraise_pct_of_bet": [] if push_fold else [300],
            "max_raises": 1 if push_fold else 2,
            "add_all_in": True,
            "all_in_threshold_pct_stack": 85,
            "allow_limp": False,
        },
        "realization": "positional",
        "max_iterations": iterations,
        "max_seconds": None,
        "check_interval": 100,
        "threads": 1,
    }


def _range_frequency(node: dict[str, Any], action: int) -> float:
    total = sum(c["weight"] for c in node["by_class"])
    return float(sum(c["weight"] * c["frequencies"][action] for c in node["by_class"]) / total)


def test_heads_up_push_fold_matches_the_validated_equilibrium(live: TestClient, tmp_path: Path) -> None:
    meta = {c["key"]: c["status"] for c in live.get("/api/v1/meta").json()["capabilities"]}
    assert meta["preflop_solver"] == "approximate"
    created = live.post("/api/v1/solver/preflop/jobs", json={"spec": preflop_spec()})
    assert created.status_code == 202, created.json()
    assert created.json()["kind"] == "preflop"
    job = wait(live, created.json()["id"], {"succeeded", "failed"}, timeout=300)
    assert job["status"] == "succeeded", job

    res = live.get(f"/api/v1/solver/jobs/{job['id']}/preflop").json()
    assert res["validated_scope"] == "heads_up_push_fold"
    assert res["positions"] == ["SB", "BB"]
    assert res["tree_available"] is True
    assert len(res["gaps_mbb"]) == 2
    root = res["root"]
    assert root["actor"] == "SB"
    jam = next(i for i, a in enumerate(root["actions"]) if a["kind"] == "jam")
    # RiverLab's exact combo-level equilibrium at 10bb: SB jams 58.3%, BB calls 37.4%.
    assert root["actions"][jam]["frequency"] == pytest.approx(0.583, abs=0.02)
    assert _range_frequency(root, jam) == pytest.approx(0.583, abs=0.02)

    base = f"/api/v1/solver/jobs/{job['id']}/preflop/node"
    bb = live.get(base, params={"path": str(jam)}).json()
    assert bb["actor"] == "BB"
    call = next(i for i, a in enumerate(bb["actions"]) if a["kind"] == "call")
    assert bb["actions"][call]["frequency"] == pytest.approx(0.374, abs=0.02)
    assert bb["history"][0]["chosen"] == jam
    assert live.get(base, params={"path": "9"}).status_code == 422
    assert live.get(base, params={"path": "a1"}).status_code == 422
    # The postflop endpoints refuse preflop jobs, pointing at the right one.
    wrong = live.get(f"/api/v1/solver/jobs/{job['id']}/result")
    assert wrong.status_code == 404
    assert "/preflop" in wrong.json()["error"]["message"]

    listed = live.get("/api/v1/solver/jobs", params={"kind": "preflop"}).json()
    assert [j["id"] for j in listed] == [job["id"]]
    assert live.get("/api/v1/solver/jobs", params={"kind": "postflop"}).json() == []
    saved = list((tmp_path / "results").glob("*.gtp"))
    assert len(saved) == 1
    assert live.delete(f"/api/v1/solver/jobs/{job['id']}").status_code == 204
    assert not saved[0].exists()


def test_three_handed_preflop_is_labelled_approximate(live: TestClient) -> None:
    spec = preflop_spec(heads_up=False, push_fold=False, iterations=30, rake_bps=500)
    job = wait(
        live,
        live.post("/api/v1/solver/preflop/jobs", json={"spec": spec}).json()["id"],
        {"succeeded", "failed"},
        timeout=300,
    )
    assert job["status"] == "succeeded", job
    res = live.get(f"/api/v1/solver/jobs/{job['id']}/preflop").json()
    assert res["validated_scope"] == "approximate_model"
    assert res["model_notes"][0].startswith("Approximate model")
    assert any("Rake rounding" in n for n in res["model_notes"])
    assert res["positions"] == ["BTN", "SB", "BB"]
    assert res["root"]["actor"] == "BTN"
    assert res["root"]["actions"][0]["kind"] == "fold"


def test_postflop_drills_are_dealt_from_saved_solves(live: TestClient, db_engine: Engine) -> None:
    with db_engine.begin() as conn:
        conn.execute(text("DELETE FROM postflop_answers"))
    assert live.post("/api/v1/trainer/postflop/drill", json={}).status_code == 409, "no solves yet"
    job = wait(
        live,
        live.post("/api/v1/solver/jobs", json={"spec": toy_spec()}).json()["id"],
        {"succeeded", "failed"},
    )
    assert job["status"] == "succeeded", job
    sources = live.get("/api/v1/trainer/postflop/sources").json()
    assert [s["job_id"] for s in sources] == [job["id"]]
    # In position: bet the pot or check (the toy spot's out-of-position player can only check).
    r = live.post("/api/v1/trainer/postflop/drill", json={"job_id": job["id"], "hero": 1, "seed": 5})
    assert r.status_code == 200, r.json()
    dealt = r.json()
    drill = dealt["drill"]
    assert drill["hero_player"] == 1
    assert drill["actions"] == ["check", "bet 10bb"]
    assert drill["hero_class"] in ("KK", "43o")
    grades = [
        live.post(
            "/api/v1/trainer/postflop/answer", json={"job_id": job["id"], "drill": drill, "choice": c}
        ).json()
        for c in range(2)
    ]
    best = grades[0]["best"]
    assert grades[best]["verdict"] == "correct"
    if drill["hero_class"] == "KK":
        # Kings always value-bet the pot here: checking gives up a lot.
        assert best == 1
        assert grades[0]["verdict"] == "mistake"
    stats = live.get("/api/v1/analytics/postflop").json()
    assert stats["total"]["answers"] == 2
    assert stats["by_street"][2]["answers"] == 2  # river
    assert stats["by_position"]["ip"]["answers"] == 2
    bad = live.post(
        "/api/v1/trainer/postflop/answer", json={"job_id": job["id"], "drill": drill, "choice": 7}
    )
    assert bad.status_code == 422


def test_preflop_drills_are_dealt_from_saved_preflop_solves(live: TestClient, db_engine: Engine) -> None:
    with db_engine.begin() as conn:
        conn.execute(text("DELETE FROM preflop_answers"))
    spec = preflop_spec(heads_up=False, push_fold=False, iterations=30, rake_bps=500)
    job = wait(
        live,
        live.post("/api/v1/solver/preflop/jobs", json={"spec": spec}).json()["id"],
        {"succeeded", "failed"},
        timeout=300,
    )
    assert job["status"] == "succeeded", job
    sources = live.get("/api/v1/trainer/preflop/sources").json()
    mine = [s for s in sources if s["job_id"] == job["id"]]
    assert len(mine) == 1
    assert mine[0]["validated_scope"] == "approximate_model"

    for spot in ("open", "facing_raise", "any"):
        r = live.post("/api/v1/trainer/preflop/drill", json={"job_id": job["id"], "spot": spot, "seed": 11})
        assert r.status_code == 200, r.json()
        dealt = r.json()
        assert dealt["validated_scope"] == "approximate_model", "every drill states what it is graded against"
        d = dealt["drill"]
        assert d["spot"] == spot
        assert "by_class" not in d, "the drill hides the answer"
        assert "frequency" not in d["actions"][0]
        if spot == "open":
            assert d["position"] != "BB"
            assert all(h["chosen"] == 0 for h in d["history"]), "only folds before an open"

    d = live.post(
        "/api/v1/trainer/preflop/drill", json={"job_id": job["id"], "spot": "open", "seed": 3}
    ).json()
    body = {"job_id": job["id"], "drill": d["drill"], "choice": 0}
    g = live.post("/api/v1/trainer/preflop/answer", json=body).json()
    assert g["verdict"] in ("matches", "mixed", "differs")
    assert abs(sum(a["frequency"] for a in g["actions"]) - 1) < 1e-3
    assert g["your_frequency"] == g["actions"][0]["frequency"]
    assert g["model_notes"][0].startswith("Approximate model")
    assert live.post("/api/v1/trainer/preflop/answer", json={**body, "choice": 99}).status_code == 422
    # Guests are graded but not recorded.
    guest = live.post("/api/v1/trainer/preflop/answer", json=body, headers={"X-RiverLab-Guest": "1"})
    assert guest.status_code == 200
    stats = live.get("/api/v1/analytics/preflop").json()
    assert stats["total"]["answers"] == 1
    assert stats["by_spot"][0] == {"spot": "open", **stats["total"]}
