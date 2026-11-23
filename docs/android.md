# Running RiverLab on Android (Termux)

Everything runs on the phone: the database, the API with the Rust engine and solver, and the
web app. After installation no internet connection is needed. You use RiverLab in the phone's
browser at `http://localhost:3000`.

Tested so far: the scripts are written for Termux. Building the engine and the web bundle was
checked off the phone, but not yet on a phone. Expect the first run to need a fix or two; if a
step fails, send the end of the log (`tail -n 60 ~/riverlab-data/setup.log`).

## 1. Install Termux
1. Install **F-Droid** (f-droid.org), then install **Termux** from F-Droid. Don't use the
   Play Store version; it is outdated and can't install packages.
2. Stop Android from killing Termux during long solves:
   - Settings → Apps → Termux → Battery → **Unrestricted**.
   - Samsung: Settings → Battery → Background usage limits → make sure Termux is not in
     "Sleeping apps" or "Deep sleeping apps"; add it to **Never sleeping apps**.
3. Have about **4 GB free**, plug in the charger, and keep the screen on for the first
   install (it compiles for 30–60 minutes).

## 2. Install RiverLab
In Termux:
```bash
pkg update -y && pkg install -y git
git clone https://github.com/DentedCrepe/riverlabs
bash riverlabs/scripts/termux/setup.sh
```
`setup.sh` does the following:
- installs Rust, Python, Node.js and PostgreSQL;
- builds RiverLab's engine for the phone;
- creates a local database;
- downloads the prebuilt web app;
- installs the `riverlab` command.

Re-running it is safe: steps that are already done are skipped.

## 3. Use it
```bash
riverlab start      # then open http://localhost:3000 in Chrome or Samsung Internet
riverlab status
riverlab logs       # last 40 lines of the API and web logs
riverlab stop
```
Add the page to your home screen from the browser menu to open it like an app. Leave Termux
running in the background while you use it. `riverlab start` holds a wake lock so long solves
are not suspended; `riverlab stop` releases it.

Defaults suited to a phone: solves use 2 threads and at most 3 GB of memory. To change them,
re-run setup with `RIVERLAB_SOLVER_THREADS=4 RIVERLAB_SOLVER_MAX_MEMORY_MB=4096 bash
riverlabs/scripts/termux/setup.sh`.

## 4. Update
```bash
riverlab update
```
This pulls the new code, rebuilds only what changed, applies database migrations, downloads
the matching web app and restarts. You need a connection for this; your hotspot is fine.
- **Rust changes** recompile in a few minutes.
- **Database migrations** keep your data.
- **A new GTOpen commit:** saved solve trees from older versions may stop opening (their
  summaries stay).

## How the web app gets onto the phone
Next.js's bundler (Turbopack) has no Android build, so the web app is built off the phone with
`scripts/build-android-web.sh`. The result is plain JavaScript run by Node, about 5 MB. It is
published as `riverlab-web-<hash of frontend/>.tar.gz` on the repository's `android-web`
branch, and `setup.sh` downloads the bundle matching the checkout. **Whenever `frontend/`
changes on the branch the phone runs, publish a new bundle**, or setup stops with
"No prebuilt web app for this version".

## Where things live on the phone
| What | Where |
|---|---|
| Code | `~/riverlabs` |
| Python environment, logs, solver results, web app | `~/riverlab-data` |
| Database | `$PREFIX/var/lib/postgresql` (Termux's own storage) |

To remove everything: `riverlab stop`, then delete those three locations.
