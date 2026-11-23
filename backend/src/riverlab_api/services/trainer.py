"""Push/fold trainer: drills graded against the exact heads-up push/fold equilibrium.

The exact all-in equity table (7 MB) is needed first. It is loaded from
`<result_storage_dir>/hu_combo_equity.bin` (the Android setup downloads a pre-built copy) and
verified on load: structure, symmetry and entries re-computed exactly. If it is missing, it can
be built here, which takes tens of minutes. Nothing is loaded until the trainer is used.
"""

import json
import logging
import threading
from pathlib import Path
from typing import Any, Literal

from riverlab_api.config import get_settings
from riverlab_api.errors import ConflictError
from riverlab_api.services import activity, engine

logger = logging.getLogger(__name__)

TABLE_NAME = "hu_combo_equity.bin"
# Entries re-computed exactly when a table is loaded (about 1 s of work).
VERIFY_SAMPLES = 200

State = Literal["missing", "loading", "building", "ready", "error"]


class PushFoldService:
    def __init__(self, table_path: Path) -> None:
        self.table_path = table_path
        self._lock = threading.Lock()
        self._state: State = "missing"
        self._progress: float | None = None
        self._message: str | None = None
        self._trainer: Any = None
        self._thread: threading.Thread | None = None

    def status(self) -> dict[str, Any]:
        """Also starts loading a table that is present but not loaded yet."""
        with self._lock:
            if self._state == "missing" and self.table_path.exists():
                self._start_locked(build=False)
            return {
                "state": self._state,
                "progress": self._progress,
                "message": self._message,
                "table_present": self.table_path.exists(),
            }

    def build(self) -> dict[str, Any]:
        """Builds the table (or loads it if present)."""
        with self._lock:
            if self._state in ("loading", "building", "ready"):
                raise ConflictError(f"The equity table is already {self._state}")
            self._start_locked(build=not self.table_path.exists())
        return self.status()

    def _start_locked(self, build: bool) -> None:
        if self._thread is not None and self._thread.is_alive():
            return
        self._state = "building" if build else "loading"
        self._progress = 0.0 if build else None
        self._message = None
        self._thread = threading.Thread(target=self._run, args=(build,), name="push-fold-table", daemon=True)
        self._thread.start()

    def _run(self, build: bool) -> None:
        with activity.working():
            try:
                if build:
                    self.table_path.parent.mkdir(parents=True, exist_ok=True)

                    def progress(done: int, total: int) -> None:
                        with self._lock:
                            self._progress = done / total

                    engine.push_fold_build_table(str(self.table_path), progress)
                    with self._lock:
                        self._state = "loading"
                        self._progress = None
                trainer = engine.push_fold_trainer(str(self.table_path), VERIFY_SAMPLES)
                with self._lock:
                    self._trainer = trainer
                    self._state = "ready"
            except Exception as exc:
                logger.exception("push/fold equity table failed")
                with self._lock:
                    self._state = "error"
                    self._message = str(exc)

    def _ready(self) -> Any:
        with self._lock:
            trainer = self._trainer
        if trainer is None:
            raise ConflictError("The push/fold trainer is not ready: its equity table is not loaded yet")
        return trainer

    def drill(self, min_mbb: int, max_mbb: int, seat: str | None, seed: int) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().drill, min_mbb, max_mbb, seat, seed)
        return result

    def grade(self, drill: dict[str, Any], choice: str) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().grade, json.dumps(drill), choice)
        return result

    def chart(self, stack_mbb: int) -> dict[str, Any]:
        result: dict[str, Any] = engine.play_call(self._ready().chart, stack_mbb)
        return result


_service: PushFoldService | None = None
_service_lock = threading.Lock()


def service() -> PushFoldService:
    global _service
    with _service_lock:
        if _service is None:
            _service = PushFoldService(Path(get_settings().result_storage_dir) / TABLE_NAME)
        return _service
