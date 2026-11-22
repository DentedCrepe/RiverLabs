from collections.abc import Iterator
from functools import lru_cache

from sqlalchemy import Engine, create_engine
from sqlalchemy.orm import Session, sessionmaker

from riverlab_api.config import get_settings


def make_engine(url: str) -> Engine:
    return create_engine(url, pool_pre_ping=True, future=True)


@lru_cache
def get_engine() -> Engine:
    return make_engine(str(get_settings().database_url))


@lru_cache
def get_sessionmaker() -> sessionmaker[Session]:
    return sessionmaker(bind=get_engine(), expire_on_commit=False)


def get_db() -> Iterator[Session]:
    """FastAPI dependency: one session per request, committed by the service layer."""
    session = get_sessionmaker()()
    try:
        yield session
    except Exception:
        session.rollback()
        raise
    finally:
        session.close()
