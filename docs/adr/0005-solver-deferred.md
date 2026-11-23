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

## Amendment (2026-11-23): third-party solver backend
RiverLabs is open source and for private use (the owner and friends), so AGPL obligations
are acceptable: anyone using a hosted instance must be offered the source, which is public.
After running both candidates and checking them independently (`docs/solver-comparison.md`):
- **b-inary/postflop-solver** (AGPL-3.0) is the planned primary heads-up postflop backend,
  used through a RiverLabs-maintained fork pinned to a commit, behind `solver-interface`.
  Adopting it means licensing RiverLabs under AGPL-3.0. That is the owner's decision, still to
  be confirmed.
- **ucsandman/postflop** (MIT) is kept as an independent cross-check in the validation harness.
- The original rule stands: no solver output is shown in the app until our Milestone 2
  validation (independent best response, toy games, exploitability) passes.

## Amendment (2026-11-23, later): GTOpen audited
GTOpen (Apache-2.0) was audited and checked with the same harness. It matches both other
solvers and the hand-derived equilibrium, including under rake, and is the only candidate with
a (modelled) six-max preflop solver. **Recommendation: GTOpen as the primary backend**, with
b-inary/postflop-solver and ucsandman/postflop as independent cross-checks in validation
tooling. This removes the need to license RiverLabs under AGPL. GTOpen's preflop output must
be labelled an approximate model until validated. See `docs/solver-comparison.md`.
