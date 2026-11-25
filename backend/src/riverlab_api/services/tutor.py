"""Academy AI tutor (docs/academy-ai-tutor.md): Claude answers questions about a lesson, with
RiverLab's engine and stored results as its only source of numbers.

The loop runs here, on the Messages API: Claude calls the tools below (all local, read-only),
the backend runs them, and Claude's final answer comes back as structured JSON (answer, lesson
sections used, works cited, lessons to read first). Then a post-check:
- every number in the answer must be traceable to a tool result, the lesson text, the
  conversation or a small whole number; otherwise Claude is asked once to recompute or remove
  them, and anything still untraced is flagged in the answer;
- works cited must be in a lesson's reading list, and section ids must exist.

Only the lesson, the learner's Academy progress and the conversation are sent to Anthropic, and
only when the request says the learner agreed (`consent`). Hand histories never are.
"""

import ast
import json
import logging
import math
import operator
import re
import uuid
from dataclasses import dataclass, field
from typing import Any

from sqlalchemy import select
from sqlalchemy.orm import Session

from riverlab_api.config import get_settings
from riverlab_api.db.models import AcademyLesson, JobStatus, SolverJob, SolverResult, StrategyNode
from riverlab_api.errors import ApiError, EngineValidationError, NotFoundError
from riverlab_api.services import academy, engine

logger = logging.getLogger(__name__)

LEVELS = ("simple", "intermediate", "advanced", "math")
THEORY_KINDS = [
    "pot_odds",
    "bet_math",
    "ev_call",
    "ev_bluff",
    "outs",
    "implied_odds",
    "semi_bluff",
    "zero_sum_2x2",
    "icm",
    "icm_call",
    "sample_frequency",
]
# Tool rounds per answer (each may call several tools), and correction passes after the check.
MAX_ROUNDS = 8
MAX_CORRECTIONS = 1
# Monte Carlo fallback when exact equity is over budget: fixed and seeded, so it is repeatable.
MC_ITERATIONS = 200_000
MC_SEED = 7


class TutorUnavailableError(ApiError):
    status_code = 503
    code = "tutor_unavailable"


class TutorConsentError(ApiError):
    status_code = 403
    code = "tutor_consent_required"


SYSTEM = """You are the AI tutor in RiverLab Academy, a no-limit hold'em theory course. You help one
learner with the lesson they are reading. Your answers are labelled "AI tutor (may be wrong)"; the
reviewed lessons remain the authority.

How to answer
- Answer at the learner's explanation level (given with the lesson): simple = plain words, no
  formulas; intermediate = some notation; advanced = full reasoning; math = derivations.
- Keep answers short and focused on the question. Use Markdown. Point to lesson sections by id.
- If the question depends on a topic the learner has not completed, say which lesson to read
  first (list it in `suggested_lessons`). Use the `curriculum` tool to find lessons.

Numbers: you never produce a number yourself
- Every number you state must come from a tool call in this conversation, or appear in the
  lesson text or the conversation. Use `theory`, `equity`, `analyze_range` and `solver_result`
  for poker numbers and `arithmetic` for any sum, ratio or percentage, even an easy one.
- If no tool can produce a number, say so instead of estimating.
- When you quote a tool result, say where it came from (e.g. "RiverLab's engine, exact
  enumeration" or "stored solve <job id>, exploitability X% of the pot").

Four kinds of claim, kept apart (as in the lessons)
- Established fact: state it directly, with the model it holds for (e.g. one bet, no rake).
- Heuristic: a rule of thumb, never a law.
- Approximation: with its stated limits.
- Solver result: only by citing a stored RiverLab result from `solver_result`, with its
  exploitability and approximations. A solve is an approximate equilibrium for its configured
  bet sizes, never "GTO" without that qualification.
- You do not know the "GTO" or "correct" frequency of any spot unless a stored result gives it.
  If asked and there is none, decline and suggest solving the spot in RiverLab's Solver.

References
- Cite only works that appear in a lesson's reading list (given with the lesson, or from the
  `lesson` tool). Never invent references, statistics about real players or solver output.

Output
- Finish with the structured answer: `answer` (Markdown), `sections` (ids of the sections of the
  current lesson you relied on, or "slug#id" for other lessons), `readings` (exact titles of
  works you cited, from reading lists), `suggested_lessons` (lesson slugs)."""

