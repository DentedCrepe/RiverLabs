+++
slug = "polarised-ranges"
title = "Polarised and merged ranges, and bet sizing"
level = 4
order = 4
version = 1
status = "published"
summary = "Why a range of very strong hands and bluffs bets big, why a range of medium hands checks, and how the bluff ratio follows the bet size. Every figure is from a solve run for this lesson."
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
notes = "Drafted with AI assistance. Every figure comes from small river solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium), or from poker-theory; awaiting review by a person."

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
explanation = "None. Against hands that can only call or fold, the polarised range gains by betting as much as it is allowed to."
concepts = ["overbets", "bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "polar", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.a1", scale = 100.0 }

[[questions]]
id = "polar-bluff-share"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb. You are in position with a set (99 or 55) or nothing (76 suited or T8 suited) against KQ only, and the solution bets 15bb (150% of the pot). What share (in %) of those bets are bluffs?"
answer = 37.49
tolerance = 0.1
unit = "%"
explanation = "37.49% of the 15bb bets are the 76s and T8s bluffs, which is what makes KQ indifferent between calling and folding. Compare it with the formula in the next question."
concepts = ["bluff-to-value-ratios", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "polar", path = [{ type = "action", value = 0 }], subset = ["76s", "T8s"], max_exploitability_pct = 0.05 }, field = "subset_of_action.a3", scale = 100.0 }

[[questions]]
id = "theory-bluff-share"
type = "numeric"
prompt = "A player bets 15bb into a 10bb pot on the river with a polarised range. By the indifference formula B / (P + 2B), what share (in %) of the bets should be bluffs to make a bluff-catcher indifferent?"
answer = 37.5
tolerance = 0.05
unit = "%"
explanation = "15 / (10 + 30) = 37.5%. The solved strategy's 37.49% agrees: in this spot the bluff-catchers never block the bluffs, so the single-bet model describes it almost exactly."
concepts = ["bluff-to-value-ratios"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 15000 }, field = "bluff_share", scale = 100.0 }

[[questions]]
id = "merged-checks"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb. Now you are in position with KJ, KT and a few 76s; your opponent has checked with KQ, QJ, JT or 99 (a set). You may bet 33%, 75% or 150%. What share (in %) of your range bets in the solved strategy?"
answer = 0.0
tolerance = 0.1
unit = "%"
explanation = "None. KJ and KT beat QJ and JT, which fold to a bet, and lose to KQ and 99, which call. Betting only gets called by better hands, so the whole range checks."
concepts = ["merged-ranges"]
compute = { kind = "solve", params = { solve = "merged", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[[questions]]
id = "why-big"
type = "multiple_choice"
prompt = "In this kind of spot (your range is very strong hands or bluffs, your opponent's is hands that can only call or fold), why does a bigger bet suit you?"
options = [
  "Bigger bets always make the opponent fold more",
  "Your strong hands win more when called, and a bigger bet can carry a larger share of bluffs while the caller stays indifferent",
  "Bigger bets reduce the variance of your results",
]
answer = 1
explanation = "Against hands that only call or fold, every extra chip you bet with a strong hand is won when called, and the indifference ratio B / (P + 2B) lets more bluffs into a bigger bet. In this model the polarised player's EV rises with the bet size."
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
The spots below are solved by RiverLab each time this lesson is loaded: river K♠ 9♦ 5♣ 3♥ 2♠,
pot 10bb, 20bb behind, no rake. Your opponent (out of position) holds only KQ and checks to you.
You hold sets (99, 55) or nothing (76 suited, T8 suited), and may bet 33%, 75% or 150% of the
pot. The solve is within 0.05% of the pot of an equilibrium of this tree.
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
you bet.
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
Same board and sizes, but now you hold KJ, KT and a few 76s, and your opponent holds KQ, QJ, JT or
99. Your hands beat some of theirs and lose to others.
:::

```riverlab:question
{"id": "merged-checks"}
```

::: heuristic
A medium-strength hand gains little from betting when worse hands fold and better ones call. It
prefers to check and reach showdown cheaply. With a merged range against a range that has some
worse hands that call, small bets often do best; the exact answer depends on the ranges.
:::
