+++
slug = "node-locking"
title = "Node locking and exploitative adjustments"
level = 6
order = 3
version = 2
status = "published"
summary = "Fix an opponent's strategy at one decision, solve again, and see the best answer to it: the jump from mixing to calling or folding everything, and what that is worth."
estimated_minutes = 20
objectives = [
  "Explain what node locking does and what a locked solve is (and is not).",
  "Predict how a best response changes when an opponent bluffs too much or too little.",
  "Weigh the gain from exploiting against the risk of being wrong about the opponent.",
]
prerequisites = ["exploitability", "polarised-ranges", "bluff-to-value-ratios"]
tags = ["strategy", "solver", "exploitation"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every figure comes from river solves run by RiverLab's solver each time the content is synced; the locked solves are checked with the locked decision held fixed (docs/solver-validation.md). Awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Open any solve's strategy tree, lock a decision with \"Lock this decision\", and compare the locked solve with the original."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Maximally exploitive play against a fixed strategy, and why it moves to the edges."

[solves.equilibrium]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "10"
ip_bets = [100]
iterations = 2000

[solves.overbluff]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "10"
ip_bets = [100]
locks = [{ path = [0], frequencies = [0.0, 1.0] }]
iterations = 2000

[solves.value_only]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "10"
ip_bets = [100]
locks = [{ path = [0], frequencies = [0.5714285714285714, 0.4285714285714286] }]
iterations = 2000

[[questions]]
id = "eq-call"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent holds sets (99, 55) or nothing (76 suited, T8 suited) and bets the pot, 10bb all-in. In the solved (unlocked) strategy, how often (in %) do you call?"
answer = 50.0
tolerance = 0.1
unit = "%"
explanation = "Half the time (the solve gives 49.99%): exactly enough that the bluffs gain nothing over checking, the minimum defence frequency for a pot-sized bet."
concepts = ["minimum-defence-frequency", "node-locking"]
compute = { kind = "solve", params = { solve = "equilibrium", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], max_exploitability_pct = 0.05 }, field = "share.call", scale = 100.0 }

[questions.scene]
players = 2
hero = "BB"
board = ["Ks", "9d", "5c", "3h", "2s"]
pot_bb = "10"
bets_bb = { SB = "10" }
stacks_bb = { SB = "0", BB = "10" }
actions = { BB = "checks", SB = "bets 10bb (all-in)" }

[[questions]]
id = "eq-value"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ; your opponent holds sets (99, 55) or nothing (76 suited, T8 suited) and may bet the pot (10bb, all-in) after you check; in this solved tree you may only check and there are no raises. In the solved (unlocked) strategy, what is your value per hand, in bb, counted net of your half of the starting pot?"
answer = -1.43
tolerance = 0.01
unit = "bb"
explanation = "About −1.43bb: you put 5bb into the starting pot and get back about 3.57bb of it on average (5 − 1.43). Your opponent's sets get called half the time; the bluffs, which on average gain nothing over checking, are what make you call."
concepts = ["node-locking"]
compute = { kind = "solve", params = { solve = "equilibrium", path = [], max_exploitability_pct = 0.05 }, field = "values_bb.0" }

[[questions]]
id = "over-call"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent holds sets (99, 55) or nothing (76 suited, T8 suited). Your opponent's decision after your check is locked to bet the pot, 10bb all-in, with every hand, and the rest of the tree is solved. How often (in %) does the best response call the bet?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "Every time. Against a range with more bluffs than the indifference ratio allows, calling wins more than folding with every bluff-catcher, so the best response stops mixing."
concepts = ["node-locking", "exploitative-adjustments"]
compute = { kind = "solve", params = { solve = "overbluff", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], max_exploitability_pct = 0.05 }, field = "share.call", scale = 100.0 }

[[questions]]
id = "over-value"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent is locked to bet the pot, 10bb all-in, with every hand (sets 99 and 55, 76 suited and T8 suited). Against that, and calling as the best response does, what is your value per hand in bb, net of your half of the starting pot?"
answer = 2.14
tolerance = 0.01
unit = "bb"
explanation = "About +2.14bb, up from about −1.43bb in the unlocked solve. Every bluff is now called and loses."
concepts = ["exploitative-adjustments"]
compute = { kind = "solve", params = { solve = "overbluff", path = [], max_exploitability_pct = 0.05 }, field = "values_bb.0" }

[[questions]]
id = "under-fold"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent holds sets (99, 55: 6 combinations) or nothing (76 suited, T8 suited: 8 combinations). Your opponent's decision after your check is locked to bet the pot, 10bb all-in, with 6/14 (about 43%) of its range; scaled from the unlocked solve, that keeps the sets betting and all but removes the bluffs. How often (in %) does the best response fold to the bet?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "Every time: a bet now almost always means a set, so calling loses."
concepts = ["node-locking", "exploitative-adjustments"]
compute = { kind = "solve", params = { solve = "value_only", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], max_exploitability_pct = 0.05 }, field = "share.fold", scale = 100.0 }