ANSWER_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "answer": {"type": "string"},
        "sections": {"type": "array", "items": {"type": "string"}},
        "readings": {"type": "array", "items": {"type": "string"}},
        "suggested_lessons": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["answer", "sections", "readings", "suggested_lessons"],
    "additionalProperties": False,
}


def _strict(name: str, description: str, properties: dict[str, Any], required: list[str]) -> dict[str, Any]:
    return {
        "name": name,
        "description": description,
        "strict": True,
        "input_schema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": False,
        },
    }


TOOLS: list[dict[str, Any]] = [
    {
        "name": "theory",
        "description": (
            "RiverLab's closed-form poker theory, exact for each stated model. Kinds and params: "
            "pot_odds {pot_mbb, to_call_mbb} (pot includes the bet faced); bet_math {pot_mbb, bet_mbb} "
            "(MDF, bluff ratio, alpha); ev_call {equity, pot_mbb, to_call_mbb}; ev_bluff "
            "{fold_frequency, pot_mbb, bet_mbb}; outs {outs, unseen}; implied_odds {pot_mbb, "
            "to_call_mbb, equity}; semi_bluff {pot_mbb, bet_mbb, fold_frequency, equity}; "
            "zero_sum_2x2 {payoff: [[a,b],[c,d]]}; icm {stacks, prizes}; icm_call {stacks, hero, "
            "villain, pot, to_call, prizes}; sample_frequency {successes, trials}. Amounts are "
            "integer milli-big-blinds (1bb = 1000); frequencies and equities are 0-1."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "kind": {"type": "string", "enum": THEORY_KINDS},
                "params": {"type": "object"},
            },
            "required": ["kind", "params"],
        },
    },
    _strict(
        "equity",
        "All-in equity of 2 to 6 hands or ranges (e.g. 'AsKd', 'QQ+,AKs') on a board of 0, 3, 4 or 5 "
        "cards (e.g. 'Ks7d2c'), from RiverLab's equity engine: exact enumeration when within budget, "
        "otherwise a seeded Monte Carlo estimate (the result says which). Equity counts ties as shares.",
        {
            "ranges": {"type": "array", "items": {"type": "string"}},
            "board": {"type": "string"},
        },
        ["ranges", "board"],
    ),
    _strict(
        "analyze_range",
        "Combinations in a range (e.g. 'QQ+,AK'), with cards removed by `dead` (a board or hand, e.g. "
        "'Kh7d2c'): live combos, weighted combos and the share of all hands.",
        {"notation": {"type": "string"}, "dead": {"type": "string"}},
        ["notation", "dead"],
    ),
    _strict(
        "arithmetic",
        "Evaluates an arithmetic expression exactly (+ - * / ** and parentheses, decimals), e.g. "
        "'7.5 / (10 + 7.5) * 100'. Use it for every calculation you show.",
        {"expression": {"type": "string"}},
        ["expression"],
    ),
    _strict(
        "lesson",
        "Another Academy lesson's text at the learner's level, with its section ids and reading list.",
        {"slug": {"type": "string"}},
        ["slug"],
    ),
    _strict(
        "curriculum",
        "The Academy's levels and topics, which lesson covers each, and the learner's status for each.",
        {},
        [],
    ),
    _strict(
        "list_solves",
        "The learner's most recent successful RiverLab solves (id, board, ranges, exploitability).",
        {"limit": {"type": "integer"}},
        ["limit"],
    ),
    _strict(
        "solver_result",
        "One stored RiverLab solve by job id: spot, solver, iterations, exploitability, approximations "
        "and the first decision's overall strategy. The only source of solver results.",
        {"job_id": {"type": "string"}},
        ["job_id"],
    ),
]


