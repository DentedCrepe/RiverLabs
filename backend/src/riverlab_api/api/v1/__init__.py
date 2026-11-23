from fastapi import APIRouter

from riverlab_api.api.v1 import academy, analytics, config, meta, play, poker, solver, theory, trainer

router = APIRouter(prefix="/api/v1")
router.include_router(meta.router)
router.include_router(config.router)
router.include_router(poker.router)
router.include_router(solver.router)
router.include_router(play.router)
router.include_router(academy.router)
router.include_router(theory.router)
router.include_router(trainer.router)
router.include_router(trainer.postflop)
router.include_router(analytics.router)
