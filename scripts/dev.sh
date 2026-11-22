#!/usr/bin/env bash
# Runs PostgreSQL (docker), applies migrations, then the API (:8000) and the web app (:3000)
# with hot reload. Ctrl-C stops both servers.
source "$(dirname "$0")/lib.sh"
load_env
ensure_postgres

say "Rebuilding the engine extension if Rust sources changed"
(cd "$ROOT/backend" && uv sync --quiet --reinstall-package riverlab-engine)

say "Applying migrations"
(cd "$ROOT/backend" && uv run alembic upgrade head)

say "Starting API on http://localhost:${BACKEND_PORT:-8000} and web app on http://localhost:${FRONTEND_PORT:-3000}"
trap 'kill 0' EXIT INT TERM
(cd "$ROOT/backend" && uv run uvicorn riverlab_api.main:app --reload --port "${BACKEND_PORT:-8000}") &
(cd "$ROOT/frontend" && RIVERLAB_API_URL="http://localhost:${BACKEND_PORT:-8000}" npx next dev -p "${FRONTEND_PORT:-3000}") &
wait
