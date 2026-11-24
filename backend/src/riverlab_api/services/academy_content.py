"""Reads and validates Academy content files (format: docs/academy.md).

A lesson file is TOML front matter between `+++` lines, then a Markdown body:

- `## Title {#section-id}` starts a section.
- `::: simple|intermediate|advanced|math` ... `:::` is text for one explanation level.
- `::: fact|heuristic|approximation|solver` ... `:::` is a labelled callout.
- A code fence with language `riverlab:<component>` and a JSON body embeds an interactive component.
- Anything else is Markdown shown at every level.

Numeric answers with a `compute` table are recomputed by the engine and must match. A question
may set a table `scene` (seats, your cards, the board, the pot and bets); it is checked at sync and
must agree with the question's pot-odds numbers.
"""

import hashlib
import json
import re
import tomllib
from dataclasses import dataclass, field
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

from riverlab_api.services import engine

LEVELS = ("simple", "intermediate", "advanced", "math")
CALLOUTS = ("fact", "heuristic", "approximation", "solver")
STATUSES = ("draft", "in_review", "published")
QUESTION_KINDS = ("multiple_choice", "decision", "numeric")
SECTION_RE = re.compile(r"^##\s+(.+?)\s*\{#([a-z0-9][a-z0-9-]*)\}\s*$")
SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,79}$")


@dataclass
class ParsedLesson:
    slug: str
    level: int
    position: int
    title: str
    summary: str
    status: str
    version: int
    meta: dict[str, Any]
    sections: list[dict[str, Any]]
    questions: list[dict[str, Any]]
    content_hash: str
    source_path: str


@dataclass
class ContentReport:
    lessons: list[ParsedLesson] = field(default_factory=list)
    curriculum: list[dict[str, Any]] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def load_components(root: Path) -> set[str]:
    data: dict[str, str] = json.loads((root / "components.json").read_text())
    return set(data)


def load_curriculum(root: Path, report: ContentReport) -> dict[str, int]:
    """Returns topic slug -> level."""
    data = tomllib.loads((root / "curriculum.toml").read_text())
    topics: dict[str, int] = {}
    numbers = [lv.get("number") for lv in data.get("levels", [])]
    if numbers != [1, 2, 3, 4, 5, 6]:
        report.errors.append(f"curriculum.toml: levels must be numbered 1-6 in order, got {numbers}")
    for lv in data.get("levels", []):
        for t in lv.get("topics", []):
            slug = t.get("slug", "")
            if not SLUG_RE.match(slug) or slug in topics:
                report.errors.append(f"curriculum.toml: invalid or duplicate topic slug {slug!r}")
            topics[slug] = lv["number"]
    report.curriculum = data.get("levels", [])
    return topics