[[questions]]
id = "under-value"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent holds sets (99, 55: 6 combinations) or nothing (76 suited, T8 suited: 8 combinations), and its decision after your check is locked to bet the pot, 10bb all-in, with 6/14 of its range, which (scaled from the unlocked solve) keeps the sets betting and all but removes the bluffs. Folding to every bet, what is your value per hand in bb, net of your half of the starting pot?"
answer = 0.71
tolerance = 0.01
unit = "bb"
explanation = "About +0.71bb. You never pay off a set, and when your opponent checks you win the pot at showdown against 76 suited and T8 suited."
concepts = ["exploitative-adjustments"]
compute = { kind = "solve", params = { solve = "value_only", path = [], max_exploitability_pct = 0.05 }, field = "values_bb.0" }

[[questions]]
id = "under-bluffs"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind. You hold KQ and check (in this solved tree you may only check); your opponent holds sets (99, 55: 6 combinations) or nothing (76 suited, T8 suited: 8 combinations). Its decision after your check is locked to bet the pot, 10bb all-in, with 6/14 of its range, the lock scaling every hand's betting frequency from the unlocked solve. In the locked solve, how often (in %) do 76 suited and T8 suited bet? Answer to the nearest whole percent."
answer = 0.0
tolerance = 0.5
unit = "%"
explanation = "About 0% (the solve gives about 0.1%). In the unlocked solve the sets always bet, so cutting the range's betting to 6/14 squeezes out the bluffs: the lock is in effect 'bet only the sets'."
concepts = ["node-locking"]
compute = { kind = "solve", params = { solve = "value_only", path = [{ type = "action", value = 0 }], subset = ["76s", "T8s"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[[questions]]
id = "risk"
type = "multiple_choice"
prompt = "You believe an opponent bluffs too often on the river, so you start calling with every bluff-catcher. What is the main risk?"
options = [
  "If your read is wrong and they bluff less than the equilibrium amount, calling everything loses a lot; the equilibrium mix could not have been exploited that way",
  "There is no risk: calling more is always correct",
  "The equilibrium strategy would have lost the same amount",
]
answer = 0
explanation = "A maximal exploit can always be chosen pure, and usually is, so it is itself very exploitable. How far to move depends on how confident you are in the read and how much the opponent can adjust."
concepts = ["exploitative-adjustments"]
+++

## What node locking does {#what}

::: fact
**Node locking** fixes one player's strategy at a decision, for example "this player bets with
every hand here", and solves the rest of the game again. The result is an approximate
equilibrium of the game with that decision held fixed: the other player's strategy answers the
lock, and the locked player's other decisions adapt around it. When the locked decision is the
locked player's only one, as in this lesson, the other player's strategy is a best response to
the lock.
:::

::: solver
A locked solve is an **exploit of the lock, not an equilibrium of the original game**. Its
exploitability counts only
what a player could gain at decisions that are not locked; the locked decision itself is
exactly as exploitable as the frequencies you chose. RiverLab labels every locked solve this
way and shows how the values moved from the unlocked solve.
:::

## The unlocked solve (approximate equilibrium) {#equilibrium}

::: solver
River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 10bb behind, no rake. You hold KQ and act first; your
opponent holds sets (99, 55) or nothing (76 suited, T8 suited). In this tree you may only check;
your opponent may then check or bet exactly the pot (10bb, all-in), and there are no raises.
Solved by RiverLab each time the content is synced, within 0.05% of the starting pot of an
equilibrium of this tree.
:::

```riverlab:question
{"id": "eq-call"}
```

```riverlab:question
{"id": "eq-value"}
```

## Lock the opponent to over-bluff {#over}

::: solver
The same spot, with your opponent's decision after your check locked to "bet the pot with every
hand". The rest is solved again; the exploitability check of that solve holds the lock fixed.
:::

```riverlab:question
{"id": "over-call"}
```

```riverlab:question
{"id": "over-value"}
```

## Lock the opponent to never bluff {#under}

::: solver
The same spot, with your opponent's decision after your check locked to bet 6/14 of its range
(the share of its range that is sets). RiverLab's range lock scales every hand's betting frequency
from the unlocked solve; there the sets always bet, so the lock squeezes out the bluffs. The rest
is solved again, and the exploitability check holds the lock fixed.
:::

```riverlab:question
{"id": "under-bluffs"}
```

```riverlab:question
{"id": "under-fold"}
```

```riverlab:question
{"id": "under-value"}
```

::: intermediate
In the unlocked solve you are indifferent between calling and folding, which is why you can mix.
Here every hand you hold is the same bluff-catcher, so any lock that breaks the bluff ratio breaks
the indifference for all of them at once, and the best response jumps to the edge: call
everything or fold everything. With hands of different strength, the best response instead calls
with the hands above some threshold and folds the rest.
:::

## Exploiting in practice {#practice}

::: heuristic
Exploits pay most when the read is strong and the deviation is big, but a full exploit is a pure
strategy that an alert opponent can punish. Many players move only part of the way from the
equilibrium towards the exploit, more as the evidence grows. Lock a decision in the Solver to
see how much a suspected leak is worth before you change your play.
:::

```riverlab:question
{"id": "risk"}
```
