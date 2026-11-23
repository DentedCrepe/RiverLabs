"""Solver leaks: EV lost at your decisions across every finished hand review (docs/play.md).

Every number comes from the engine's per-decision analysis of a solved street
(`play_engine::review::DecisionReview`); this module only adds them up. A decision counts as a
leak when it loses more than `LEAK_PCT_POT` of its street's starting pot, the threshold the
postflop drills use between "correct" and "close".
"""

import uuid
from typing import Any

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.db.models import HandReview, PlayHand, PlaySession, SolverJob

LEAK_PCT_POT = 1.0
TOP = 20

KIND_WORDS = {"fold": "folding", "check": "checking", "call": "calling", "bet": "betting", "raise": "raising"}


def kind_of(label: str) -> str:
    """ "bet 6.6bb" -> "bet", "raise to 20bb" -> "raise"."""
    return "raise" if label.startswith("raise") else label.split(" ")[0]


def category(taken: str, best: str) -> str:
    """A plain description of the mistake, e.g. "Called when folding was better"."""
    t, b = kind_of(taken), kind_of(best)
    if t == b:
        return (
            "Bet the wrong size"
            if t == "bet"
            else "Raised the wrong size"
            if t == "raise"
            else f"{t.title()}"
        )
    past = {"fold": "Folded", "check": "Checked", "call": "Called", "bet": "Bet", "raise": "Raised"}[t]
    return f"{past} when {KIND_WORDS[b]} was better"


def _bucket() -> dict[str, Any]:
    return {"decisions": 0, "leaks": 0, "ev_lost_bb": 0.0}


def summary(db: Session) -> dict[str, Any]:
    # Only your games: reviews of guest games never count as your leaks.
    reviews = list(
        db.scalars(
            select(HandReview)
            .join(PlaySession, PlaySession.id == HandReview.session_id)
            .where(HandReview.status == "done", PlaySession.tracked)
        )
    )
    pots: dict[str, float] = {}
    job_ids = [uuid.UUID(st["job_id"]) for r in reviews for st in r.streets if st.get("job_id")]
    for job in db.scalars(select(SolverJob).where(SolverJob.id.in_(job_ids))) if job_ids else []:
        pots[str(job.id)] = float(job.spec.get("starting_pot_mbb", 0)) / 1000.0
    sessions = {r.session_id for r in reviews}
    hands = (
        {
            (h.session_id, h.hand_no): h.record
            for h in db.scalars(select(PlayHand).where(PlayHand.session_id.in_(sessions)))
        }
        if sessions
        else {}
    )
    total = _bucket()
    by_street: dict[str, dict[str, Any]] = {s: _bucket() for s in ("flop", "turn", "river")}
    by_position: dict[str, dict[str, Any]] = {}
    by_category: dict[str, dict[str, Any]] = {}
    costly: list[dict[str, Any]] = []
    for r in reviews:
        record = hands.get((r.session_id, r.hand_no))
        if record is None:
            continue
        hero = next(i for i, s in enumerate(record["seats"]) if s["is_hero"])
        position = record["seats"][hero]["position"]
        for st in r.streets:
            rv = st.get("review")
            if st.get("status") != "done" or not rv:
                continue
            pot = pots.get(st.get("job_id", ""), 0.0)
            for d in rv["decisions"]:
                loss = d.get("ev_loss_bb")
                if loss is None:
                    continue
                pct = 100.0 * loss / pot if pot > 0 else 0.0
                leak = pct > LEAK_PCT_POT
                for b in (total, by_street[st["street"]], by_position.setdefault(position, _bucket())):
                    b["decisions"] += 1
                    b["ev_lost_bb"] += loss
                    b["leaks"] += int(leak)
                if not leak or d.get("best") is None:
                    continue
                taken, best = d["actions"][d["taken"]], d["actions"][d["best"]]
                c = by_category.setdefault(category(taken, best), {"leaks": 0, "ev_lost_bb": 0.0})
                c["leaks"] += 1
                c["ev_lost_bb"] += loss
                costly.append(
                    {
                        "session_id": str(r.session_id),
                        "hand_no": r.hand_no,
                        "street": st["street"],
                        "decision": d["decision"],
                        "position": position,
                        "taken": taken,
                        "best": best,
                        "ev_loss_bb": loss,
                        "ev_loss_pct_pot": pct,
                        "category": category(taken, best),
                    }
                )
    costly.sort(key=lambda x: -x["ev_loss_bb"])
    return {
        "reviews": len(reviews),
        "threshold_pct_pot": LEAK_PCT_POT,
        "total": total,
        "by_street": [{"street": k, **v} for k, v in by_street.items()],
        "by_position": [{"position": k, **v} for k, v in sorted(by_position.items())],
        "by_category": sorted(
            ({"category": k, **v} for k, v in by_category.items()), key=lambda x: -x["ev_lost_bb"]
        ),
        "costliest": costly[:TOP],
    }
