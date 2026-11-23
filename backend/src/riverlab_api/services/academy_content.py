"""Reads and validates Academy content files (format: docs/academy.md).

A lesson file is TOML front matter between `+++` lines, then a Markdown body:

- `## Title {#section-id}` starts a section.
- `::: simple|intermediate|advanced|math` ... `:::` is text for one explanation level.
- `::: fact|heuristic|approximation|solver` ... `:::` is a labelled callout.
- A code fence with language `riverlab:<component>` and a JSON body embeds an interactive component.
- Anything else is Markdown shown at every level.

Numeric answers with a `compute` table are recomputed by the engine and must match.
"""

import hashlib
import json
import re
import tomllib
from dataclasses import dataclass, field
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
    for part in path.split("."):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def compute_answer(spec: dict[str, Any]) -> float:
    """Runs a question's `compute` table through the engine."""
    kind, params = spec["kind"], spec.get("params", {})
    if kind == "range":
        result: Any = engine.analyze_range(params["notation"], params.get("dead", ""))
    else:
        result = engine.theory(kind, params)
    return float(_field(result, spec["field"])) * float(spec.get("scale", 1.0))


def validate_questions(lesson: str, questions: list[Any], errors: list[str]) -> list[dict[str, Any]]:
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
        if kind in ("multiple_choice", "decision"):
            options = q.get("options", [])
            answer = q.get("answer")
            if len(options) < 2 or not isinstance(answer, int) or not 0 <= answer < len(options):
                errors.append(f"{where}: needs at least two options and an answer index")
            data.update(options=options, answer=answer)
        else:
            answer, tol = q.get("answer"), q.get("tolerance", 0)
            if not isinstance(answer, int | float) or not isinstance(tol, int | float) or tol < 0:
                errors.append(f"{where}: numeric questions need a number answer and tolerance >= 0")
                continue
            if "compute" in q:
                try:
                    computed = compute_answer(q["compute"])
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
    questions = validate_questions(slug, fm.get("questions", []), errors)
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
