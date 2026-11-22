from fastapi import APIRouter

from riverlab_api.api.v1 import config, meta, poker, solver

router = APIRouter(prefix="/api/v1")
router.include_router(meta.router)
router.include_router(config.router)
router.include_router(poker.router)
router.include_router(solver.router)
