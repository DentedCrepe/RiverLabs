# ADR-0005: Solver interface first, no solver implementation in Milestone 1

Status: Accepted (2026-11-22)

## Decision
Milestone 1 delivers the solver *contract* (Rust traits, validated configuration, backend job
API shape and database tables) but no solving algorithm. Job submission returns
`503 solver_unavailable`. No `solver-rust` crate is created until real solver work begins.

Third-party solvers were audited (`docs/solver-research.md`); none is adopted. Copyleft
licences (e.g. AGPL) are incompatible with an undecided commercial/hosted licensing model,
and no candidate was adopted without our own validation harness (exploitability
measurement on small games with known solutions, e.g. Kuhn/Leduc) existing first.

## Consequences
The UI shows the solver as "Planned". The first solver milestone starts by implementing
Kuhn/Leduc poker against the same traits to validate CFR/CFR+/DCFR and exploitability
computation before any hold'em tree is solved.
