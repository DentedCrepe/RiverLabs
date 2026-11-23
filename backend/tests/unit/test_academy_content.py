"""Academy content format and validation. Also guards the real curriculum: every lesson in
content/academy must validate, and every engine-computed answer must agree."""

from pathlib import Path

from riverlab_api.services.academy_content import parse_body, read_content, validate_questions

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
