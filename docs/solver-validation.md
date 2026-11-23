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

## What is not validated yet
- GTOpen's preflop leaves that continue postflop (modelled values; see `solver-comparison.md`).
  Push/fold (all-in) is validated above.
- Kuhn/Leduc-style toy games for future in-house CFR variants.
- Miri on GTOpen's `unsafe` code.

The app labels results accordingly: postflop results show their exploitability (re-checked
independently), and preflop results beyond heads-up push/fold carry an "Approximate model"
banner.