# ---- lesson text for the model ----

CALLOUT = {
    "fact": "Established fact",
    "heuristic": "Heuristic (rule of thumb)",
    "approximation": "Approximation",
    "solver": "Solver-specific result",
}


def _blocks_for_level(blocks: list[dict[str, Any]], level: str) -> list[dict[str, Any]]:
    """The same choice the lesson page makes (frontend/src/lib/academy.ts): untagged blocks and
    callouts, plus the learner's level, else the nearest lower one, else the nearest higher."""
    available = {b.get("level") for b in blocks if b.get("type") == "markdown" and b.get("level")}
    order = LEVELS.index(level) if level in LEVELS else 0
    candidates = list(reversed(LEVELS[: order + 1])) + list(LEVELS[order + 1 :])
    shown = next((lv for lv in candidates if lv in available), None)
    return [b for b in blocks if b.get("type") != "markdown" or not b.get("level") or b.get("level") == shown]


def lesson_text(row: AcademyLesson, level: str) -> str:
    out = [f"# {row.title} (lesson `{row.slug}`, level {row.level})", row.summary, ""]
    for s in row.sections:
        out.append(f"## {s.get('title', '')} {{#{s.get('id', '')}}}")
        for b in _blocks_for_level(s.get("blocks", []), level):
            if b["type"] == "markdown":
                out.append(b.get("text", ""))
            elif b["type"] == "callout":
                out.append(f"[{CALLOUT.get(b.get('kind', ''), b.get('kind', ''))}] {b.get('text', '')}")
            elif b["type"] == "component":
                props = json.dumps(b.get("props", {}), sort_keys=True)
                out.append(f"[interactive widget `{b.get('name')}` {props}]")
        out.append("")
    reading = row.meta.get("reading", [])
    if reading:
        out.append("Reading list (the only works you may cite from this lesson):")
        for r in reading:
            out.append(f"- {r.get('title')} ({r.get('authors')}, {r.get('year')})")
    return "\n".join(out)


def readings(db: Session) -> set[str]:
    return {
        str(r.get("title", "")).strip() for row in academy.lessons(db) for r in row.meta.get("reading", [])
    }


def learner_state(db: Session) -> str:
    dash = academy.dashboard(db)
    done = [c["slug"] for c in dash.get("completed", [])]
    rev = dash.get("revision", {})
    mis = [m["concept"] for m in rev.get("misconceptions", [])] if isinstance(rev, dict) else []
    return (
        f"Completed lessons: {', '.join(done) if done else 'none yet'}.\n"
        f"Concepts the learner has answered wrongly recently: {', '.join(mis) if mis else 'none recorded'}."
    )


# ---- tools ----


_OPS: dict[type, Any] = {
    ast.Add: operator.add,
    ast.Sub: operator.sub,
    ast.Mult: operator.mul,
    ast.Div: operator.truediv,
    ast.Pow: operator.pow,
    ast.USub: operator.neg,
    ast.UAdd: operator.pos,
}


def arithmetic(expression: str) -> float:
    """+ - * / ** on numbers only; anything else (names, calls, attributes) is refused."""
    if len(expression) > 300:
        raise ValueError("expression too long")

    def ev(node: ast.AST) -> float:
        if isinstance(node, ast.Expression):
            return ev(node.body)
        if (
            isinstance(node, ast.Constant)
            and isinstance(node.value, int | float)
            and not isinstance(node.value, bool)
        ):
            return float(node.value)
        if isinstance(node, ast.BinOp) and type(node.op) in _OPS:
            left, right = ev(node.left), ev(node.right)
            if isinstance(node.op, ast.Pow) and abs(right) > 16:
                raise ValueError("exponent too large")
            return float(_OPS[type(node.op)](left, right))
        if isinstance(node, ast.UnaryOp) and type(node.op) in _OPS:
            return float(_OPS[type(node.op)](ev(node.operand)))
        raise ValueError("only numbers, + - * / ** and parentheses are allowed")

    value = ev(ast.parse(expression, mode="eval"))
    if not math.isfinite(value):
        raise ValueError("the result is not a finite number")
    return value