def parse_body(body: str, components: set[str], path: str, errors: list[str]) -> list[dict[str, Any]]:
    sections: list[dict[str, Any]] = [{"id": "intro", "title": "", "blocks": []}]
    block_kind: str | None = None
    buf: list[str] = []
    fence: str | None = None  # "riverlab:<name>" or "code"
    fence_buf: list[str] = []

    def flush(level: str | None = None, callout: str | None = None) -> None:
        text = "\n".join(buf).strip()
        buf.clear()
        if not text:
            return
        if callout:
            sections[-1]["blocks"].append({"type": "callout", "kind": callout, "text": text})
        else:
            sections[-1]["blocks"].append({"type": "markdown", "level": level, "text": text})

    def current() -> tuple[str | None, str | None]:
        if block_kind in LEVELS:
            return block_kind, None
        if block_kind in CALLOUTS:
            return None, block_kind
        return None, None

    for n, line in enumerate(body.splitlines(), 1):
        stripped = line.strip()
        if fence is not None:
            if stripped == "```":
                if fence == "code":
                    buf.extend([*fence_buf, "```"])
                else:
                    name = fence.split(":", 1)[1]
                    try:
                        props = json.loads("\n".join(fence_buf) or "{}")
                    except json.JSONDecodeError as exc:
                        errors.append(f"{path}:{n}: component {name}: invalid JSON ({exc})")
                        props = {}
                    if name not in components:
                        errors.append(f"{path}:{n}: unknown component {name!r}")
                    flush(*current())
                    sections[-1]["blocks"].append({"type": "component", "name": name, "props": props})
                fence, fence_buf = None, []
            else:
                fence_buf.append(line)
            continue
        if stripped.startswith("```riverlab:"):
            fence = stripped[3:]
            continue
        if stripped.startswith("```"):
            fence = "code"
            fence_buf = [line]
            continue
        m = SECTION_RE.match(stripped)
        if m:
            if block_kind is not None:
                errors.append(f"{path}:{n}: section starts inside an unclosed ::: block")
            flush(*current())
            if any(s["id"] == m.group(2) for s in sections):
                errors.append(f"{path}:{n}: duplicate section id {m.group(2)!r}")
            sections.append({"id": m.group(2), "title": m.group(1), "blocks": []})
            continue
        if stripped.startswith("## "):
            errors.append(f"{path}:{n}: section headings need an id, e.g. '## Title {{#title}}'")
        if stripped.startswith(":::"):
            kind = stripped[3:].strip()
            if kind == "":
                if block_kind is None:
                    errors.append(f"{path}:{n}: ':::' closes nothing")
                flush(*current())
                block_kind = None
            elif kind in LEVELS or kind in CALLOUTS:
                if block_kind is not None:
                    errors.append(f"{path}:{n}: ::: blocks cannot be nested")
                flush(*current())
                block_kind = kind
            else:
                errors.append(f"{path}:{n}: unknown block '::: {kind}'")
            continue
        buf.append(line)
    if fence is not None:
        errors.append(f"{path}: unclosed code fence")
    if block_kind is not None:
        errors.append(f"{path}: unclosed '::: {block_kind}' block")
    flush(*current())
    return [s for s in sections if s["blocks"] or s["id"] != "intro"]


def _field(value: Any, path: str) -> Any:
    if path == "":  # the result itself is the number
        return value
    for part in path.split("."):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


# Solves run for lesson questions, by spec: several questions can read the same solve.
_SOLVES: dict[str, dict[str, Any]] = {}


def solve_query(params: dict[str, Any]) -> dict[str, Any]:
    """Runs a small postflop solve (cached) and reads one node of it.

    `params`: `spec` (a SolveSpec), `path` (steps from the root, default the root),
    `max_exploitability_pct` (default 0.5: the solve must be at least this close to equilibrium,
    so a lesson can state the bound), `subset` (optional hand-class labels, e.g. the bluffs).
    Returns `exploitability_pct_pot`, `values_bb` and, at a decision node, `share`: for each
    action kind ("check", "bet"), label ("bet 15bb") and index ("a3"), the share of the acting
    player's range (by combination weight) that takes it. With a subset: `subset_share` (the
    same, within the subset) and `subset_of_action` (for each action, the part of it that comes
    from the subset).
    """
    import tempfile

    spec = params["spec"]
    key = json.dumps(spec, sort_keys=True)
    if key not in _SOLVES:
        with tempfile.TemporaryDirectory() as tmp:
            path = f"{tmp}/lesson.gto"
            outcome = engine.solve_postflop(spec, engine.CancelToken(), lambda _: None, 1 << 30, 1, path)
            _SOLVES[key] = {"outcome": outcome, "game": engine.load_solved(spec, path)}
    solved = _SOLVES[key]
    outcome = solved["outcome"]
    expl = float(outcome["progress"]["exploitability_pct_pot"])
    bound = float(params.get("max_exploitability_pct", 0.5))
    if expl > bound:
        raise ValueError(
            f"the solve is {expl:.3f}% of the pot exploitable, above the lesson's bound of {bound}%"
        )
    node = engine.solved_node(solved["game"], params.get("path", []))
    subset = set(params.get("subset", []))

    def shares(classes: list[dict[str, Any]]) -> dict[str, float]:
        out: dict[str, float] = {}
        total = sum(c["weight"] for c in classes)
        if node["kind"] != "decision" or total <= 0:
            return out
        for a, label in enumerate(node["actions"]):
            mass = sum(c["weight"] * c["frequencies"][a] for c in classes) / total
            kind = "raise" if label.startswith("raise") else label.split(" ")[0]
            out[kind] = out.get(kind, 0.0) + mass
            out[f"a{a}"] = mass  # by index, for sizes whose labels contain a dot ("bet 3.3bb")
            if label != kind:
                out[label] = out.get(label, 0.0) + mass
        return out

    everyone = shares(node["by_class"])
    result: dict[str, Any] = {
        "exploitability_pct_pot": expl,
        "values_bb": [v / 1000.0 for v in outcome["values_mbb"]],
        # A node-locked solve: the values of the unlocked solve the locks were applied to.
        **(
            {"unlocked_values_bb": [v / 1000.0 for v in outcome["unlocked"]["values_mbb"]]}
            if outcome.get("unlocked")
            else {}
        ),
        "share": everyone,
        "actions": node["actions"],
    }
    if subset:
        part = [c for c in node["by_class"] if c["label"] in subset]
        within = shares(part)
        frac = sum(c["weight"] for c in part) / max(sum(c["weight"] for c in node["by_class"]), 1e-12)
        result["subset_share"] = within
        result["subset_of_action"] = {k: frac * within.get(k, 0.0) / v for k, v in everyone.items() if v > 0}
    return result


