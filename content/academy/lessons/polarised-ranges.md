+++
slug = "polarised-ranges"
title = "Polarised and merged ranges, and bet sizing"
level = 4
order = 4
version = 2
status = "published"
summary = "Why a range of very strong hands and bluffs bets big, why a range of medium hands often checks, and how the bluff ratio follows the bet size. Every figure is from a solve run for this lesson or from the bet-math formulas."
estimated_minutes = 20
objectives = [
  "Tell a polarised range from a merged one.",
  "Explain why a polarised range facing bluff-catchers prefers the largest bet.",
  "Check a solver's bluff ratio against the indifference formula.",
  "Explain why a merged range often checks.",
]
prerequisites = ["minimum-defence-frequency", "bluff-to-value-ratios", "nash-equilibrium"]
tags = ["strategy", "bet-sizing", "solver"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every figure comes from small river solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium), or from poker-theory; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Solve your own river spots with several bet sizes and see which ones the solution uses."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Solves polarised river games (the clairvoyance game) and shows why bigger bets favour the polarised player."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[solves.polar]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo", "99,55,76s,T8s"]
pot_bb = "10"
stack_bb = "20"
ip_bets = [33, 75, 150]
iterations = 1000

[solves.merged]
board = "Ks9d5c3h2s"
ranges = ["KQs,KQo,QJs,JTs,99", "KJs,KJo,KTs,KTo,76s"]
pot_bb = "10"
stack_bb = "20"
ip_bets = [33, 75, 150]
iterations = 1000

[[questions]]
id = "polar-overbet"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You are in position with a set (99 or 55) or nothing (76 suited or T8 suited); your opponent holds only top pair, KQ, and has checked. You may bet 33%, 75% or 150% of the pot. In the solved strategy, what share (in %) of your whole range bets 150%?"
answer = 68.56
tolerance = 0.1
unit = "%"
explanation = "All 6 sets bet, and enough of the 8 bluffs to make KQ indifferent: 6 + 3.6 of 14 combinations ≈ 68.6%, all with the biggest size. The 33% and 75% sizes are not used at all."
concepts = ["polarised-ranges", "overbets", "bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "polar", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.a3", scale = 100.0 }

[questions.scene]
players = 2
hero = "SB"
board = ["Ks", "9d", "5c", "3h", "2s"]
pot_bb = "10"
stacks_bb = { SB = "20", BB = "20" }
actions = { BB = "checks" }

[[questions]]
id = "polar-small"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You are in position with a set (99 or 55) or nothing (76 suited or T8 suited); your opponent holds only KQ and has checked. You may bet 33%, 75% or 150% of the pot. In the solved strategy, what share (in %) of your range bets the smallest size, 33%?"
answer = 0.0
tolerance = 0.1
unit = "%"
explanation = "None. Against hands that can only call or fold, the polarised range gains by betting as much as it is allowed to: here 15bb (150%), the largest size this tree offers. An all-in bet of 20bb is not in the tree."
concepts = ["overbets", "bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "polar", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.a1", scale = 100.0 }

[[questions]]
id = "polar-bluff-share"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You are in position with a set (99 or 55) or nothing (76 suited or T8 suited); your opponent holds only top pair, KQ, and has checked. You may bet 33%, 75% or 150% of the pot, and the solved strategy bets only 15bb (150%). What share (in %) of those 15bb bets are bluffs?"
answer = 37.49
tolerance = 0.1
unit = "%"
explanation = "37.49% of the 15bb bets are the 76s and T8s bluffs, which is what makes KQ indifferent between calling and folding. The indifference formula B / (P + 2B) gives 15 / (10 + 30) = 37.5%."
concepts = ["bluff-to-value-ratios", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "polar", path = [{ type = "action", value = 0 }], subset = ["76s", "T8s"], max_exploitability_pct = 0.05 }, field = "subset_of_action.a3", scale = 100.0 }

[[questions]]
id = "theory-bluff-share"
type = "numeric"
prompt = "A player bets 15bb into a 10bb pot on the river with a polarised range. By the indifference formula B / (P + 2B), what share (in %) of the bets should be bluffs to make a bluff-catcher indifferent?"
answer = 37.5
tolerance = 0.05
unit = "%"
explanation = "15 / (10 + 30) = 37.5%. This lesson's solved river spot (K♠ 9♦ 5♣ 3♥ 2♠, KQ against 99, 55, 76s and T8s) gives 37.49%: there the bluff-catcher never shares a card with the bettor's hands and cannot bet or raise, so the spot is exactly the single-bet model, and the 0.01-point gap is the solve's convergence error."
concepts = ["bluff-to-value-ratios"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 15000 }, field = "bluff_share", scale = 100.0 }

