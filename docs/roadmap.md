# Roadmap and Risks

## Milestones
| # | Milestone | Content | Exit criteria |
|---|-----------|---------|---------------|
| 1 | Foundation (this) | Monorepo, engine core, evaluator, ranges, equity, rake, betting state machine, DB schema + migrations, API, app shell | All tests pass; evaluator exhaustively validated; app runs via Docker Compose |
| 2 | Solver validation harness | `solver-rust` crate; CFR, CFR+, DCFR on Kuhn and Leduc; best response + exploitability; convergence tests vs known values | Exploitability → 0 at expected rates on toy games |
| 3 | Hold'em postflop tree | Bet-size abstraction, tree builder from `GameState`, suit isomorphism, memory estimates, river-only solves validated against independent results | River solves match brute-force best response |
| 4 | Job infrastructure | Redis queue, worker process, progress/cancellation, S3-compatible result storage | Jobs survive restarts; cancellation works |
| 5 | Preflop | Range editor persistence, preflop solution import/solve, 13×13 strategy visualisation | Only solver-produced data displayed. **In progress:** GTOpen's preflop model runs in the app, labelled approximate; heads-up push/fold validated |
| 6 | Trainer | Spot generation from stored solutions, grading by EV loss | Grades reproducible from stored results |
| 7 | Hand analysis & analytics | Hand-history parsing, leak reports | Parser validated on real histories |
| 8 | Accounts | Authentication, per-user settings, quotas | Security review passed |

## Risks
| Risk | Impact | Mitigation |
|------|--------|-----------|
| Solver correctness (subtle CFR bugs give plausible but wrong strategies) | Critical | Toy-game validation first; exploitability reported with every result; never display unconverged results as GTO |
| Solver memory for full postflop trees | High | Isomorphism, bet-size abstraction, memory estimation before solving, cloud workers |
| Rake rule diversity between sites | Medium | Configurable rule model; unsupported rules rejected explicitly, not approximated |
| Licence contamination from third-party solver code | High | Audit before reading for reuse; no copying; ADR-0005 |
| PyO3 build friction (Rust toolchain in backend build) | Medium | uv path dependency + multi-stage Docker build, documented commands |
| Rust/Python schema drift for configs | Medium | Engine is the semantic validator; round-trip tests in backend |
| Scope creep in UI before math exists | Medium | Planned sections clearly labelled; roadmap gates |
