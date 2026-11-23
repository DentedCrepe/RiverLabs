# Testing

`scripts/test.sh` runs every check: Rust (fmt, clippy, tests), backend (ruff, mypy, pytest
including the database and validation suites), frontend (prettier, eslint, tsc, vitest) and the
end-to-end browser tests. `scripts/test.sh <engine|backend|frontend|e2e>` runs one part.

## End-to-end tests
`scripts/e2e.sh` (`frontend/e2e/*.e2e.mjs`) drives the real app in a phone-sized browser, through
the same web proxy the phone uses, so problems that unit tests cannot see (such as a path the proxy
refuses) are caught. It:

1. creates a fresh database `<POSTGRES_DB>_e2e_test` (dropped afterwards);
2. starts the API on port 8100 with its own result storage, reusing the cached bot models and,
   when built, the exact push/fold table (`engine/target/hu_combo_equity.bin`);
3. builds the web app for production (standalone, as on Android) and serves it on port 3100;
4. runs the browser tests: the dashboard and daily goals, an Academy answer, a hand at the table
   and its replay, a push/fold drill (skipped without the table), a solve shown in the library, a
   postflop drill from it, Analytics, and no uncaught page errors.

It needs Playwright with Chromium. Without it the step says so and is skipped.
