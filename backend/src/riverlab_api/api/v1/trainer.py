"""Push/fold trainer: heads-up jam/fold drills graded against the exact equilibrium."""

import secrets
from typing import Annotated, Any, Literal

from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session

from riverlab_api.db.models import PushFoldAnswer
from riverlab_api.db.session import get_db
from riverlab_api.schemas.common import StrictModel
from riverlab_api.services import trainer

router = APIRouter(prefix="/trainer/push-fold", tags=["trainer"])
DbSession = Annotated[Session, Depends(get_db)]


class DrillIn(StrictModel):
    min_stack_mbb: int
    max_stack_mbb: int
    seat: Literal["sb", "bb"] | None = None
    seed: int | None = None


class AnswerIn(StrictModel):
    drill: dict[str, Any]
    choice: Literal["fold", "play"]


@router.get("")
def status() -> dict[str, Any]:
    """Whether the exact equity table is loaded, loading, being built, missing or failed."""
    return trainer.service().status()


@router.post("/build")
def build() -> dict[str, Any]:
    """Builds the exact equity table if it is missing (tens of minutes), then loads it."""
    return trainer.service().build()


@router.post("/drill")
def drill(body: DrillIn) -> dict[str, Any]:
    seed = body.seed if body.seed is not None else secrets.randbits(63)
    return trainer.service().drill(body.min_stack_mbb, body.max_stack_mbb, body.seat, seed)


@router.post("/answer")
def answer(body: AnswerIn, db: DbSession) -> dict[str, Any]:
    """Grades a choice: EVs of both actions against the equilibrium opponent, and the EV lost.
    Every graded answer is stored for Analytics."""
    g = trainer.service().grade(body.drill, body.choice)
    d = g["drill"]
    db.add(
        PushFoldAnswer(
            stack_mbb=d["stack_mbb"],
            seat=d["seat"],
            cards="".join(d["cards"]),
            hand_class=d["class"],
            choice=g["choice"],
            verdict=g["verdict"],
            ev_loss_bb=g["ev_loss_bb"],
            grade=g,
        )
    )
    db.commit()
    return g


@router.get("/chart")
def chart(stack_mbb: Annotated[int, Query(ge=0, le=1_000_000)]) -> dict[str, Any]:
    """The exact equilibrium at one stack depth, by hand class."""
    return trainer.service().chart(stack_mbb)
