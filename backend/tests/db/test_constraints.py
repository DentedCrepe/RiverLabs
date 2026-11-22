"""Database-level guarantees that hold even if application code is wrong."""

import uuid
from datetime import UTC, datetime

import pytest
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from riverlab_api.db.models import JobStatus, Range, RangeCombo, SolverJob, SolverResult, User

pytestmark = pytest.mark.db


def _expect_integrity_error(db: Session, obj: object) -> None:
    db.add(obj)
    with pytest.raises(IntegrityError):
        db.flush()
    db.rollback()


def test_email_unique_case_insensitive(db: Session) -> None:
    db.add(User(email="Player@Example.com", display_name="a"))
    db.flush()
    _expect_integrity_error(db, User(email="player@example.com", display_name="b"))


def test_range_combo_weights_and_indices(db: Session) -> None:
    r = Range(name="r", notation="AA", combo_count=6, source="user")
    db.add(r)
    db.flush()
    _expect_integrity_error(db, RangeCombo(range_id=r.id, combo_index=0, weight=1.5))
    r = Range(name="r", notation="AA", combo_count=6, source="user")
    db.add(r)
    db.flush()
    _expect_integrity_error(db, RangeCombo(range_id=r.id, combo_index=1326, weight=1.0))


def test_solver_sourced_range_must_reference_a_result(db: Session) -> None:
    _expect_integrity_error(db, Range(name="r", notation="AA", combo_count=6, source="solver"))


def _job(**kw: object) -> SolverJob:
    return SolverJob(spec={}, backend="test", **kw)


def test_job_terminal_status_requires_finish_time(db: Session) -> None:
    _expect_integrity_error(db, _job(status=JobStatus.SUCCEEDED))
    _expect_integrity_error(db, _job(status=JobStatus.RUNNING, finished_at=datetime.now(UTC)))


def test_failed_job_requires_error_code(db: Session) -> None:
    _expect_integrity_error(db, _job(status=JobStatus.FAILED, finished_at=datetime.now(UTC)))


def test_result_requires_versions_and_exploitability_unit(db: Session) -> None:
    job = _job(status=JobStatus.SUCCEEDED, finished_at=datetime.now(UTC))
    db.add(job)
    db.flush()
    base = {
        "job_id": job.id,
        "engine_version": "0.1.0",
        "solver_backend": "test",
        "solver_version": "0",
        "config_schema_version": 1,
        "assumptions": {},
        "iterations": 1,
    }
    _expect_integrity_error(db, SolverResult(**{**base, "exploitability": 0.1}))
    job = _job(id=uuid.uuid4(), status=JobStatus.SUCCEEDED, finished_at=datetime.now(UTC))
    db.add(job)
    db.flush()
    _expect_integrity_error(db, SolverResult(**{**base, "job_id": job.id, "engine_version": None}))
