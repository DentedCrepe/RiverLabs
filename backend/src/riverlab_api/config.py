"""Environment-based settings. No credentials have defaults; see .env.example."""

from functools import lru_cache
from typing import Literal

from pydantic import Field, PostgresDsn
from pydantic_settings import BaseSettings, SettingsConfigDict


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

    @property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]


@lru_cache
def get_settings() -> Settings:
    return Settings()  # values come from the environment
