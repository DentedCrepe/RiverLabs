"""Persistence of game configurations (ADR-0004).

Every document is validated and canonicalised by the engine before it is stored; scalar
columns are derived from the canonical document so they can never disagree with it.
"""

import hashlib
import json
import uuid
from typing import Any

from sqlalchemy import select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from riverlab_api.db.models import GameConfig, RakeConfig, StackConfig
from riverlab_api.errors import ConflictError, NotFoundError
from riverlab_api.services import engine

STACK_PRESETS = {20, 40, 50, 75, 100, 150, 200}


def _preset_label(mbb: int) -> str | None:
    return f"{mbb // 1000}bb" if mbb % 1000 == 0 and mbb // 1000 in STACK_PRESETS else None


def _rake_name(rake: dict[str, Any]) -> str:
    pct = rake["percentage_bps"] / 100
    parts = [f"{pct:g}%"]
    cap = rake.get("cap")
    if cap is None:
        parts.append("uncapped")
    elif cap["unit"] == "big_blinds":
        parts.append(f"cap {cap['mbb'] / 1000:g}bb")
    else:
        parts.append(f"cap {cap['minor_units']} minor units")
    if rake["no_flop_no_drop"]:
        parts.append("NFND")
    digest = hashlib.sha256(json.dumps(rake, sort_keys=True).encode()).hexdigest()[:8]
    return f"{' '.join(parts)} [{digest}]"


def _insert_or_lose_race(db: Session, row: StackConfig | RakeConfig) -> bool:
    """Inserts inside a savepoint. Returns False if a concurrent request inserted the same
    unique row first (the caller then re-reads it)."""
    try:
        with db.begin_nested():
            db.add(row)
            db.flush()
    except IntegrityError:
        if row in db:
            db.expunge(row)
        return False
    return True


def _constraint_name(exc: IntegrityError) -> str:
    diag = getattr(exc.orig, "diag", None)
    return str(getattr(diag, "constraint_name", "") or "")


def _find_or_create_stack(db: Session, owner_id: uuid.UUID | None, stack: dict[str, Any]) -> StackConfig:
    mbb = int(stack["effective_stack"])
    row = db.scalar(
        select(StackConfig).where(
            StackConfig.owner_id.is_(None) if owner_id is None else StackConfig.owner_id == owner_id,
            StackConfig.effective_stack_mbb == mbb,
        )
    )
    if row is None:
        candidate = StackConfig(
            owner_id=owner_id,
            name=_preset_label(mbb) or f"{mbb / 1000:g}bb",
            effective_stack_mbb=mbb,
            preset=_preset_label(mbb),
            schema_version=1,
            config=stack,
        )
        if _insert_or_lose_race(db, candidate):
            return candidate
        return _find_or_create_stack(db, owner_id, stack)
    return row


def _find_or_create_rake(db: Session, owner_id: uuid.UUID | None, rake: dict[str, Any]) -> RakeConfig:
    owner_clause = RakeConfig.owner_id.is_(None) if owner_id is None else RakeConfig.owner_id == owner_id
    row = db.scalar(select(RakeConfig).where(owner_clause, RakeConfig.config == rake))
    if row is None:
        candidate = RakeConfig(
            owner_id=owner_id,
            name=_rake_name(rake),
            percentage_bps=rake["percentage_bps"],
            no_flop_no_drop=rake["no_flop_no_drop"],
            schema_version=1,
            config=rake,
        )
        if _insert_or_lose_race(db, candidate):
            return candidate
        return _find_or_create_rake(db, owner_id, rake)
    return row


def create_game_config(
    db: Session, name: str, config: dict[str, Any], owner_id: uuid.UUID | None = None
) -> GameConfig:
    canonical = engine.validate_game_config(config)
    stack = _find_or_create_stack(db, owner_id, canonical["stack"])
    rake = _find_or_create_rake(db, owner_id, canonical["rake"])
    stake = canonical.get("stake")
    row = GameConfig(
        owner_id=owner_id,
        name=name,
        players=canonical["players"],
        small_blind_mbb=canonical["small_blind"],
        ante_mbb=canonical["ante"],
        stack_config_id=stack.id,
        rake_config_id=rake.id,
        stake_currency=stake["currency"] if stake else None,
        stake_big_blind_minor_units=stake["big_blind_minor_units"] if stake else None,
        schema_version=canonical["schema_version"],
        config=canonical,
    )
    db.add(row)
    try:
        db.commit()
    except IntegrityError as exc:
        db.rollback()
        if _constraint_name(exc) == "uq_game_configs_owner_id_name":
            raise ConflictError(f"A game configuration named '{name}' already exists") from exc
        raise ConflictError(
            f"The configuration violates a database constraint ({_constraint_name(exc)})"
        ) from exc
    db.refresh(row)
    return row


def list_game_configs(db: Session) -> list[GameConfig]:
    return list(db.scalars(select(GameConfig).order_by(GameConfig.is_builtin.desc(), GameConfig.name)))


def get_game_config(db: Session, config_id: uuid.UUID) -> GameConfig:
    row = db.get(GameConfig, config_id)
    if row is None:
        raise NotFoundError(f"Game configuration {config_id} not found")
    return row


def delete_game_config(db: Session, config_id: uuid.UUID) -> None:
    row = get_game_config(db, config_id)
    if row.is_builtin:
        raise ConflictError("The built-in default configuration cannot be deleted")
    db.delete(row)
    try:
        db.commit()
    except IntegrityError as exc:
        db.rollback()
        raise ConflictError("Configuration is referenced by other records and cannot be deleted") from exc
