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
- **No waiting mid-game:** the starting table's model is prepared before the first hand. If
  stacks later move into a bucket whose model is not ready yet (common in a sit & go as the
  blinds rise), the hand is dealt at once with the nearest stack depth already prepared for that
  table, and the exact model is prepared in the background and used from the next hand after it
  is ready. The table's note names the depth actually used. In a sit & go, the model for one
  player fewer is prepared in the background too, ready for the next bust.

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
  40% less; its bluffs, set as a share of its value bets, fall by 40% with them.
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

## Practice leaks

Each bot can be given one documented river leak (`leaks` in the table config, one per
opponent; the setup form applies one to every bot). It changes only that bot's river rules
(board of five cards, `bot.rs`), on top of its difficulty; everything else stays as above. The
bot's name says which leak it has: the point is to practise exploiting a known tendency, as a
node lock does in the Solver.

| Leak | River change |
| --- | --- |
| Over-bluffs the river | bluffs three times the balanced share, B/(P+2B) of the bets |
| Never bluffs the river | no bluffs at all |
| Folds too much to river bets | continues with half the minimum defence share |
| Calls too much on the river | continues with 1.6 times the minimum defence share (at most all) |

(Multiway, the continuing share is cut by 20% as for every bot.) The range tracking follows
the leak exactly, because it uses the bot's own policy, so your equity against its range, and
every pot-odds check, measures you against the leaky range as it really is.

**Spot the leak** (`hide_leaks`, cash or sit & go: fast fold reshuffles the bots every hand):
each bot draws one of the four leaks or none, from the session seed, and its name does not say
which. At the end the report asks you to guess each bot's leak, then reveals the answers and the
sections below, including whether your river decisions matched the best response more often in
the second half of the graded decisions than in the first. The leak is hidden in the interface
only: it is in the session's records, so this is practice, not a test.

The report adds a section per leak:
- **River calls and folds** facing that bot's bet heads-up: how many matched the best response
  for the decision. Your call ends the betting, so calling is better than folding exactly when
  your equity against the bot's actual range is at least the price; your equity is the same
  Monte Carlo estimate as everywhere else, so very close cases can go either way. Raises are
  left out.
- **Your river bets** that bot faced: the share of its range (combos that do not share a card
  with yours) that would have folded to each bet, recorded whatever it chose, against the fold
  share a pure bluff of that bet needed (bet ÷ pot after the bet). It is a measure, not a
  grade: whether a bet was right also depends on your hand.

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

