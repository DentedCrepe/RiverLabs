#!/data/data/com.termux/files/usr/bin/bash
# The `riverlab` command on Android (installed by scripts/termux/setup.sh). See docs/android.md.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
DATA="$HOME/riverlab-data"
RUN="$DATA/run"
PGDATA="$PREFIX/var/lib/postgresql"
mkdir -p "$RUN"

# Processes are found by their command line, not by saved PIDs: a stale PID file must never
# leave an old server running (it would keep the port and serve the old version).
pattern() {
  case "$1" in
    api) echo "uvicorn riverlab_api.main:app" ;;
    # Next.js renames its process to "next-server (vX)", so match that, not server.js.
    web) echo "next-server|next-render-worker|$DATA/web/riverlab-web/server.js" ;;
    watchdog) echo "riverlab.sh watchdog" ;;
  esac
}
running() { pgrep -f -- "$(pattern "$1")" >/dev/null 2>&1; }
# Detach from the launching shell, so a home-screen shortcut can exit without taking RiverLab down.
detach() { if command -v setsid >/dev/null; then setsid nohup "$@"; else nohup "$@"; fi; }

start() {
  [ -f "$DATA/riverlab.env" ] || { echo "Run setup first: bash $REPO/scripts/termux/setup.sh"; exit 1; }
  # No wake lock here: the API takes one only while a solve or table preparation is running
  # (RIVERLAB_WAKE_LOCK_COMMAND), so an idle phone can sleep.
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
      detach uvicorn riverlab_api.main:app --host 127.0.0.1 --port 8000 >>"$DATA/api.log" 2>&1 &
      echo $! >"$RUN/api.pid"
    )
  fi
  if ! running web; then
    (
      cd "$DATA/web/riverlab-web"
      PORT=3000 HOSTNAME=127.0.0.1 RIVERLAB_API_URL=http://127.0.0.1:8000 NODE_ENV=production \
        detach node "$DATA/web/riverlab-web/server.js" >>"$DATA/web.log" 2>&1 &
      echo $! >"$RUN/web.pid"
    )
  fi
  if ! running watchdog; then
    detach bash "$REPO/scripts/termux/riverlab.sh" watchdog >>"$DATA/watchdog.log" 2>&1 &
    echo $! >"$RUN/watchdog.pid"
  fi
  echo -n "Starting"
  for _ in $(seq 1 60); do
    if curl -sf http://127.0.0.1:8000/health >/dev/null && curl -sf -o /dev/null http://127.0.0.1:3000/; then
      echo
      echo "RiverLab is running: open http://localhost:3000 in your browser."
      echo "It stops by itself after ${RIVERLAB_IDLE_MINUTES:-20} idle minutes; or: riverlab stop"
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
  for p in watchdog web api; do
    # Never kill this script itself (the watchdog calls stop).
    pgrep -f -- "$(pattern "$p")" | grep -vx "$$" | xargs -r kill 2>/dev/null || true
    rm -f "$RUN/$p.pid"
  done
  # Wait for the ports to free up, so a following start can bind them.
  for _ in $(seq 1 20); do
    curl -s -o /dev/null http://127.0.0.1:8000/health || curl -s -o /dev/null http://127.0.0.1:3000/ || break
    sleep 0.5
  done
  pg_ctl -D "$PGDATA" status >/dev/null 2>&1 && pg_ctl -D "$PGDATA" -m fast stop >/dev/null
  command -v termux-wake-unlock >/dev/null && termux-wake-unlock
  echo "RiverLab stopped."
}

# Stops RiverLab after RIVERLAB_IDLE_MINUTES (default 20; 0 = never) without user requests,
# unless a solve or table preparation is running. Checks once a minute.
watchdog() {
  local limit=$((${RIVERLAB_IDLE_MINUTES:-20} * 60)) status idle
  [ "$limit" -gt 0 ] || exit 0
  while sleep 60; do
    status="$(curl -sf http://127.0.0.1:8000/health/activity)" || continue
    case "$status" in *'"busy":true'*) continue ;; esac
    idle="$(sed -n 's/.*"idle_seconds":\([0-9]*\).*/\1/p' <<<"$status")"
    if [ -n "$idle" ] && [ "$idle" -ge "$limit" ]; then
      echo "$(date) idle for ${idle}s: stopping RiverLab"
      rm -f "$RUN/watchdog.pid"
      stop
      exit 0
    fi
  done
}

case "${1:-}" in
  start) start ;;
  open)
    # Start if needed, then open the app (the installed RiverLab app, or the browser).
    running api && running web || start
    termux-open-url http://localhost:3000
    ;;
  watchdog) watchdog ;;
  stop) stop ;;
  restart) stop; start ;;
  status)
    for p in api web; do running "$p" && echo "$p: running" || echo "$p: stopped"; done
    pg_ctl -D "$PGDATA" status >/dev/null 2>&1 && echo "database: running" || echo "database: stopped"
    ;;
  logs) tail -n "${2:-40}" "$DATA/api.log" "$DATA/web.log" ;;
  update)
    # Pull, then continue with the *new* version of this script (bash would otherwise keep
    # running the old one it already read).
    git -C "$REPO" pull --ff-only
    exec bash "$REPO/scripts/termux/riverlab.sh" update-installed
    ;;
  update-installed)
    stop || true
    bash "$REPO/scripts/termux/setup.sh"
    start
    ;;
  *)
    echo "Usage: riverlab {open|start|stop|restart|status|logs [lines]|update}"
    exit 1
    ;;
esac
