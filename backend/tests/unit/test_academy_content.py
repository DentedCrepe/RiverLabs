"""Academy content format and validation. Also guards the real curriculum: every lesson in
content/academy must validate, and every engine-computed answer must agree."""

from pathlib import Path

from riverlab_api.services.academy_content import (
    lesson_solves,
    parse_body,
    read_content,
    solve_query,
    validate_questions,
)

CONTENT = Path(__file__).resolve().parents[3] / "content" / "academy"


def test_the_real_content_validates_without_errors() -> None:
    report = read_content(CONTENT)
    assert report.errors == []
    assert report.warnings == []
    slugs = {lesson.slug for lesson in report.lessons}
    assert {"pot-odds", "outs-and-draws", "combinatorics", "mdf-and-bluff-ratios", "regret-matching"} <= slugs
    assert [lv["number"] for lv in report.curriculum] == [1, 2, 3, 4, 5, 6]
    for lesson in report.lessons:
        # Published lessons offer every explanation level somewhere, and a knowledge check.
        levels = {b.get("level") for s in lesson.sections for b in s["blocks"] if b["type"] == "markdown"}
        assert {"simple", "intermediate", "advanced", "math"} & levels, lesson.slug
        assert lesson.questions, lesson.slug


def test_body_parsing_levels_callouts_components() -> None:
    body = """Intro text.

## First {#first}
Shared.

::: simple
Easy words.
:::

::: math
$$x^2$$
:::

```riverlab:pot-odds
{"pot_bb": "10", "bet_bb": "5"}
```

```python
::: not a block inside code
```

::: fact
A fact.
:::
"""
    errors: list[str] = []
    sections = parse_body(body, {"pot-odds"}, "t.md", errors)
    assert errors == []
    assert [s["id"] for s in sections] == ["intro", "first"]
    blocks = sections[1]["blocks"]
    assert [b["type"] for b in blocks] == [
        "markdown",
        "markdown",
        "markdown",
        "component",
        "markdown",
        "callout",
    ]
    assert blocks[1] == {"type": "markdown", "level": "simple", "text": "Easy words."}
    assert blocks[3] == {"type": "component", "name": "pot-odds", "props": {"pot_bb": "10", "bet_bb": "5"}}
    assert "::: not a block" in blocks[4]["text"], "code fences are passed through untouched"
    assert blocks[5] == {"type": "callout", "kind": "fact", "text": "A fact."}


def test_body_errors_are_reported_with_line_numbers() -> None:
    errors: list[str] = []
    parse_body(
        "## No id\n::: simple\n::: math\n:::\n```riverlab:nope\n{bad\n```\n:::\n", set(), "t.md", errors
    )
    text = "\n".join(errors)
    assert "t.md:1: section headings need an id" in text
    assert "cannot be nested" in text
    assert "unknown component 'nope'" in text
    assert "invalid JSON" in text
    assert "closes nothing" in text


def test_questions_are_checked_against_the_engine() -> None:
    q = {
        "id": "q",
        "type": "numeric",
        "prompt": "p",
        "explanation": "e",
        "answer": 30.0,
        "tolerance": 0.5,
        "compute": {
            "kind": "pot_odds",
            "params": {"pot_mbb": 15000, "to_call_mbb": 5000},
            "field": "required_equity",
            "scale": 100.0,
        },
    }
    errors: list[str] = []
    validate_questions("l", [q], errors)
    assert any("disagrees with the engine (25)" in e for e in errors), errors
    errors.clear()
    out = validate_questions("l", [{**q, "answer": 25.0}], errors)
    assert errors == []
    assert out[0]["data"]["computed"] is True
    errors.clear()
    validate_questions(
        "l",
        [
            {
                "id": "m",
                "type": "multiple_choice",
                "prompt": "p",
                "explanation": "e",
                "options": ["a"],
                "answer": 3,
            }
        ],
        errors,
    )
    assert any("at least two options" in e for e in errors)


def _pot_odds_question(**scene: object) -> dict[str, object]:
    return {
        "id": "q",
        "type": "numeric",
        "prompt": "p",
        "explanation": "e",
        "answer": 25.0,
        "tolerance": 0.5,
        "compute": {
            "kind": "pot_odds",
            "params": {"pot_mbb": 30000, "to_call_mbb": 10000},
            "field": "required_equity",
            "scale": 100.0,
        },
        "scene": {"players": 6, "hero": "BB", **scene},
    }


def test_scenes_are_checked_and_must_match_the_numbers() -> None:
    errors: list[str] = []
    good = _pot_odds_question(
        hero_cards=["Qh", "Qc"], board=["Ks", "7d", "2c"], pot_bb="20", bets_bb={"BTN": "10"}, folded=["UTG"]
    )
    out = validate_questions("l", [good], errors)
    assert errors == []
    scene = out[0]["data"]["scene"]
    assert scene["pot_mbb"] == 20000
    assert scene["bets_mbb"] == {"BTN": 10000}
    assert scene["seats"] == ["UTG", "HJ", "CO", "BTN", "SB", "BB"]
    # A picture that disagrees with the computed price is rejected.
    validate_questions("l", [_pot_odds_question(pot_bb="15", bets_bb={"BTN": "10"})], errors)
    assert any("scene shows a pot of 25000" in e for e in errors), errors
    for bad, message in [
        ({"hero": "XX"}, "hero must be one of"),
        ({"hero_cards": ["Ah", "Ah"]}, "repeats a card"),
        ({"hero_cards": ["Zz", "Ah"]}, "scene cards"),
        ({"board": ["Ks", "7d"]}, "board of 0, 3, 4 or 5"),
        ({"pot_bb": "1.0005"}, "at most 3 decimals"),
        ({"folded": ["BB"]}, "cannot have folded"),
        ({"bets_bb": {"UTG2": "1"}}, "not at a 6-player table"),
        ({"stack": "1"}, "unknown scene fields"),
    ]:
        errors.clear()
        validate_questions("l", [_pot_odds_question(**bad)], errors)
        assert any(message in e for e in errors), (bad, errors)


