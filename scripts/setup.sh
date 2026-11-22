#!/usr/bin/env bash
# One-time local setup: .env, Python environment (builds the Rust engine), Node modules.
source "$(dirname "$0")/lib.sh"

for tool in cargo uv npm docker; do
  command -v "$tool" >/dev/null || die "'$tool' is required (see README: Prerequisites)"
done

if [[ ! -f "$ROOT/.env" ]]; then
  say "Creating .env with a random local database password"
  pw="local$(head -c 12 /dev/urandom | od -An -tx1 | tr -d ' \n')"
  sed "s/change-me-local-only/$pw/g" "$ROOT/.env.example" > "$ROOT/.env"
fi

say "Python environment (compiles the Rust engine via maturin)"
(cd "$ROOT/backend" && uv sync)

say "Frontend dependencies"
(cd "$ROOT/frontend" && npm ci)

say "Done. Next: scripts/dev.sh to run the app, scripts/test.sh to run every check."
