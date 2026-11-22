# ADR-0001: Rust engine is the single source of truth for poker mathematics

Status: Accepted (2026-11-22)

## Context
Poker logic (evaluation, ranges, betting rules, pots, rake) is needed by the API now and by the
solver and trainer later. Implementing it in both Python and Rust would create two
implementations that drift apart.

## Decision
All poker mathematics lives in the `engine/` Rust workspace. Python reaches it through a PyO3
extension module (`riverlab_engine`, built with maturin). The backend contains no poker rules.

## Consequences
+ One implementation to validate; solver work later reuses the same evaluator and game rules.
+ Performance headroom for equity enumeration and solving.
- The backend build requires a Rust toolchain (handled by a multi-stage Docker build and `uv`
  building the path dependency).
- Configuration objects cross the boundary as JSON (serde ↔ Pydantic). Pydantic validates shape;
  the engine validates semantics, so there is exactly one semantic validator.
