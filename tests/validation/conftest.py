import pytest


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line("markers", "validation: cross-implementation checks against independent evaluators")
