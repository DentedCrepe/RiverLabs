#!/data/data/com.termux/files/usr/bin/bash
# The `riverlab` command on Android (installed by scripts/termux/setup.sh). See docs/android.md.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
DATA="$HOME/riverlab-data"
RUN="$DATA/run"
PGDATA="$PREFIX/var/lib/postgresql"
mkdir -p "$RUN"

running() { [ -f "$RUN/$1.pid" ] && kill -0 "$(cat "$RUN/$1.pid")" 2>/dev/null; }

start() {
  [ -f "$DATA/riverlab.env" ] || { echo "Run setup first: bash $REPO/scripts/termux/setup.sh"; exit 1; }
  # Keep the phone from suspending Termux while RiverLab runs (released by `riverlab stop`).
  command -v termux-wake-lock >/dev/null && termux-wake-lock
  pg_ctl -D "$PGDATA" status >/dev/null 2>&1 || pg_ctl -D "$PGDATA" -l "$DATA/postgres.log" -w start >/dev/null
  set -a
  # shellcheck disable=SC1091
  source "$DATA/riverlab.env"
  set +a
  if ! running api; then
    (
      cd "$REPO/backend"
      # shellcheck disable=SC1091
      source "$DATA/venv/bin/activate"
      alembic upgrade head >>"$DATA/api.log" 2>&1
      nohup uvicorn riverlab_api.main:app --host 127.0.0.1 --port 8000 >>"$DATA/api.log" 2>&1 &
      echo $! >"$RUN/api.pid"
    )
  fi
  if ! running web; then
    (
      cd "$DATA/web/riverlab-web"
      PORT=3000 HOSTNAME=127.0.0.1 RIVERLAB_API_URL=http://127.0.0.1:8000 NODE_ENV=production \
        nohup node server.js >>"$DATA/web.log" 2>&1 &
      echo $! >"$RUN/web.pid"
    )
  fi
  echo -n "Starting"
  for _ in $(seq 1 60); do
    if curl -sf http://127.0.0.1:8000/health >/dev/null && curl -sf -o /dev/null http://127.0.0.1:3000/; then
      echo
      echo "RiverLab is running: open http://localhost:3000 in your browser."
      echo "Leave Termux running in the background; stop with: riverlab stop"
      return
    fi
    echo -n "."
    sleep 2
  done
  echo
  echo "RiverLab did not come up in 2 minutes. Check: riverlab logs"
  exit 1
}

stop() {
  for p in web api; do
    if running "$p"; then kill "$(cat "$RUN/$p.pid")"; fi
    rm -f "$RUN/$p.pid"
  done
  pg_ctl -D "$PGDATA" status >/dev/null 2>&1 && pg_ctl -D "$PGDATA" -m fast stop >/dev/null
  command -v termux-wake-unlock >/dev/null && termux-wake-unlock
  echo "RiverLab stopped."
}

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  restart) stop; start ;;
  status)
    for p in api web; do running "$p" && echo "$p: running" || echo "$p: stopped"; done
    pg_ctl -D "$PGDATA" status >/dev/null 2>&1 && echo "database: running" || echo "database: stopped"
    ;;
  logs) tail -n "${2:-40}" "$DATA/api.log" "$DATA/web.log" ;;
  update)
    # New code, then the same setup (it only redoes what changed), then a restart.
    git -C "$REPO" pull --ff-only
    stop || true
    bash "$REPO/scripts/termux/setup.sh"
    start
    ;;
  *)
    echo "Usage: riverlab {start|stop|restart|status|logs [lines]|update}"
    exit 1
    ;;
esac
