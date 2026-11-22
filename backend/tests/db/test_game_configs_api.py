from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from riverlab_api.db.models import RakeConfig, StackConfig

pytestmark = pytest.mark.db


def _config(client: TestClient, **changes: Any) -> dict[str, Any]:
    cfg = client.get("/api/v1/game-configs/default").json()
    for path, value in changes.items():
        target = cfg
        keys = path.split("__")
        for k in keys[:-1]:
            target = target[k]
        target[keys[-1]] = value
    return cfg


def test_list_contains_builtin_default(db_client: TestClient) -> None:
    rows = db_client.get("/api/v1/game-configs").json()
    default = next(r for r in rows if r["builtin"])
    assert default["name"] == "6-max 100bb (default)"
    assert default["stack_preset"] == "100bb"


def test_create_get_delete(db_client: TestClient, db: Session) -> None:
    cfg = _config(db_client, stack__effective_stack=62_500, rake__cap={"unit": "big_blinds", "mbb": 3000})
    r = db_client.post("/api/v1/game-configs", json={"name": "62.5bb capped", "config": cfg})
    assert r.status_code == 201, r.json()
    created = r.json()
    assert created["stack_preset"] is None
    assert created["config"] == cfg
    assert db_client.get(f"/api/v1/game-configs/{created['id']}").json() == created
    assert db_client.delete(f"/api/v1/game-configs/{created['id']}").status_code == 204
    assert db_client.get(f"/api/v1/game-configs/{created['id']}").status_code == 404


def test_stack_and_rake_rows_are_reused_independently(db_client: TestClient, db: Session) -> None:
    stacks_before = db.scalar(select(func.count()).select_from(StackConfig))
    rakes_before = db.scalar(select(func.count()).select_from(RakeConfig))
    # Same 100bb preset + same default rake → no new stack/rake rows.
    db_client.post("/api/v1/game-configs", json={"name": "copy", "config": _config(db_client)})
    # New rake, same stack → one new rake row only.
    db_client.post(
        "/api/v1/game-configs",
        json={"name": "4% rake", "config": _config(db_client, rake__percentage_bps=400)},
    )
    assert db.scalar(select(func.count()).select_from(StackConfig)) == stacks_before
    assert db.scalar(select(func.count()).select_from(RakeConfig)) == rakes_before + 1


def test_duplicate_name_conflicts(db_client: TestClient) -> None:
    body = {"name": "dup", "config": _config(db_client)}
    assert db_client.post("/api/v1/game-configs", json=body).status_code == 201
    r = db_client.post("/api/v1/game-configs", json=body)
    assert r.status_code == 409
    assert r.json()["error"]["code"] == "conflict"


def test_engine_rejects_semantically_invalid_config(db_client: TestClient) -> None:
    cfg = _config(db_client, stack__effective_stack=1_000)
    r = db_client.post("/api/v1/game-configs", json={"name": "too short", "config": cfg})
    assert r.status_code == 422
    assert "effective stack" in r.json()["error"]["message"]


def test_builtin_default_cannot_be_deleted(db_client: TestClient) -> None:
    default = next(r for r in db_client.get("/api/v1/game-configs").json() if r["builtin"])
    assert db_client.delete(f"/api/v1/game-configs/{default['id']}").status_code == 409


def test_readiness_reports_database(db_client: TestClient) -> None:
    assert db_client.get("/health/ready").json()["database"] == "ok"
