"""Play against bots: create a table, deal, act, leave, review the report."""

import uuid
from typing import Annotated, Any, Literal

from fastapi import APIRouter, Depends, Query, Response
from pydantic import Field
from sqlalchemy.orm import Session

from riverlab_api.api.guest import Guest, GuestForbiddenError
from riverlab_api.db.models import PlaySession
from riverlab_api.db.session import get_db
from riverlab_api.errors import NotFoundError
from riverlab_api.schemas.common import StrictModel
from riverlab_api.schemas.play import PlayAction, PlaySessionCreate, PlaySessionOut, PlayState, RevealIn
from riverlab_api.services import hand_review, play

router = APIRouter(prefix="/play", tags=["play"])
DbSession = Annotated[Session, Depends(get_db)]


def _out(row: PlaySession) -> PlaySessionOut:
    return PlaySessionOut(
        id=row.id,
        format=row.format,
        status=row.status,
        error_message=row.error_message,
        hands=row.hands,
        hero_net=row.hero_net,
        created_at=row.created_at,
        finished_at=row.finished_at,
        config=row.config,
        tracked=row.tracked,
    )


def _state(db: Session, session_id: uuid.UUID, table: dict[str, Any] | None) -> PlayState:
    row = play.require().get(db, session_id)
    db.refresh(row)
    return PlayState(session=_out(row), table=table)


@router.post("/sessions", response_model=PlayState, status_code=201)
def create_session(body: PlaySessionCreate, db: DbSession, guest: Guest) -> PlayState:
    """Creates a table. Its status is "preparing" until the bots' preflop model is ready.
    With `tracked` false, or in guest mode, it never counts towards your statistics."""
    m = play.require()
    row = m.create(db, body.config, tracked=body.tracked and not guest)
    return _state(db, row.id, m.view(db, row.id))


@router.get("/sessions", response_model=list[PlaySessionOut])
def list_sessions(
    db: DbSession, guest: Guest, limit: Annotated[int, Query(ge=1, le=200)] = 50
) -> list[PlaySessionOut]:
    """All games (untracked ones are marked), or in guest mode only untracked games."""
    return [_out(r) for r in play.require().list(db, limit, only_untracked=guest)]


@router.get("/sessions/{session_id}", response_model=PlayState)
def get_session(session_id: uuid.UUID, db: DbSession) -> PlayState:
    return _state(db, session_id, play.require().view(db, session_id))


@router.post("/sessions/{session_id}/next", response_model=PlayState)
def next_hand(session_id: uuid.UUID, db: DbSession) -> PlayState:
    return _state(db, session_id, play.require().next_hand(db, session_id))


@router.post("/sessions/{session_id}/act", response_model=PlayState)
def act(session_id: uuid.UUID, body: PlayAction, db: DbSession) -> PlayState:
    return _state(db, session_id, play.require().act(db, session_id, body.action))


@router.post("/sessions/{session_id}/reveal", response_model=PlayState)
def reveal(session_id: uuid.UUID, body: RevealIn, db: DbSession) -> PlayState:
    """Practice mode: show the bots' cards during hands. Hands played with it on are flagged."""
    return _state(db, session_id, play.require().set_reveal(db, session_id, body.on))


@router.post("/sessions/{session_id}/leave", response_model=PlayState)
def leave(session_id: uuid.UUID, db: DbSession) -> PlayState:
    return _state(db, session_id, play.require().leave(db, session_id))


@router.get("/sessions/{session_id}/report")
def report(session_id: uuid.UUID, db: DbSession) -> dict[str, Any]:
    """Statistics and findings (play_engine::report::Report)."""
    return play.require().report(db, session_id)


@router.get("/sessions/{session_id}/hands")
def hands(session_id: uuid.UUID, db: DbSession) -> list[dict[str, Any]]:
    return play.require().records(db, session_id)


@router.get("/hands")
def all_hands(
    db: DbSession,
    guest: Guest,
    verdict: Annotated[Literal["mistake", "rare"] | None, Query()] = None,
    limit: Annotated[int, Query(ge=1, le=100)] = 30,
    offset: Annotated[int, Query(ge=0)] = 0,
) -> dict[str, Any]:
    """Your hands from every session, newest first; `verdict` keeps hands with a decision the
    feedback flagged ("mistake") or found unusual ("rare"). In guest mode, only untracked games."""
    return play.require().all_hands(db, verdict, limit, offset, only_untracked=guest)


@router.get("/sessions/{session_id}/hands/{hand_no}")
def one_hand(session_id: uuid.UUID, hand_no: int, db: DbSession) -> dict[str, Any]:
    """One stored hand record."""
    return play.require().hand(db, session_id, hand_no)


@router.get("/sessions/{session_id}/hands/{hand_no}/spot")
def spot(
    session_id: uuid.UUID, hand_no: int, db: DbSession, decision: Annotated[int, Query(ge=0, le=100)]
) -> dict[str, Any]:
    """A postflop solve spec for one of your heads-up decisions after the flop (SpotExport)."""
    return play.require().spot(db, session_id, hand_no, decision)


@router.post("/sessions/{session_id}/hands/{hand_no}/review", status_code=202)
def start_review(session_id: uuid.UUID, hand_no: int, db: DbSession) -> dict[str, Any]:
    """Solver review of the whole hand: one chained postflop solve per street, in the background."""
    reviewer = hand_review.require()
    return hand_review.review_out(reviewer.start(db, session_id, hand_no), db)


class BatchIn(StrictModel):
    scope: Literal["recent", "flagged", "biggest"] = "recent"
    limit: int = Field(default=10, ge=1, le=30)


@router.post("/reviews/batch", status_code=202)
def batch_reviews(body: BatchIn, db: DbSession, guest: Guest) -> dict[str, Any]:
    """Queues solver reviews for several of your hands; they run one after another."""
    if guest:
        raise GuestForbiddenError(
            "Batch reviews look for leaks in the owner's hands; turn guest mode off first"
        )
    return hand_review.require().batch(db, body.scope, body.limit)


@router.get("/reviews/status")
def reviews_status(db: DbSession) -> dict[str, Any]:
    return hand_review.require().status(db)


@router.get("/sessions/{session_id}/hands/{hand_no}/review")
def get_review(session_id: uuid.UUID, hand_no: int, db: DbSession) -> dict[str, Any]:
    row = hand_review.require().get(db, session_id, hand_no)
    if row is None:
        raise NotFoundError(f"Hand {hand_no} has no solver review yet")
    return hand_review.review_out(row, db)


@router.delete("/sessions/{session_id}", status_code=204)
def delete_session(session_id: uuid.UUID, db: DbSession, guest: Guest) -> Response:
    if guest and play.require().get(db, session_id).tracked:
        raise GuestForbiddenError("Guests can only delete guest games")
    play.require().delete(db, session_id)
    return Response(status_code=204)
