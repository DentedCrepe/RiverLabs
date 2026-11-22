import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from riverlab_api import __version__
from riverlab_api.api import health
from riverlab_api.api.v1 import router as v1_router
from riverlab_api.config import get_settings
from riverlab_api.errors import install_error_handlers


def create_app() -> FastAPI:
    settings = get_settings()
    logging.basicConfig(level=settings.log_level)
    app = FastAPI(
        title="RiverLab API",
        version=__version__,
        description="Poker theory and training platform API. Solver endpoints are interface-only.",
        docs_url=None if settings.env == "production" else "/docs",
    )
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
