#!/usr/bin/env bash
# Runs GTOpen's solver paths that RiverLab uses under Miri (Rust's undefined-behaviour checker).
# See docs/solver-validation.md ("Miri"). Needs: rustup toolchain install nightly --component miri,rust-src
# Takes ~1 h on 4 cores. Works on a scratch copy of the pinned commit; nothing here ships.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ -d vendor/gtopen ]] || git clone -q https://github.com/MatthewPDingle/GTOpen.git vendor/gtopen
cd vendor/gtopen
git checkout -q eb5ccd671223293527f36d219cfe33baf0b0475c -- .
# Miri cannot run mimalloc's C code: fall back to the system allocator under Miri only.
git apply ../../miri/miri-allocator.patch
cp ../../miri/miri_smoke.rs crates/solver/tests/
cd crates/solver
F="-Zmiri-disable-isolation -Zmiri-ignore-leaks" # rayon's global pool is never joined

echo "== 1. As pinned, Stacked Borrows (expected: UB report in ArenaBuf)"
MIRIFLAGS="$F" cargo +nightly miri test --test miri_smoke river || true
echo "== 2. As pinned, Tree Borrows"
MIRIFLAGS="$F -Zmiri-tree-borrows" cargo +nightly miri test --test miri_smoke || true

git -C ../.. apply ../../miri/arenabuf-provenance.patch
echo "== 3. ArenaBuf fixed, Tree Borrows (expected: all pass)"
MIRIFLAGS="$F -Zmiri-tree-borrows" cargo +nightly miri test --test miri_smoke
echo "== 4. ArenaBuf fixed, Stacked Borrows, river + preflop (turn trips a known crossbeam-epoch report)"
MIRIFLAGS="$F" cargo +nightly miri test --test miri_smoke river
MIRIFLAGS="$F" cargo +nightly miri test --test miri_smoke preflop
rm -f tests/miri_smoke.rs && git -C ../.. checkout -q -- .
