#!/usr/bin/env bash
# Exhaustive evaluator validation over all 133,784,560 seven-card hands (release build,
# ~30 s including compilation), plus the long-running property tests at higher case counts.
source "$(dirname "$0")/lib.sh"
cd "$ROOT/engine"
say "Exhaustive 7-card enumeration"
cargo test --release -p hand-evaluator --test exhaustive -- --include-ignored
say "Engine property tests (release)"
cargo test --release --workspace --quiet
say "Engine validation passed"
