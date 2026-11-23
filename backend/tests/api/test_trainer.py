"""Push/fold trainer API: table states, verification, and graded drills."""

import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from fastapi.testclient import TestClient

from riverlab_api.services import trainer

REAL_TABLE = Path(__file__).resolve().parents[3] / "engine" / "target" / trainer.TABLE_NAME


@pytest.fixture
def use_table(tmp_path: Path) -> Iterator[Any]:
    def install(path: Path) -> trainer.PushFoldService:
        trainer._service = trainer.PushFoldService(path)
        return trainer._service

    yield install
    trainer._service = None


def wait_state(client: TestClient, states: set[str], timeout: float = 120) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        s: dict[str, Any] = client.get("/api/v1/trainer/push-fold").json()
        if s["state"] in states:
            return s
        time.sleep(0.2)
    pytest.fail(f"trainer did not reach {states}")


def test_missing_table_is_reported_and_drills_refused(
    client: TestClient, use_table: Any, tmp_path: Path
) -> None:
    use_table(tmp_path / trainer.TABLE_NAME)
    s = client.get("/api/v1/trainer/push-fold").json()
    assert s["state"] == "missing"
    assert s["table_present"] is False
    r = client.post(
        "/api/v1/trainer/push-fold/drill", json={"min_stack_mbb": 10_000, "max_stack_mbb": 10_000}
    )
    assert r.status_code == 409


def test_a_corrupt_table_is_rejected(client: TestClient, use_table: Any, tmp_path: Path) -> None:
    bad = tmp_path / trainer.TABLE_NAME
    bad.write_bytes(b"RLHUEQ01" + b"\0" * 16)
    use_table(bad)
    s = wait_state(client, {"error", "ready"})
    assert s["state"] == "error"
    assert "rejected" in s["message"]


@pytest.mark.skipif(
    not REAL_TABLE.exists(), reason="exact equity table not built (scripts/build-push-fold-table.sh)"
)
def test_drill_answer_and_chart_with_the_exact_table(client: TestClient, use_table: Any) -> None:
    use_table(REAL_TABLE)
    assert wait_state(client, {"ready", "error"})["state"] == "ready"
    r = client.post(
        "/api/v1/trainer/push-fold/drill",
        json={"min_stack_mbb": 8_000, "max_stack_mbb": 12_000, "seat": "sb", "seed": 4},
    )
    assert r.status_code == 200, r.json()
    drill = r.json()
    assert drill["seat"] == "sb"
    assert 8_000 <= drill["stack_mbb"] <= 12_000
    graded = client.post("/api/v1/trainer/push-fold/answer", json={"drill": drill, "choice": "play"}).json()
    assert graded["actions"] == ["fold", "jam"]
    assert graded["evs_bb"][0] == -0.5
    assert graded["verdict"] in {"correct", "close", "mistake"}
    assert graded["exploitability_bb"] < 1e-4
    aces = {"stack_mbb": 10_000, "seat": "bb", "cards": ["Ah", "Ad"], "class": ""}
    fold = client.post("/api/v1/trainer/push-fold/answer", json={"drill": aces, "choice": "fold"}).json()
    assert fold["verdict"] == "mistake"
    assert fold["drill"]["class"] == "AA"
    chart = client.get("/api/v1/trainer/push-fold/chart", params={"stack_mbb": 10_000}).json()
    assert abs(chart["sb_jam_share"] - 0.583) < 0.002
    assert len(chart["sb"]) == 169
    # Out-of-scope depths are rejected by the engine, not rounded.
    assert client.get("/api/v1/trainer/push-fold/chart", params={"stack_mbb": 10_250}).status_code == 422
    assert client.post("/api/v1/trainer/push-fold/build").status_code == 409