def _solver_result(db: Session, job_id: str) -> dict[str, Any]:
    try:
        jid = uuid.UUID(job_id)
    except ValueError as exc:
        raise ValueError(f"{job_id!r} is not a job id") from exc
    job = db.get(SolverJob, jid)
    result = db.scalar(select(SolverResult).where(SolverResult.job_id == jid)) if job else None
    if job is None or result is None or job.status != JobStatus.SUCCEEDED:
        raise ValueError(f"no stored result for job {job_id}")
    spec = result.assumptions.get("spec", {})
    out: dict[str, Any] = {
        "job_id": str(jid),
        "kind": job.kind,
        "solver": f"{result.solver_backend} @ {result.solver_version}",
        "engine_version": result.engine_version,
        "iterations": result.iterations,
        "approximations": result.assumptions.get("approximations")
        or result.assumptions.get("model_notes")
        or [],
    }
    if job.kind == "preflop":
        out["model"] = result.assumptions.get("model")
        out["validated_scope"] = result.assumptions.get("validated_scope")
        out["model_gap_mbb"] = result.summary.get("model_gap_mbb")
        out["positions"] = result.summary.get("positions")
        return out
    out.update(
        board="".join(spec.get("board", [])),
        ranges=spec.get("ranges"),
        starting_pot_bb=spec.get("starting_pot_mbb", 0) / 1000,
        effective_stack_bb=spec.get("effective_stack_mbb", 0) / 1000,
        tree=spec.get("tree"),
        rake=spec.get("game", {}).get("rake"),
        exploitability_pct_pot=result.summary.get("exploitability_pct_pot"),
        values_bb=[v / 1000 for v in result.summary.get("values_mbb", [])],
        values_convention=result.assumptions.get("values_convention"),
        independent_check=result.summary.get("independent_check"),
    )
    root = db.scalar(
        select(StrategyNode).where(StrategyNode.result_id == result.id, StrategyNode.node_path == "root")
    )
    if root is not None:
        by_class = root.strategy.get("by_class", [])
        total = sum(c["weight"] for c in by_class) or 1.0
        out["root"] = {
            "player": "out of position" if root.player == 0 else "in position",
            "actions": root.actions,
            "overall_frequencies": [
                sum(c["weight"] * c["frequencies"][a] for c in by_class) / total
                for a in range(len(root.actions))
            ],
        }
    return out


def _list_solves(db: Session, limit: int) -> list[dict[str, Any]]:
    rows = db.execute(
        select(SolverJob, SolverResult)
        .join(SolverResult, SolverResult.job_id == SolverJob.id)
        .where(SolverJob.status == JobStatus.SUCCEEDED)
        .order_by(SolverJob.created_at.desc())
        .limit(max(1, min(limit, 20)))
    ).all()
    out = []
    for job, result in rows:
        spec = result.assumptions.get("spec", {})
        out.append(
            {
                "job_id": str(job.id),
                "kind": job.kind,
                "board": "".join(spec.get("board", [])),
                "ranges": spec.get("ranges"),
                "exploitability_pct_pot": result.summary.get("exploitability_pct_pot"),
                "created_at": job.created_at.isoformat(),
            }
        )
    return out


