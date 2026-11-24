+++
slug = "range-checking"
title = "Range checking, and why the tree matters"
level = 4
order = 6
version = 1
status = "published"
summary = "When a player should check every hand, shown with river solves, and how the sizes a solve allows can change the answer completely."
estimated_minutes = 20
objectives = [
  "Explain when a range checks all of its hands.",
  "Read a solve's strategy as a share of a range.",
  "Explain why a solve is only an equilibrium of the bet sizes it was given.",
]
prerequisites = ["polarised-ranges", "range-advantage"]
tags = ["strategy", "solver", "ranges"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every figure comes from small river solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium of its tree); awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Solve the same spot with and without raises and compare."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[solves.catchers]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [33, 75]
ip_bets = [75]
oop_raises = [100]
ip_raises = [100]
iterations = 1500

[solves.catchers_no_raise]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [33, 75]
ip_bets = [75]
iterations = 1500

[solves.polar_first]
board = "Ks9d5c3h2s"
ranges = ["99,55,76s,T8s", "KQs,KQo"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [33, 75]
ip_bets = [75]
oop_raises = [100]
ip_raises = [100]
iterations = 1500

[[questions]]
id = "catchers-check"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You act first and hold only top pair (KQ); your opponent holds sets (99, 55) or nothing (76 suited, T8 suited). You may check or bet 33% or 75%, and either player may raise. In the solved strategy, what share (in %) of your range checks?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "All of it. Every hand in your range beats exactly the same hands. A bet is called only by sets and folds out everything you beat, so betting can only lose. Checking lets your opponent bluff into you."
concepts = ["range-checking"]
compute = { kind = "solve", params = { solve = "catchers", path = [], max_exploitability_pct = 0.05 }, field = "share.check", scale = 100.0 }

[questions.scene]
players = 2
hero = "BB"
board = ["Ks", "9d", "5c", "3h", "2s"]
pot_bb = "10"
stacks_bb = { SB = "20", BB = "20" }

[[questions]]
id = "catchers-no-raise"
type = "numeric"
prompt = "Same river (K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind), same ranges: you act first with only KQ, against sets or nothing. This time the solve allows no raises at all. What share (in %) of your range bets the small size, 33% of the pot?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "With no raises allowed, a small bet caps the pot: your opponent can only call with sets or fold, and can never make you face a big bet with a bluff. Betting small turns out better than checking and facing the 75% bet. Allow raises and the small bet disappears: the answer depends on the tree."
concepts = ["range-checking", "bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "catchers_no_raise", path = [], max_exploitability_pct = 0.05 }, field = "share.a1", scale = 100.0 }

[[questions]]
id = "polar-first-checks"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind, raises allowed. Now you act first with sets (99, 55) or nothing (76 suited, T8 suited), and your opponent holds only KQ. What share (in %) of your range checks in the solved strategy?"
answer = 38.79
tolerance = 0.1
unit = "%"
explanation = "Your sets always bet; only some of the bluffs check. With the strong hands on your side, you bet a lot of the time, with enough bluffs that KQ is indifferent to calling."
concepts = ["range-checking", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "polar_first", path = [], max_exploitability_pct = 0.05 }, field = "share.check", scale = 100.0 }

[[questions]]
id = "polar-first-sets"
type = "numeric"
prompt = "Same spot: you act first with sets (99, 55) or nothing (76 suited, T8 suited) against KQ only. What share (in %) of your sets bet 75% of the pot?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "Every set bets the bigger size: against a range that can only call or fold, the strongest hands want as much money in as they are allowed."
concepts = ["polarised-ranges"]
compute = { kind = "solve", params = { solve = "polar_first", path = [], subset = ["99", "55"], max_exploitability_pct = 0.05 }, field = "subset_share.a2", scale = 100.0 }

[[questions]]
id = "when-range-check"
type = "multiple_choice"
prompt = "In which situation does checking your whole range make the most sense?"
options = [
  "Your range is mostly medium hands that worse hands will not call, and your opponent has the strong hands and the bluffs",
  "Your range has all the strong hands",
  "You were the preflop raiser",
]
answer = 0
explanation = "Betting needs worse hands that call or better hands that fold. When neither is true for your range, checking every hand does better, and your opponent's bluffs are what your hands get value from."
concepts = ["range-checking"]
+++

## What a range check is {#what}

::: fact
A player **range-checks** when they check every hand they can hold in a spot. That is a
statement about the whole range, not about one hand: in a solve it shows as a 100% check at
that decision.
:::

## Bluff-catchers act first: check everything {#catchers}

::: solver
River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind, no rake. You act first with KQ only; your
opponent has sets (99, 55) or nothing (76 suited, T8 suited). Bets of 33% or 75% of the pot,
raises of 100%. Solved by RiverLab each time this lesson loads, within 0.05% of the pot of an
equilibrium of this tree.
:::

```riverlab:question
{"id": "catchers-check"}
```

::: intermediate
Every one of your hands beats the bluffs and loses to the sets. A bet is called only by better
hands, and anything worse folds. Checking gives up nothing and lets the bluffs bet into you.
:::

## The tree changes the answer {#tree}

```riverlab:question
{"id": "catchers-no-raise"}
```

::: approximation
A solve is an equilibrium **of the game it was given**: these bet and raise sizes and no
others. Remove raises and a "blocking bet" appears that would be a mistake in a game where your
opponent can raise. Before trusting a strategy, check which options the tree allowed.
:::

## The polarised player acts first: bet {#polar}

```riverlab:question
{"id": "polar-first-checks"}
```

```riverlab:question
{"id": "polar-first-sets"}
```

::: heuristic
Range checks are most common for the player whose range is capped: lots of medium hands, few
of the strongest. The player with the strong hands and the bluffs does most of the betting.
:::

```riverlab:question
{"id": "when-range-check"}
```
