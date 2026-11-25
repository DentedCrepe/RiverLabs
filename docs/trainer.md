# Trainer

The Trainer grades decisions only against real solutions, never invented answers. Two kinds of
drill exist: **heads-up push/fold**, graded against RiverLab's exact equilibrium, and **postflop
drills**, dealt from your own saved solves and graded by the EV the solve gives each action.

## Push/fold drills
- **Game:** heads-up, blinds 0.5/1, equal stacks of 2bb to 25bb (steps of 0.5bb), no ante, no
  rake. The small blind (the button) jams or folds; the big blind calls or folds against a jam.
  This is the scope validated in `docs/solver-validation.md`; antes, rake and other sizes are
  not offered.
- **Deal:** a stack depth from your range, a seat (or both), and one of the 1,326 hands, chosen
  uniformly at random by the engine.
- **Reference:** `solver_validation::push_fold`, the exact game on all 1,326 combos with full card
  removal. All-in equities are exact (every one of the 1,712,304 boards per pair). Each depth is
  solved with 1,000 CFR+ iterations on first use and kept in memory (about 2 seconds on four
  cores). Its exploitability, measured by exact best response, is shown with every answer; at 10bb
  it is below 0.000001bb per hand. The 10bb and 20bb solutions reproduce the validation table.

## Grading
For your exact hand, the engine computes the expected value of both actions against the opponent
playing the equilibrium, in bb from the start of the hand with blinds included: folding is −0.5bb
in the small blind and −1bb in the big blind facing a jam. Your **EV loss** is the best action's
EV minus yours.

| Verdict | EV loss |
|---|---|
| Correct | at most 0.001bb (actions this close are treated as equal, which covers hands the equilibrium mixes) |
| Close | at most 0.05bb |
| Mistake | more than 0.05bb |

The thresholds are `EQUAL_BB` and `CLOSE_BB` in `engine/play-engine/src/push_fold.rs`. The
answer also shows how often the equilibrium plays each action with your hand, and a chart of
the whole equilibrium by hand class. The session tally (hands, verdicts, average EV lost) lives
in the page; every graded answer is also stored for Analytics (`docs/analytics.md`).

EV loss is measured against an opponent who plays the equilibrium. Against a different opponent
another action can be better; that is exploitation, which these drills do not grade.

**Tournaments and ICM.** The drill is heads-up. With two players left, ICM (docs/play.md) is
linear in chips (your value is the 2nd prize plus your chip share of the difference), so the
chip-EV equilibrium is also the ICM one. The drill does not model three or more players left.

## The equity table
Grading needs a 7 MB table of every hand-against-hand all-in equity. It is stored as
`<RIVERLAB_RESULT_STORAGE_DIR>/hu_combo_equity.bin` and loaded the first time the Trainer page is
opened. Before use, RiverLab checks it: every entry is in [0, 1], and `NaN` exactly where two
hands share a card; each pair's two entries sum to 1; and 200 entries are re-computed exactly. A
table that fails is rejected with the reason.

If the table is missing, the Trainer page can build it (about 20 minutes on four cores; longer
on a phone). The Android setup downloads a pre-built copy from the `android-web` branch instead.
It is the same table, checked the same way when loaded. Build one yourself with
`cargo run --release -p solver-validation --example build_hu_equity -- <path>`.

## Postflop drills
Postflop drills come from postflop solves you have already run and saved (Solver page, "Solve
this spot", or a solver review of a hand). Choose one solve or any, and a position or either.
Implementation: `engine/play-engine/src/drill.rs`.

- **Deal:** a hand is drawn for each player from their starting range in the solve (by weight,
  without shared cards). The hand is then played forward: each decision follows the solver's
  strategy for that player's hand, and turn and river cards are dealt at random from the cards
  left. At each of your decisions on the way the deal stops with probability one half; that
  decision is the drill. Spots therefore come up about as often as they would when both players
  follow the solution, and early decisions come up more often than late ones.
- **Shown:** the table (pot, bets on this street, stacks, the board, your cards, the actions so
  far) and one button per action in the solve's tree.
- **Graded by EV:** the engine computes the EV of each action for your exact hand, against your
  opponent's range at that point, with both players following the solution afterwards. EV lost is
  the best action's EV minus yours.

| Verdict | EV lost |
|---|---|
| Correct | at most 1% of the solve's starting pot |
| Close | at most 5% of the starting pot |
| Mistake | more than 5% of the starting pot |

The thresholds are `CORRECT_PCT_POT` and `CLOSE_PCT_POT` in `drill.rs`. The answer also shows the
solver's mix for your hand, and the solve's exploitability. A solve is an approximate equilibrium
for its own bet sizes and starting ranges; differences between actions smaller than its
exploitability are within its error, which is why near-equal options all count as correct or
close. Every answer is stored for Analytics (`docs/analytics.md`).

## Preflop drills
The **Preflop** tab deals decisions from your saved preflop solves (Preflop page, `kind =
"preflop"` jobs). Those solves are **GTOpen's preflop model, an approximate model**: pots that
reach the flop are valued by a model, not played out, and RiverLab has validated only heads-up
push/fold without ante or rake. Every drill and grade carries that label (the same banner as the
Preflop page), the model's notes and its own best-response gap.

**Dealing** (`play_engine::preflop_drill::deal`, seeded):
- *Open or fold*: a position other than the big blind is picked at random and everyone before it
  folds;
- *Facing a raise*: a first-in raiser is picked at random, its raise size is drawn from how often
  the model uses each size, and everyone between it and a randomly picked later player folds;
- *Any decision*: the tree is walked from the start, each action drawn from the model's own
  frequencies, stopping at each decision with probability 0.35.

Decisions the model never reaches, or has no strategy for, are skipped. Your hand is drawn from
the model's range at the decision, every combo in it equally likely, so hands you would never
hold there are not dealt.

**Grading** (`play_engine::preflop_drill::grade`). The preflop model gives each hand class's
strategy but no value for each action, so preflop drills are **not graded by EV**. Instead:

| Verdict | Rule |
|---|---|
| Matches the model | your action is the one the model takes most often with your hand class |
| Mixed | not the most frequent, but the model takes it at least 15% of the time (`MIXED_MIN`) |
| Differs | the model takes it less than 15% of the time |

The 15% line is RiverLab's choice, to separate actions the model mixes in from ones it rarely
uses; it is not a measure of how costly a play is. After answering you see the model's frequency
for every action with your hand and its whole range at the decision.

Answers are stored in `preflop_answers` (not in guest mode) and count towards the daily study
goal. Analytics shows how often you matched the model, by spot and position, and the hands that
most often differed.

