"""Idle tracking and the busy-only wake lock (used to let a phone sleep)."""

import time
from pathlib import Path
from types import SimpleNamespace

import pytest
from fastapi.testclient import TestClient

from riverlab_api.services import activity


def test_wake_lock_is_held_only_while_work_runs(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    flag = tmp_path / "locked"
    monkeypatch.setattr(
        activity,
        "get_settings",
        lambda: SimpleNamespace(wake_lock_command=f"touch {flag}", wake_unlock_command=f"rm {flag}"),
    )
    assert not activity.busy()
    activity.begin()
    activity.begin()  # nested work: the lock is taken once
    assert activity.busy()
    assert flag.exists()
    activity.end()
    assert flag.exists(), "still busy"
    with activity.working():
        assert activity.busy()
    activity.end()
    assert not activity.busy()
    assert not flag.exists(), "released when the last work ends"
    activity.end()  # never goes negative
    assert not activity.busy()


def test_a_failing_wake_lock_command_does_not_break_work(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        activity,
        "get_settings",
        lambda: SimpleNamespace(wake_lock_command="false", wake_unlock_command="no-such-cmd"),
    )
    with activity.working():
        assert activity.busy()
    assert not activity.busy()


def test_only_user_requests_count_as_activity(client: TestClient) -> None:
    client.get("/api/v1/positions")
    time.sleep(0.2)
    client.get("/health")
    client.get("/api/v1/positions", headers={"X-RiverLab-Poll": "1"})
    status = client.get("/health/activity").json()
    assert status["busy"] is False
    assert status["idle_seconds"] >= 0.2
    client.get("/api/v1/positions")
    assert client.get("/health/activity").json()["idle_seconds"] < 0.2
