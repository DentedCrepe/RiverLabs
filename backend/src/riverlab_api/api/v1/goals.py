"""Daily goals, streak and today's study plan."""

from typing import Annotated, Any

from fastapi import APIRouter, Depends
from pydantic import Field
from sqlalchemy.orm import Session

from riverlab_api.db.session import get_db
from riverlab_api.schemas.common import StrictModel
from riverlab_api.services import daily

router = APIRouter(prefix="/goals", tags=["goals"])
DbSession = Annotated[Session, Depends(get_db)]


class GoalsIn(StrictModel):
    hands_target: int = Field(ge=1, le=1000)
    study_target: int = Field(ge=1, le=1000)


@router.get("/today")
def today(db: DbSession) -> dict[str, Any]:
    """Progress towards today's targets, the streak, the last 14 days and what to study."""
    return daily.plan(db)


@router.put("")
def set_goals(body: GoalsIn, db: DbSession) -> dict[str, Any]:
    daily.set_goals(db, body.hands_target, body.study_target)
    return daily.plan(db)
