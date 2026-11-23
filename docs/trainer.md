# Trainer

The Trainer grades decisions only against solutions RiverLab has validated. Today that is one
game: **heads-up push/fold** (Trainer page). Other drills will follow once their solutions are
validated; nothing is graded against invented answers.

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
