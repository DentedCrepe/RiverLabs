# Audit: ucsandman/postflop

- **Repository:** https://github.com/ucsandman/postflop
- **Audited commit:** `5fc7ee3d92b823b6c58e4f58cbee7d50d5e9e6de` (2026-09-02)
- **Audit date:** 2026-11-22
- **Scope:** static review of the engine crate: the CFR core, best response, the showdown/fold
  payoffs, chance weights, the tree and rake. **Not yet done:** building the code, running
  its tests, and independent numerical validation. Executing third-party code needs the
  project owner's approval in this environment (see "Next steps").

## Summary verdict
**Conditionally suitable as RiverLab's first postflop solver backend, behind our solver
interface. It is not ready to adopt as-is.**

The core algorithm is a careful, textbook implementation of vector-form Discounted CFR.
Adoption is blocked until three things are resolved:
1. the rake/exploitability defect below is fixed;
2. its tests pass when we run them;
3. its results pass RiverLab's independent validation harness (Milestone 2).

## Provenance and maintenance
| Item | Finding |
|---|---|
| Licence | MIT (© 2026 Wes Sander). Permissive; keep the notice if any code is used. |
| History | 39 commits, single author, all within about 37 hours (2026-09-01 02:39 → 2026-09-02 15:47). |
| Development style | Commit messages ("untrack AI artifacts", "Fix all 11 adversarial-review findings") suggest heavily AI-assisted development. No track record of real-world use. |
| Size | About 14.5k lines of Rust (engine + CLI); also WASM bindings, a web UI and marketing site (out of scope). |
| Dependencies | rayon, serde, serde_json, toml, clap (all mainstream); rand in dev only. No `build.rs`, no proc-macro crates of its own. Low supply-chain risk. |

## Capabilities
- **Game:** heads-up no-limit hold'em, **postflop only** (flop, turn or river roots).
  No preflop, no multiway (`examples/multiway_spike.rs` is an experiment).
- **Algorithm:** vector-form Discounted CFR (α = 1.5, β = 0, γ = 2 by default), alternating
  updates, an optional CFR+-style regret floor, full traversal (no sampling except an opt-in
  approximate `turn_chance_sampling` mode).
- **Bet abstraction:** per-street bet sizes, raise cap and all-in threshold
  (`BetSizings`).
- **Other features:** node locking, ICM payoffs, optional i16-compressed storage,
  bit-deterministic parallelism, solution export.
- **Rake:** a percentage and a cap only (`config.rs:186`). No rounding increment and no
  minimum pot. No-flop-no-drop doesn't apply postflop.
- **Units:** chip amounts are `f64`/`f32` floats compared with an epsilon. RiverLab uses
  integer milli-big-blinds, so the adapter must convert at the boundary.
- **Isomorphism:** flop-level only (22,100 flops → 1,755 classes for aggregate reports).
  **Isomorphic turn and river cards are not merged inside a solve**: every runout card gets
  its own subtree (`tree.rs:672`), so memory per solve is higher than in solvers that merge
  them.

## Correctness review (static)
| Area | Location | Finding |
|---|---|---|
| DCFR discounting | `cfr.rs:640` | Matches Brown & Sandholm (2019): positive regrets × tᵅ/(tᵅ+1), negative × tᵝ/(tᵝ+1), strategy sum × (t/(t+1))ᵞ, once per full iteration. ✅ |
| Regret / average-strategy update | `cfr.rs:516` | Regret = action counterfactual value − node value. Strategy sum weighted by own reach × σ. Standard. ✅ |
| Showdown payoff with card removal | `terminal.rs:199` | O(N+M) rank sweep with inclusion–exclusion. I traced the add-back terms, including the identical-hand case. Card-removal sums are `f64`. ✅ |
| Fold payoff with card removal | `terminal.rs:101` | Same inclusion–exclusion. ✅ |
| Chance-node weights | `nlhe.rs:373` | Conditional deal weight `1/(outcomes − 4)`, correct given that both hands are removed from the deck. ✅ |
| Best response | `br.rs` | Separate walk with a per-combo maximum, not a by-product of CFR. It does share the game/terminal/evaluator code, so it **cannot** catch bugs in those. ⚠️ |
| **Exploitability under rake** | `nlhe.rs:777`, `br.rs:126` | **Defect.** `zero_sum()` returns `true` whenever there is no ICM, including raked games. Raked utilities sum to −rake(terminal), which varies by terminal, so the game is not constant-sum. The reported value `BR₀ + BR₁` then equals the true exploitability minus the expected rake under the profile. It **under-reports** distance from equilibrium (and can go negative), and `target_exploitability` can stop raked solves early. Likely fix: treat raked games as non-zero-sum (`icm.is_none() && rake.percent == 0`) so the existing NashConv path is used. **Must be fixed and verified before use with RiverLab's default 5% rake.** ❌ |
| `unsafe` in parallel storage | `cfr.rs:320–430` | Raw pointers hand out `&'static mut` per-node slices to rayon tasks. The safety argument (one subtree per chance outcome, node-disjoint writes) holds for the current tree builder (no in-tree isomorphism), but it is a **fragile invariant**: adding runout isomorphism would make it unsound. Run under `cargo miri` (sequential feature) and keep a test asserting subtree disjointness. ⚠️ |
| Float precision | throughout | Regrets and strategy sums are `f32`. DCFR discounting limits growth, but long solves should be checked against an `f64` reference on small spots. ⚠️ |
| Tests (claimed) | `engine/tests`, unit tests | The README claims Kuhn (game value −1/18 ✅ textbook), AKQ closed forms, river indifference checks, bit-identical results across thread counts, and fingerprint locks. **Not yet run by us.** |

## Fit with RiverLab
| Requirement | Fit |
|---|---|
| Six-max preflop | ❌ Not supported. Needs a separate solution (Milestone 5). |
| Heads-up postflop spots from six-max | ✅ This is the common case (single-raised and 3-bet pots). |
| Multiway postflop | ❌ |
| Our rake model | Partial: % + cap map directly. Rounding and minimum pot aren't modelled, so results are approximate for those settings and must be labelled. |
| Exploitability reported with every result | ✅ once the rake defect is fixed |
| Replaceable behind `solver-interface` | ✅ An adapter maps `SolveSpec` → its `SolveConfig` and its average strategy → our result format. Its `Game`/`Solver` shapes are close to our traits. |

## Recommendation and integration plan
1. **Run** its test suite at the pinned commit (blocked on approval), including
   `cargo test --release` and `cargo miri test` on the sequential build.
2. **Fix** the rake exploitability defect (patch in a fork, offer it upstream) and add a
   regression test: on a raked spot, NashConv at convergence must approach 0.
3. **Validate independently** in RiverLab's Milestone 2 harness, using RiverLab's own
   evaluator and an independent best response:
   - Kuhn and Leduc game values and exploitability converging to 0;
   - river spots with one bet size, where exploitability is computed by our own
     best-response code from its exported strategies;
   - cross-check DCFR against noambrown/poker_solver (MIT) on river spots.
4. **Integrate** as a dependency pinned to an exact commit of a RiverLab-controlled fork,
   wrapped by a `solver-postflop` adapter crate that implements `solver-interface`. Record
   `solver_backend = "ucsandman-postflop"` and the commit hash with every result.
5. Only after steps 1–3 pass may the UI show its output, labelled with its exploitability.
