#!/data/data/com.termux/files/usr/bin/bash
# Installs (or updates) RiverLab inside Termux on Android. Safe to re-run: every step skips work
# that is already done. Afterwards: `riverlab start`. See docs/android.md.
#
# What it does:
#   1. Termux packages: Rust, Python, Node.js, PostgreSQL, build tools.
#   2. A Python environment with the backend's dependencies.
#   3. Compiles RiverLab's Rust engine for the phone (20–40 minutes the first time).
#   4. A local PostgreSQL database, plus migrations.
#   5. Downloads the prebuilt web app that matches this checkout (it can't be built on Android).
#   6. Installs the `riverlab` command (start / stop / status / logs / update).
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
DATA="$HOME/riverlab-data"
LOG="$DATA/setup.log"
mkdir -p "$DATA"
exec > >(tee -a "$LOG") 2>&1
CURRENT="checking the environment"
trap 'echo; echo "!! Setup failed while $CURRENT."; echo "!! Send Claude the end of the log: tail -n 60 $LOG"' ERR
step() { CURRENT="$1"; echo; echo "==> $1 ($(date +%H:%M:%S))"; }

if [[ "${PREFIX:-}" != *com.termux* ]]; then
  echo "Run this inside Termux (installed from F-Droid)." >&2
  exit 1
fi
echo "RiverLab setup: repository $REPO, data $DATA"
# Android's df only knows -k; "Available" is the third column from the end even if a long
# filesystem name wraps the line.
FREE_MB=$(df -k "$HOME" 2>/dev/null | tail -n 1 | awk '{print int($(NF-2) / 1024)}' || true)
if [[ "$FREE_MB" =~ ^[0-9]+$ ]]; then
  echo "Free space: ${FREE_MB} MB"
  if [ "$FREE_MB" -lt 4000 ]; then
    echo "!! Only ${FREE_MB} MB free. The first build needs about 4 GB; free some space first." >&2
    exit 1
  fi
else
  echo "(Could not read free space; continuing. The first build needs about 4 GB.)"
fi

step "installing Termux packages"
pkg update -y
pkg upgrade -y -o Dpkg::Options::=--force-confnew
pkg install -y git rust python nodejs-lts postgresql clang make cmake pkg-config binutils openssl libffi curl procps

step "preparing the Python environment"
# Rust-based Python packages (pydantic-core, the engine) need the Android API level to build.
export ANDROID_API_LEVEL="$(getprop ro.build.version.sdk)"
[ -d "$DATA/venv" ] || python -m venv "$DATA/venv"
# shellcheck disable=SC1091
source "$DATA/venv/bin/activate"
pip install --quiet --upgrade pip wheel

step "installing the backend's Python dependencies (some compile on the phone; this can take a while)"
# Same versions as backend/pyproject.toml, without the extras that have no Android build:
# psycopg's binary wheel (the pure-Python driver uses Termux's libpq) and uvicorn[standard].
mapfile -t DEPS < <(python - "$REPO/backend/pyproject.toml" <<'PY'
import sys, tomllib
deps = tomllib.load(open(sys.argv[1], "rb"))["project"]["dependencies"]
for d in deps:
    if d.startswith("riverlab-engine"):
        continue
    print(d.replace("psycopg[binary]", "psycopg").replace("uvicorn[standard]", "uvicorn"))
PY
)
pip install "${DEPS[@]}"

step "compiling the RiverLab engine (Rust; 20–40 minutes the first time, faster on updates)"
pip install --quiet "maturin>=1.9,<2"
# Faster, lower-memory build than the desktop release profile; results are identical.
export CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
# Android's loader does not let an extension borrow Python's symbols from the running
# interpreter (as Linux does), so link the engine against libpython explicitly.
PY_LIBDIR="$(python -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')"
PY_LDVERSION="$(python -c 'import sysconfig; print(sysconfig.get_config_var("LDVERSION"))')"
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-L$PY_LIBDIR -C link-arg=-lpython$PY_LDVERSION"
rm -rf "$DATA/wheels"
maturin build --release -m "$REPO/engine/riverlab-py/Cargo.toml" -o "$DATA/wheels"
# Rust tags the wheel for Android (android_<api>_arm64_v8a); Termux's Python only accepts its
# own platform tag (e.g. linux_aarch64). Retag the wheel with what this pip accepts.
PLAT="$(python -c 'from pip._vendor.packaging import tags; print(next(iter(tags.sys_tags())).platform)')"
for whl in "$DATA"/wheels/riverlab_engine-*.whl; do
  case "$whl" in *"-$PLAT.whl") ;; *) python -m wheel tags --remove --platform-tag "$PLAT" "$whl" >/dev/null ;; esac
