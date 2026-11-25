"""A standalone solver worker: `uv run python -m riverlab_api.worker`.

Claims queued solves from the database and runs them (services/local_solver.py, ADR-0007). Run
the API with `RIVERLAB_WORKER_MODE=external` and as many workers as you like, on this machine or
others that reach the same database and result storage directory. Stop with Ctrl-C or SIGTERM:
a solve in progress is left `running` and is picked up again by a worker once its heartbeat is
stale.
"""

import logging
import signal
import sys
from types import FrameType

from riverlab_api.config import get_settings
from riverlab_api.db.session import get_sessionmaker
from riverlab_api.services.local_solver import LocalSolverBackend


def main() -> int:
    settings = get_settings()
    logging.basicConfig(level=settings.log_level)
    if not settings.solver_enabled:
        logging.error("RIVERLAB_SOLVER_ENABLED is false: nothing to do")
        return 1
    worker = LocalSolverBackend(
        get_sessionmaker(),
        settings.solver_max_memory_mb,
        settings.solver_threads,
        settings.result_storage_dir,
        run_worker=False,
    )

    def stop(signum: int, _frame: FrameType | None) -> None:
        logging.info("signal %d: stopping after the current step", signum)
        worker.shutdown()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)
    worker.run_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
