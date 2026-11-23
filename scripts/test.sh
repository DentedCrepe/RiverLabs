#!/usr/bin/env bash
# The single repeatable check: formatting, linting, type checking and every test suite.
# Exits non-zero on the first failing step.
#   scripts/test.sh            everything (engine, backend incl. DB + validation, frontend)
#   scripts/test.sh engine     only the Rust engine
#   scripts/test.sh backend    only the backend (needs PostgreSQL; started automatically)
#   scripts/test.sh frontend   only the frontend
#   scripts/test.sh e2e        only the end-to-end browser tests (scripts/e2e.sh)
source "$(dirname "$0")/lib.sh"
target="${1:-all}"

engine() {
  say "Engine: rustfmt, clippy, tests"
  cd "$ROOT/engine"
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --quiet -- -D warnings
  cargo test --workspace --quiet
}

backend() {
  load_env
  ensure_postgres
  say "Backend: rebuild engine extension, ruff, mypy, pytest (unit, API, DB, validation)"
  cd "$ROOT/backend"
  uv sync --quiet --reinstall-package riverlab-engine
  uv run ruff format --check src tests alembic ../tests/validation
  uv run ruff check src tests alembic ../tests/validation
  uv run mypy
  uv run pytest -q
}

frontend() {
  say "Frontend: prettier, eslint, tsc, vitest"
  cd "$ROOT/frontend"
  [[ -d node_modules ]] || npm ci
  npm run --silent format:check
  npm run --silent lint
  npm run --silent typecheck
  npm test --silent
}

e2e() {
  bash "$ROOT/scripts/e2e.sh"
}

# Each step runs in its own subshell as a plain command (never inside && / || lists), so
# `set -e` stays active inside the step and any failing command aborts the script.
case "$target" in
  engine) (engine) ;;
  backend) (backend) ;;
  frontend) (frontend) ;;
  e2e) (e2e) ;;
  all)
    (engine)
    (backend)
    (frontend)
    (e2e)
    ;;
  *) die "unknown target '$target' (engine|backend|frontend|e2e|all)" ;;
esac
say "All checks passed ($target)"
