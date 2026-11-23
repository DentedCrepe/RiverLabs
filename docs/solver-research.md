# Third-party solver audit

Research date: 2026-11-22. Method: shallow clones of HEAD; LICENSE, README and main sources read.
**No third-party code was built, run or copied into RiverLab.** Capability claims marked
"(README)" are the authors' claims and were not tested by us.

## Verified facts

| Candidate | Repository | Licence | Language | Activity / status |
|---|---|---|---|---|
| "Pokeria" | github.com/cbaezp/Pokeria | MIT (solver dir © Wes Sander) | Python/TS/Rust | active; solver is a modified vendored copy of ucsandman/postflop |
| ucsandman/postflop | github.com/ucsandman/postflop | MIT | Rust | active, ~3 months old, single author |
| cRITlamb/poker_solver | github.com/cRITlamb/poker_solver | MIT (copyright holder ≠ committer) | Rust + Python | low activity; comments say parts follow AGPL postflop-solver patterns |
| Noam Brown | github.com/noambrown/poker_solver | MIT | Python + C++ | quiet; river subgames + Kuhn/Leduc; no committed tests found |
| b-inary/postflop-solver (+ wasm/desktop) | github.com/b-inary/postflop-solver | **AGPL-3.0** | Rust | development suspended (Oct 2023) |
| bupticybee/TexasSolver | github.com/bupticybee/TexasSolver | **AGPL-3.0** (commercial licence sold for network use) | C++ | maintained; minimal tests |
| krukah/robopoker | github.com/krukah/robopoker | MIT | Rust | active; MCCFR blueprint + abstraction (not audited in depth) |

Capabilities (README / source headers):
- ucsandman/postflop: heads-up postflop NLHE only; vector-form DCFR; suit isomorphism; separate
  best-response exploitability; README claims validation on Kuhn/AKQ closed forms.
- cRITlamb/poker_solver: heads-up preflop blueprint + postflop; DCFR; Kuhn/Leduc; differential
  tests against a Python reference.
- noambrown/poker_solver: CFR, CFR+, ES-MCCFR, fictitious play, DCFR; river-only NLHE.
- b-inary/postflop-solver: postflop DCFR, isomorphic chance nodes, i16 compression,
  `compute_exploitability`, Kuhn/Leduc tests.
- TexasSolver: postflop CFR+ / DCFR, best response.

Python evaluators considered as test oracles: `phevaluator` 0.6.0 (Apache-2.0, maintained,
declares 3.10–3.14), `pokerkit` 0.7.8 (MIT, maintained), `treys` 0.1.8 (MIT, unmaintained since
2023). Rust evaluator crates (`poker`, `aya_poker`, `rust_poker`) exist but were not needed.

## Decisions (judgement)
- **Update 2026-11-23:** RiverLabs is open source and used privately, so AGPL is
  acceptable. **b-inary/postflop-solver is the recommended primary backend** (via a
  maintained fork), with ucsandman/postflop as an independent cross-check. See
  [solver-comparison.md](solver-comparison.md) and ADR-0005.
- **TexasSolver:** still not adopted; its author asks for a commercial licence for
  network use, and b-inary covers the same ground in Rust.
- *(Superseded)* AGPL projects (b-inary, TexasSolver): reference for published algorithms only;
  no code. AGPL would force release of the whole hosted service.
- **ucsandman/postflop: strongest MIT candidate.** Deep static audit in
  [`solver-audit-ucsandman-postflop.md`](solver-audit-ucsandman-postflop.md): a sound DCFR core,
  but it misreports exploitability under rake. Conditionally suitable as the first heads-up
  postflop backend once its tests run, that defect is fixed, and it passes our validation
  harness.
- **Pokeria: reject as a source** (derivative; use upstream if anything).
- **cRITlamb/poker_solver: reference only** (provenance risk).
- **noambrown/poker_solver: reference only** — useful to cross-check CFR-variant results on
  Kuhn/Leduc during Milestone 2.
- **Test oracles: `phevaluator` (primary) and `pokerkit` (secondary).** Used in
  `tests/validation/` for differential testing of our evaluator.
