+++
slug = "range-checking"
title = "Range checking, and why the tree matters"
level = 4
order = 6
version = 2
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
notes = "Drafted with AI assistance. Every figure comes from small river solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium of its tree); awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

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
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You act first and hold only top pair (KQ); your opponent holds sets (99, 55) or nothing (76 suited, T8 suited). You may check or bet 33% or 75% of the pot. If you bet, your opponent may fold, call or raise by 100% of the pot after calling (to 19.9bb over a 3.3bb bet, all-in for 20bb over a 7.5bb bet); if you check, they may check or bet 75%, and you may then raise. One raise at most. In the solved strategy, what share (in %) of your range checks?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "All of it. Every hand in your range beats the bluffs and loses to the sets. A bet is called only by sets and folds the bluffs, which you beat anyway, so it wins nothing extra. Its only possible use is as a blocking bet, a small bet that stops your opponent making a bigger one, and here that fails: your opponent can raise, with sets or as a bluff. Checking instead lets their bluffs bet into you. Without raises the answer is different: with no raises allowed, the solve bets 33% with every hand."
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
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You act first and hold only top pair (KQ); your opponent holds sets (99, 55) or nothing (76 suited, T8 suited). You may check or bet 33% or 75% of the pot; if you check, your opponent may check or bet 75%. Nobody may raise. In the solved strategy, what share (in %) of your range bets the small size, 33% of the pot?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "With no raises allowed, a small bet caps the pot: your opponent can only call with sets or fold, and can never make you face a big bet with a bluff. Betting small turns out better than checking and facing the 75% bet. Allow raises in the same spot and the solve checks every hand instead: the answer depends on the tree."
concepts = ["range-checking", "bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "catchers_no_raise", path = [], max_exploitability_pct = 0.05 }, field = "share.a1", scale = 100.0 }

[[questions]]
id = "polar-first-checks"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You act first with sets (99, 55) or nothing (76 suited, T8 suited), and your opponent holds only KQ. You may check or bet 33% or 75% of the pot; if you check, they may bet 75%; either player may raise by 100% of the pot after calling, once. What share (in %) of your range checks in the solved strategy?"
answer = 38.79
tolerance = 0.1
unit = "%"
explanation = "Your 6 set combinations always bet 75%, with enough bluffs that KQ is indifferent to calling: B / (P + 2B) = 7.5 / 25 = 30% of the bets, so 6 × 0.3 / 0.7 ≈ 2.57 of your 8 bluff combinations bet and about 5.43 check. 5.43 of your 14 combinations is about 38.8%."
concepts = ["range-checking", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "polar_first", path = [], max_exploitability_pct = 0.05 }, field = "share.check", scale = 100.0 }

[[questions]]
id = "polar-first-sets"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You act first with sets (99, 55) or nothing (76 suited, T8 suited), and your opponent holds only KQ. You may check or bet 33% or 75% of the pot; if you check, they may bet 75%; either player may raise by 100% of the pot after calling, once. In the solved strategy, what share (in %) of your sets bet 75% of the pot?"
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
  "Your range is mostly medium hands that worse hands will not call, your opponent has the strong hands and the bluffs, and they can raise",
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
opponent has sets (99, 55) or nothing (76 suited, T8 suited). You may bet 33% or 75% of the pot,
and if you check your opponent may bet 75%. Either player may raise once, by 100% of the pot
after calling: to 3.3 + (10 + 3.3 + 3.3) = 19.9bb over a 3.3bb bet, and over a 7.5bb bet the
raise to 7.5 + 25 = 32.5bb is cut to the 20bb stack (all-in). Solved by RiverLab each time the
content is synced, within 0.05% of the pot of an equilibrium of this tree: an approximate
equilibrium for these sizes only.
:::

```riverlab:question
{"id": "catchers-check"}
```

::: intermediate
Every one of your hands beats the bluffs and loses to the sets. A bet is called only by the
sets and folds the bluffs, which you beat anyway, so it wins nothing extra. A small bet could
still stop your opponent from making a bigger one (a blocking bet), but here they can raise,
with a set or as a bluff, so it does not work. Checking lets their bluffs bet into you.
(Without raises, the next section shows a different answer.)
:::

## The tree changes the answer {#tree}

```riverlab:question
{"id": "catchers-no-raise"}
```

::: fact
A solve is an (approximate) equilibrium **of the game it was given**: these bet and raise sizes
and no others.
:::

::: solver
In this spot, removing raises makes a **blocking bet** appear: a small bet made by a hand that
wants to see a showdown cheaply, so that it does not face a bigger bet. With raises allowed, the
same spot checks every hand instead. Before trusting a strategy, check which options the tree
allowed.
:::

## The polarised player acts first: bet {#polar}

```riverlab:question
{"id": "polar-first-checks"}
```

```riverlab:question
{"id": "polar-first-sets"}
```

::: heuristic
Range checks are most common for the player whose range is **capped** (it holds few or none of
the strongest hands): lots of medium hands, few of the strongest. The player with the strong hands and the bluffs does most of the betting.
:::

```riverlab:question
{"id": "when-range-check"}
```
