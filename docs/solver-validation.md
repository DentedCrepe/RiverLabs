# Solver validation (Milestone 2)

RiverLab does not trust any solver's self-reported results. `engine/solver-validation`
re-computes, for **any** solver's strategy profile on a postflop spot (river, turn or flop root,
with every remaining card dealt by chance nodes):
- each player's expected value,
- each player's best-response value,
- exploitability (`NashConv / 2`).

It uses only RiverLab's own evaluator and combo code (exhaustively validated; see
`hand-evaluator.md`), so it shares no code with the solver under test. Rules: uncalled bets are
returned before rake; rake is `min(pot × rate, cap)`. At a chance node each card is weighted
`1/(52 − board − 4)` and hands holding it are removed below it. That is exact, because every
pair of non-overlapping hands is compatible with exactly that many cards.

## The validator's own tests
| Test | Result |
|---|---|
| Hand-solved equilibrium (pot-size bet, polarised vs bluff-catcher) | exploitability exactly 0 |
| Same spot with 5% rake (bluff 10/18.5 per value combo, call 9.5/19.5) | exploitability exactly 0 |
| OOP always calls → IP's gain from never bluffing | exactly 1.0 chip, as derived by hand |
| IP never bluffs → OOP's gain from always folding | exactly 1.0 chip |
| Uncalled bet returned before rake | IP nets 4.5 when OOP folds to a 10-chip bet into 10 |
| Turn with no betting, weighted ranges and specific combos, two boards | values equal `pot × equity − stake` from the equity engine to 1e-9 |
| Chance node missing a card / showdown before the river | rejected |
| Malformed profiles (probabilities not summing to 1, duplicate combos) | rejected |

## GTOpen through the adapter (`engine/solver-gtopen`)
`cargo run --release -p solver-gtopen --example validate_river`, 3,000 DCFR iterations:

| Spot | Values (mbb, OOP / IP): GTOpen | RiverLab check | Exploitability (mbb): GTOpen | RiverLab check |
|---|---|---|---|---|
| Hand-solved river, no rake | 2000.000 / −2000.000 | 2000.000 / −2000.000 | 0.2528 | 0.2527 |
| Hand-solved river, 5% rake | 1573.003 / −2223.104 | 1573.003 / −2223.104 | 0.3732 | 0.3732 |
| Wide river (173 v 251 combos), no rake | 3672.574 / −3672.574 | 3672.574 / −3672.574 | 0.0250 | 0.0251 |
| Wide river, 5% rake | −279.530 / −7274.065 | −279.530 / −7274.065 | 0.0326 | 0.0327 |

Turn spots (OOP bets 50% / IP 75% on turn and river; wide ranges; pot 100bb, stacks 1000bb;
1,000 iterations):

| Spot | Values (mbb, OOP / IP): GTOpen | RiverLab check | Exploitability (mbb): GTOpen | RiverLab check |
|---|---|---|---|---|
| Ks7d2c8h (44 river cards), no rake | 6147.755 / −6147.754 | 6147.755 / −6147.755 | 1.7034 | 1.7034 |
| Ks7d2c8h, 5% rake | 858.265 / −10878.801 | 858.265 / −10878.801 | 1.6932 | 1.6932 |
| Ks7s2d8d (suit-isomorphic runouts GTOpen copies with suits relabelled) | 4557.233 / −4557.232 | 4557.233 / −4557.233 | 0.8069 | 0.8069 |

A flop spot (Ks7d2c, small ranges, bets on every street) is also checked, through both turn and
river chance layers. The checks are not vacuous: corrupting a single river decision in the
exported turn tree is detected as a clear rise in exploitability.

The hand-solved value (IP −2bb) and the wide-river value (OOP +3.6726bb, agreed by three
independent solvers) also match. These run as automated tests in
`engine/solver-gtopen/tests/validation.rs` on every `scripts/test.sh`.

## GTOpen preflop: heads-up push/fold in the exact game
Ground truth (`solver-validation::push_fold`) works on all 1,326 combos with full card
removal. Every pairwise all-in equity comes from exact enumeration of all 1,712,304 boards with
RiverLab's evaluator; a fast enumerator is checked against the general equity engine to 1e-12.
The table takes ~20 minutes on 4 cores; its A♠A♥ vs K♠K♥ entry is 0.826366, matching the
phevaluator-validated value. The exact game is solved with CFR+ and verified by exact best
response. GTOpen's class-level strategies are then scored in that exact game.
Reproduce: `cargo run --release -p solver-validation --example build_hu_equity`, then
`cargo run --release -p solver-gtopen --example validate_push_fold`.

