"""Evaluation set for the Academy AI tutor (docs/academy-ai-tutor.md).

    cd backend && uv run python -m riverlab_api.tutor_eval [--only ID ...]

Runs each case through the real tutor (Claude via the Anthropic API: it needs ANTHROPIC_API_KEY
and costs money; roughly one tutor answer per case) against the synced Academy content in the
configured database, and scores:
- accuracy: the answer contains the expected number(s), within tolerance (expected values come
  from the lessons' engine-backed questions, recomputed here with the engine);
- grounding: no untraced numbers were flagged, and no reference was dropped;
- refusals: trap questions must be declined (no invented "GTO" frequency, no invented
  population statistic), and caveat questions must state the model's limits.

Not part of `scripts/test.sh` (no API calls in CI); run it after changing the tutor's prompt,
tools or model, and compare the totals.
"""

import argparse
import re
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from riverlab_api.db.session import get_sessionmaker
from riverlab_api.services import engine, tutor


@dataclass
class Case:
    id: str
    lesson: str
    question: str
    level: str = "intermediate"
    # Expected numbers as they should appear (percent or bb), with absolute tolerance.
    numbers: list[tuple[Callable[[], float], float]] = field(default_factory=list)
    # The answer must decline to give a number of this kind.
    must_decline: bool = False
    # Phrases at least one of which must appear (case-insensitive), e.g. a model's limits.
    any_of: list[str] = field(default_factory=list)


def _pct(kind: str, params: dict[str, Any], key: str) -> Callable[[], float]:
    return lambda: float(engine.theory(kind, params)[key]) * 100


def _equity(ranges: list[str], board: str = "") -> Callable[[], float]:
    return lambda: float(engine.equity(ranges, board, "", "exact", 0, 0, 10**10)["equity"][0]) * 100


def _icm_value(stacks: list[int], prizes: list[int], player: int) -> Callable[[], float]:
    return lambda: float(engine.theory("icm", {"stacks": stacks, "prizes": prizes})["values"][player])


CASES = [
    Case(
        "half-pot-price",
        "pot-odds",
        "My opponent bets half the pot on the river, heads-up, no rake. What equity do I need to call?",
        numbers=[(_pct("pot_odds", {"pot_mbb": 15000, "to_call_mbb": 5000}, "required_equity"), 0.6)],
    ),
    Case(
        "pot-bet-price",
        "pot-odds",
        "What equity does calling a pot-sized river bet need, heads-up with no rake?",
        numbers=[(_pct("pot_odds", {"pot_mbb": 20000, "to_call_mbb": 10000}, "required_equity"), 0.6)],
    ),
    Case(
        "aa-vs-kk",
        "equity",
        "How often does A♠A♥ win against K♦K♣ all-in before the flop, counting ties as half?",
        numbers=[(_equity(["AsAh", "KdKc"]), 0.2)],
    ),
    Case(
        "flush-draw-outs",
        "outs-and-draws",
        "I have a flush draw on the flop (9 outs). How likely am I to hit by the river "
        "if all the money goes in now?",
        numbers=[(_pct("outs", {"outs": 9, "unseen": 47}, "two_cards"), 0.6)],
    ),
    Case(
        "mdf-pot",
        "mdf-and-bluff-ratios",
        "Against a pot-sized bet, what is the minimum defence frequency?",
        numbers=[(_pct("bet_math", {"pot_mbb": 10000, "bet_mbb": 10000}, "mdf"), 0.6)],
        any_of=["model", "one bet", "single bet", "assum", "bluff-catcher"],
    ),
    Case(
        "icm-big-stack",
        "icm",
        "Three players left with 5,000, 3,000 and 2,000 chips, paying 50/30/20. "
        "What is the big stack worth under ICM?",
        numbers=[(_icm_value([5000, 3000, 2000], [50, 30, 20], 0), 0.1)],
    ),
    Case(
        "gto-trap",
        "polarised-ranges",
        "What is the GTO c-bet frequency for the button on K♠7♦2♣ in a single-raised pot?",
        must_decline=True,
        any_of=["solve", "solver", "no stored", "don't have", "do not have", "cannot"],
    ),
    Case(
        "population-trap",
        "population-tendencies",
        "What percentage of players at 1/2 live games fold to a river overbet?",
        must_decline=True,
        any_of=["no data", "don't have", "do not have", "cannot", "measure", "your own"],
    ),
    Case(
        "mdf-always",
        "mdf-and-bluff-ratios",
        "Is MDF the defence frequency I should always use?",
        any_of=["model", "assum", "exploit", "not always", "only"],
    ),
    Case(
        "simple-level",
        "equity",
        "What does equity mean?",
        level="simple",
        any_of=["share", "win", "pot"],
    ),
]


NUMBER = re.compile(r"\d+(?:\.\d+)?")


def score(case: Case, out: dict[str, Any]) -> dict[str, Any]:
    text = out["answer"].lower()
    found = [float(n) for n in NUMBER.findall(out["answer"].replace(",", ""))]
    checks: dict[str, bool] = {}
    for i, (expected, tol) in enumerate(case.numbers):
        value = expected()
        checks[f"number {i + 1} ({value:.2f})"] = any(abs(f - value) <= tol for f in found)
    if case.must_decline:
        # Declining means giving no percentage for the asked-for frequency.
        checks["declined"] = "%" not in out["answer"]
    if case.any_of:
        checks["caveat"] = any(p in text for p in case.any_of)
    checks["grounded"] = not out["unverified_numbers"] and not out["dropped_readings"]
    return checks


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--only", nargs="*", help="case ids to run")
    args = parser.parse_args()
    cases = [c for c in CASES if not args.only or c.id in args.only]
    if not tutor.status()["available"]:
        print(f"The tutor is not available: {tutor.status()['reason']}", file=sys.stderr)
        return 2
    passed = total = 0
    with get_sessionmaker()() as db:
        for case in cases:
            try:
                out = tutor.ask(db, case.lesson, case.level, case.question, [], consent=True)
                checks = score(case, out)
            except Exception as exc:  # report and continue: one failure should not hide the rest
                print(f"{case.id:18} ERROR {exc}")
                total += 1
                continue
            ok = all(checks.values())
            passed += ok
            total += 1
            detail = ", ".join(f"{k}: {'ok' if v else 'FAIL'}" for k, v in checks.items())
            print(f"{case.id:18} {'PASS' if ok else 'FAIL'}  {detail}")
    print(f"\n{passed} of {total} cases passed")
    return 0 if passed == total else 1


if __name__ == "__main__":
    sys.exit(main())
