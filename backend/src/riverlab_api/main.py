import logging
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from typing import Any

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware

from riverlab_api import __version__
from riverlab_api.api import health
from riverlab_api.api.v1 import router as v1_router
from riverlab_api.config import get_settings
from riverlab_api.errors import install_error_handlers
from riverlab_api.services import activity, compute, play
from riverlab_api.services.local_solver import BACKEND_NAME, LocalSolverBackend, fail_interrupted_jobs

logger = logging.getLogger(__name__)


def _sync_academy(content_dir: str) -> None:
    from pathlib import Path

    from riverlab_api.db.session import get_sessionmaker
    from riverlab_api.services import academy

    try:
        with get_sessionmaker()() as session:
            report = academy.sync(session, Path(content_dir))
        logger.info("academy: %d lesson(s) synced, %d error(s)", len(report.lessons), len(report.errors))
    except Exception:
        logger.exception("could not sync Academy content (database unavailable?)")


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    settings = get_settings()
    if settings.academy_sync_on_startup:
        _sync_academy(settings.academy_content_dir)
    backend: LocalSolverBackend | None = None
    if settings.solver_enabled and not any(b.name == BACKEND_NAME for b in compute.registry.backends):
        from riverlab_api.db.session import get_sessionmaker

        factory = get_sessionmaker()
        try:
            with factory() as session:
                n = fail_interrupted_jobs(session)
            if n:
                logger.warning("marked %d interrupted solver job(s) as failed", n)
        except Exception:
            logger.exception("could not check for interrupted solver jobs (database unavailable?)")
        backend = LocalSolverBackend(
            factory, settings.solver_max_memory_mb, settings.solver_threads, settings.result_storage_dir
        )
        compute.registry.register(backend)
        try:
            with factory() as session:
                n = play.fail_interrupted_games(session)
            if n:
                logger.warning("marked %d unfinished game(s) as interrupted", n)
        except Exception:
            logger.exception("could not check for interrupted games (database unavailable?)")
        play.manager = play.PlayManager(
            factory, settings.result_storage_dir, settings.solver_max_memory_mb, settings.solver_threads
        )
    try:
        yield
    finally:
        if backend is not None:
            compute.registry.backends.remove(backend)
            backend.shutdown()
        if play.manager is not None:
            play.manager.shutdown()
            play.manager = None


def create_app() -> FastAPI:
    settings = get_settings()
    logging.basicConfig(level=settings.log_level)
    app = FastAPI(
        title="RiverLab API",
        version=__version__,
        description="Poker theory, solving and training platform API.",
        docs_url=None if settings.env == "production" else "/docs",
        lifespan=lifespan,
    )

    @app.middleware("http")
    async def track_activity(request: Request, call_next: Any) -> Any:
        # User requests count as activity; health checks and background polling do not.
        if not request.url.path.startswith("/health") and request.headers.get("x-riverlab-poll") != "1":
            activity.touch()
        return await call_next(request)

    if settings.cors_origin_list:
        app.add_middleware(
            CORSMiddleware,
            allow_origins=settings.cors_origin_list,
            allow_methods=["GET", "POST", "DELETE"],
            allow_headers=["Content-Type"],
        )
    install_error_handlers(app)
    app.include_router(health.router)
    app.include_router(v1_router)
    return app


app = create_app()