def run_tool(db: Session, name: str, args: dict[str, Any], level: str) -> Any:
    """Runs one tool. Errors become a message for the model (`ValueError`, engine errors)."""
    settings = get_settings()
    if name == "theory":
        kind = args.get("kind")
        if kind not in THEORY_KINDS:
            raise ValueError(f"unknown theory kind {kind!r}")
        return engine.theory(str(kind), dict(args.get("params") or {}))
    if name == "equity":
        ranges = [str(r) for r in args.get("ranges", [])]
        if not 2 <= len(ranges) <= 6:
            raise ValueError("give 2 to 6 hands or ranges")
        board = str(args.get("board", ""))
        try:
            return engine.equity(ranges, board, "", "exact", 0, 0, settings.equity_exact_budget)
        except EngineValidationError as exc:
            if "budget" not in str(exc).lower():
                raise
            return engine.equity(ranges, board, "", "monte_carlo", MC_ITERATIONS, MC_SEED, 0)
    if name == "analyze_range":
        result = engine.analyze_range(str(args.get("notation", "")), str(args.get("dead", "")))
        return {k: v for k, v in result.items() if k != "matrix"}
    if name == "arithmetic":
        return {"expression": args.get("expression"), "value": arithmetic(str(args.get("expression", "")))}
    if name == "lesson":
        try:
            row = academy.lesson(db, str(args.get("slug", "")))
        except NotFoundError as exc:
            raise ValueError(str(exc)) from exc
        return lesson_text(row, level)
    if name == "curriculum":
        return academy.curriculum(db)
    if name == "list_solves":
        return _list_solves(db, int(args.get("limit", 5)))
    if name == "solver_result":
        return _solver_result(db, str(args.get("job_id", "")))
    raise ValueError(f"unknown tool {name!r}")


# ---- the number check ----

NUMBER = re.compile(r"(?<![\w♠♥♦♣#/.])(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)(?![\w♠♥♦♣+])")


def numbers_in(text: str) -> list[str]:
    """Numbers as written in prose. Card ranks and hand classes (9♦, 76s, T9, 22+) and section
    anchors are not numbers here."""
    return [m.group(1) for m in NUMBER.finditer(text)]


def _values(obj: Any, out: list[float]) -> None:
    if isinstance(obj, bool):
        return
    if isinstance(obj, int | float):
        out.append(float(obj))
    elif isinstance(obj, dict):
        for v in obj.values():
            _values(v, out)
    elif isinstance(obj, list | tuple):
        for v in obj:
            _values(v, out)
    elif isinstance(obj, str):
        out.extend(float(n.replace(",", "")) for n in numbers_in(obj))


def traced(written: str, sources: list[float]) -> bool:
    """Whether a number as written matches a source value: as is, as a percentage of a share, or
    converted between mbb and bb, at the precision it was written with."""
    n = float(written.replace(",", ""))
    if n.is_integer() and 0 <= n <= 10:
        return True  # small counts ("two players", "step 3")
    decimals = len(written.split(".")[1]) if "." in written else 0
    half_ulp = 0.5 * 10**-decimals + 1e-9
    for v in sources:
        for c in (v, v * 100, v / 1000, v * 1000, v / 100):
            if abs(c - n) <= half_ulp:
                return True
    return False


@dataclass
class Turn:
    """What one answer drew on, for the check and the citations."""

    tool_calls: list[dict[str, Any]] = field(default_factory=list)
    sources: list[float] = field(default_factory=list)


# ---- the loop ----


def _client() -> Any:
    settings = get_settings()
    if not settings.tutor_enabled:
        raise TutorUnavailableError(
            "The AI tutor is turned off on this server (RIVERLAB_TUTOR_ENABLED=false)."
        )
    import anthropic

    key = settings.anthropic_api_key.get_secret_value() if settings.anthropic_api_key else None
    try:
        return anthropic.Anthropic(api_key=key) if key else anthropic.Anthropic()
    except anthropic.AnthropicError as exc:
        raise TutorUnavailableError(
            "The AI tutor needs an Anthropic API key: set ANTHROPIC_API_KEY in RiverLab's .env and restart."
        ) from exc


