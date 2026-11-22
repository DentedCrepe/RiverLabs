# RiverLab — instructions for AI-assisted development

RiverLab is a six-max no-limit hold'em theory, solving and training platform. Read
`docs/architecture.md` and the ADRs in `docs/adr/` before changing structure.

## Non-negotiable rules
- **Never fabricate poker data.** No invented strategies, frequencies, EVs, statistics or
  "example" solver output — not in the UI, fixtures, seeds or docs. Unfinished features show a
  labelled empty state ("Planned", "Interface only").
- **No solver exists yet.** Do not describe anything as GTO, solved or equilibrium until a
  solver passes the validation harness in `docs/roadmap.md` (Milestone 2).
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
           solver-interface (traits/config only), riverlab-py (PyO3 module riverlab_engine)
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
