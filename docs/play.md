# Play against bots

Source: `engine/play-engine` (game loop, bots, records, report), `backend/.../services/play.py`
(sessions, persistence), `frontend/src/components/play` (table and report).

## Game types
| Type | Rules |
|---|---|
| Cash | Fixed table of 2–6 seats with a saved game configuration (blinds, ante, rake). Stacks carry over between hands; busted bots rebuy to the starting stack; you rebuy if "Rebuy when I bust" is on, otherwise the session ends. |
| Fast-fold | A new table every hand: opponents are reshuffled with starting stacks and your seat is random. Your stack carries over and is topped up to the starting stack when below it (counted as a rebuy). |
| Sit & go | Freezeout in tournament chips. Blinds (and optional antes) rise every N hands through the levels you give. Players who bust get finishing places, and the session ends when you bust or win. |

Hands use RiverLab's betting engine (`poker-core`), including heads-up rules (the small blind
is the button) and short stacks posting all-in. Tournament hands are played in whole chips at
the level's blinds (`GameState::tournament`); cash hands in milli-big-blinds. Deals and bot
choices come from a seeded RNG, so a table config with the same seed replays identically.

## Bots
Bots are practice opponents, **not solvers**.

**Preflop: GTOpen's preflop model** (an approximate model; see `solver-validation.md`), solved
once per table shape and cached:
- **Players:** the number in the hand.
- **Stack:** the second-largest stack in the hand, rounded to the nearest of 10, 15, 20, 25,
  30, 40, 50, 75, 100, 150 or 200bb.
- **Blinds and ante:** converted to bb, rounded to 0.001bb.
- **Rake:** the table's.
- **Tree:** 15bb or less is push/fold. Above that: open 2bb (under 30bb) or 2.5bb, re-raise 3×,
  at most two raises, all-in always offered (and forced above 85% of the stack).
- **Solve:** blocker model on; up to 200 iterations or 45 s.

A real bet is mapped to the nearest model action. When a hand leaves the model tree (a limp,
a 4-bet, or a size the model doesn't have), bots use the rules below for the rest of
preflop.

**Rules: postflop, and preflop off the model tree.**
- **Hand ranking:** a bot ranks every combo still in its own range by strength against a
  random hand on this board. That is Monte Carlo equity, 150 seeded samples per combo, so it
  credits draws. Preflop it uses class equity against a random hand.
- **No bet to face:** the bot bets the top `value` share of its range. It bluffs with the
  bottom share that matches the bet size's indifference ratio, B/(P+2B), scaled by `bluff`.
  The bet is `bet size` × pot.
- **Facing a bet:** the bot continues with the top `defend` × minimum-defence share,
  P/(P+B). It raises (to 3× the bet) with the top `raise` share.
- **Multiway:** in pots with three or more players the bot defends 20% less and value-bets
  40% less.
- **Preflop off the model tree:** continuing is capped at 20% / 30% / 45% of the range (hard /
  medium / easy), and there are no bluffs.

| Difficulty | value | bluff | bet size | defend | raise | Preflop leak (vs the model) |
|---|---|---|---|---|---|---|
| Hard | 0.33 | 1.0 | 0.66 pot | 1.0 | 0.08 | none: samples the model |
| Medium | 0.28 | 0.5 | 0.5 pot | 0.9 | 0.06 | facing a raise, a quarter of the model's folds become calls |
| Easy | 0.20 | 0 (never bluffs) | 0.4 pot | 1.3 (calls too much) | 0.02 | facing a raise, half the folds become calls with top-60% hands; re-raises only with top-5% hands |

These parameters are design choices for practice opponents, not claims about how people play.

Every bot decision also yields the probability that each combo would have taken the same
action, so the session tracks each bot's **exact range** as the hand goes on.

## Report
Statistics are counts from your hands:
- VPIP, PFR, 3-bet, fold to 3-bet, c-bet, fold to c-bet;
- went to / won at showdown;
- aggression factor, result by position, bb/100;
- **all-in luck**: actual result against your exact all-in equity at the moment the money went in.

**Weaknesses** are measured against two stated references. No "typical player" numbers are
used.

1. **The preflop model.** For each preflop decision on the model tree, the model's frequency
   for your hand class at that spot:
   - "Played hands the model folds": the model folds that class at least 90% of the time.
   - "Folded hands the model plays": the model folds it at most 10% of the time.
   - "Called where the model raises": the model raises it at least 90% of the time.

   The report also compares how often you played and raised with how often the model would
   have, over the same hands in the same spots.
2. **Pot odds against the bots' actual ranges.** When you faced a bet, the report compares
   your Monte Carlo equity against the live bots' tracked ranges with the break-even equity
   (to call ÷ (pot + to call)):
   - **River or all-in** (no later betting): flagged when your equity misses the break-even by
     more than 5 points.
   - **Flop and turn:** flagged only beyond 15 points, labelled as ignoring later betting.
   - **Preflop:** only all-in calls are graded this way.

Not graded: your bets and raises after the flop (that needs a solver for the exact spot), and
sit & go decisions beyond chip equity (no ICM yet).

## Persistence and limits
- A game lives in the API process while you play. Every finished hand is stored in
  `play_hands` immediately, so reports survive a restart. The game itself does not: it is
  marked `interrupted`.
- Preparing a new table shape solves the bots' preflop model: seconds heads-up, up to about a
  minute at six seats. Models are cached on disk (`RIVERLAB_RESULT_STORAGE_DIR`), so later
  tables of the same shape start immediately.
- Playing needs the solver backend (`RIVERLAB_SOLVER_ENABLED=true`); otherwise the endpoints
  return 503.
