# RiverLab Architecture

Status: Milestone 1 (foundation). Last updated 2026-11-22.

## 1. Goals that shape the architecture

1. **Mathematical correctness first.** Every number shown to a user must come from validated
   code. Placeholder content is labelled as such; no fabricated strategies or statistics.
2. **One source of truth for poker math.** Card logic, hand evaluation, ranges, equity, betting
   rules, pots and rake live in one place (the Rust engine). Other layers call it; they do not
   re-implement it.
3. **Replaceable solver.** The frontend talks to a stable job API. Which solver backend runs a
   job, and whether it runs locally or in the cloud, is invisible to the UI.
4. **No speculative infrastructure.** Redis, workers and object storage are designed for but not
   deployed until a feature needs them.

## 2. Repository layout

```
.
├── engine/                  Rust workspace — all poker mathematics
│   ├── poker-core/          cards, deck, positions, chips, game config, rake, betting state machine, pots
│   ├── hand-evaluator/      7-card evaluator (bitmask-based, exhaustively validated)
│   ├── range-engine/        169 hand classes, 1326 combos, weighted ranges, notation, card removal
│   ├── equity-engine/       exact enumeration equity (+ seeded Monte Carlo)
│   ├── solver-interface/    traits + configuration models for CFR-family solvers (no solver)
│   └── riverlab-py/         PyO3 bindings → Python module `riverlab_engine`
├── backend/                 FastAPI + Pydantic + SQLAlchemy + Alembic (package `riverlab_api`)
├── frontend/                Next.js (App Router) + React + TypeScript + Tailwind
├── infrastructure/docker/   Dockerfiles
├── tests/validation/        cross-implementation checks (Rust engine vs independent evaluator)
├── scripts/                 dev / test / validation entry points
├── docs/                    architecture, ADRs, roadmap, domain rules
└── docker-compose.yml
```

Deviation from the suggested layout: the suggested `engine/solver-rust/` crate is **not created
yet** (ADR-0005). An empty crate would imply a solver exists. It will be added when solver work
starts; the `solver-interface` crate defines the contract it must satisfy.

## 3. Dependency direction

```
hand-evaluator ─┐
range-engine ───┼──► poker-core            (no I/O, no async, no serde_json at the core)
equity-engine ──┘    ▲
solver-interface ────┘
riverlab-py ──► all engine crates          (thin binding layer, JSON at the boundary for configs)
backend ──► riverlab_engine (Python module) + PostgreSQL
frontend ──► backend HTTP API only          (via a same-origin proxy route)
```

Rules:
- `engine/*` crates never depend on the backend, the database or the frontend.
- The backend never re-implements poker rules; it validates transport shapes (Pydantic) and
  delegates semantics to the engine.
- The frontend contains presentation logic only. The single exception is static layout data
  (e.g. the 13×13 grid labels), which is not mathematics.

## 4. Engine (Rust)

### 4.1 Units
- Chips are integers in **milli-big-blinds (mbb)**: 1 bb = 1000 mbb, SB = 500 mbb. All betting,
  pot and rake arithmetic is integer; no floating point touches chip counts.
- Currency amounts (rake caps, rounding increments in money) are integer *minor units* (e.g.
  cents) and are converted to mbb using the configured stake. Conversions that are not exact
  are rejected at validation time rather than silently rounded (see `docs/rake.md`).
- Range weights and equities are `f64` in `[0, 1]`.

### 4.2 Hand evaluator
Bitmask evaluator producing a `HandStrength(u32)`: category in the high bits, up to five rank
nibbles below. Integer comparison gives the complete hand order; equal values mean an exact tie.
Validation (see `docs/hand-evaluator.md`):
- all 2,598,960 five-card hands → exactly 7,462 distinct strengths, textbook category counts;
- all 133,784,560 seven-card hands → textbook category counts (release-mode check);
- 7-card result equals the best of its 21 five-card subsets (property test);
- pairwise ordering agrees with the independent `phevaluator` and `pokerkit` evaluators on random hands.