3. **ICM in sit & goes.** A sit & go pays places, not chips, so an all-in call is judged by
   prize money. When you face a bet heads-up (everyone else has folded) and calling ends the
   betting (you or your opponent is all-in), the decision records your ICM value, as a share of
   the remaining prize pool, if you fold, call and win, or call and lose
   (`poker_theory::icm::icm_call`). The equity a call needs is then
   (fold − lose) ÷ (win − lose), and it replaces the chip break-even for that decision, with the
   same 5-point margin (keys `icm_called_without_equity`, `icm_folded_with_equity`).
   - **The model:** Malmuth–Harville. Your chance of finishing first is your share of the
     chips; each lower place is drawn the same way from the stacks that are left. It ignores
     skill, position, the blinds and how many hands remain, so it is a model, not a solution of
     the tournament. Computed exactly over every finishing order; players with no chips take
     the bottom places.
   - **Payouts:** set per sit & go as percentages for 1st, 2nd, ... (whole numbers adding up
     to 100, never increasing). Left empty, the default is winner takes all with up to three
     players and 65/35 with four to six, a common small sit & go structure, not a rule. Only the
     places still to be paid count: with n players left, prizes for places 1 to n.
   - **Ties** count as half a win (your equity), not modelled separately.
   - **Not ICM:** multiway pots and calls after which betting could continue keep the chip
     comparison. With winner takes all, or two players left, ICM is linear in chips, so it
     agrees with chip pot odds; the heads-up push/fold trainer is therefore unaffected.
   - **At the table:** each seat shows its ICM share of the prize pool from the stacks at the
     start of the hand (a busted player's share is the prize for their place), and the header
     shows the payouts.

Not graded: your bets and raises after the flop (that needs a solver for the exact spot), and
your own all-in shoves in a sit & go (they need an ICM model of the callers' ranges).

## Instant feedback and practice aids
- **Feedback after every decision.** Straight after you act, the table shows how the decision
  compares with the references the report uses. It is computed by the same code
  (`engine/play-engine/src/judge.rs`), so the two always agree. Verdicts:
  - **Flagged:** a report finding, e.g. a fold with 15 points more equity than the flop price
    needs.
  - **Unusual:** the model takes your action 10% of the time or less, but it isn't a flagged
    pattern.
  - **Fine:** not flagged. The feedback states the model's frequency for your action, or the
    price and your equity.
  - **Not graded:** no reference applies. Examples: bets and raises after the flop, and spots
    off the model tree.

  Preflop feedback lists the model's whole mix for your hand class in that spot.
- **Your odds on your turn.** Your equity against the live bots' tracked ranges and, when you
  face a bet, the equity the price needs. The table's **Equity** setting (kept per device)
  chooses how it appears: **On request** (the default: a "Show my equity" button for the current
  decision), **Always** (the stored Monte Carlo figure, shown at every decision) or **Off**. The
  equity is stored with every decision either way, so the review and the report work the same.
- **Why this number?** On your turn, `GET /api/v1/play/sessions/{id}/equity` returns the engine's
  breakdown (`Session::equity_breakdown`, `engine/play-engine/src/session.rs`):
  - your cards, the board and your made hand;
  - for each bot still in the hand: its tracked range (combinations, share of all hands, a
    13×13 grid of how much of each class remains); every action it took this hand, with the
    share of the range that would have taken it (the bot's own policy probability, so exact for
    its rules and model, not a guess); and, from the flop on, what its range makes on the board
    with your equity against each part;
  - your equity against all of them together: exact enumeration against one bot from the flop
    on, otherwise seeded Monte Carlo with 20,000 samples (the method is shown). It can differ
    from the stored figure by Monte Carlo noise;
  - the range the bots perceive for you, when the bot model tracks one.

  Combinations that share a card with yours or the board are excluded. Outside your turn the
  endpoint returns 422.
- **Saved for the hand review.** The same breakdown (same seeds, so the same figures) is saved
  with every decision you make, and the review shows it under "Why this equity — and what they
  really held", adding each bot's real cards and how much of that exact hand its range kept
  (the probability the bot would have played it this way). The real hand always has a weight
  above 0, which the engine's tests check: the range tracking can never rule out the cards the
  bot really held. It is never sent while the hand is live. To keep records small the saved
  grid is packed and rounded to 0.001, and bots that have not acted yet (any hand still
  possible) get no grid: about 20 KB per six-player hand, a few KB heads-up. Computing it adds
  about 30 ms to a typical action (under 0.4 s at worst, six-handed preflop).
- **Hand history and review.** Every hand of the session is listed with your cards, the board,
  your result and how many decisions were flagged or unusual. Tap one to replay it street by
  street, with everyone's cards. Each of your decisions shows:
  - the pot, the amount to call, your equity and the price;
  - the verdict;
  - preflop, the model's frequencies for your hand class, with your choice marked;
  - heads-up after the flop, a **Solve this spot** button (below).
- **Reveal bots' cards** (practice mode) shows their cards during hands. Hands played with it
  on are flagged, and the report says how many there were.
- **Show all hands** after a hand reveals cards that weren't shown at showdown, without
  flagging anything.
- **Auto-deal** deals the next hand after a short pause. It stops after a flagged or unusual
  decision so you can read the feedback.

## Solve this spot
A heads-up decision after the flop has a **Solve this spot** button in the hand review. It
opens the Solver with a postflop solve filled in (every field can still be edited). When the
solve finishes, the result walks the real line to your decision and shows the solver's mix
for your hand class, with your action marked.

What the export contains (`engine/play-engine/src/spot.rs`), recorded at the decision:
- **Root:** the start of the decision's street: the board, the pot then, and the smaller of the
  two stacks then. Tournament chips are converted to big blinds (chip EV, no ICM: the solver
  works in chips).
- **Rake:** the table's rake (none for sit & go).
- **Bot's range:** the range its rules actually played this way, tracked exactly from the
  probabilities of every decision it made, as it was when the street was dealt (the solve starts
  there and models the bot's own actions on the street), with weights rounded to 4 decimals.
- **Your range:** your range at the flop in the bots' preflop model (an approximate model; your
  real bets are mapped to the nearest model sizes). It is not narrowed by your own actions after
  the flop. If the hand left the model (e.g. a limp), it is every hand, and the note says to
  edit it.
- **Tree:** the sizes used in the hand on that street are always included, as whole
  percentages in GTOpen's convention (bets: % of the pot; raises: the raise increment as a % of
  the pot after calling). If an observed bet or raise was all-in, bets of 98% of the stack or more
  become all-in. Default sizes are added from the richest of these that fits in the solver's
  memory limit (`RIVERLAB_SOLVER_MAX_MEMORY_MB`):
  1. 33% and 75% bets and a 100% raise on the street; 33% and 75% bets on later streets;
  2. the same with only 75% bets on later streets;
  3. a 75% bet on the street, no default raise; 75% on later streets;
  4. only the hand's sizes (or a 75% bet where none was made); 75% on later streets.

  Later streets never have raises. The export's notes say which tree was used, and when even the
  smallest needs more memory than allowed (narrow the ranges then).
- **Solver:** Discounted CFR, up to 500 iterations, stopping at 0.5% of the pot.

The solve is an approximate equilibrium between these two ranges for this tree, not the bot's
real strategy and not a verdict on the hand; following the line uses the nearest size in the
tree for each real action. Preflop and multiway decisions cannot be solved (RiverLab solves only
heads-up postflop spots).

## Solver review of a whole hand
**Review this hand with the solver** (hand review, heads-up hands after the flop) solves every
street where you made decisions, one after the other, and shows at each of your decisions the
solver's mix for your exact hand, the EV of each action, and the EV you lost against the best
one. Implementation: `engine/play-engine/src/review.rs`, run by `services/hand_review.py`.

- **Which streets:** the flop, turn and river where you acted and only two players were in the
  hand when the street began. Preflop decisions keep the model comparison above; multiway streets
  are marked "not solvable" (RiverLab solves heads-up spots only).
- **First solved street:** the same starting point as "Solve this spot": your range at the flop in
  the bots' preflop model, and the bot's range as its rules actually played it, tracked exactly,
  **as it was when the street was dealt** (before its own actions on that street).
- **Later streets (subgame re-solving):** each starts from both players' ranges at the end of the
  previous solved street, following what actually happened there: every combination's weight is
  multiplied by how often the solution takes each real action with it, and hands that collide with
  the new card are removed. If a street could not be solved, the next one starts afresh from the
  tracked ranges. If the solution never reaches a street with your hand, your hand is added with
  a weight of 0.0001 so its strategy can be shown, and a note says so.
- **Trees:** as for "Solve this spot": the sizes used in the hand plus the richest defaults that fit
  in the memory limit (each street's notes say which).
- **Following the line:** each real action is matched to the tree action of the same kind with the
  nearest size.
- **EV of an action** for your hand: from the start of that street's solve, net of your half of
  its starting pot, against the range that reaches the decision, with both players following the
  solution afterwards. **EV lost** is the best action's EV minus yours (never negative). It is
  exact for the solved tree; how much it says about real play depends on the tree's bet sizes and
  the starting ranges.

Each street is an ordinary solve with its exploitability. In the Solution Library a hand's
street solves are shown together as one hand: the replay with the solver's view at each of your
decisions, and each street's solve can be opened at any of your decisions (following the real
line) or from its start. Postflop drills dealt from these solves also show the real hand up to
that street. A review interrupted by a restart is marked as such and can be started again.

## Solver leaks
Analytics → **Solver leaks** queues solver reviews for several hands at once: those with a flagged
or unusual decision, the most recent, or your biggest wins and losses (5 to 30 hands; hands
already reviewed and hands with nothing to solve are skipped). Reviews run one at a time in the
background; one you start from a hand jumps the queue.

It then adds up the engine's per-decision results from every finished review
(`services/leaks.py`): decisions reviewed, EV lost, and **leaks**, decisions losing more than 1% of
their street's starting pot (the postflop drills' threshold). Leaks are grouped by kind of
mistake (your action against the solver's best one, e.g. "Called when folding was better" or
"Bet the wrong size"), by street and by position, and the costliest decisions can be flicked
through with their full replays. The Today card recommends your biggest leak.

## Persistence and limits
- A game lives in the API process while you play. Every finished hand is stored in
  `play_hands` immediately, so reports survive a restart. The game itself does not: it is
  marked `interrupted`.
- Preparing a new table shape solves the bots' preflop model: seconds heads-up, up to about a
  minute at six seats (scaling with `RIVERLAB_SOLVER_THREADS`: 71 s on 2 threads and 37 s on 4
  for the same six-seat work on the test machine). Models are cached on disk
  (`RIVERLAB_RESULT_STORAGE_DIR`), so later tables of the same shape start immediately.
- Playing needs the solver backend (`RIVERLAB_SOLVER_ENABLED=true`); otherwise the endpoints
  return 503.

## Guest mode

What a guest does in guest mode never counts here; see [guest-mode.md](guest-mode.md).
