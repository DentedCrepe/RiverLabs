"""Environment-based settings. No credentials have defaults; see .env.example."""

from functools import lru_cache
from typing import Literal

from pydantic import Field, PostgresDsn, model_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

PLACEHOLDER_PASSWORD = "change-me-local-only"  # noqa: S105 - the value we refuse


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="RIVERLAB_", env_file=".env", extra="ignore")

    env: Literal["development", "test", "production"] = "development"
    database_url: PostgresDsn
    log_level: str = "INFO"
    # Comma-separated list. Empty by default: the frontend reaches the API via a same-origin proxy.
    cors_origins: str = ""
    # Upper bound on exact-equity work per request (tuple x runout evaluations).
    equity_exact_budget: int = Field(default=50_000_000, ge=1)
    equity_max_iterations: int = Field(default=2_000_000, ge=1)
    # In-process GTOpen solver (one solve at a time; others queue).
    solver_enabled: bool = True
    solver_max_memory_mb: int = Field(default=2048, ge=16)
    # 0 = use every CPU core.
    solver_threads: int = Field(default=0, ge=0, le=256)

    @model_validator(mode="after")
    def _no_placeholder_credentials_in_production(self) -> "Settings":
        if self.env == "production" and PLACEHOLDER_PASSWORD in str(self.database_url):
            raise ValueError("RIVERLAB_DATABASE_URL still uses the .env.example placeholder password")
        return self

    @property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]


@lru_cache
def get_settings() -> Settings:
    return Settings()  # values come from the environment