### 4.3 Betting state machine (`poker-core::game`)
Event-sourced: `GameState` is built from a `GameConfig` and advanced by `apply(Action)` and
`deal_board(cards)`. It enforces no-limit rules: minimum bet/raise, all-in for less, the
"short all-in does not reopen action" rule (with cumulative reopening), uncalled-bet return,
side pots, street transitions and showdown settlement with odd-chip assignment. Rules are
documented in `docs/poker-rules.md`.

### 4.4 Solver interface
Traits (`GameTree`, `Solver`, `StrategyProfile`, `ExploitabilityEvaluator`) and validated
configuration (`SolverConfig`, `CfrVariant::{Vanilla, CfrPlus, Discounted, MonteCarlo}`,
stopping criteria). **No solver implements these traits yet.** The backend reports
"no solver backend available" rather than pretending.

## 5. Backend (FastAPI)

```
riverlab_api/
  config.py        pydantic-settings; all secrets from environment
  db/              engine/session factory, SQLAlchemy models (one module per aggregate)
  schemas/         Pydantic transport models
  services/        engine adapter, game-config service, compute service
  api/v1/          routers: health, meta, cards/hands, ranges, equity, rake, game-configs, solver
```

- `services/engine.py` is the only module that imports `riverlab_engine`.
- `services/compute.py` defines the `ComputeBackend` protocol (submit / status / cancel /
  progress / result). The registry is empty in Milestone 1, so solver job creation returns
  `503 solver_unavailable`. Local (in-process) and cloud (queued worker) backends will implement
  the same protocol; the job record stores which backend ran it, never exposed as a UI concern.
- Errors: engine errors become structured `422` responses (`{"error": {"code", "message"}}`);
  nothing is swallowed.

## 6. Data model (PostgreSQL)

Tables (Alembic migration `0001`): users, user_settings, game_configs (embeds stack + rake
configuration columns as versioned JSONB documents validated by the engine), positions
(reference data), ranges, range_combos, solver_configs, solver_jobs, solver_results,
strategy_nodes, training_sessions, training_decisions, hand_histories, performance_stats,
saved_solutions.

Every solver result row stores `engine_version`, `solver_backend`, `solver_version`,
`config_schema_version` and the full serialized configuration/assumptions, so results remain
interpretable after the engine evolves. Large result payloads will go to S3-compatible storage
(`storage_uri` column); only metadata lives in PostgreSQL.

See ADR-0004 for why stack and rake configuration are JSONB documents rather than separate
tables, while remaining independently versioned and independently selectable.

## 7. Frontend (Next.js)

- App Router, server components by default, client components for interactive tools.
- Browser → `/api/backend/*` (Next route handler) → `RIVERLAB_API_URL`. Same-origin, no CORS,
  backend URL is runtime configuration.
- Design tokens (colours, spacing, typography) are CSS variables consumed by Tailwind.
- Every section that is not implemented is labelled **Planned** or **In development**.

## 8. Local vs cloud computation

| Work                                   | Where (target)        | Milestone 1 status |
|----------------------------------------|-----------------------|--------------------|
| Hand evaluation, combinatorics, ranges | engine, in-process    | Implemented        |
| Equity (exact enumeration)             | engine, in-process    | Implemented (bounded) |
| Training grading                       | engine, in-process    | Planned            |
| Postflop solves                        | ComputeBackend (local worker or cloud) | Interface only |
| Batch / high-iteration jobs            | cloud workers + Redis queue | Planned       |

Job lifecycle (designed, not active): `queued → running → (succeeded | failed | cancelled)`,
progress as `{iterations, exploitability_mbb_per_hand?, elapsed_ms}`, cancellation by
cooperative flag, results persisted with full assumptions.

## 9. Configuration
All runtime configuration is environment-based (`.env`, see `.env.example`). No credentials
are hard-coded; `docker-compose.yml` refuses to start without `POSTGRES_PASSWORD`.