def test_showdown_answers_and_equity_are_checked_by_the_engine() -> None:
    q = {
        "id": "s",
        "type": "decision",
        "prompt": "p",
        "explanation": "e",
        "options": ["You win", "Your opponent wins", "Split pot"],
        "answer": 0,
        "showdown_check": {"board": "KsKd7c7h2s", "you": "Ac3d", "opponent": "QsQd"},
    }
    errors: list[str] = []
    validate_questions("l", [q], errors)
    assert any("your opponent wins" in e for e in errors), errors
    errors.clear()
    validate_questions("l", [{**q, "answer": 1}], errors)
    assert errors == []
    split = {**q, "answer": 2, "showdown_check": {"board": "5h6d7c8s9h", "you": "Ac2d", "opponent": "Kh3s"}}
    validate_questions("l", [split], errors)
    assert errors == []
    equity = {
        "id": "e",
        "type": "numeric",
        "prompt": "p",
        "explanation": "e",
        "answer": 82.64,
        "tolerance": 0.01,
        "compute": {
            "kind": "equity",
            "params": {"ranges": ["AsAh", "KsKh"]},
            "field": "equity.0",
            "scale": 100.0,
        },
    }
    validate_questions("l", [equity], errors)
    assert errors == []
    validate_questions("l", [{**equity, "answer": 80.0}], errors)
    assert any("disagrees with the engine (82.6" in e for e in errors), errors


_SOLVE_TABLE = {
    "toy": {
        "board": "KsQd7h7c2s",
        "ranges": ["AQ,JJ", "KK,43o,TT"],
        "pot_bb": "10",
        "stack_bb": "10",
        "ip_bets": [100],
        "iterations": 300,
    }
}


def _solve_question(**params: object) -> dict[str, object]:
    return {
        "id": "s",
        "type": "numeric",
        "prompt": "p",
        "explanation": "e",
        "answer": 0.0,
        "tolerance": 100.0,
        "compute": {
            "kind": "solve",
            "params": {"solve": "toy", "path": [{"type": "action", "value": 0}], **params},
            "field": "share.check",
            "scale": 100.0,
        },
    }


def test_lesson_solves_build_valid_specs_and_reject_bad_ones() -> None:
    errors: list[str] = []
    solves = lesson_solves("l", _SOLVE_TABLE, errors)
    assert errors == []
    assert solves["toy"]["starting_pot_mbb"] == 10000
    assert solves["toy"]["tree"]["ip"]["bet_pct_pot"] == [[], [], [100]]
    lesson_solves("l", {"bad": {**_SOLVE_TABLE["toy"], "board": "KsQd7h"}}, errors)
    assert any("turn or a river" in e for e in errors), errors
    errors.clear()
    lesson_solves("l", {"bad": {**_SOLVE_TABLE["toy"], "pot_bb": "10.0001"}}, errors)
    assert errors, "an inexact pot must be rejected"


def test_solve_questions_run_the_solver_and_enforce_the_exploitability_bound() -> None:
    errors: list[str] = []
    solves = lesson_solves("l", _SOLVE_TABLE, errors)
    validate_questions("l", [_solve_question()], errors, solves)
    assert errors == []
    validate_questions("l", [_solve_question(max_exploitability_pct=0.0)], errors, solves)
    assert any("above the lesson's bound" in e for e in errors), errors
    errors.clear()
    validate_questions(
        "l",
        [
            {
                **_solve_question(),
                "compute": {"kind": "solve", "params": {"solve": "missing"}, "field": "share.check"},
            }
        ],
        errors,
        solves,
    )
    assert any("no solve named 'missing'" in e for e in errors), errors


def test_solve_query_shares_sum_to_one_and_subset_parts_are_fractions() -> None:
    solves = lesson_solves("l", _SOLVE_TABLE, [])
    r = solve_query({"spec": solves["toy"], "path": [{"type": "action", "value": 0}], "subset": ["43o"]})
    n = len(r["actions"])
    assert abs(sum(r["share"][f"a{i}"] for i in range(n)) - 1.0) < 1e-6  # strategies are f32
    for k, v in r["subset_of_action"].items():
        assert -1e-9 <= v <= 1 + 1e-9, (k, v)


def test_prerequisites_come_earlier_and_prompts_stand_alone(tmp_path: Path) -> None:
    import shutil

    root = tmp_path / "academy"
    shutil.copytree(CONTENT, root)
    lesson = root / "lessons" / "equity.md"
    text = lesson.read_text()
    # equity (level 1) now requires a later lesson, and one prompt refers to another question.
    text = text.replace("prerequisites = [", 'prerequisites = ["node-locking", ', 1)
    text = text.replace('prompt = "', 'prompt = "In the same spot as the previous question, ', 1)
    lesson.write_text(text)
    report = read_content(root)
    assert any("prerequisite 'node-locking'" in e for e in report.errors), report.errors
    assert any("prompts must stand alone" in e for e in report.errors), report.errors
