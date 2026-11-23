"""The only module that talks to the Rust engine (ADR-0001).

The engine validates all poker semantics. Errors raised by the engine (ValueError) become
EngineValidationError (HTTP 422) with the engine's message.
"""

import json
import logging
from collections.abc import Callable
from typing import Any, Literal, TypeVar

import riverlab_engine as _engine

from riverlab_api.errors import EngineInternalError, EngineValidationError

logger = logging.getLogger(__name__)

T = TypeVar("T")


def _call(fn: Callable[..., str], *args: Any, **kwargs: Any) -> Any:
    try:
        raw = fn(*args, **kwargs)
    except ValueError as exc:
        raise EngineValidationError(str(exc)) from exc
    except OverflowError as exc:
        raise EngineValidationError(f"Number out of range: {exc}") from exc
    except BaseException as exc:
        # A Rust panic arrives as pyo3's PanicException (a BaseException). It indicates an
        # engine bug: report it explicitly instead of letting it escape as an opaque failure.
        if type(exc).__name__ == "PanicException":
            logger.exception("Engine panic in %s", getattr(fn, "__name__", fn))
            raise EngineInternalError("The poker engine failed unexpectedly; this is a bug") from exc
        raise
    return json.loads(raw)


def engine_version() -> str:
    return str(_engine.engine_version())


def evaluate_hand(cards: str) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.evaluate_hand, cards)
    return result


def compare_hands(board: str, hands: list[str]) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.compare_hands, board, hands)
    return result


def analyze_range(notation: str, dead: str = "") -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.analyze_range, notation, dead)
    return result


def equity(
    ranges: list[str],
    board: str,
    dead: str,
    method: Literal["exact", "monte_carlo"],
    iterations: int,
    seed: int,
    exact_budget: int,
) -> dict[str, Any]:
    result: dict[str, Any] = _call(
        _engine.equity,
        ranges,
        board=board,
        dead=dead,
        method=method,
        iterations=iterations,
        seed=seed,
        exact_budget=exact_budget,
    )
    return result


def default_game_config() -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.default_game_config)
    return result


def validate_game_config(config: dict[str, Any]) -> dict[str, Any]:
    """Returns the canonical engine form of a valid configuration."""
    result: dict[str, Any] = _call(_engine.validate_game_config, json.dumps(config))
    return result


def compute_rake(config: dict[str, Any], pot_mbb: int, saw_flop: bool) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.compute_rake, json.dumps(config), pot_mbb, saw_flop)
    return result


def stack_presets() -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.stack_presets)
    return result


def positions(players: int = 6) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = _call(_engine.positions, players)
    return result


def validate_solver_config(config: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.validate_solver_config, json.dumps(config))
    return result


def validate_solve_spec(spec: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.validate_solve_spec, json.dumps(spec))
    return result


def replay_hand(
    config: dict[str, Any], events: list[dict[str, Any]], stacks_mbb: list[int] | None
) -> dict[str, Any]:
    result: dict[str, Any] = _call(_engine.replay_hand, json.dumps(config), json.dumps(events), stacks_mbb)
    return result


CancelToken = _engine.CancelToken


def solver_backends() -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = _call(_engine.solver_backends)
    return result


class SolveRejectedError(Exception):
    """The backend refused the spec (invalid, unsupported or too large)."""


class SolveFailedError(Exception):
    """The solver failed while running."""


def solve_postflop(
    spec: dict[str, Any],
    cancel: Any,
    progress: Callable[[dict[str, Any]], None],
    max_bytes: int,
    threads: int,
) -> dict[str, Any]:
    """Runs a GTOpen solve (blocking; the GIL is released while solving)."""
    try:
        raw = _engine.solve_postflop(
            json.dumps(spec),
            cancel,
            lambda payload: progress(json.loads(payload)),
            max_bytes=max_bytes,
            threads=threads,
        )
    except ValueError as exc:
        raise SolveRejectedError(str(exc)) from exc
    except RuntimeError as exc:
        raise SolveFailedError(str(exc)) from exc
    result: dict[str, Any] = json.loads(raw)
    return result
