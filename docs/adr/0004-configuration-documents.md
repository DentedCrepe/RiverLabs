# ADR-0004: Stack and rake configuration as versioned documents

Status: Accepted (2026-11-22)

## Context
Game configurations combine an effective stack depth and a rake structure, which must be
independently configurable (the same rake with many stack depths and vice versa). Solver
results must record the exact configuration that produced them, forever.

## Decision
- The engine owns `StackConfig`, `RakeConfig` and `GameConfig` as serde types with a
  `schema_version`.
- PostgreSQL stores `stack_configs` and `rake_configs` as separate rows (independently
  reusable), each holding the engine-validated JSON document plus indexed scalar columns used
  for querying (e.g. `effective_stack_mbb`, `rake_bps`). `game_configs` references one of each.
- Solver results copy the full resolved configuration JSON (not just foreign keys), so later
  edits to a config row never change the meaning of a stored result.
