"""Daily goals, streak and today's study plan (docs/goals.md).

A day counts towards the streak when you play at least `hands_target` hands against the bots and
complete at least `study_target` study items (Academy questions answered, push/fold and postflop
drills). Days follow this device's local time. The plan recommends what to study from your own
data: your most frequent weakness at the tables, your Academy revision queue, and your weakest
trainer area. Nothing is recommended from data you do not have.
"""

from collections import Counter
from datetime import date, datetime, timedelta
from typing import Any

from sqlalchemy import func, select
from sqlalchemy.orm import Session

from riverlab_api.db.models import (
    AcademyAttempt,
    DailyGoals,
    PlayHand,
    PostflopAnswer,
    PushFoldAnswer,
)
from riverlab_api.services import academy, engine

HISTORY_DAYS = 400
RECENT_HANDS = 300

# Where to study each weakness the session report can find (play_engine::judge keys).
WEAKNESS_STUDY: dict[str, tuple[str, str]] = {
    "preflop_too_loose": ("/preflop", "Compare your opening and calling ranges with the preflop model"),
    "preflop_too_tight": ("/preflop", "Compare your opening and calling ranges with the preflop model"),
    "preflop_passive": ("/preflop", "See which hands the preflop model raises rather than calls"),
    "called_without_odds": ("/academy/pot-odds", "Pot odds and break-even equity"),
    "called_without_odds_early": ("/academy/implied-odds", "Implied and reverse implied odds"),
    "folded_with_odds": ("/academy/pot-odds", "Pot odds and break-even equity"),
    "folded_with_odds_early": ("/academy/equity", "Equity against a range"),
}


def _local_day(ts: datetime) -> date:
    return ts.astimezone().date()


def _counts(db: Session, since: datetime) -> tuple[Counter[date], Counter[date]]:
    hands: Counter[date] = Counter(
        _local_day(t) for t in db.scalars(select(PlayHand.created_at).where(PlayHand.created_at >= since))
    )
    study: Counter[date] = Counter()
    for model in (AcademyAttempt, PushFoldAnswer, PostflopAnswer):
        study.update(
            _local_day(t) for t in db.scalars(select(model.created_at).where(model.created_at >= since))
        )
    return hands, study


def goals(db: Session) -> DailyGoals:
    row = db.get(DailyGoals, 1)
    if row is None:
        row = DailyGoals(id=1, hands_target=30, study_target=10)
        db.add(row)
        db.commit()
    return row


def set_goals(db: Session, hands_target: int, study_target: int) -> DailyGoals:
    row = goals(db)
    row.hands_target, row.study_target = hands_target, study_target
    db.commit()
    return row


def _streaks(met: set[date], today: date) -> tuple[int, int]:
    """Current streak (today counts once met; until then it runs to yesterday) and the best."""
    start = today if today in met else today - timedelta(days=1)
    current = 0
    while start - timedelta(days=current) in met:
        current += 1
    best, run, prev = 0, 0, None
    for d in sorted(met):
        run = run + 1 if prev is not None and d == prev + timedelta(days=1) else 1
        best, prev = max(best, run), d
    return current, max(best, current)


def _weakness(db: Session) -> dict[str, Any] | None:
    rows = list(db.scalars(select(PlayHand).order_by(PlayHand.created_at.desc()).limit(RECENT_HANDS)))
    cash = [r.record for r in rows if not r.record.get("tournament")]
    if not cash:
        return None
    report = engine.play_report(list(reversed(cash)), False)
    for f in report["findings"]:  # most frequent first
        if f["count"] >= 2 and f["key"] in WEAKNESS_STUDY:
            href, what = WEAKNESS_STUDY[f["key"]]
            return {
                "kind": "weakness",
                "title": what,
                "reason": (
                    f"{f['title']}: {f['count']} of {f['out_of']} decisions "
                    f"in your last {len(cash)} cash hands."
                ),
                "href": href,
                "also": {"href": "/analytics", "label": "Flick through those hands"},
            }
    return None


def _trainer(db: Session) -> dict[str, Any] | None:
    pf = PostflopAnswer
    worst = db.execute(
        select(pf.street, func.count(pf.id), func.avg(pf.ev_loss_pct_pot))
        .group_by(pf.street)
        .having(func.count(pf.id) >= 5)
        .order_by(func.avg(pf.ev_loss_pct_pot).desc())
        .limit(1)
    ).first()
    if worst is not None and worst[2] > 1.0:
        return {
            "kind": "trainer",
            "title": f"Postflop drills on the {worst[0]}",
            "reason": (
                f"Your {worst[1]} {worst[0]} answers lose {worst[2]:.1f}% of the pot on average, "
                "your weakest street."
            ),
            "href": "/trainer",
        }
    pff = PushFoldAnswer
    seat = db.execute(
        select(pff.seat, func.count(pff.id), func.avg(pff.ev_loss_bb))
        .group_by(pff.seat)
        .having(func.count(pff.id) >= 5)
        .order_by(func.avg(pff.ev_loss_bb).desc())
        .limit(1)
    ).first()
    if seat is not None and seat[2] > 0.001:
        side = "small blind (jam or fold)" if seat[0] == "sb" else "big blind (call or fold)"
        return {
            "kind": "trainer",
            "title": f"Push/fold drills in the {side}",
            "reason": f"You lose {seat[2]:.3f}bb per answer there, more than in the other seat.",
            "href": "/trainer",
        }
    return {
        "kind": "trainer",
        "title": "Push/fold drills",
        "reason": "Graded against the exact heads-up equilibrium: a quick way to complete your study target.",
        "href": "/trainer",
    }


def plan(db: Session) -> dict[str, Any]:
    g = goals(db)
    today = datetime.now().astimezone().date()
    since = datetime.now().astimezone() - timedelta(days=HISTORY_DAYS)
    hands, study = _counts(db, since)
    met = {d for d in set(hands) | set(study) if hands[d] >= g.hands_target and study[d] >= g.study_target}
    current, best = _streaks(met, today)
    recs: list[dict[str, Any]] = []
    if w := _weakness(db):
        recs.append(w)
    rev = academy.revision(db)
    if rev["due"]:
        n = len(rev["due"])
        recs.append(
            {
                "kind": "revision",
                "title": f"Revise {n} Academy question{'s' if n != 1 else ''}",
                "reason": "They are due under your spaced-repetition schedule.",
                "href": "/academy/revision",
            }
        )
    else:
        nxt = academy.dashboard(db).get("recommended")
        if nxt:
            recs.append(
                {
                    "kind": "lesson",
                    "title": nxt["title"],
                    "reason": "The next Academy lesson whose prerequisites you have covered.",
                    "href": f"/academy/{nxt['slug']}",
                }
            )
    if t := _trainer(db):
        recs.append(t)
    days = [today - timedelta(days=i) for i in range(13, -1, -1)]
    return {
        "today": {
            "date": today.isoformat(),
            "hands": hands[today],
            "study": study[today],
            "done": today in met,
        },
        "targets": {"hands": g.hands_target, "study": g.study_target},
        "streak": current,
        "best_streak": best,
        "days": [
            {"date": d.isoformat(), "hands": hands[d], "study": study[d], "met": d in met} for d in days
        ],
        "recommended": recs,
    }
