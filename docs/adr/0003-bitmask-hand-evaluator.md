# ADR-0003: Own bitmask hand evaluator, validated exhaustively

Status: Accepted (2026-11-22)

## Options considered
1. Third-party crate evaluator — would need a licence + correctness audit and still need our
   own validation; adds a dependency at the heart of the system.
2. Large lookup table (two-plus-two style, ~130 MB) — fastest, but opaque and heavy.
3. Bitmask evaluator (rank masks per suit, popcounts, straight detection by shifts) —
   transparent, allocation-free, ~tens of ns per 7-card hand.

## Decision
Option 3, with exhaustive validation over every 5-card and every 7-card hand and differential
testing against the independent `phevaluator` (Apache-2.0) and `pokerkit` (MIT) evaluators. If profiling later shows the evaluator
is a solver bottleneck, a table-based evaluator can be added behind the same `evaluate7`
signature and validated against this one.
