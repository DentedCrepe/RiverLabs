from typing import Annotated

from fastapi import APIRouter, Depends
from sqlalchemy import text
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from riverlab_api import __version__
from riverlab_api.db.session import get_db
from riverlab_api.schemas.meta import HealthResponse, ReadinessResponse
from riverlab_api.services import activity, engine

router = APIRouter(tags=["health"])


@router.get("/health", response_model=HealthResponse)
def health() -> HealthResponse:
    """Liveness: the process is up."""
    return HealthResponse(status="ok", version=__version__)


@router.get("/health/ready", response_model=ReadinessResponse)
def ready(db: Annotated[Session, Depends(get_db)]) -> ReadinessResponse:
    """Readiness: database reachable and engine loaded."""
    try:
        db.execute(text("SELECT 1"))
        database = "ok"
    except SQLAlchemyError:
        database = "unavailable"
    return ReadinessResponse(
        status="ok" if database == "ok" else "degraded",
        database=database,
        engine=engine.engine_version(),
    )


@router.get("/health/activity")
def activity_status() -> dict[str, float | bool]:
    """For an idle watchdog: seconds since the last user request, and whether work is running."""
    return {"idle_seconds": round(activity.idle_seconds(), 1), "busy": activity.busy()}
