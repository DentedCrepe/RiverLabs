"""Activity tracking and wake locks, so a phone can sleep (and RiverLab can stop) when idle.

- `touch()` records a user request (the app's middleware calls it; health checks and
  background polling don't count). `idle_seconds()` reports the time since then.
- `begin()` / `end()` bracket work that must not be interrupted (a queued or running solve, a
  table being prepared). While any is active, `busy()` is true and, when configured, the
  device wake lock is held (e.g. Termux's `termux-wake-lock`), released when the last one ends.
"""

import logging
import shlex
import subprocess
import threading
import time
from collections.abc import Iterator
from contextlib import contextmanager

from riverlab_api.config import get_settings

logger = logging.getLogger(__name__)
_lock = threading.Lock()
_last = time.monotonic()
_active = 0


def touch() -> None:
    global _last
    with _lock:
        _last = time.monotonic()


def idle_seconds() -> float:
    with _lock:
        return time.monotonic() - _last


def busy() -> bool:
    with _lock:
        return _active > 0


def _run(command: str) -> None:
    if not command:
        return
    try:
        # The command comes from the operator's own configuration (RIVERLAB_WAKE_*), never from requests.
        result = subprocess.run(shlex.split(command), timeout=15, check=False, capture_output=True)  # noqa: S603
        if result.returncode != 0:
            logger.warning("%s exited with %d: %s", command, result.returncode, result.stderr.decode()[:200])
    except (OSError, subprocess.TimeoutExpired) as exc:
        logger.warning("could not run %s: %s", command, exc)


def begin() -> None:
    global _active
    with _lock:
        _active += 1
        first = _active == 1
    if first:
        _run(get_settings().wake_lock_command)


def end() -> None:
    global _active
    with _lock:
        _active = max(0, _active - 1)
        last = _active == 0
    touch()  # idle time counts from when the work finished
    if last:
        _run(get_settings().wake_unlock_command)


@contextmanager
def working() -> Iterator[None]:
    begin()
    try:
        yield
    finally:
        end()
