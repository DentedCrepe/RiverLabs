# RiverLab

RiverLab is a poker theory, solving and training platform for **six-max no-limit Texas
hold'em cash games**. The default is 100bb deep, and both stack depth and rake are
configurable.

> **Status:** the foundation (engine, evaluator, ranges, equity, betting rules, rake, schema,
> API, app) is complete. **Heads-up postflop solving** runs in the app through GTOpen, an
> audited open-source solver pinned to a commit. Every result shows its exploitability, re-checked
> by RiverLab's own independent best-response code (flop, turn and river) whenever it fits in
> memory. Results are approximate equilibria for the configured bet sizes. Preflop and multiway
> solving are not available in the app yet; unfinished areas are labelled.

## What works today

| Area | Status | Notes |
|---|---|---|
| Cards, deck, positions, game configuration | ✅ | 3–6 players, stack presets 20–200bb and custom depths (2–1000bb) |
| Rake | ✅ | %, cap (bb or currency), minimum pot, rounding, no-flop-no-drop, side-pot allocation |
| Betting engine | ✅ | Min-raise rules, short all-ins, uncalled bets, side pots, odd chips, settlement |
| Seven-card evaluator | ✅ | Checked against all 133,784,560 seven-card hands plus two independent evaluators |
| Range engine | ✅ | 169 classes, 1,326 combos, weights, notation, card removal, blockers |
| Equity | ✅ | Exact enumeration (bounded) or seeded Monte Carlo with standard error |
| Postflop solver | ✅ Heads-up | GTOpen (Apache-2.0, pinned) behind RiverLab's solver interface; local worker with queue, progress and cancel; results stored with provenance |
| Independent validation | ✅ | Best response and exploitability for river/turn/flop trees; exact heads-up push/fold (see `docs/solver-validation.md`) |
| Trainer, hand-history import, analytics, solution library | ⏳ Planned | See `docs/roadmap.md` |

## Architecture in one paragraph
All poker mathematics lives in a Rust workspace (`engine/`). It is exposed to Python as the
`riverlab_engine` module through PyO3. A FastAPI backend (`backend/`) validates request
shapes, calls the engine, and persists configurations and jobs in PostgreSQL through
SQLAlchemy and Alembic. A Next.js frontend (`frontend/`) talks only to the backend, through a
same-origin proxy. See [`docs/architecture.md`](docs/architecture.md) and the decision records
in [`docs/adr/`](docs/adr).

