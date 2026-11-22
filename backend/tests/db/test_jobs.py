"""Job lifecycle, exercised with a test-only backend (no real solver exists)."""

import uuid
from typing import Any

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from riverlab_api.db.models import JobStatus
from riverlab_api.errors import ConflictError, SolverUnavailableError
from riverlab_api.services.compute import BackendRegistry, JobService
from tests.api.test_meta_and_solver import _spec

pytestmark = pytest.mark.db


class RecordingBackend:
    name = "test"

    def __init__(self, fail: bool = False) -> None:
        self.submitted: list[uuid.UUID] = []
        self.cancelled: list[uuid.UUID] = []
        self.fail = fail

    def can_run(self, spec: dict[str, Any]) -> bool:
        return True

    def submit(self, job_id: uuid.UUID, spec: dict[str, Any]) -> None:
        if self.fail:
            raise RuntimeError("boom")
        self.submitted.append(job_id)

    def request_cancel(self, job_id: uuid.UUID) -> None:
        self.cancelled.append(job_id)


@pytest.fixture
def spec(client: TestClient) -> dict[str, Any]:
    return _spec(client)


def _service(db: Session, backend: RecordingBackend | None) -> JobService:
    reg = BackendRegistry()
    if backend:
        reg.register(backend)
    return JobService(db, reg)


def test_no_backend_means_no_job(db: Session, spec: dict[str, Any]) -> None:
    with pytest.raises(SolverUnavailableError):
        _service(db, None).submit(spec)
    assert _service(db, None).list() == []


def test_full_lifecycle(db: Session, spec: dict[str, Any]) -> None:
    backend = RecordingBackend()
    svc = _service(db, backend)
    job = svc.submit(spec)
    assert job.status == JobStatus.QUEUED
    assert backend.submitted == [job.id]
    assert job.spec["solver"]["variant"]["algorithm"] == "discounted"
    svc.mark_running(job.id)
    svc.report_progress(job.id, 100, {"exploitability": 1.2})
    with pytest.raises(ConflictError, match="decrease"):
        svc.report_progress(job.id, 50, {})
    svc.cancel(job.id)
    assert svc.get(job.id).cancel_requested
    assert backend.cancelled == [job.id]
    done = svc.mark_finished(job.id, JobStatus.CANCELLED)
    assert done.finished_at is not None
    assert done.started_at is not None
    with pytest.raises(ConflictError):
        svc.mark_running(job.id)
    with pytest.raises(ConflictError, match="already"):
        svc.cancel(job.id)


def test_queued_job_cancels_immediately(db: Session, spec: dict[str, Any]) -> None:
    svc = _service(db, RecordingBackend())
    job = svc.submit(spec)
    assert svc.cancel(job.id).status == JobStatus.CANCELLED


def test_failure_requires_error_and_submit_failure_is_recorded(db: Session, spec: dict[str, Any]) -> None:
    svc = _service(db, RecordingBackend())
    job = svc.submit(spec)
    svc.mark_running(job.id)
    with pytest.raises(ConflictError, match="error code"):
        svc.mark_finished(job.id, JobStatus.FAILED)
    svc.mark_finished(job.id, JobStatus.FAILED, error=("diverged", "numerical failure"))
    assert svc.get(job.id).error_code == "diverged"

    failing = _service(db, RecordingBackend(fail=True))
    with pytest.raises(RuntimeError):
        failing.submit(spec)
    failed = [j for j in failing.list() if j.error_code == "backend_submit_failed"]
    assert len(failed) == 1
    assert failed[0].status == JobStatus.FAILED


def test_job_endpoints(db_client: TestClient) -> None:
    assert db_client.get("/api/v1/solver/jobs").json() == []
    assert db_client.get(f"/api/v1/solver/jobs/{uuid.uuid4()}").status_code == 404
