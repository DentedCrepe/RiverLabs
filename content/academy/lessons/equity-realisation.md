+++
slug = "equity-realisation"
title = "Equity realisation: what your equity is really worth"
level = 2
order = 2
version = 1
status = "published"
summary = "Equity is what a hand would win if the cards were dealt out with no more betting. With betting, hands win more or less than that. Measured in real solves: the same range is worth more in position, and a weaker range keeps far less of its equity."
estimated_minutes = 15
objectives = [
  "Define equity realisation as what you win from the pot, counting later betting, divided by your equity.",
  "Explain why the player in position realises more of the same equity.",
  "Explain why a weaker range realises less of its equity than a stronger one.",
]
prerequisites = ["equity", "expected-value", "positions"]
tags = ["math", "solver", "position"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every figure comes from turn solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium of its tree) and from exact equity enumeration; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Solve a turn or river spot and compare each player's value with their equity."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Raw equity: what each range wins if the remaining cards are dealt with no more betting."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019
note = "Solver-based no-limit hold'em theory, including equity realisation and how position and range strength affect it."

[solves.same]
board = "Kh8h4c2s"
ranges = ["AA,KK,QQ,JJ,TT,99,88,77,66,55,AK,AQ,AJ,KQ,KJ,QJ,JT,T9s,98s,87s,76s,65s", "AA,KK,QQ,JJ,TT,99,88,77,66,55,AK,AQ,AJ,KQ,KJ,QJ,JT,T9s,98s,87s,76s,65s"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [50, 100]
ip_bets = [50, 100]
oop_raises = [100]
ip_raises = [100]
iterations = 600

[solves.weak_oop]
board = "Qh9h5c2s"
ranges = ["QJ,JTs,AhKh,8h7h,66", "QT,99,AQ,JhTh,44"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [50, 100]
ip_bets = [50, 100]
oop_raises = [100]
ip_raises = [100]
iterations = 600

[solves.weak_ip]
board = "Qh9h5c2s"
ranges = ["QT,99,AQ,JhTh,44", "QJ,JTs,AhKh,8h7h,66"]
pot_bb = "10"
stack_bb = "20"
oop_bets = [50, 100]
ip_bets = [50, 100]
oop_raises = [100]
ip_raises = [100]
iterations = 600
[[questions]]
id = "same-equity"
type = "numeric"
prompt = "Turn K♥ 8♥ 4♣ 2♠, pot 10bb, 20bb behind. Both players hold exactly the same range (pairs 55 to AA; AK, AQ, AJ, KQ, KJ, QJ and JT, suited and offsuit; T9, 98, 87, 76 and 65 suited). What is the out-of-position player's equity (in %): their share of the pot if both just checked to showdown?"
answer = 50.0
tolerance = 0.01
unit = "%"
explanation = "Exactly 50%: the two ranges are identical, so neither is ahead."
concepts = ["equity-realisation", "equity"]
compute = { kind = "equity", params = { ranges = ["AA,KK,QQ,JJ,TT,99,88,77,66,55,AK,AQ,AJ,KQ,KJ,QJ,JT,T9s,98s,87s,76s,65s", "AA,KK,QQ,JJ,TT,99,88,77,66,55,AK,AQ,AJ,KQ,KJ,QJ,JT,T9s,98s,87s,76s,65s"], board = "Kh8h4c2s" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "same-oop"
type = "numeric"
prompt = "Turn K♥ 8♥ 4♣ 2♠, pot 10bb, 20bb behind. Both players hold exactly the same range (pairs 55 to AA; AK, AQ, AJ, KQ, KJ, QJ and JT, suited and offsuit; T9, 98, 87, 76 and 65 suited). On each street either player may bet 50% or 100% of the pot, and after a bet one raise is allowed, all-in; there is no rake. In RiverLab's solve of this spot (an approximate equilibrium for these sizes, within 0.05% of the pot), how much of their equity (in %) does the out-of-position player realise: what they win from the 10bb pot on average, counting chips won or lost in later betting, divided by their equity?"
answer = 93.31
tolerance = 0.1
unit = "%"
explanation = "About 93%: they win about 50% × 93.3% ≈ 46.7% of the pot instead of 50%, so acting first costs them about 3.3% of the pot with the same cards. A common explanation (a heuristic interpretation, not something this number shows): the opponent sees what they do before choosing, so can bet when betting gains and check when it does not."
concepts = ["equity-realisation", "positions"]
compute = { kind = "solve", params = { solve = "same", path = [], max_exploitability_pct = 0.05, realisation = true }, field = "realisation.0", scale = 100.0 }

[[questions]]
id = "same-ip"
type = "numeric"
prompt = "Turn K♥ 8♥ 4♣ 2♠, pot 10bb, 20bb behind. Both players hold exactly the same range (pairs 55 to AA; AK, AQ, AJ, KQ, KJ, QJ and JT, suited and offsuit; T9, 98, 87, 76 and 65 suited). On each street either player may bet 50% or 100% of the pot, and after a bet one raise is allowed, all-in; there is no rake. In RiverLab's solve of this spot (an approximate equilibrium for these sizes, within 0.05% of the pot), how much of their equity (in %) does the player in position realise: what they win from the 10bb pot on average, counting chips won or lost in later betting, divided by their equity?"
answer = 106.69
tolerance = 0.1
unit = "%"
explanation = "About 107%, more than their equity: with no rake the two players' shares of the pot add up to the whole pot, so what the out-of-position player loses to acting first (about 3.3% of the pot), the player in position gains: 53.3% ÷ 50% ≈ 107%."
concepts = ["equity-realisation", "positions"]
compute = { kind = "solve", params = { solve = "same", path = [], max_exploitability_pct = 0.05, realisation = true }, field = "realisation.1", scale = 100.0 }

[[questions]]
id = "weak-equity"
type = "numeric"
prompt = "Turn Q♥ 9♥ 5♣ 2♠, pot 10bb, 20bb behind. One player holds QJ, JT suited, A♥K♥, 8♥7♥ and 66; the other holds QT, 99, AQ, J♥T♥ and 44. What is the equity (in %) of the QJ, JT suited, A♥K♥, 8♥7♥ and 66 range if both players just checked to showdown?"
answer = 37.78
tolerance = 0.01
unit = "%"
explanation = "About 38%: the other range's set (99) and top pair with the best kicker (AQ) are ahead of most of these hands."
concepts = ["equity-realisation", "equity"]
compute = { kind = "equity", params = { ranges = ["QJ,JTs,AhKh,8h7h,66", "QT,99,AQ,JhTh,44"], board = "Qh9h5c2s" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "weak-oop"
type = "numeric"
prompt = "Turn Q♥ 9♥ 5♣ 2♠, pot 10bb, 20bb behind. One player holds QJ, JT suited, A♥K♥, 8♥7♥ and 66 (about 38% equity); the other holds QT, 99, AQ, J♥T♥ and 44. On each street either player may bet 50% or 100% of the pot, and after a bet one raise is allowed, all-in; there is no rake. The player with QJ, JT suited, A♥K♥, 8♥7♥ and 66 acts first. In RiverLab's solve (an approximate equilibrium for these sizes, within 0.05% of the pot), how much of their equity (in %) do they realise: what they win from the 10bb pot on average, counting chips won or lost in later betting, divided by their equity?"
answer = 41.18
tolerance = 0.1
unit = "%"
explanation = "Only about 41%: they win about 37.8% × 41.2% ≈ 15.6% of the pot, well under half of their equity. A common explanation (a heuristic interpretation, not shown by this number): facing bets they must fold hands that would sometimes have won at showdown, and when they continue they pay more against a stronger range."
concepts = ["equity-realisation"]
compute = { kind = "solve", params = { solve = "weak_oop", path = [], max_exploitability_pct = 0.05, realisation = true }, field = "realisation.0", scale = 100.0 }

[[questions]]
id = "strong-ip"
type = "numeric"
prompt = "Turn Q♥ 9♥ 5♣ 2♠, pot 10bb, 20bb behind. One player holds QT, 99, AQ, J♥T♥ and 44 (about 62% equity); the other holds QJ, JT suited, A♥K♥, 8♥7♥ and 66. On each street either player may bet 50% or 100% of the pot, and after a bet one raise is allowed, all-in; there is no rake. The player with QT, 99, AQ, J♥T♥ and 44 acts last (in position). In RiverLab's solve (an approximate equilibrium for these sizes, within 0.05% of the pot), how much of their equity (in %) do they realise: what they win from the 10bb pot on average, counting chips won or lost in later betting, divided by their equity?"
answer = 135.70
tolerance = 0.1
unit = "%"
explanation = "About 136%: the stronger range wins about 100% − 15.6% ≈ 84.4% of the pot with about 62.2% equity (84.4 ÷ 62.2 ≈ 1.36), while the weaker range out of position realises only about 41%."
concepts = ["equity-realisation"]
compute = { kind = "solve", params = { solve = "weak_oop", path = [], max_exploitability_pct = 0.05, realisation = true }, field = "realisation.1", scale = 100.0 }

[[questions]]
id = "weak-ip"
type = "numeric"
prompt = "Turn Q♥ 9♥ 5♣ 2♠, pot 10bb, 20bb behind. One player holds QJ, JT suited, A♥K♥, 8♥7♥ and 66 (about 38% equity); the other holds QT, 99, AQ, J♥T♥ and 44. On each street either player may bet 50% or 100% of the pot, and after a bet one raise is allowed, all-in; there is no rake. The player with QJ, JT suited, A♥K♥, 8♥7♥ and 66 acts last (in position). In RiverLab's solve (an approximate equilibrium for these sizes, within 0.05% of the pot), how much of their equity (in %) do they realise: what they win from the 10bb pot on average, counting chips won or lost in later betting, divided by their equity?"
answer = 50.1
tolerance = 0.1
unit = "%"
explanation = "About 50%, up from about 41% when the same range acts first, with exactly the same ranges and equity. Position helps, but here the weaker range still realises only about half of its equity."
concepts = ["equity-realisation", "positions"]
compute = { kind = "solve", params = { solve = "weak_ip", path = [], max_exploitability_pct = 0.05, realisation = true }, field = "realisation.1", scale = 100.0 }

[[questions]]
id = "why-position"
type = "multiple_choice"
prompt = "Two players hold exactly the same range on the turn, with betting still to come. Why does the player who acts last usually win more than half of the pot on average?"
options = [
  "They see what the other player does before they choose, so they can bet when that gains and check when it does not",
  "The player in position always has better cards",
  "Acting last means you never have to fold",
]
answer = 0
explanation = "Information (the usual explanation, a heuristic). Their cards are no better, but every decision of theirs is made knowing the other player's choice, which is worth part of the pot."
concepts = ["equity-realisation", "positions"]

[[questions]]
id = "why-weak"
type = "multiple_choice"
prompt = "Why does a range with less equity usually realise a smaller share of it, with betting still to come?"
options = [
  "Facing bets it must fold some hands that would sometimes have won at showdown, and the hands that continue pay more against a stronger range",
  "Equity is computed wrongly for weak ranges",
  "A weaker range always checks, so it never wins the pot",
]
answer = 0
explanation = "The usual explanation (a heuristic): equity assumes every hand sees the showdown for free. Betting takes that away: the stronger range bets, and the weaker range either pays to continue or gives up the share of the pot it would have won."
concepts = ["equity-realisation"]
+++

## Equity is not the whole story {#what}

::: fact
Your **equity** is your share of the pot if the remaining cards are dealt with **no more
betting** (see the equity lesson). With betting still to come, a hand wins more or less than
that. **Equity realisation** measures how much:

$$ \text{realisation} = \frac{\text{what you win from the current pot on average, counting later bets}}{\text{equity} \times \text{pot}} $$

"Counting later bets" means chips you win or lose in the betting that follows are included, so
what you win can be more than the whole pot, or less than nothing. Above 100% you win more than
your equity; below 100%, less.
:::

::: simple
Equity says how often your cards would win if nobody bet again. Real hands have more betting:
you may fold a hand that would have won, or win a pot your cards would have lost. Realisation
is the part of your equity you actually keep.
:::

::: approximation
A **solver** works out strategies for both players that neither could improve by much; a
**solve** is its result for one spot. Here realisation is measured in solves of small turn
spots: in each, the strategies could be exploited by at most 0.05% of the pot, so they are an
approximate equilibrium for the bet and raise sizes listed in each question (no rake), and
nothing more. RiverLab divides each player's average result by their exact equity. Other
sizes, stacks or ranges give other numbers.
:::

## Position {#position}

::: solver
The turn K♥ 8♥ 4♣ 2♠ with 10bb in the pot and 20bb behind, and **the same range for both
players**. Either may bet 50% or 100% of the pot; after a bet one raise is allowed, and it is
all-in (a 100%-of-pot raise would be more than the stacks). Equity is exactly even, so any
difference comes from who acts first.
:::

```riverlab:question
{"id": "same-equity"}
```

```riverlab:question
{"id": "same-oop"}
```

```riverlab:question
{"id": "same-ip"}
```

::: intermediate
With no rake the two shares add up to the whole pot. Because both equities here are exactly
50%, the two realisations average 100%: what one player loses by acting first, the other gains.
As a heuristic interpretation, the player in position chooses after seeing the check or the
bet, and that information is worth part of the pot.
:::

```riverlab:question
{"id": "why-position"}
```

## Weaker ranges realise less {#strength}

::: solver
The turn Q♥ 9♥ 5♣ 2♠ with 10bb in the pot and 20bb behind, with the same bet and raise sizes.
One range has top pair with a jack kicker (QJ), draws (JT suited, A♥K♥, 8♥7♥) and a small pair
(66). The other has a set (99), top pair with the best kicker (AQ), top pair with a ten kicker
(QT), a combination draw (J♥T♥) and a smaller pair (44). Solved twice: once with each range
acting first.
:::

```riverlab:question
{"id": "weak-equity"}
```

```riverlab:question
{"id": "weak-oop"}
```

```riverlab:question
{"id": "strong-ip"}
```

```riverlab:question
{"id": "weak-ip"}
```

::: heuristic
In these solves the weaker range realises far less of its equity than the stronger one, and
position softens that but does not fix it. The usual interpretation: when your range is behind,
betting from the other side keeps taking away the free showdowns that equity counts on.
:::

```riverlab:question
{"id": "why-weak"}
```
