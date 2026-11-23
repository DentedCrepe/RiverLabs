"""Analytics from your own data only: hands played against the bots and graded trainer answers.
Nothing here is sample data; with no hands or answers the responses are empty."""

from typing import Annotated, Any, Literal

from fastapi import APIRouter, Depends, Query
from sqlalchemy import case, func, select
from sqlalchemy.orm import Session

from riverlab_api.db.models import PlayHand, PlaySession, PostflopAnswer, PushFoldAnswer
from riverlab_api.db.session import get_db
from riverlab_api.services import engine, leaks

router = APIRouter(prefix="/analytics", tags=["analytics"])
DbSession = Annotated[Session, Depends(get_db)]

# Stack bands (mbb, inclusive) for the trainer breakdown.
BANDS = [(2_000, 5_500), (6_000, 10_000), (10_500, 15_000), (15_500, 20_000), (20_500, 25_000)]


@router.get("/play")
def play(
    db: DbSession,
    scope: Annotated[Literal["cash", "sit_and_go"], Query()] = "cash",
) -> dict[str, Any]:
    """Every hand you played in this scope (cash and fast-fold, or sit & go), as one engine report,
    and the sessions they came from."""
    formats = ["cash", "fast_fold"] if scope == "cash" else ["sit_and_go"]
    sessions = list(
        db.scalars(
            select(PlaySession)
            .where(PlaySession.format.in_(formats), PlaySession.hands > 0)
            .order_by(PlaySession.created_at.desc())
        )
    )
    rows = list(
        db.scalars(
            select(PlayHand)
            .join(PlaySession, PlaySession.id == PlayHand.session_id)
            .where(PlaySession.format.in_(formats))
            .order_by(PlaySession.created_at, PlayHand.hand_no)
        )
    )
    records = [h.record for h in rows]
    report = engine.play_report(records, scope == "sit_and_go") if records else None
    if report is not None:
        # Each example names the game it came from, so its hand can be opened.
        for finding in report["findings"]:
            for ex in finding["examples"]:
                ex["session_id"] = str(rows[ex["record"]].session_id)
    return {
        "scope": scope,
        "sessions": [
            {
                "id": s.id,
                "format": s.format,
                "status": s.status,
                "created_at": s.created_at,
                "hands": s.hands,
                "hero_net": s.hero_net,
            }
            for s in sessions
        ],
        "report": report,
    }


def _summary(rows: Any) -> dict[str, Any]:
    n, correct, close, mistakes, lost = rows
    return {
        "answers": int(n or 0),
        "correct": int(correct or 0),
        "close": int(close or 0),
        "mistakes": int(mistakes or 0),
        "ev_lost_bb": float(lost or 0.0),
    }


def _cols(a: Any = PushFoldAnswer) -> list[Any]:
    return [
        func.count(a.id),
        func.sum(case((a.verdict == "correct", 1), else_=0)),
        func.sum(case((a.verdict == "close", 1), else_=0)),
        func.sum(case((a.verdict == "mistake", 1), else_=0)),
        func.sum(a.ev_loss_bb),
    ]


@router.get("/trainer")
def trainer(db: DbSession) -> dict[str, Any]:
    """Your push/fold answers: totals, by seat, by stack band, the hands that cost the most EV,
    and the latest answers. EV losses are the engine's, against the exact equilibrium."""
    a = PushFoldAnswer
    total = _summary(db.execute(select(*_cols())).one())
    by_seat = {
        seat: _summary(db.execute(select(*_cols()).where(a.seat == seat)).one()) for seat in ("sb", "bb")
    }
    by_band = [
        {
            "min_mbb": lo,
            "max_mbb": hi,
            **_summary(db.execute(select(*_cols()).where(a.stack_mbb.between(lo, hi))).one()),
        }
        for lo, hi in BANDS
    ]
    costly = db.execute(
        select(a.hand_class, a.seat, *_cols())
        .group_by(a.hand_class, a.seat)
        .having(func.sum(a.ev_loss_bb) > 0)
        .order_by(func.sum(a.ev_loss_bb).desc())
        .limit(10)
    ).all()
    recent = db.scalars(select(a).order_by(a.created_at.desc()).limit(20))
    return {
        "total": total,
        "by_seat": by_seat,
        "by_stack": by_band,
        "costliest": [{"hand_class": r[0], "seat": r[1], **_summary(r[2:])} for r in costly],
        "recent": [
            {
                "created_at": r.created_at,
                "stack_mbb": r.stack_mbb,
                "seat": r.seat,
                "cards": r.cards,
                "hand_class": r.hand_class,
                "choice": r.choice,
                "verdict": r.verdict,
                "ev_loss_bb": r.ev_loss_bb,
            }
            for r in recent
        ],
    }


@router.get("/postflop")
def postflop(db: DbSession) -> dict[str, Any]:
    """Your postflop drill answers: totals, by street and position, and the costliest hands. EV
    losses are the engine's, from the saved solves the drills came from."""
    a = PostflopAnswer
    total = _summary(db.execute(select(*_cols(a))).one())
    by_street = [
        {"street": st, **_summary(db.execute(select(*_cols(a)).where(a.street == st)).one())}
        for st in ("flop", "turn", "river")
    ]
    by_position = {
        name: _summary(db.execute(select(*_cols(a)).where(a.hero_player == p)).one())
        for p, name in ((0, "oop"), (1, "ip"))
    }
    costly = db.execute(
        select(a.hand_class, a.street, *_cols(a))
        .group_by(a.hand_class, a.street)
        .having(func.sum(a.ev_loss_bb) > 0)
        .order_by(func.sum(a.ev_loss_bb).desc())
        .limit(10)
    ).all()
    return {
        "total": total,
        "by_street": by_street,
        "by_position": by_position,
        "costliest": [{"hand_class": r[0], "street": r[1], **_summary(r[2:])} for r in costly],
    }


@router.get("/leaks")
def leaks_summary(db: DbSession) -> dict[str, Any]:
    """EV lost at your decisions across every finished solver review of your hands."""
    return leaks.summary(db)
