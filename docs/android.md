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

## 3. Use it as an app
**One-time setup**
1. Run `riverlab open`. RiverLab starts and opens in your browser.
2. In Chrome's menu, choose **Add to home screen → Install**. RiverLab gets its own icon and
   opens full-screen, without the address bar.
3. Optional: install **Termux:Widget** from F-Droid, then add its widget to your home screen.
   It shows two buttons, made by setup:
   - **RiverLab:** starts RiverLab if needed and opens the app.
   - **Stop RiverLab.**

**Daily use:** tap the **RiverLab** widget button. If RiverLab is already running, the
installed app icon works too.

**From Termux**
```bash
riverlab open       # start if needed, then open the app
riverlab status
riverlab logs       # last 40 lines of the API and web logs
riverlab stop
```

**Battery**
- **No wake lock while idle.** The phone is kept awake only while a solve or table preparation is
  running, so it can sleep mid-solve without the solve stopping. The lock is released when that
  work ends.
- **Stops itself after 20 idle minutes.** With no use and nothing running, RiverLab shuts down
  completely, database included. Tap the widget to start it again; that takes a few seconds.
  Background refreshes and health checks don't count as use.
- **Changing the idle time:** re-run setup with `RIVERLAB_IDLE_MINUTES=60` (0 = never stop).
- **Checking it:** look at Settings → Battery → Termux after a day of normal use. While idle,
  the three processes use almost no CPU.
- **No refreshing in the background.** Pages that refresh themselves (solve lists, status) stop
  while the screen is off or the browser is in the background, and catch up as soon as you look
  again. Solve lists refresh every 1.5 s only while a solve is queued or running, otherwise
  every 15 s.
- **Fast restarts.** Checking every Academy lesson runs its solves, so it is done only when the
  lessons, the engine or the checker changed since the last clean check (recorded in
  `~/riverlab-data/solver-results/academy-sync.json`); otherwise starting takes about 0.01 s
  for the Academy instead of a few seconds.

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

The push/fold trainer's exact equity table (`hu_combo_equity.bin.gz`, about 7 MB) is published
on the same branch. `setup.sh` downloads it once, if it is not already in
`~/riverlab-data/solver-results`. RiverLab re-checks entries exactly before using it
(`docs/trainer.md`). If the download fails, setup carries on and the Trainer page offers to
build the table on the phone instead.

## Where things live on the phone
| What | Where |
|---|---|
| Code | `~/riverlabs` |
| Python environment, logs, solver results, web app | `~/riverlab-data` |
| Database | `$PREFIX/var/lib/postgresql` (Termux's own storage) |

To remove everything: `riverlab stop`, then delete those three locations.