done
pip install --force-reinstall --no-deps "$DATA"/wheels/riverlab_engine-*-"$PLAT".whl
pip install --quiet --no-deps -e "$REPO/backend"
python -c "import riverlab_engine; print('engine', riverlab_engine.engine_version())"

step "setting up the database"
PGDATA="$PREFIX/var/lib/postgresql"
if [ ! -f "$PGDATA/PG_VERSION" ]; then
  mkdir -p "$PGDATA"
  # Only reachable from this phone (localhost / the Termux socket), so no password is needed.
  initdb -D "$PGDATA" -U riverlab --auth-local=trust --auth-host=trust >/dev/null
fi
pg_ctl -D "$PGDATA" status >/dev/null 2>&1 || pg_ctl -D "$PGDATA" -l "$DATA/postgres.log" -w start
psql -U riverlab -h 127.0.0.1 -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = 'riverlab'" | grep -q 1 ||
  createdb -U riverlab -h 127.0.0.1 riverlab

cat > "$DATA/riverlab.env" <<EOF
RIVERLAB_ENV=development
RIVERLAB_DATABASE_URL=postgresql+psycopg://riverlab@127.0.0.1:5432/riverlab
RIVERLAB_SOLVER_ENABLED=true
RIVERLAB_SOLVER_THREADS=${RIVERLAB_SOLVER_THREADS:-2}
RIVERLAB_SOLVER_MAX_MEMORY_MB=${RIVERLAB_SOLVER_MAX_MEMORY_MB:-3072}
RIVERLAB_RESULT_STORAGE_DIR=$DATA/solver-results
RIVERLAB_ACADEMY_CONTENT_DIR=$REPO/content/academy
# Battery: the device stays awake only while a solve or table preparation runs, and RiverLab
# stops itself after this many idle minutes (0 = never).
RIVERLAB_WAKE_LOCK_COMMAND=termux-wake-lock
RIVERLAB_WAKE_UNLOCK_COMMAND=termux-wake-unlock
RIVERLAB_IDLE_MINUTES=${RIVERLAB_IDLE_MINUTES:-20}
EOF
set -a
# shellcheck disable=SC1091
source "$DATA/riverlab.env"
set +a
(cd "$REPO/backend" && alembic upgrade head)

step "downloading the web app"
# The web app is built off the phone (Next.js's bundler has no Android build) and published per
# version of frontend/ on the repository's android-web branch.
TREE="$(git -C "$REPO" rev-parse HEAD:frontend)"
if [ "$(cat "$DATA/web/VERSION" 2>/dev/null)" != "$TREE" ]; then
  URL="https://raw.githubusercontent.com/DentedCrepe/riverlabs/android-web/riverlab-web-$TREE.tar.gz"
  if ! curl -fsSL "$URL" -o "$DATA/web.tar.gz"; then
    echo "!! No prebuilt web app for this version yet ($TREE). Ask Claude to publish it, then re-run." >&2
    false
  fi
  rm -rf "$DATA/web" && mkdir -p "$DATA/web"
  tar xzf "$DATA/web.tar.gz" -C "$DATA/web" && rm "$DATA/web.tar.gz"
  echo "$TREE" > "$DATA/web/VERSION"
fi

step "installing the riverlab command"
cat > "$PREFIX/bin/riverlab" <<EOF
#!/data/data/com.termux/files/usr/bin/bash
exec bash "$REPO/scripts/termux/riverlab.sh" "\$@"
EOF
chmod +x "$PREFIX/bin/riverlab"

step "adding home-screen shortcuts (for the Termux:Widget add-on)"
# Termux:Widget lists scripts in ~/.shortcuts; those in tasks/ run in the background.
mkdir -p "$HOME/.shortcuts/tasks"
printf '#!/data/data/com.termux/files/usr/bin/bash\nexec riverlab open\n' >"$HOME/.shortcuts/tasks/RiverLab"
printf '#!/data/data/com.termux/files/usr/bin/bash\nexec riverlab stop\n' >"$HOME/.shortcuts/tasks/Stop RiverLab"
chmod +x "$HOME/.shortcuts/tasks/RiverLab" "$HOME/.shortcuts/tasks/Stop RiverLab"

echo
echo "==> Done. Open RiverLab with:  riverlab open   (or the RiverLab home-screen shortcut)"
