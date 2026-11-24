#!/usr/bin/env bash
# Checks that third_party/gtopen is exactly GTOpen at the audited commit plus RiverLab's patches
# (tools/solver-audit/miri/arenabuf-provenance.patch), and nothing else. Run it after any change
# to third_party/gtopen, and when moving to a new GTOpen commit (with run.sh and the validation
# examples in docs/solver-validation.md).
set -euo pipefail
cd "$(dirname "$0")"
COMMIT=eb5ccd671223293527f36d219cfe33baf0b0475c
VENDORED=../../third_party/gtopen
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git clone -q https://github.com/MatthewPDingle/GTOpen.git "$tmp/gtopen"
git -C "$tmp/gtopen" checkout -q "$COMMIT"
git -C "$tmp/gtopen" apply "$PWD/miri/arenabuf-provenance.patch"
status=0
# Source, licence and the data files the source compiles in.
diff -r "$tmp/gtopen/crates/solver/src" "$VENDORED/crates/solver/src" || status=1
for f in LICENSE tools/datagen/mixes.json cache/preflop_eq169.bin cache/contextual/ignition-nl10-reraise-v1.json \
  research/ignition-reraise/examples.json research/preflop-evolution/behavior/pass3/evidence-metadata.json; do
  cmp "$tmp/gtopen/$f" "$VENDORED/$f" || status=1
done
# The manifest differs only by dropping the [[example]] entries (the examples are not vendored).
manifest() { tr -d '\r' < "$1" | grep -v -e '^\[\[example\]\]' -e '^required-features' -e '^[[:space:]]*$'; }
diff <(manifest "$tmp/gtopen/crates/solver/Cargo.toml" | grep -v '^name = "\(learned\|convergence\|balanced\|range_value\|continuation\|integrated\|entry\|hu_\|large\)') \
  <(manifest "$VENDORED/crates/solver/Cargo.toml") || status=1
if [[ $status == 0 ]]; then echo "third_party/gtopen = GTOpen $COMMIT + arenabuf-provenance.patch"; fi
exit $status