def status() -> dict[str, Any]:
    settings = get_settings()
    if not settings.tutor_enabled:
        return {"available": False, "reason": "turned off on this server", "model": None}
    import os

    if settings.anthropic_api_key is None and not (
        os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("ANTHROPIC_AUTH_TOKEN")
    ):
        return {
            "available": False,
            "reason": "no Anthropic API key is set (ANTHROPIC_API_KEY)",
            "model": None,
        }
    return {"available": True, "reason": None, "model": settings.tutor_model}


# Indirection so tests can supply a fake client.
client_factory = _client


def _create(client: Any, system: list[dict[str, Any]], messages: list[dict[str, Any]]) -> Any:
    settings = get_settings()
    return client.beta.messages.create(
        model=settings.tutor_model,
        max_tokens=16000,
        system=system,
        tools=TOOLS,
        messages=messages,
        output_config={
            "effort": settings.tutor_effort,
            "format": {"type": "json_schema", "schema": ANSWER_SCHEMA},
        },
        # If a safety classifier declines, the API re-runs the request on a fallback model.
        betas=["server-side-fallback-2026-07-01"],
        fallbacks="default",
    )


def _answer_loop(
    db: Session,
    client: Any,
    system: list[dict[str, Any]],
    messages: list[dict[str, Any]],
    level: str,
    turn: Turn,
) -> tuple[dict[str, Any], Any]:
    import anthropic

    for _ in range(MAX_ROUNDS):
        try:
            response = _create(client, system, messages)
        except anthropic.AuthenticationError as exc:
            raise TutorUnavailableError("Anthropic rejected the API key (ANTHROPIC_API_KEY).") from exc
        except anthropic.RateLimitError as exc:
            raise TutorUnavailableError(
                "The tutor is rate-limited by Anthropic; try again in a minute."
            ) from exc
        except anthropic.APIStatusError as exc:
            raise TutorUnavailableError(f"Anthropic's API returned an error ({exc.status_code}).") from exc
        except anthropic.APIConnectionError as exc:
            raise TutorUnavailableError("Could not reach Anthropic's API (no network?).") from exc
        if response.stop_reason == "refusal":
            return {
                "answer": (
                    "The tutor declined to answer this question. Try rephrasing it, or ask about the lesson."
                ),
                "sections": [],
                "readings": [],
                "suggested_lessons": [],
            }, response
        messages.append({"role": "assistant", "content": response.content})
        if response.stop_reason == "tool_use":
            results = []
            for block in response.content:
                if block.type != "tool_use":
                    continue
                try:
                    out = run_tool(db, block.name, dict(block.input), level)
                    _values(out, turn.sources)
                    turn.tool_calls.append({"tool": block.name, "input": block.input, "ok": True})
                    results.append(
                        {
                            "type": "tool_result",
                            "tool_use_id": block.id,
                            "content": json.dumps(out, default=str),
                        }
                    )
                except Exception as exc:  # reported to the model, never swallowed silently
                    turn.tool_calls.append(
                        {"tool": block.name, "input": block.input, "ok": False, "error": str(exc)}
                    )
                    results.append(
                        {
                            "type": "tool_result",
                            "tool_use_id": block.id,
                            "content": f"Error: {exc}",
                            "is_error": True,
                        }
                    )
            messages.append({"role": "user", "content": results})
            continue
        if response.stop_reason == "max_tokens":
            raise TutorUnavailableError("The tutor's answer was too long; ask a narrower question.")
        text = next((b.text for b in response.content if b.type == "text"), "")
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError as exc:
            raise TutorUnavailableError(
                "The tutor returned an answer RiverLab could not read; try again."
            ) from exc
        return parsed, response
    raise TutorUnavailableError(
        "The tutor used too many tool calls without answering; ask a narrower question."
    )


