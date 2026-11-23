"""Browsing saved solve trees: a small LRU cache of loaded games."""

import threading
import uuid
from collections import OrderedDict
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

from riverlab_api.config import get_settings
from riverlab_api.db.models import SolverResult
from riverlab_api.errors import EngineValidationError, NotFoundError
from riverlab_api.services import engine

_lock = threading.Lock()
_cache: "OrderedDict[uuid.UUID, Any]" = OrderedDict()


def path_from_uri(uri: str) -> Path:
    parsed = urlparse(uri)
    if parsed.scheme != "file":
        raise NotFoundError(f"Unsupported result storage location: {parsed.scheme}")
    return Path(unquote(parsed.path))


def forget(result_id: uuid.UUID) -> None:
    with _lock:
        _cache.pop(result_id, None)


def _game(result: SolverResult) -> Any:
    with _lock:
        if result.id in _cache:
            _cache.move_to_end(result.id)
            return _cache[result.id]
    if not result.storage_uri:
        raise NotFoundError("This result's tree was not saved; only the first decision is available")
    path = path_from_uri(result.storage_uri)
    if not path.exists():
        raise NotFoundError("This result's saved tree is missing from storage")
    game = engine.load_solved(result.assumptions["spec"], str(path))
    with _lock:
        _cache[result.id] = game
        while len(_cache) > get_settings().result_cache_size:
            _cache.popitem(last=False)
    return game


def parse_path(text: str) -> list[dict[str, Any]]:
    """`a0.a2.cAh` -> [{"type": "action", "value": 0}, ..., {"type": "card", "value": "Ah"}]."""
    steps: list[dict[str, Any]] = []
    for token in filter(None, text.split(".")):
        kind, value = token[0], token[1:]
        if kind == "a" and value.isdigit():
            steps.append({"type": "action", "value": int(value)})
        elif kind == "c" and len(value) == 2:
            steps.append({"type": "card", "value": value})
        else:
            raise EngineValidationError(f"Invalid path step '{token}' (use a<index> or c<card>)")
    return steps


def node(result: SolverResult, path: str) -> dict[str, Any]:
    return engine.solved_node(_game(result), parse_path(path))
