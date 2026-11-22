"""Differential validation of the RiverLab engine against independent evaluators.

Oracles (see docs/solver-research.md): phevaluator (Apache-2.0, C implementation) and
pokerkit (MIT, pure Python). Neither shares code with RiverLab.
"""

import itertools
import json
import random

import pytest
import riverlab_engine as eng
from phevaluator import evaluate_cards
from pokerkit import StandardHighHand

pytestmark = pytest.mark.validation

RANKS = "23456789TJQKA"
SUITS = "cdhs"
DECK = [r + s for r in RANKS for s in SUITS]

# phevaluator: rank 1 (best) .. 7462 (worst), grouped by category.
PHE_CATEGORY_BOUNDS = [
    (10, "straight_flush"),
    (166, "four_of_a_kind"),
    (322, "full_house"),
    (1599, "flush"),
    (1609, "straight"),
    (2467, "three_of_a_kind"),
    (3325, "two_pair"),
    (6185, "one_pair"),
    (7462, "high_card"),
]


def phe_category(rank: int) -> str:
    return next(cat for bound, cat in PHE_CATEGORY_BOUNDS if rank <= bound)


def ours(cards: list[str]) -> tuple[int, str]:
    r = json.loads(eng.evaluate_hand("".join(cards)))
    cat = "straight_flush" if r["category"] == "royal_flush" else r["category"]
    return r["value"], cat


def sign(x: int) -> int:
    return (x > 0) - (x < 0)


def test_strength_is_a_bijection_with_phevaluator_classes() -> None:
    """Over many random 7-card hands, our strength values and phevaluator's equivalence
    classes must map one-to-one, with matching categories and reversed order."""
    rng = random.Random(20261122)
    ours_to_phe: dict[int, int] = {}
    phe_to_ours: dict[int, int] = {}
    for _ in range(150_000):
        cards = rng.sample(DECK, 7)
        value, cat = ours(cards)
        phe = evaluate_cards(*cards)
        assert cat == phe_category(phe), cards
        assert ours_to_phe.setdefault(value, phe) == phe, cards
        assert phe_to_ours.setdefault(phe, value) == value, cards
    # Order: higher strength <=> lower phevaluator rank.
    pairs = sorted(ours_to_phe.items())
    assert all(a[1] > b[1] for a, b in itertools.pairwise(pairs))
    assert len(pairs) > 3000


def test_pairwise_order_agrees_with_pokerkit() -> None:
    rng = random.Random(7)
    for _ in range(5_000):
        cards = rng.sample(DECK, 9)
        board, a, b = cards[:5], cards[5:7], cards[7:9]
        mine = sign(ours(board + a)[0] - ours(board + b)[0])
        ha = StandardHighHand.from_game("".join(a), "".join(board))
        hb = StandardHighHand.from_game("".join(b), "".join(board))
        theirs = (ha > hb) - (ha < hb)
        assert mine == theirs, (board, a, b)


@pytest.mark.parametrize(
    "hands",
    [["AsAh", "KsKh"], ["AsKd", "QcQh"], ["7c6c", "AhKs"]],
)
def test_flop_equity_matches_phevaluator_enumeration(hands: list[str]) -> None:
    rng = random.Random(hash(tuple(hands)) & 0xFFFF)
    used = {hands[0][:2], hands[0][2:], hands[1][:2], hands[1][2:]}
    for _ in range(4):
        board = rng.sample([c for c in DECK if c not in used], 3)
        live = [c for c in DECK if c not in used and c not in board]
        share = [0.0, 0.0]
        n = 0
        for turn, river in itertools.combinations(live, 2):
            full = [*board, turn, river]
            r0 = evaluate_cards(*full, hands[0][:2], hands[0][2:])
            r1 = evaluate_cards(*full, hands[1][:2], hands[1][2:])
            if r0 < r1:
                share[0] += 1
            elif r1 < r0:
                share[1] += 1
            else:
                share[0] += 0.5
                share[1] += 0.5
            n += 1
        result = json.loads(eng.equity(hands, board="".join(board)))
        assert result["samples"] == n
        assert result["equity"][0] == pytest.approx(share[0] / n, abs=1e-12)


def test_preflop_aa_vs_kk_full_enumeration() -> None:
    """All 1,712,304 boards, independently enumerated with phevaluator (~10 s)."""
    a, b = ("As", "Ah"), ("Ks", "Kh")
    live = [c for c in DECK if c not in (*a, *b)]
    wins = ties = n = 0
    for board in itertools.combinations(live, 5):
        ra = evaluate_cards(*board, *a)
        rb = evaluate_cards(*board, *b)
        wins += ra < rb
        ties += ra == rb
        n += 1
    expected = (wins + ties / 2) / n
    result = json.loads(eng.equity(["AsAh", "KsKh"]))
    assert n == result["samples"] == 1_712_304
    assert result["equity"][0] == pytest.approx(expected, abs=1e-12)
    assert result["tie"][0] == pytest.approx(ties / n, abs=1e-12)
