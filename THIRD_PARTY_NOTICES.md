# Third-party notices

## GTOpen
RiverLab's `engine/solver-gtopen` crate depends on GTOpen
(https://github.com/MatthewPDingle/GTOpen). `third_party/gtopen` holds a copy of GTOpen's
`solver` crate (its source, its licence and the data files it compiles in) taken from commit
`eb5ccd671223293527f36d219cfe33baf0b0475c`.

**Modified by RiverLab:** `crates/solver/src/store.rs` (`ArenaBuf` keeps the pointer returned by
`Box::into_raw` and frees it in `Drop`, fixing undefined behaviour found by Miri; the change is
`tools/solver-audit/miri/arenabuf-provenance.patch`). The crate's `Cargo.toml` omits the
`[[example]]` entries, because the examples are not included. Nothing else differs, which
`tools/solver-audit/check-vendored.sh` checks.

GTOpen is licensed under the Apache License, Version 2.0. You may obtain a copy of the
licence at https://www.apache.org/licenses/LICENSE-2.0 or in the `LICENSE` file of the GTOpen
repository at that commit. GTOpen ships no `NOTICE` file at that commit.

## Validation-only tools
`tools/solver-audit/` clones b-inary/postflop-solver (AGPL-3.0-or-later) and
ucsandman/postflop (MIT) into a git-ignored `vendor/` directory for independent cross-checks.
They are not part of the RiverLab application and are not redistributed in this repository.