def compute_answer(spec: dict[str, Any]) -> float:
    """Runs a question's `compute` table through the engine."""
    kind, params = spec["kind"], spec.get("params", {})
    if kind == "range":
        result: Any = engine.analyze_range(params["notation"], params.get("dead", ""))
    elif kind == "solve":
        result = solve_query(params)
    elif kind == "equity":
        # Exact enumeration only: an answer key is never a Monte Carlo estimate.
        result = engine.equity(
            params["ranges"], params.get("board", ""), params.get("dead", ""), "exact", 0, 0, 10**10
        )
    else:
        result = engine.theory(kind, params)
    return float(_field(result, spec["field"])) * float(spec.get("scale", 1.0))


def _mbb(value: Any, what: str, errors: list[str]) -> int:
    """An exact bb amount ("2.5") as integer mbb; inexact or invalid input is an error."""
    try:
        d = Decimal(str(value)) * 1000
    except InvalidOperation:
        errors.append(f"{what}: {value!r} is not a number of bb")
        return 0
    if d != d.to_integral_value() or d < 0:
        errors.append(f"{what}: {value!r} must be a non-negative bb amount with at most 3 decimals")
        return 0
    return int(d)


def validate_scene(where: str, scene: Any, q: dict[str, Any], errors: list[str]) -> dict[str, Any] | None:
    """Checks a question's table scene and returns it with amounts in mbb (format: docs/academy.md)."""
    if not isinstance(scene, dict):
        errors.append(f"{where}: scene must be a table")
        return None
    allowed = {
        "players",
        "hero",
        "hero_cards",
        "board",
        "pot_bb",
        "bets_bb",
        "stacks_bb",
        "folded",
        "actions",
        "shown",
    }
    if unknown := set(scene) - allowed:
        errors.append(f"{where}: unknown scene fields {sorted(unknown)}")
    players = scene.get("players", 6)
    try:
        seats = [p["code"] for p in engine.positions(players)]
    except Exception as exc:
        errors.append(f"{where}: scene players: {exc}")
        return None
    hero = scene.get("hero")
    if hero not in seats:
        errors.append(f"{where}: scene hero must be one of {seats}")
        return None
    folded = scene.get("folded", [])
    shown = scene.get("shown", {})
    named = [
        *folded,
        *scene.get("bets_bb", {}),
        *scene.get("stacks_bb", {}),
        *scene.get("actions", {}),
        *shown,
    ]
    if bad := [s for s in named if s not in seats]:
        errors.append(
            f"{where}: scene names seats that are not at a {players}-player table: {sorted(set(bad))}"
        )
    if hero in folded:
        errors.append(f"{where}: you cannot have folded")
    hero_cards, board = scene.get("hero_cards", []), scene.get("board", [])
    if len(hero_cards) not in (0, 2) or len(board) not in (0, 3, 4, 5):
        errors.append(f"{where}: scene needs 0 or 2 hole cards and a board of 0, 3, 4 or 5 cards")
    if hero in shown or any(len(c) != 2 for c in shown.values()):
        errors.append(f"{where}: scene shown cards are two cards per opponent")
    cards = [*hero_cards, *board, *(c for pair in shown.values() for c in pair)]
    if len({c.lower() for c in cards}) != len(cards):
        errors.append(f"{where}: scene repeats a card")
    elif cards:
        try:  # the engine rejects anything that is not a card
            engine.analyze_range("AA", "".join(cards))
        except Exception as exc:
            errors.append(f"{where}: scene cards: {exc}")
    pot = _mbb(scene.get("pot_bb", 0), f"{where}: scene pot_bb", errors)
    bets = {k: _mbb(v, f"{where}: scene bet of {k}", errors) for k, v in scene.get("bets_bb", {}).items()}
    stacks = {
        k: _mbb(v, f"{where}: scene stack of {k}", errors) for k, v in scene.get("stacks_bb", {}).items()
    }
    # The picture must match the numbers in the question.
    compute = q.get("compute", {})
    if compute.get("kind") in ("pot_odds", "implied_odds") and bets:
        facing = max(bets.values()) - bets.get(hero, 0)
        params = compute.get("params", {})
        shown = pot + sum(bets.values())
        if params.get("pot_mbb") != shown or params.get("to_call_mbb") != facing:
            errors.append(
                f"{where}: scene shows a pot of {shown} mbb (bets included) and {facing} mbb to call, "
                f"but compute uses {params.get('pot_mbb')} and {params.get('to_call_mbb')}"
            )
    return {
        "players": players,
        "seats": seats,
        "hero": hero,
        "hero_cards": hero_cards,
        "board": board,
        "pot_mbb": pot,
        "bets_mbb": bets,
        "stacks_mbb": stacks,
        "folded": folded,
        "actions": scene.get("actions", {}),
        "shown": shown,
    }


