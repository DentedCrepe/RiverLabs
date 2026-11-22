# Shared helpers for RiverLab scripts. Source, do not execute.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

say() { printf '\n\033[1;32m==> %s\033[0m\n' "$*"; }
die() { printf '\033[1;31mERROR: %s\033[0m\n' "$*" >&2; exit 1; }

load_env() {
  [[ -f "$ROOT/.env" ]] || die ".env not found. Run scripts/setup.sh (or: cp .env.example .env)"
  set -a; source "$ROOT/.env"; set +a
}

# Starts the compose PostgreSQL service unless the configured database is already reachable.
ensure_postgres() {
  local url="${RIVERLAB_TEST_DATABASE_URL:?RIVERLAB_TEST_DATABASE_URL must be set}"
  if (cd "$ROOT/backend" && uv run --quiet python - "$url" <<'PY' >/dev/null 2>&1); then return 0; fi
import sys, psycopg
psycopg.connect(sys.argv[1].replace("postgresql+psycopg://", "postgresql://"), connect_timeout=2).close()
PY
  say "Starting PostgreSQL (docker compose)"
  (cd "$ROOT" && docker compose up -d postgres)
  for _ in $(seq 1 60); do
    if (cd "$ROOT" && docker compose exec -T postgres pg_isready -q -U "$POSTGRES_USER" -d "$POSTGRES_DB"); then
      # The init script creates <db>_test on first start; create it if the volume predates it.
      (cd "$ROOT" && docker compose exec -T postgres psql -q -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tc \
        "SELECT 1 FROM pg_database WHERE datname = '${POSTGRES_DB}_test'" | grep -q 1) ||
        (cd "$ROOT" && docker compose exec -T postgres createdb -U "$POSTGRES_USER" "${POSTGRES_DB}_test")
      return 0
    fi
    sleep 1
  done
  die "PostgreSQL did not become ready"
}