def ask(
    db: Session, slug: str, level: str, question: str, history: list[dict[str, str]], consent: bool
) -> dict[str, Any]:
    """One tutor answer. `history`: earlier turns of this chat, [{role, text}], text only."""
    if not consent:
        raise TutorConsentError(
            "The AI tutor sends this lesson, your Academy progress and your question to Anthropic. "
            "Agree to that in the tutor panel first."
        )
    if level not in LEVELS:
        raise ValueError(f"unknown explanation level {level!r}")
    row = academy.lesson(db, slug)
    client = client_factory()
    lesson_block = lesson_text(row, level)
    context = (
        f"Explanation level: {level}.\n{learner_state(db)}\n\n"
        f"The lesson the learner is reading:\n\n{lesson_block}"
    )
    # Stable instructions first (cached across questions); the lesson and the learner follow.
    system = [{"type": "text", "text": SYSTEM, "cache_control": {"type": "ephemeral"}}]
    messages: list[dict[str, Any]] = [
        {
            "role": "user",
            "content": [{"type": "text", "text": context, "cache_control": {"type": "ephemeral"}}],
        },
        {"role": "assistant", "content": "I have the lesson and the learner's progress."},
    ]
    for h in history[-12:]:
        messages.append(
            {"role": "assistant" if h.get("role") == "tutor" else "user", "content": h.get("text", "")}
        )
    messages.append({"role": "user", "content": question})

    turn = Turn()
    _values(context, turn.sources)
    _values(question, turn.sources)
    for h in history:
        _values(h.get("text", ""), turn.sources)

    parsed, response = _answer_loop(db, client, system, messages, level, turn)
    untraced = [n for n in numbers_in(parsed.get("answer", "")) if not traced(n, turn.sources)]
    corrections = 0
    while untraced and corrections < MAX_CORRECTIONS:
        corrections += 1
        messages.append(
            {
                "role": "user",
                "content": (
                    "RiverLab's check: these numbers in your answer did not come from a tool result, the "
                    f"lesson or the conversation: {', '.join(sorted(set(untraced)))}. Recompute each with a "
                    "tool (arithmetic for plain calculations) or remove it, then give the full answer again."
                ),
            }
        )
        parsed, response = _answer_loop(db, client, system, messages, level, turn)
        untraced = [n for n in numbers_in(parsed.get("answer", "")) if not traced(n, turn.sources)]

    answer = str(parsed.get("answer", ""))
    flagged = sorted(set(untraced))
    if flagged:
        answer += (
            "\n\n> ⚠ RiverLab could not trace these figures to its engine, the lesson or this "
            f"conversation: {', '.join(flagged)}. Treat them as unverified."
        )
    known_readings = readings(db)
    cited = [r for r in parsed.get("readings", []) if str(r).strip() in known_readings]
    dropped = [r for r in parsed.get("readings", []) if str(r).strip() not in known_readings]
    section_ids = {s.get("id") for s in row.sections}
    sections = [s for s in parsed.get("sections", []) if s in section_ids or "#" in str(s)]
    lessons_known = {r.slug for r in academy.lessons(db)}
    suggested = [s for s in parsed.get("suggested_lessons", []) if s in lessons_known and s != slug]
    usage = getattr(response, "usage", None)
    return {
        "answer": answer,
        "sections": sections,
        "readings": cited,
        "dropped_readings": dropped,
        "suggested_lessons": suggested,
        "tool_calls": turn.tool_calls,
        "unverified_numbers": flagged,
        "corrections": corrections,
        "model": getattr(response, "model", None),
        "label": "AI tutor (may be wrong)",
        "usage": {
            "input_tokens": getattr(usage, "input_tokens", None),
            "output_tokens": getattr(usage, "output_tokens", None),
            "cache_read_input_tokens": getattr(usage, "cache_read_input_tokens", None),
        }
        if usage is not None
        else None,
    }
