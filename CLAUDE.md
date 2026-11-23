# RiverLab — instructions for AI-assisted development

RiverLab is a six-max no-limit hold'em theory, solving and training platform. Read
`docs/architecture.md` and the ADRs in `docs/adr/` before changing structure.

## Non-negotiable rules
- **Never fabricate poker data.** No invented strategies, frequencies, EVs, statistics or
  "example" solver output — not in the UI, fixtures, seeds or docs. Unfinished features show a
  labelled empty state ("Planned", "Interface only").
- **Solver output is always shown with its exploitability** and its approximations. Postflop
  solves come from GTOpen (pinned commit, `engine/solver-gtopen`); results are re-checked by
  `engine/solver-validation` (skipped, with a note, only when memory does not allow). Never call a result "GTO" without qualification:
  it is an approximate equilibrium for the configured bet sizes. GTOpen's preflop solver
  runs in the app (Preflop page, `kind = "preflop"` jobs) and is a **modelled approximation**:
  every preflop result must say so (`ValidatedScope` in `engine/solver-gtopen/src/preflop.rs`).
  Only heads-up push/fold without ante or rake is validated; never widen that label without
  new validation in `docs/solver-validation.md`.
- **Bots are practice opponents, not solvers.** Their rules and every grading threshold are
  documented in `docs/play.md`; keep that file in sync, and never grade the user against
  invented "typical player" numbers.
- **Academy content is held to the same standard.**
  - Every number in a lesson must come from the engine: widgets call the API, and numeric
    answers use `compute`.
  - Label every claim as fact, heuristic, approximation or solver result.
  - Cite only real works.
  - Bump `version` on any change to a published lesson.
  - Never mark a lesson `reviewed` unless a person reviewed it.
  - Run `uv run pytest tests/unit/test_academy_content.py` after editing content.
- **Changing the GTOpen commit** requires re-running `tools/solver-audit/run.sh` and the
  validation examples (`docs/solver-validation.md`) before updating the pin.
- **Poker mathematics lives only in `engine/` (Rust).** The backend calls it through
  `backend/src/riverlab_api/services/engine.py`; the frontend calls the backend. Do not
  re-implement rules, evaluation, combinatorics or rake in Python or TypeScript.
- **Chips are integer milli-big-blinds** (`1bb = 1000`). Never use floats for chip amounts.
  Reject inexact input instead of rounding it silently.
- **Errors are never swallowed.** Engine errors surface as structured 422 responses.
- **No credentials in code.** Configuration comes from the environment (`.env.example`).
- **No third-party solver code** without a licence and correctness audit
  (`docs/solver-research.md`). AGPL code must not be copied.
- **Destructive migrations need safeguards.** Downgrading `0001` requires
  `-x allow_destructive=true`. Tests refuse databases not named `*_test`.

## Layout
```
engine/    Rust workspace: poker-core, hand-evaluator, range-engine, equity-engine,
           solver-interface (traits/config), solver-validation (independent best response,
           exact push/fold), solver-gtopen (GTOpen adapter), play-engine (games against bots,
           docs/play.md), poker-theory (Academy formulas), riverlab-py (PyO3 riverlab_engine)
content/   Academy curriculum and lessons (TOML front matter + Markdown; docs/academy.md)
backend/   FastAPI (package riverlab_api), SQLAlchemy models, Alembic migrations, pytest
frontend/  Next.js 16 App Router, React 19, Tailwind 4, Vitest
tests/validation/  differential tests vs independent evaluators (phevaluator, pokerkit)
scripts/   setup.sh, dev.sh, test.sh, validate-engine.sh
docs/      architecture, ADRs, poker-rules, rake, hand-evaluator, equity, roadmap
```

## Commands
- `scripts/setup.sh`: create `.env`, run `uv sync` (compiles the engine), run `npm ci`.
- `scripts/dev.sh`: start PostgreSQL, run migrations, start the API (:8000) and web (:3000).
- `scripts/test.sh [engine|backend|frontend]`: run every format, lint, type and test check.
  **Run this before declaring work done.**
- `scripts/validate-engine.sh`: run the exhaustive 133M-hand evaluator check (release build).
- After changing Rust code used by Python, run `cd backend && uv sync --reinstall-package riverlab-engine`
  (`test.sh` and `dev.sh` do this for you).
- New migration: `cd backend && uv run alembic revision --autogenerate -m "..."`, then review
  it by hand. `tests/db/test_migrations.py` fails if models and migrations diverge.

## Conventions
- Rust: `cargo fmt` (max width 120), clippy clean with `-D warnings`, no `unsafe`. Add
  proptests for invariants such as conservation, ordering and round-trips.
- Python: ruff + strict mypy. Pydantic models use `extra="forbid"`. The engine validates
  semantics; Pydantic only validates shape.
- Frontend: client data fetching goes through `src/lib/api.ts` → `/api/backend/*` proxy
  (`src/lib/proxy.ts` allow-lists paths). Use components from `src/components/ui`. Keep
  status badges honest (`src/lib/nav.ts`).
- Next.js 16 differs from older versions. Read `frontend/node_modules/next/dist/docs/` before
  using unfamiliar APIs. `cacheComponents` is on.
- Mark a feature "available" only when it is implemented and tested end to end.