## Prerequisites
- Docker with Compose v2 (needed for PostgreSQL; the full stack can also run in Docker)
- Rust ≥ 1.85 (`rustup`), tested with 1.97
- Python ≥ 3.12 and [uv](https://docs.astral.sh/uv/) ≥ 0.11, tested with Python 3.13
- Node.js ≥ 22 and npm

## Quick start

### Option A: everything in Docker
```bash
cp .env.example .env          # then change POSTGRES_PASSWORD (and the URLs that contain it)
docker compose up --build
```
- Web app: http://localhost:3000
- API: http://localhost:8000. Interactive docs are at http://localhost:8000/docs outside production.

The backend container applies migrations on start.

### Option B: local development (hot reload)
```bash
scripts/setup.sh   # creates .env with a random DB password, uv sync (compiles the engine), npm ci
scripts/dev.sh     # PostgreSQL in Docker, migrations, API on :8000 and web app on :3000
```

## Testing
One command runs every check:
```bash
scripts/test.sh            # all suites (about 30 s)
scripts/test.sh engine     # Rust only
scripts/test.sh backend    # Python only (starts PostgreSQL if needed)
scripts/test.sh frontend   # frontend only
scripts/validate-engine.sh # exhaustive 133M-hand evaluator check in release mode (~30 s)
```

| Suite | Tooling | Covers |
|---|---|---|
| Engine | `cargo fmt`, `clippy -D warnings`, `cargo test`, proptest | Cards, deck, rake, pots, betting rules, random-hand conservation, evaluator, ranges, equity, solver config |
| Backend unit + API | ruff, strict mypy, pytest, Hypothesis | Engine adapter, schema-drift guard, all endpoints, error envelope, solver 503 |
| Database | pytest against real PostgreSQL | Migrations match models, seed data matches engine, constraints, game-config CRUD, job lifecycle |
| Validation | pytest + `phevaluator` + `pokerkit` | Evaluator class bijection, showdown agreement, exact equity vs independent enumeration |
| Frontend | Prettier, ESLint, `tsc`, Vitest + Testing Library | Units, API client, proxy, navigation, range matrix, config form, tools |

Database tests use `RIVERLAB_TEST_DATABASE_URL` and refuse any database whose name does not
end in `_test`.

## Configuration
All configuration is environment-based; see [`.env.example`](.env.example). No credentials have
defaults, and `docker compose` refuses to start without `POSTGRES_PASSWORD`.

| Variable | Used by | Purpose |
|---|---|---|
| `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB` | compose | Database credentials (port bound to 127.0.0.1) |
| `RIVERLAB_DATABASE_URL` | backend | SQLAlchemy URL (`postgresql+psycopg://…`) |
| `RIVERLAB_TEST_DATABASE_URL` | tests | Test database (must end in `_test`) |
| `RIVERLAB_EQUITY_EXACT_BUDGET` / `RIVERLAB_EQUITY_MAX_ITERATIONS` | backend | CPU bounds for equity requests |
| `RIVERLAB_CORS_ORIGINS` | backend | Normally empty (same-origin proxy) |
| `RIVERLAB_SOLVER_ENABLED` / `RIVERLAB_SOLVER_MAX_MEMORY_MB` / `RIVERLAB_SOLVER_THREADS` | backend | Local solver on/off, memory cap per solve (default 2048), threads (0 = all cores) |
| `RIVERLAB_API_URL` | frontend server | Where `/api/backend/*` is forwarded |

## API overview (`/api/v1`)
| Endpoint | Description |
|---|---|
| `GET /health`, `GET /health/ready` | Liveness; database + engine readiness |
| `GET /meta` | Versions and capability status (what is and is not implemented) |
| `GET /positions`, `GET /stack-presets` | Reference data from the engine |
| `GET /game-configs/default`, `POST /game-configs/validate` | Engine default and validation |
| `GET/POST /game-configs`, `GET/DELETE /game-configs/{id}` | Saved configurations |
| `POST /rake/calculate` | Rake for a pot under a configuration |
| `POST /hands/evaluate`, `POST /hands/compare` | Evaluation and showdown |
| `POST /hands/replay` | Replays a hand event by event through the betting engine |
| `POST /ranges/analyze` | Combos, card removal and the 13×13 matrix for range notation |
| `POST /equity` | Exact or Monte Carlo equity for 2–6 hands/ranges |
| `GET /solver/capabilities`, `POST/GET /solver/jobs`, `POST /solver/jobs/{id}/cancel`, `GET /solver/jobs/{id}/result`, `DELETE /solver/jobs/{id}` | Queue, track, cancel, read and delete postflop solves |
| `GET /solver/jobs/{id}/node?path=a0.a1.cAh` | Browse a saved tree: `a<action index>` / `c<dealt card>` steps |

Errors always use `{"error": {"code", "message", "details?"}}`.

## Repository layout
```
engine/            Rust workspace (poker-core, hand-evaluator, range-engine, equity-engine,
                   solver-interface, riverlab-py)
backend/           FastAPI app, models, Alembic migrations, tests
frontend/          Next.js app
tests/validation/  Cross-implementation validation
infrastructure/    Dockerfiles, PostgreSQL init script
scripts/           setup / dev / test / validate-engine
docs/              Architecture, ADRs, rules, rake, evaluator, equity, roadmap, solver audit
```

## Documentation
- [Architecture](docs/architecture.md) · [Roadmap & risks](docs/roadmap.md) · [ADRs](docs/adr)
- [Poker rules](docs/poker-rules.md) · [Rake](docs/rake.md) · [Hand evaluator](docs/hand-evaluator.md) · [Equity](docs/equity.md)
- [Third-party solver audit](docs/solver-research.md)

## Known limitations
- Solving is heads-up postflop only. Solved trees are saved to `RIVERLAB_RESULT_STORAGE_DIR`
  (about the solve's memory size each) for browsing; delete solves you no longer need. Solves run one at a time inside the API process and do not survive a
  restart (they are marked failed).
- The independent check is skipped (and says so) when it would not fit in the memory limit
  alongside the solve.
- Heads-up tables are not supported by the betting engine yet.
- There is no authentication. The API is for local, single-user use and must not be exposed
  publicly.
- Rake caps that depend on the number of players are not modelled.
- Monte Carlo equity is capped per request, and exact enumeration refuses work above a budget.

## Troubleshooting
- **TLS errors during `docker compose build` behind a TLS-intercepting proxy**: the build
  containers need your proxy's CA certificate. This is environment-specific, so it is not part
  of the Dockerfiles.
- **Engine changes not visible in Python**: run `cd backend && uv sync --reinstall-package riverlab-engine`.
