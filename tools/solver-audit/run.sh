#!/usr/bin/env bash
# Reproduces docs/solver-comparison.md: clones both third-party solvers at the audited
# commits into tools/solver-audit/vendor/ (git-ignored), runs their own test suites, then
# runs the independent harnesses. Third-party code is built and executed: review first.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p vendor
clone() { # url dir commit
  [[ -d "vendor/$2" ]] || git clone -q "$1" "vendor/$2"
  git -C "vendor/$2" checkout -q "$3"
}
clone https://github.com/ucsandman/postflop.git ucsandman-postflop 5fc7ee3d92b823b6c58e4f58cbee7d50d5e9e6de
clone https://github.com/b-inary/postflop-solver.git b-inary-postflop-solver 9d1509fe5077d019825f833eed04b16d342dfda1

# b-inary ships no lockfile; pin the bincode pre-release it was written against, and relax the
# one lint newer compilers deny (3 sites in action_tree.rs, see the audit).
export RUSTFLAGS="-A dangerous_implicit_autorefs"
(cd vendor/b-inary-postflop-solver && cargo generate-lockfile -q \
  && cargo update -q -p bincode --precise 2.0.0-rc.3 && cargo update -q -p bincode_derive --precise 2.0.0-rc.3)

echo "== ucsandman/postflop own tests =="; (cd vendor/ucsandman-postflop && cargo test --release -q -p engine -p solver-cli)
echo "== b-inary/postflop-solver own tests =="; (cd vendor/b-inary-postflop-solver && cargo test --release -q)

cp vendor/b-inary-postflop-solver/Cargo.lock harness-binary/Cargo.lock
for mode in "" wide turn_nobet turn; do
  echo "== harness: ${mode:-toy river (with and without rake)} =="
  (cd harness-ucsandman && cargo run --release -q -- $mode)
  (cd harness-binary && cargo run --release -q -- $mode)
done
