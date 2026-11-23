# Third-party notices

## GTOpen
RiverLab's `engine/solver-gtopen` crate depends on GTOpen
(https://github.com/MatthewPDingle/GTOpen), pinned to commit
`eb5ccd671223293527f36d219cfe33baf0b0475c`, used unmodified as a Cargo dependency.

GTOpen is licensed under the Apache License, Version 2.0. You may obtain a copy of the
licence at https://www.apache.org/licenses/LICENSE-2.0 or in the `LICENSE` file of the GTOpen
repository at that commit. GTOpen ships no `NOTICE` file at that commit.

## Validation-only tools
`tools/solver-audit/` clones b-inary/postflop-solver (AGPL-3.0-or-later) and
ucsandman/postflop (MIT) into a git-ignored `vendor/` directory for independent cross-checks.
They are not part of the RiverLab application and are not redistributed in this repository.