[[questions]]
id = "merged-checks"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 20bb behind. You are in position with KJ, KT and a few 76s; your opponent has checked with KQ, QJ, JT or 99 (a set). You may bet 33%, 75% or 150%. What share (in %) of your range bets in the solved strategy?"
answer = 0.0
tolerance = 0.1
unit = "%"
explanation = "None. KJ and KT beat QJ and JT, which fold to a bet, and lose to KQ and 99, which call: betting only gets called by better hands. The 76s cannot bluff either: KQ (12 combinations) and 99 (3) beat your whole range, so 15 of their 23 combinations always call, and a bet made only by bluffs would be called by QJ and JT as well. So the whole range checks."
concepts = ["merged-ranges"]
compute = { kind = "solve", params = { solve = "merged", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[[questions]]
id = "why-big"
type = "multiple_choice"
prompt = "In this kind of spot (your range is very strong hands or bluffs, your opponent's is hands that can only call or fold), why does a bigger bet suit you?"
options = [
  "Bigger bets make your bluffs profitable",
  "Your strong hands win more when called, and a bigger bet can carry a larger share of bluffs while the caller stays indifferent",
  "Bigger bets reduce the variance of your results",
]
answer = 1
explanation = "Against hands that only call or fold, every extra chip you bet with a strong hand is won when called, and the indifference ratio B / (P + 2B) lets more bluffs into a bigger bet. The caller does fold more often against a bigger bet, but your bluffs still only break even: the gain comes from your strong hands. In this model the polarised player's EV rises with the bet size."
concepts = ["overbets", "polarised-ranges"]
+++

## Two shapes of range {#shapes}

::: fact
A **polarised** range contains very strong hands and weak hands (bluffs), with few in between. A
**merged** (or linear) range is built from the top down and includes many medium-strength hands.
:::

::: simple
Polarised: "I have it or I have nothing." Merged: "I have something decent, often not the best."
:::

## A polarised river: bet big {#polarised}

::: solver
The spots below are solved by RiverLab each time the content is synced: river K♠ 9♦ 5♣ 3♥ 2♠,
pot 10bb, 20bb behind, no rake. Your opponent (out of position) holds only KQ and, in this tree,
can only check to you. You hold sets (99, 55) or nothing (76 suited, T8 suited), and may bet 33%,
75% or 150% of the pot (no all-in, no raises). The solve is within 0.05% of the pot of an
equilibrium of this tree: an approximate equilibrium for these bet sizes only.
:::

```riverlab:question
{"id": "polar-overbet"}
```

```riverlab:question
{"id": "polar-small"}
```

::: intermediate
This matches the theory of polarised betting (Chen and Ankenman's "clairvoyance" games). The
caller only has bluff-catchers, so whatever size you choose, they call just often enough to make
your bluffs break even. What changes with size is how much your strong hands win: more, the more
you bet. (This tree stops at 150%; with 20bb behind, an all-in was simply not offered.)
:::

```riverlab:question
{"id": "why-big"}
```

## The bluff ratio, from theory and from the solver {#ratio}

::: fact
To make a bluff-catcher indifferent, a polarised bet of $B$ into a pot $P$ must be a bluff with
probability $B / (P + 2B)$, the same figure as the equity the caller needs.
:::

```riverlab:question
{"id": "polar-bluff-share"}
```

```riverlab:question
{"id": "theory-bluff-share"}
```

::: approximation
The formula assumes the caller's hands all beat every bluff and lose to every value hand, and
that no cards are shared between them. Real ranges are messier: blockers and hands in between
move the solver away from the formula, sometimes a lot.
:::

## A merged range: often check {#merged}

::: solver
Same board, pot, stacks and sizes, but now you hold KJ, KT and a few 76s, and your opponent holds
KQ, QJ, JT or 99. Your hands beat some of theirs and lose to others. Also solved at every sync,
within 0.05% of the pot of an equilibrium of this tree.
:::

```riverlab:question
{"id": "merged-checks"}
```

::: heuristic
A medium-strength hand gains little from betting when worse hands fold and better ones call. It
prefers to check and reach showdown cheaply. With a merged range against a range that has some
worse hands that call, small bets often do best; the exact answer depends on the ranges.
:::
