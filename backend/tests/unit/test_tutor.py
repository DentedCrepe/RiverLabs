"""AI tutor building blocks that need no database or network: the number check, the arithmetic
tool and the lesson text the model is given."""

from typing import Any, ClassVar

import pytest

from riverlab_api.services import tutor


def test_numbers_in_prose_skip_cards_and_hand_classes() -> None:
    text = "With 9♦ and T9s on K♠ 7♦ 2♣ the price is 25% (1,326 combos; 22+; QQ #pot-odds 42.86)."
    assert tutor.numbers_in(text) == ["25", "1,326", "42.86"]


def test_traced_numbers_match_sources_at_written_precision() -> None:
    sources = [0.4285714285714286, 7500.0, 1326.0]
    assert tutor.traced("42.86", sources)  # a share written as a percentage
    assert tutor.traced("42.9", sources)
    assert tutor.traced("7.5", sources)  # mbb written in bb
    assert tutor.traced("1,326", sources)
    assert tutor.traced("3", [])  # small counts are always allowed
    assert not tutor.traced("43.5", sources)
    assert not tutor.traced("57", sources)


def test_arithmetic_is_exact_and_refuses_anything_else() -> None:
    assert tutor.arithmetic("7.5 / (10 + 7.5) * 100") == pytest.approx(42.857142857)
    assert tutor.arithmetic("-2 ** 2") == -4
    for bad in ("__import__('os')", "a + 1", "2 ** 1000", "1 / 0", "[1, 2]", "print(1)"):
        with pytest.raises((ValueError, ZeroDivisionError)):
            tutor.arithmetic(bad)


class _Row:
    slug = "pot-odds"
    title = "Pot odds"
    level = 1
    summary = "Prices."
    meta: ClassVar[dict[str, Any]] = {
        "reading": [{"title": "The Mathematics of Poker", "authors": "Chen and Ankenman", "year": 2006}]
    }
    sections: ClassVar[list[dict[str, Any]]] = [
        {
            "id": "price",
            "title": "The price",
            "blocks": [
                {"type": "markdown", "level": None, "text": "shared"},
                {"type": "markdown", "level": "simple", "text": "easy"},
                {"type": "markdown", "level": "math", "text": "hard"},
                {"type": "callout", "kind": "fact", "text": "a fact"},
                {"type": "component", "name": "pot-odds", "props": {"pot_bb": "10"}},
            ],
        }
    ]


def test_lesson_text_follows_the_learner_level() -> None:
    simple = tutor.lesson_text(_Row(), "simple")  # type: ignore[arg-type]
    assert "easy" in simple
    assert "hard" not in simple
    assert "[Established fact] a fact" in simple
    assert "{#price}" in simple
    assert "The Mathematics of Poker" in simple
    assert "interactive widget `pot-odds`" in simple
    advanced = tutor.lesson_text(_Row(), "advanced")  # type: ignore[arg-type]
    assert "easy" in advanced, "nearest lower level"
    assert "hard" not in advanced


def test_every_eval_case_has_engine_backed_expectations() -> None:
    from riverlab_api import tutor_eval

    assert len({c.id for c in tutor_eval.CASES}) == len(tutor_eval.CASES)
    for case in tutor_eval.CASES:
        for expected, tol in case.numbers:
            assert 0 <= expected() <= 100, case.id
            assert tol > 0
        assert case.numbers or case.must_decline or case.any_of, case.id
    # Scoring: an answer that declines without a percentage passes a trap.
    trap = next(c for c in tutor_eval.CASES if c.id == "gto-trap")
    ok = tutor_eval.score(
        trap,
        {
            "answer": "I can't give that without a stored solve; solve it in the Solver.",
            "unverified_numbers": [],
            "dropped_readings": [],
        },
    )
    assert all(ok.values()), ok
