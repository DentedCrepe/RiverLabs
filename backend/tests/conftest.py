"""Shared fixtures.

DB tests use RIVERLAB_TEST_DATABASE_URL. The database name must end in `_test`; the schema
is dropped and rebuilt with Alembic once per session, and each test runs inside a
transaction that is rolled back (service-level commits become savepoint releases).
If the test database is unreachable, DB tests fail loudly — they are never skipped silently.
"""

import os
from collections.abc import Iterator
from pathlib import Path

import pytest
from sqlalchemy import Engine, create_engine, text
from sqlalchemy.engine import make_url
from sqlalchemy.orm import Session

TEST_DB_URL = os.environ.get("RIVERLAB_TEST_DATABASE_URL", "")
# The app requires a database URL at import time; point it at the test database.
os.environ["RIVERLAB_DATABASE_URL"] = TEST_DB_URL or "postgresql+psycopg://unset:unset@localhost:1/unset_test"
os.environ.setdefault("RIVERLAB_ENV", "test")

from fastapi.testclient import TestClient  # noqa: E402

from riverlab_api.db.session import get_db  # noqa: E402
from riverlab_api.main import create_app  # noqa: E402

BACKEND_DIR = Path(__file__).resolve().parents[1]


def _require_test_db() -> str:
    if not TEST_DB_URL:
        pytest.fail("RIVERLAB_TEST_DATABASE_URL is not set (see .env.example / scripts/test.sh)")
    name = make_url(TEST_DB_URL).database or ""
    if not name.endswith("_test"):
        pytest.fail(f"Refusing to run destructive test setup against database '{name}' (must end in _test)")
    return TEST_DB_URL


@pytest.fixture(scope="session")
def db_engine() -> Iterator[Engine]:
    url = _require_test_db()
    engine = create_engine(url, future=True)
    try:
        with engine.begin() as conn:
            conn.execute(text("DROP SCHEMA public CASCADE"))
            conn.execute(text("CREATE SCHEMA public"))
    except Exception as exc:  # pragma: no cover - environment problem
        pytest.fail(f"Cannot reach the test database: {exc}")
    from alembic import command
    from alembic.config import Config

    cfg = Config(str(BACKEND_DIR / "alembic.ini"))
    cfg.cmd_opts = type("Opts", (), {"x": [f"db_url={url}"]})()  # type: ignore[assignment]
    command.upgrade(cfg, "head")
    yield engine
    engine.dispose()


@pytest.fixture
def db(db_engine: Engine) -> Iterator[Session]:
    conn = db_engine.connect()
    trans = conn.begin()
    session = Session(bind=conn, join_transaction_mode="create_savepoint", expire_on_commit=False)
    try:
        yield session
    finally:
        session.close()
        trans.rollback()
        conn.close()


@pytest.fixture
def client() -> Iterator[TestClient]:
    """API client for endpoints that do not touch the database."""
    with TestClient(create_app()) as c:
        yield c


@pytest.fixture
def db_client(db: Session) -> Iterator[TestClient]:
    app = create_app()
    app.dependency_overrides[get_db] = lambda: db
    with TestClient(app) as c:
        yield c