SHOWDOWN_OUTCOMES = ("you win", "your opponent wins", "split pot")


def check_showdown(where: str, check: Any, answer: Any, errors: list[str]) -> None:
    """`showdown_check = { board, you, opponent }`: options 0/1/2 must mean you win, your opponent
    wins, split pot; the evaluator decides which is right."""
    try:
        r = engine.compare_hands(check["board"], [check["you"], check["opponent"]])
    except Exception as exc:
        errors.append(f"{where}: showdown_check failed ({exc})")
        return
    winners = r["winners"]
    truth = 2 if len(winners) == 2 else winners[0]
    if answer != truth:
        errors.append(f"{where}: evaluator says {SHOWDOWN_OUTCOMES[truth]!r}, answer is option {answer}")


def lesson_solves(lesson: str, table: Any, errors: list[str]) -> dict[str, dict[str, Any]]:
    """`[solves.<name>]` in a lesson's front matter: small heads-up spots the lesson's questions
    solve (compute kind "solve" with `solve = "<name>"`). Fields: `board` (turn or river),
    `ranges` ([oop, ip]), `pot_bb`, `stack_bb`, `oop_bets`, `ip_bets` (% of pot on the board's
    street and after), optional `oop_raises`, `ip_raises` (% of pot) and `max_raises` (per street;
    1 when raise sizes are given, else 0), optional `locks` (node locks: `{path, frequencies}`),
    `iterations`. No rake."""
    out: dict[str, dict[str, Any]] = {}
    if not isinstance(table, dict):
        errors.append(f"{lesson}: solves must be a table of named spots")
        return out
    game = engine.default_game_config()
    game["rake"]["percentage_bps"] = 0
    for name, sp in table.items():
        where = f"{lesson}: solve {name!r}"
        try:
            board = [sp["board"][i : i + 2] for i in range(0, len(sp["board"]), 2)]
            first = len(board) - 3
            if first not in (1, 2):
                raise ValueError("board must be a turn or a river")

            def sizing(bets: list[int], raises: list[int], first: int = first) -> dict[str, Any]:
                return {
                    "bet_pct_pot": [list(bets) if i >= first else [] for i in range(3)],
                    "raise_pct_pot": [list(raises) if i >= first else [] for i in range(3)],
                }

            iterations = int(sp.get("iterations", 1000))
            out[name] = {
                "game": game,
                "ranges": list(sp["ranges"]),
                "board": board,
                "starting_pot_mbb": _mbb(sp["pot_bb"], f"{where} pot_bb", errors),
                "effective_stack_mbb": _mbb(sp["stack_bb"], f"{where} stack_bb", errors),
                "tree": {
                    "oop": sizing(sp.get("oop_bets", []), sp.get("oop_raises", [])),
                    "ip": sizing(sp.get("ip_bets", []), sp.get("ip_raises", [])),
                    "max_raises_per_street": int(
                        sp.get("max_raises", 1 if sp.get("oop_raises") or sp.get("ip_raises") else 0)
                    ),
                    "all_in_threshold_pct_stack": 100,
                    "add_all_in": False,
                },
                **({"locks": list(sp["locks"])} if sp.get("locks") else {}),
                "solver": {
                    "schema_version": 1,
                    "variant": {"algorithm": "discounted", "alpha": 1.5, "beta": 0.0, "gamma": 2.0},
                    "averaging": "quadratic",
                    "stopping": {
                        "max_iterations": iterations,
                        "target_exploitability_pct_pot": None,
                        "max_seconds": None,
                        "exploitability_check_interval": iterations,
                    },
                    "threads": 1,
                    "seed": None,
                },
            }
            engine.validate_solve_spec(out[name])
        except Exception as exc:
            errors.append(f"{where}: {exc}")
            out.pop(name, None)
    return out


