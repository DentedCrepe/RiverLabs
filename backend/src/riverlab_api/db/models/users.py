import uuid
from typing import Any

from sqlalchemy import ForeignKey, Index, String, func
from sqlalchemy.dialects.postgresql import JSONB, UUID
from sqlalchemy.orm import Mapped, mapped_column, relationship

from riverlab_api.db.base import Base, Timestamps, UUIDPk


class User(UUIDPk, Timestamps, Base):
    """An account. Authentication is not implemented yet (no credential columns exist)."""

    __tablename__ = "users"

    email: Mapped[str] = mapped_column(String(320), nullable=False)
    display_name: Mapped[str] = mapped_column(String(100), nullable=False)
    is_active: Mapped[bool] = mapped_column(default=True, nullable=False)

    settings: Mapped["UserSettings | None"] = relationship(
        back_populates="user", cascade="all, delete-orphan", uselist=False
    )

    __table_args__ = (
        # Case-insensitive uniqueness.
        Index("uq_users_email_lower", func.lower(email), unique=True),
    )


class UserSettings(Timestamps, Base):
    __tablename__ = "user_settings"

    user_id: Mapped[uuid.UUID] = mapped_column(
        UUID(as_uuid=True), ForeignKey("users.id", ondelete="CASCADE"), primary_key=True
    )
    default_game_config_id: Mapped[uuid.UUID | None] = mapped_column(
        UUID(as_uuid=True), ForeignKey("game_configs.id", ondelete="SET NULL"), nullable=True
    )
    # UI preferences only (theme, table layout...). Never poker results.
    preferences: Mapped[dict[str, Any]] = mapped_column(JSONB, nullable=False, default=dict)

    user: Mapped[User] = relationship(back_populates="settings")