| Stack | Strategy | SB jams | BB calls | SB EV (bb) | Exploitability in exact game (bb/hand) |
|---|---|---|---|---|---|
| 10bb | Exact (RiverLab) | 58.3% | 37.4% | −0.04537 | 0.000000 |
| | GTOpen, blocker model on | 58.6% | 37.3% | −0.04536 | 0.00002 |
| | GTOpen, blocker model off | 58.2% | 37.0% | −0.04570 | 0.00079 |
| 20bb | Exact (RiverLab) | 40.2% | 21.7% | −0.18319 | 0.000000 |
| | GTOpen, blocker model on | 40.0% | 21.7% | −0.18319 | 0.00000 |
| | GTOpen, blocker model off | 40.0% | 20.7% | −0.18340 | 0.00153 |

With its blocker model (`card_removal`) on, GTOpen's push/fold is essentially exact. The only
classes differing by more than 25 points (43s at 10bb, Q5s at 20bb) are near-indifferent mixes.
With it off, a few marginal classes flip and the strategy is measurably exploitable.
**RiverLab must run GTOpen preflop with `card_removal` on.** This validates only the all-in
part of GTOpen's preflop. Leaves that continue to the flop use GTOpen's modelled values and
remain unvalidated.

## Fast evaluator and full-range flops
`evaluate` computes the same quantities with per-card inclusion–exclusion (each hand's
compatible opponent weight is `total − card[a] − card[b] + identical combo`), a strength-sorted
sweep at showdowns, and chance subtrees in parallel. The original O(n·m) implementation is
kept as `evaluate_reference`, and a randomised test requires both to agree to 1e-9 on generated
turn and river games. Those games use overlapping weighted ranges, random trees mixing folds,
showdowns and chance nodes, and random strategies, with and without rake.

Full-range flop spots (173 v 251 combos; OOP bets 33%, IP 75% on every street; pot 6.5bb,
stacks 97.5bb; 300 iterations; `cargo run --release -p solver-gtopen --example validate_flop`):

| Spot | Tree | Values (mbb): GTOpen = RiverLab | Exploitability: GTOpen / RiverLab | Solve | Check |
|---|---|---|---|---|---|
| Ks7d2c, no rake | 191,844 nodes, 313 MB | 541.634 / −541.634 | 2.1933 / 2.1933 | 54 s | 2.1 s |
| Ks7d2c, 5% rake, one raise size | 716,346 nodes, 1.2 GB | −195.514 / −644.791 | 6.0572 / 6.0572 | 189 s | 8.2 s |
| Qh8h3h (monotone; suit-isomorphic runouts) | 191,844 nodes, 313 MB | 251.928 / −251.928 | 0.9501 / 0.9501 | 15 s | 2.0 s |

