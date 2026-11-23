# Solver comparison: b-inary/postflop-solver, ucsandman/postflop and GTOpen

Run on 2026-11-23. Reproduce with `tools/solver-audit/run.sh`, which clones both projects at
the commits below, runs their own test suites, then runs our harnesses
(`tools/solver-audit/harness-*`). Static audits:
[ucsandman](solver-audit-ucsandman-postflop.md) · [b-inary](#b-inary-static-audit) (below).

| | b-inary/postflop-solver | ucsandman/postflop | GTOpen |
|---|---|---|---|
| Commit | `9d1509f` (2023-10-01) | `5fc7ee3` (2026-09-02) | `eb5ccd6` (2026-11-22) |
| Licence | AGPL-3.0-or-later | MIT | Apache-2.0 |
| History | 281 commits, mostly one author, ~2 years; development suspended | 39 commits, one author, ~37 hours | 1,486 commits, one main author with AI assistance, ~5 months, active |
| Scope | Heads-up postflop | Heads-up postflop | Heads-up postflop; preflop for 2–9 players (modelled, see below); 3-way postflop (beta); optional CUDA |
| Builds on current Rust (1.97) | **No, without changes** (see below) | Yes | Yes (warnings only) |
| Own test suite | 54 passed, 4 ignored (Kuhn, Leduc, game tests) | 160 passed, 3 ignored | 421 passed, **1 failed**, 7 ignored. The failure (`datagen_mix`) is in its training-data pipeline, which needs a published dataset file not in the repo, not in the solver. |

## Independent checks

### 1. Analytically solved river spot
Board K♠Q♦7♥7♣2♠ (no straights or flushes possible). OOP holds 12 AQ combos and can only
check. IP holds 3 KK combos (nuts) and 12 offsuit 43 combos (air), and can check or bet the
pot. Pot 10, stacks 10. No hand shares cards with another, so the textbook equilibrium is
exact.

| | Theory | b-inary | ucsandman | GTOpen |
|---|---|---|---|---|
| KK bet frequency | 100% | 100.00% | 100.00% | 100.00% |
| Air bluff frequency | 12.50% | 12.50% | 12.51% | 12.51% |
| OOP call frequency | 50.00% | 50.01% | 50.01% | 50.01% |
| IP pot share | 3.0000 | 3.0000 | 3.0000 | 3.0000 |

**With 5% uncapped rake.** Theory, with the uncalled bet returned before rake: bluff 10/18.5 × 3 ÷ 12 = 13.51%; call 9.5/19.5 = 48.72%, from IP's air indifference c·(−15) + (1−c)·4.5 = −5.

| | Theory | b-inary | ucsandman | GTOpen |
|---|---|---|---|---|
| Air bluff frequency | 13.51% | 13.52% | 13.52% | 13.52% |
| OOP call frequency | 48.72% | 48.72% | 48.73% | 48.72% |
| IP pot share | — | 2.7769 | 2.7769 | 2.7769 |
| **Reported exploitability** | ≈ 0 | **0.00027** ✅ | **−0.64947** ❌ (true value +0.00069) | **0.00012** ✅ |

ucsandman's strategy is correct under rake, but its exploitability figure is wrong: it
reports `true value − expected rake`. Its "stop at target exploitability" check would end a
raked solve immediately.

### 2. Wide ranges, river
K♠7♦2♣8♥3♦, pot 100, stacks 1000. Ranges are 173 OOP and 251 IP combos (b-inary's example
ranges). OOP may bet 50% pot, IP 75%, no raises.

| | b-inary | ucsandman | GTOpen |
|---|---|---|---|
| OOP pot share | **53.6726** | **53.6726** | **53.6726** |
| Time to converge | 0.02 s | 0.06 s | 0.04 s |

Three independent implementations agree to four decimals. (GTOpen reports values net of each
player's half of the starting pot; half the pot is added back here.)

### 3. Turn spot (turn betting + 44 river cards)
Same ranges, K♠7♦2♣8♥, pot 1000, stacks 10000. Turn and river: OOP bets 50%, IP 75%, no
raises. 500 iterations each.

| | b-inary | ucsandman | GTOpen |
|---|---|---|---|
| OOP pot share | 561.4771 | 561.4789 | 561.4783 |
| Exploitability (summed over both players) | 0.122 | 0.120 | 0.114 |
| Time for 500 iterations | **0.43 s** | 1.34 s | 0.77 s |
| Solver memory | 2.4 MB | 1.8 MB | 1.9 MB |

All differences are well within each solver's exploitability. Without turn betting the three
give 551.6077 / 551.6079 / 551.6078.

## Pitfalls found while setting up identical games
1. **b-inary uses integer chips.** Non-integer bet sizes are rounded (a 75% bet into a pot of
   10 is not 7.5). Use small enough units. RiverLab's milli-big-blind integers can be passed
   straight through, and are fine enough that rounding is negligible.
2. **ucsandman's donk tables are empty by default.** If OOP is not the previous street's
   aggressor, its river lead uses `sizings.oop.river.donk`. Leaving that unset **silently
   removes OOP's lead** from the tree. b-inary's default reuses the normal bet sizes. Our
   adapter must always set donk sizes explicitly.
3. **Exploitability conventions differ.** b-inary reports the per-player average; ucsandman
   reports the sum over both players.
4. **"Uncapped" rake in b-inary needs a large cap.** A cap of 0 means "not raked".

## GTOpen static audit
- **Licence / provenance:** Apache-2.0 (permissive; keep the notice). 1,486 commits since
  2026-06-13; the main author is credited alongside "Claude" commits, so it is heavily
  AI-assisted. About 95k lines of Rust in `crates/` (plus ~550k lines of committed research
  artefacts). Very actively developed, and the API is still moving.
- **Dependencies:** rayon, serde, mimalloc (bundled C allocator); axum/tokio for its server;
  `cudarc` only with the optional `gpu` feature. No `build.rs` in its crates.
- **Postflop algorithm:** standard Discounted CFR (α = 1.5, β = 0, γ = 2), alternating updates.
- **Rake:** a percentage plus a cap (cap 0 = uncapped). With rake on, the game is treated as
  general-sum and exploitability compares best responses with actual values
  (`best_response.rs:269`, `tree.rs:122`). ✅ Verified above.
- **Donk tables:** same convention as ucsandman. OOP leading into the previous street's
  aggressor uses `donk` sizes, which default to empty. A 2026-09 fix stopped this applying after
  check-through streets, but it still applies after bet/call. The adapter must set donk sizes.
- **`unsafe` code:** about 90 sites outside GPU code (cfr.rs 24, multiway 20, preflop 16,
  store 10, …), the same shared-arena pattern as the other two. Not yet run under Miri.
- **Preflop is a model, not a full solve.** In its own words
  (`crates/solver/src/preflop/mod.rs`): it solves the preflop tree "with modeled continuation
  payoffs, not a full postflop game". Heads-up leaves use an equity table with optional learned
  realisation. 3+ player leaves use a showdown-rank approximation. "The 169-class chance model
  ignores joint card removal." And: "a small gap is not evidence of an accurate poker model or
  a Nash equilibrium." RiverLabs must therefore label its preflop output as an **approximate
  model**, never as GTO. This has not been validated by us yet.

## b-inary static audit
- **Licence / provenance:** AGPL-3.0-or-later; 281 commits by essentially one author over two
  years; widely used through desktop-postflop and wasm-postflop. Development suspended
  (October 2023).
- **Build problems on today's toolchain:**
  - No `Cargo.lock`, so `bincode = "2.0.0-rc.3"` now resolves to bincode 2.x final, whose API
    changed. Both `bincode` and `bincode_derive` must be pinned to `2.0.0-rc.3`.
  - Rust has since made `dangerous_implicit_autorefs` deny-by-default. It fires at
    `src/action_tree.rs:393,396,408` (`&*(*node).children[...]` through a raw pointer).
    These are single-threaded tree navigation with valid pointers, so low practical risk, but
    they need an explicit-reference fix in our fork.
  - `zstd` (optional, default-on) compiles bundled C code. It's mainstream, but can be dropped
    if file compression isn't needed.
- **Algorithm:** a modified Discounted CFR (α = 1.5, β = 0, γ = 3, with the average strategy
  restarted at powers of 4). This is a practical variant; convergence is judged by measured
  exploitability, as above.
- **Rake:** `rate` + `cap` applied at showdown and fold terminals. With rake on, exploitability
  correctly uses each player's gain (`utility.rs:279`).
- **`unsafe` code:** 106 sites, including `MutexLike`, an unlocked "mutex" whose own docs call
  it "extremely unsafe". This is the same kind of risk as ucsandman's shared storage. Not yet
  run under Miri.
- **Scope:** heads-up postflop only, same as ucsandman. It merges isomorphic turn/river
  cards and has optional 16-bit compression.

## Conclusion (updated after GTOpen)
All three postflop engines produce the same equilibrium values on every test, so correctness
does not separate them. The choice comes down to licence, scope, maintenance and speed:

| | b-inary | ucsandman | GTOpen |
|---|---|---|---|
| Correct exploitability under rake | ✅ | ❌ | ✅ |
| Licence for RiverLabs | AGPL (RiverLabs must be AGPL) | MIT | Apache-2.0 |
| Maintained | No (since 2023) | Yes (new) | Yes (very active) |
| Builds as-is | No | Yes | Yes |
| Speed (turn test) | Fastest | Slowest | Middle |
| Preflop / multiway | No | No | Yes (modelled; 3-way postflop beta) |

**Recommended:** **GTOpen as the primary backend.** It is permissive, maintained, builds as-is,
is correct under rake, and is the only option with a six-max preflop path. Pin a commit and
wrap it in the adapter. Treat its preflop output as an approximate model until RiverLabs
validates it. **b-inary** becomes the independent cross-check for postflop. Keeping it in the
validation tooling only, and out of the shipped application, should avoid making RiverLabs
itself AGPL. That is a judgement, not legal advice. ucsandman stays a third cross-check once
its rake bug is patched.

Risks with GTOpen: one main author, heavy AI assistance, a fast-moving API, and its preflop
model's accuracy is unproven. Mitigations: pin commits, keep the cross-checks, and validate
preflop independently (e.g. heads-up push/fold against an exact equilibrium we compute
ourselves).

## Conclusion before GTOpen was audited (superseded)
- **Primary backend: b-inary/postflop-solver**, via a RiverLabs-maintained fork. Fix the three
  autoref sites, commit a lockfile, keep pinned to a commit, and record the commit hash with
  every result. It is the more mature engine, 2.6–3× faster here, and correct under rake.
  Using it makes RiverLabs AGPL-3.0, which is acceptable for an open, privately used project.
- **Secondary: ucsandman/postflop** as an independent cross-check in the validation harness.
  Two independent implementations agreeing on a spot's value is a strong correctness signal.
  Its rake exploitability defect must be patched (or rake-free values compared) before using
  it that way.
- **Still to do before any solver output is shown in the app:**
  - our own independent best-response check (Milestone 2);
  - Miri on both `unsafe` cores;
  - flop-sized memory and performance tests;
  - an adapter from RiverLabs' `SolveSpec`.
  
  Neither solver handles preflop or multiway spots.