def validate_questions(
    lesson: str, questions: list[Any], errors: list[str], solves: dict[str, dict[str, Any]] | None = None
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    seen: set[str] = set()
    for q in questions:
        qid = q.get("id", "")
        where = f"{lesson}: question {qid!r}"
        if not SLUG_RE.match(qid) or qid in seen:
            errors.append(f"{where}: invalid or duplicate id")
            continue
        seen.add(qid)
        kind = q.get("type")
        if kind not in QUESTION_KINDS:
            errors.append(f"{where}: type must be one of {QUESTION_KINDS}")
            continue
        if not q.get("prompt") or not q.get("explanation"):
            errors.append(f"{where}: prompt and explanation are required")
        data: dict[str, Any] = {"explanation": q.get("explanation", "")}
        if "scene" in q:
            data["scene"] = validate_scene(where, q["scene"], q, errors)
        if kind in ("multiple_choice", "decision"):
            options = q.get("options", [])
            answer = q.get("answer")
            if len(options) < 2 or not isinstance(answer, int) or not 0 <= answer < len(options):
                errors.append(f"{where}: needs at least two options and an answer index")
            data.update(options=options, answer=answer)
            if "showdown_check" in q:
                check_showdown(where, q["showdown_check"], answer, errors)
        else:
            answer, tol = q.get("answer"), q.get("tolerance", 0)
            if not isinstance(answer, int | float) or not isinstance(tol, int | float) or tol < 0:
                errors.append(f"{where}: numeric questions need a number answer and tolerance >= 0")
                continue
            if "compute" in q:
                try:
                    c = q["compute"]
                    if c.get("kind") == "solve" and "solve" in c.get("params", {}):
                        named = c["params"]["solve"]
                        if named not in (solves or {}):
                            raise ValueError(f"no solve named {named!r} in this lesson")
                        c = {**c, "params": {**c["params"], "spec": (solves or {})[named]}}
                    computed = compute_answer(c)
                except Exception as exc:
                    errors.append(f"{where}: compute failed ({exc})")
                else:
                    if abs(computed - answer) > max(tol, 1e-9):
                        errors.append(f"{where}: answer {answer} disagrees with the engine ({computed:.6g})")
            data.update(answer=answer, tolerance=tol, unit=q.get("unit", ""), computed="compute" in q)
        out.append(
            {
                "id": qid,
                "kind": kind,
                "prompt": q.get("prompt", ""),
                "data": data,
                "concepts": q.get("concepts", []),
            }
        )
    return out


def read_lesson(
    path: Path, root: Path, components: set[str], topics: dict[str, int], report: ContentReport
) -> None:
    rel = str(path.relative_to(root))
    text = path.read_text()
    parts = text.split("+++\n", 2)
    if len(parts) != 3 or parts[0].strip():
        report.errors.append(f"{rel}: must start with TOML front matter between '+++' lines")
        return
    errors: list[str] = []
    try:
        fm = tomllib.loads(parts[1])
    except tomllib.TOMLDecodeError as exc:
        report.errors.append(f"{rel}: invalid front matter ({exc})")
        return
    slug = fm.get("slug", "")
    if not SLUG_RE.match(slug) or path.stem != slug:
        errors.append(f"{rel}: slug must be lowercase-with-dashes and match the file name")
    for key in ("title", "summary"):
        if not isinstance(fm.get(key), str) or not fm[key]:
            errors.append(f"{rel}: {key} is required")
    level, version, status = fm.get("level"), fm.get("version"), fm.get("status")
    if level not in (1, 2, 3, 4, 5, 6):
        errors.append(f"{rel}: level must be 1-6")
    if not isinstance(version, int) or version < 1:
        errors.append(f"{rel}: version must be a positive integer")
    if status not in STATUSES:
        errors.append(f"{rel}: status must be one of {STATUSES}")
    review = fm.get("review", {})
    if review.get("status") not in ("pending", "reviewed"):
        errors.append(f"{rel}: review.status must be 'pending' or 'reviewed'")
    if review.get("status") == "reviewed" and not (review.get("reviewer") and review.get("reviewed_on")):
        errors.append(f"{rel}: a reviewed lesson needs review.reviewer and review.reviewed_on")
    for p in fm.get("prerequisites", []):
        if p not in topics:
            errors.append(f"{rel}: prerequisite {p!r} is not a curriculum topic")
    for t in fm.get("tools", []):
        if not str(t.get("href", "")).startswith("/") or not t.get("label"):
            errors.append(f"{rel}: tools need an internal href and a label")
    for r in fm.get("reading", []):
        if not (r.get("title") and r.get("authors") and isinstance(r.get("year"), int)):
            errors.append(f"{rel}: reading entries need title, authors and year")
    sections = parse_body(parts[2], components, rel, errors)
    solves = lesson_solves(slug, fm.get("solves", {}), errors)
    questions = validate_questions(slug, fm.get("questions", []), errors, solves)
    qids = {q["id"] for q in questions}
    for s in sections:
        for b in s["blocks"]:
            if b["type"] == "component" and b["name"] == "question" and b["props"].get("id") not in qids:
                errors.append(f"{rel}: embedded question {b['props'].get('id')!r} is not defined")
    if errors:
        report.errors.extend(errors)
        return
    meta = {
        k: fm.get(k, d)
        for k, d in (
            ("objectives", []),
            ("prerequisites", []),
            ("tags", []),
            ("tools", []),
            ("reading", []),
            ("review", {}),
            ("estimated_minutes", None),
        )
    }
    report.lessons.append(
        ParsedLesson(
            slug=slug,
            level=int(fm["level"]),
            position=int(fm.get("order", 0)),
            title=fm["title"],
            summary=fm["summary"],
            status=str(status),
            version=int(fm["version"]),
            meta=meta,
            sections=sections,
            questions=questions,
            content_hash=hashlib.sha256(text.encode()).hexdigest(),
            source_path=rel,
        )
    )


def read_content(root: Path) -> ContentReport:
    report = ContentReport()
    if not (root / "curriculum.toml").exists():
        report.errors.append(f"{root}: curriculum.toml not found")
        return report
    topics = load_curriculum(root, report)
    components = load_components(root)
    for path in sorted((root / "lessons").glob("*.md")):
        read_lesson(path, root, components, topics, report)
    linked = {t.get("lesson") for lv in report.curriculum for t in lv.get("topics", [])} - {None}
    slugs = {lesson.slug for lesson in report.lessons}
    for missing in sorted(linked - slugs):
        report.warnings.append(f"curriculum.toml links to lesson {missing!r}, which is missing or invalid")
    for unlinked in sorted(slugs - linked):
        report.warnings.append(f"lesson {unlinked!r} is not linked from any curriculum topic")
    return report