Every solve in the app is now re-checked automatically. The check is skipped, with an explicit
note, only when the exported tree (about the size of GTOpen's own arenas) would not fit in the
memory limit alongside the solve.

## Node locking
A solve can fix one or more decisions on its first street to range-wide action frequencies
(`SolveSpec::locks`). RiverLab first solves the spot unlocked, turns the solved strategy at each
locked decision into one with the requested frequencies (GTOpen's range lock: each hand's
frequencies are scaled from the solved ones, so hands keep their relative preferences), then
solves again **from scratch** with only the locks carried over, so the averages hold no
pre-lock iterations.

What is checked, and how:
- **Exploitability is constrained.** GTOpen's best response cannot deviate at a locked node, and
  RiverLab's validator does the same (`PostflopGame::locked`): the figure measures how far the
  rest of the strategy is from answering the locks, not how exploitable the locks are.
- **Tests** (`solver-gtopen/tests/validation.rs`, `solver-validation`): on the hand-solved river
  spot, a 50% bet lock is reproduced exactly across the range; GTOpen and the independent
  validator agree on values and on the constrained exploitability; the out-of-position player
  gains against the over-bluffing lock; and the same profile with the lock not marked is
  clearly exploitable, so the check is not vacuous.
- A locked solve's result is labelled "an exploit, not an equilibrium", shows the unlocked
  solve's values beside its own, and is never used as a drill answer key.

## Miri (undefined-behaviour check) on GTOpen
Run on 2026-11-23 with nightly Miri; reproduce with `tools/solver-audit/miri/run-miri.sh`.
The test (`tools/solver-audit/miri/miri_smoke.rs`) covers the GTOpen paths RiverLab uses, on
games small enough for Miri:
- a raked river solve with DCFR;
- a turn solve with chance nodes and CFR+;
- exploitability, and save and load.

GTOpen's mimalloc allocator is switched off under Miri only, because Miri cannot run C code.

| Run | Result |
|---|---|
| As pinned, Stacked Borrows | ❌ Undefined behaviour in `ArenaBuf` (`crates/solver/src/store.rs`) |
| As pinned, Tree Borrows | ❌ The same `ArenaBuf` report |
| `ArenaBuf` fixed, Tree Borrows, river + turn | ✅ No undefined behaviour |
| `ArenaBuf` fixed, Stacked Borrows, river | ✅ No undefined behaviour |
| `ArenaBuf` fixed, Stacked Borrows, turn | Stops in `crossbeam-epoch` 0.9.18 (rayon's dependency), not in GTOpen |
| Preflop (heads-up push/fold, blocker model) | **Not completed.** Stopped after the 2-hour limit with no error reported up to then |

**The finding.** `ArenaBuf::new` takes a raw pointer into a `Box<[T]>` and then moves the `Box`
into the struct. Moving a `Box` asserts unique ownership, which invalidates the earlier pointer
under both of Rust's proposed aliasing models. Every later write through that pointer (all of
CFR's regret and strategy storage) is therefore undefined behaviour. The fix,
`tools/solver-audit/miri/arenabuf-provenance.patch`, keeps only the pointer from
`Box::into_raw` and frees it in `Drop`; GTOpen's own `store` tests pass with it.

**Practical risk.** Today's compiler produced correct results in every check on this page, so
the bug had not shown up as wrong output. It could have started to on a future compiler.

**Fixed in RiverLab (2026-11-24).** RiverLab now builds from `third_party/gtopen`: GTOpen's
`solver` crate at the audited commit with this one patch and nothing else
(`tools/solver-audit/check-vendored.sh` proves it against a fresh clone). Results record the
solver as `eb5ccd67…+arenabuf-provenance`, so they can be told apart from earlier ones. With the
patched build, every figure in the GTOpen tables above (river, turn, push/fold at 10bb and 20bb,
and all three flops) was reproduced exactly, and GTOpen's own `store` tests pass. The Miri runs
of the fixed code are the rows marked "`ArenaBuf` fixed" above. The patch has not yet been
offered upstream.

One of GTOpen's storage unit tests (`i16_roundtrip_accuracy`) also fails under Miri. Miri
deliberately perturbs the results of functions such as `sin`, and the test's tolerance is
tighter than one float step. That is a test tolerance issue, not a memory-safety one.

## ICM push/fold for 3 to 6 players (`play_engine::icm_push_fold`)

RiverLab's own solver for tournament push/fold, used by the ICM drills and the sit & go hand
review. It is a **modelled approximation** (all-in or fold first in, call or fold facing an
all-in, the first call ends the betting, folded players' cards ignored, ICM payoffs), and it is
labelled that way wherever it is shown. What is checked (`engine/play-engine/tests/icm_push_fold.rs`,
with the exact equity table):
- **Two players** (winner takes all, so ICM is linear in chips): the model is heads-up push/fold
  and matches the exact solver above: the small blind's value agrees within 2e-5 of the chips in
  play, and the overall jam and call shares within 0.3 points (single hands near indifference
  can differ, as any two near-equilibria can).
- **Simulation:** for 3- and 4-player spots the solved profile is played out with real cards
  (every hand dealt, actions sampled from the profile, a board dealt, the showdown evaluated,
  the stacks valued with ICM). The model's value for every player agrees within 0.002 of the
  prize pool (measured gaps 0.0003 or less with 60,000 deals), which also bounds the effect of
  ignoring the folded players' cards.
- **Epsilon:** with three or more players CFR+ has no convergence guarantee, so every solve
  measures each player's best-response gain and reports the largest (in % of the prize pool).
  Drill spots use 150 iterations; measured values are under 0.001% for 3- to 6-player spots.
- **Conservation and ICM:** values add up to the prizes in play; ICM itself is checked against
  every finishing order (`poker_theory::icm`).

Not covered: overcalls and re-shoves, limps, and any non-all-in preflop play. The model is not
an equilibrium of real tournament poker.

## What is not validated yet
- GTOpen's preflop leaves that continue postflop (modelled values; see `solver-comparison.md`).
  Push/fold (all-in) is validated above.
- Kuhn/Leduc-style toy games for future in-house CFR variants.
- Miri on GTOpen's preflop solver (the run did not finish; see "Miri" above), and on its
  multithreaded paths beyond the `crossbeam-epoch` report.

The app labels results accordingly: postflop results show their exploitability (re-checked
independently), and preflop results beyond heads-up push/fold carry an "Approximate model"
banner.
