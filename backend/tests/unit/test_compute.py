from riverlab_api.db.models import JobStatus
from riverlab_api.services.compute import TRANSITIONS, BackendRegistry


def test_terminal_states_have_no_transitions() -> None:
    for status in JobStatus:
        assert (len(TRANSITIONS[status]) == 0) == status.is_terminal


def test_every_state_is_reachable_from_queued() -> None:
    seen, frontier = {JobStatus.QUEUED}, [JobStatus.QUEUED]
    while frontier:
        for nxt in TRANSITIONS[frontier.pop()]:
            if nxt not in seen:
                seen.add(nxt)
                frontier.append(nxt)
    assert seen == set(JobStatus)


def test_registry_starts_empty_until_the_app_registers_a_backend() -> None:
    from riverlab_api.services.compute import registry

    assert registry.backends == []
    assert BackendRegistry().select({}) is None


def test_production_refuses_placeholder_password() -> None:
    import pytest

    from riverlab_api.config import Settings

    with pytest.raises(ValueError, match="placeholder"):
        Settings(env="production", database_url="postgresql+psycopg://u:change-me-local-only@h/db")
    Settings(env="development", database_url="postgresql+psycopg://u:change-me-local-only@h/db")
