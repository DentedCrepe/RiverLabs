#!/usr/bin/env bash
# End-to-end tests: the real app (production web build + API + PostgreSQL) driven in a browser,
# through the same web proxy the phone uses. Runs against its own database (<db>_e2e_test),
# created fresh and dropped afterwards, and its own result storage. See docs/testing.md.
source "$(dirname "$0")/lib.sh"
load_env
command -v node >/dev/null || die "node is required"
if ! node -e "require('playwright')" 2>/dev/null; then
  echo "Playwright is not installed here: skipping the end-to-end tests (npm i -g playwright to run them)."
  exit 0
fi
ensure_postgres

API_PORT=8100
WEB_PORT=3100
DB="${POSTGRES_DB}_e2e_test"
E2E_DB_URL="postgresql+psycopg://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT:-5432}/${DB}"
WORK="$(mktemp -d)"
STORE="$WORK/results"
mkdir -p "$STORE"
psql_admin() { (cd "$ROOT" && docker compose exec -T postgres psql -q -U "$POSTGRES_USER" -d "$POSTGRES_DB" "$@"); }

cleanup() {
  [[ -n "${API_PID:-}" ]] && kill "$API_PID" 2>/dev/null || true
  [[ -n "${WEB_PID:-}" ]] && kill "$WEB_PID" 2>/dev/null || true
  wait 2>/dev/null || true
  psql_admin -c "DROP DATABASE IF EXISTS ${DB} WITH (FORCE)" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

say "End to end: fresh database ${DB}"
psql_admin -c "DROP DATABASE IF EXISTS ${DB} WITH (FORCE)" >/dev/null
psql_admin -c "CREATE DATABASE ${DB}" >/dev/null
(cd "$ROOT/backend" && RIVERLAB_DATABASE_URL="$E2E_DB_URL" uv run --quiet alembic upgrade head >/dev/null)

# Solved bot models and equity tables are slow to build: reuse the caches the other tests keep.
for f in "$ROOT"/backend/.cache/*; do [[ -e "$f" ]] && ln -s "$f" "$STORE/"; done
if [[ -f "$ROOT/engine/target/hu_combo_equity.bin" ]]; then
  ln -s "$ROOT/engine/target/hu_combo_equity.bin" "$STORE/hu_combo_equity.bin"
  export E2E_PUSH_FOLD=1
fi

say "End to end: API on :${API_PORT}"
(cd "$ROOT/backend" && RIVERLAB_DATABASE_URL="$E2E_DB_URL" RIVERLAB_RESULT_STORAGE_DIR="$STORE" \
  RIVERLAB_ACADEMY_SYNC_ON_STARTUP=true RIVERLAB_SOLVER_MAX_MEMORY_MB=1024 \
  exec uv run --quiet uvicorn riverlab_api.main:app --port "$API_PORT" >"$WORK/api.log" 2>&1) &
API_PID=$!

say "End to end: production web build on :${WEB_PORT}"
(cd "$ROOT/frontend" && npm run --silent build >"$WORK/build.log" 2>&1) || { tail -40 "$WORK/build.log"; die "web build failed"; }
APP="$WORK/web"
mkdir -p "$APP/.next"
cp -r "$ROOT/frontend/.next/standalone/." "$APP/"
cp -r "$ROOT/frontend/.next/static" "$APP/.next/static"
[[ -d "$ROOT/frontend/public" ]] && cp -r "$ROOT/frontend/public" "$APP/"
(cd "$APP" && PORT="$WEB_PORT" HOSTNAME=127.0.0.1 RIVERLAB_API_URL="http://127.0.0.1:${API_PORT}" exec node server.js \
  >"$WORK/web.log" 2>&1) &
WEB_PID=$!

for _ in $(seq 1 120); do
  curl -sf "http://127.0.0.1:${API_PORT}/health" >/dev/null && curl -sf -o /dev/null "http://127.0.0.1:${WEB_PORT}/" && break
  sleep 1
done
curl -sf "http://127.0.0.1:${API_PORT}/health" >/dev/null || { tail -40 "$WORK/api.log"; die "the API did not start"; }

say "End to end: browser tests"
if ! E2E_URL="http://127.0.0.1:${WEB_PORT}" node --test --test-concurrency=1 "$ROOT"/frontend/e2e/*.e2e.mjs; then
  echo "--- API log (tail) ---"; tail -40 "$WORK/api.log"
  echo "--- web log (tail) ---"; tail -20 "$WORK/web.log"
  exit 1
fi
