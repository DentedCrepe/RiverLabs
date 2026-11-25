+++
slug = "icm"
title = "Tournaments and ICM: chips are not money"
level = 6
order = 8
version = 1
status = "published"
summary = "Why a tournament chip is worth less the more you have, how the Independent Chip Model turns stacks into prize money, and why calling an all-in near the bubble needs more equity than pot odds say."
estimated_minutes = 20
objectives = [
  "Explain why tournament chips and prize money are not proportional.",
  "Work out ICM values with the Malmuth–Harville model for a small example.",
  "Compute the equity an all-in call needs under ICM, and compare it with chip pot odds.",
  "State what ICM leaves out.",
]
prerequisites = ["expected-value", "pot-odds"]
tags = ["tournaments", "icm", "math"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every ICM figure is computed by RiverLab's engine (poker_theory::icm, checked against every finishing order) each time the content is synced; awaiting review by a person."

[[tools]]
href = "/play"
label = "Play: sit & go"
why = "Set the payouts for a sit & go: each seat shows its ICM share, and all-in calls when everyone else has folded are graded against the equity ICM requires."

[[reading]]
title = "Assigning Probabilities to the Outcomes of Multi-Entry Competitions"
authors = "David A. Harville"
year = 1973
note = "Journal of the American Statistical Association. The finishing-order model ICM is built on, first written for horse races."

[[questions]]
id = "big-stack-value"
type = "numeric"
prompt = "Three players are left in a sit & go paying 50%, 30% and 20% of the prize pool. The stacks are 5,000, 3,000 and 2,000 chips. Under ICM (the Malmuth–Harville model), what share of the prize pool (in %) is the 5,000-chip stack worth?"
answer = 38.39
tolerance = 0.01
unit = "%"
explanation = "About 38.4%, although that stack holds 50% of the chips. Under the model it finishes first 50% of the time, second about 33.9% and third about 16.1% (100 − 50 − 33.9), so its value is 50% × 50 + 33.9% × 30 + 16.1% × 20 ≈ 38.4. Half the time it does not win, and then it collects much less than its chip share."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm", params = { stacks = [5000, 3000, 2000], prizes = [50, 30, 20] }, field = "values.0" }

[[questions]]
id = "short-stack-value"
type = "numeric"
prompt = "Three players are left in a sit & go paying 50%, 30% and 20% of the prize pool. The stacks are 5,000, 3,000 and 2,000 chips. Under ICM, what share of the prize pool (in %) is the 2,000-chip stack worth?"
answer = 28.86
tolerance = 0.01
unit = "%"
explanation = "About 28.9%, with only 20% of the chips: with three left it is already guaranteed the 20% third prize, and every place above that is a bonus."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm", params = { stacks = [5000, 3000, 2000], prizes = [50, 30, 20] }, field = "values.2" }

[[questions]]
id = "second-place"
type = "numeric"
prompt = "Three players are left with 5,000, 3,000 and 2,000 chips. Under the Malmuth–Harville model, how likely (in %) is the 5,000-chip stack to finish exactly second?"
answer = 33.93
tolerance = 0.01
unit = "%"
explanation = "About 33.9%: 3/10 × 5/7 (the 3,000 stack wins, then the 5,000 stack has 5 of the remaining 7 thousand chips) plus 2/10 × 5/8 (the 2,000 stack wins first)."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm", params = { stacks = [5000, 3000, 2000], prizes = [50, 30, 20] }, field = "places.0.1", scale = 100.0 }

[[questions]]
id = "winner-take-all"
type = "numeric"
prompt = "Three players are left in a winner-take-all tournament with 1,000, 3,000 and 6,000 chips. Under ICM, what share of the prize pool (in %) is the 1,000-chip stack worth?"
answer = 10.0
tolerance = 0.01
unit = "%"
explanation = "Exactly 10%, its share of the chips. With one prize, ICM is just the chance of winning, which the model sets equal to the chip share, so chips and money are proportional."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm", params = { stacks = [1000, 3000, 6000], prizes = [100] }, field = "values.0" }

[[questions]]
id = "bubble-chip-price"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips and are not in the blinds. A player with a bigger stack moves all-in; everyone else folds, and 500 chips of blinds, posted by the players who folded, are already in the pot. Calling risks your 2,000 chips to win their matching 2,000 plus the 500 blinds (the rest of their bet goes back to them). Counting chips only, and ignoring split pots, what equity (in %) does a call need?"
answer = 44.44
tolerance = 0.01
unit = "%"
explanation = "2,000 ÷ (2,500 + 2,000) ≈ 44.4%: the chip pot-odds price, risk ÷ (pot you win + risk)."
concepts = ["tournament-theory-icm", "pot-odds"]
compute = { kind = "pot_odds", params = { pot_mbb = 2500, to_call_mbb = 2000 }, field = "required_equity", scale = 100.0 }

[[questions]]
id = "bubble-icm-price"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips and are not in the blinds; the two players who folded have 1,000 each left after posting 500 chips of blinds, which are in the pot. A player who started the hand with 6,000 moves all-in for all of it. Calling puts in all your 2,000; the 4,000 of their bet you cannot match goes back to them. Under ICM, and ignoring split pots, what equity (in %) does calling need?"
answer = 73.18
tolerance = 0.01
unit = "%"
explanation = "About 73%, far above the 44% chips alone need. Losing knocks you out in 4th with nothing, while folding keeps you in with two short stacks who may bust before you; winning adds less prize money than losing costs."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm_call", params = { stacks = [2000, 0, 1000, 1000], hero = 0, villain = 1, pot = 6500, to_call = 6000, prizes = [50, 30, 20] }, field = "required_equity", scale = 100.0 }

[[questions]]
id = "bubble-fold-value"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips and are not in the blinds; the two players who folded have 1,000 each left after posting 500 chips of blinds, which are in the pot. A player who started the hand with 6,000 moves all-in for all of it. If you fold, what share of the prize pool (in %) is your stack worth under ICM?"
answer = 25.97
tolerance = 0.01
unit = "%"
explanation = "About 26%: you keep 2,000 chips, the all-in player takes the 6,500 in the pot, and the other two keep 1,000 each. Calling and losing is worth 0, because 4th place pays nothing."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm_call", params = { stacks = [2000, 0, 1000, 1000], hero = 0, villain = 1, pot = 6500, to_call = 6000, prizes = [50, 30, 20] }, field = "fold", scale = 100.0 }

[[questions]]
id = "bubble-win-value"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips and are not in the blinds; the two players who folded have 1,000 each left after posting 500 chips of blinds, which are in the pot. A player who started the hand with 6,000 moves all-in for all of it. You call with all your 2,000 and win: you now have 4,500 chips, the all-in player keeps the 4,000 you could not match, and the others have 1,000 each. What share of the prize pool (in %) is your stack worth under ICM?"
answer = 35.49
tolerance = 0.01
unit = "%"
explanation = "About 35.5%. Folding is worth about 26.0% and losing 0, so a call risks about 26.0 points to gain about 35.5 − 26.0 = 9.5. Break-even equity is 26.0 ÷ 35.5 ≈ 73%, the ICM price of the call."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm_call", params = { stacks = [2000, 0, 1000, 1000], hero = 0, villain = 1, pot = 6500, to_call = 6000, prizes = [50, 30, 20] }, field = "win", scale = 100.0 }

[[questions]]
id = "when-linear"
type = "multiple_choice"
prompt = "In which situation does ICM always lead to the same call-or-fold decisions as counting chips (chip pot odds)?"
options = [
  "Winner takes all, or only two players are left",
  "Whenever you have the biggest stack",
  "On the bubble, when one more player will bust before the money",
]
answer = 0
explanation = "With one prize, or two players left, a player's ICM value is a straight line in their chips (second prize plus their chip share of the difference), so every comparison of outcomes comes out as it would in chips. With two left the values are not equal to the chip shares, but the decisions agree. On the bubble they differ the most."
concepts = ["tournament-theory-icm"]

[[questions]]
id = "limits"
type = "multiple_choice"
prompt = "Which of these does ICM take into account?"
options = [
  "The stacks and the payouts only",
  "The players' skill",
  "Who has position and how high the blinds are",
]
answer = 0
explanation = "ICM looks at the stacks and the payouts and nothing else. It assumes every player is equally skilled and ignores the blinds, position and how many hands are left."
concepts = ["tournament-theory-icm"]
+++

## Why chips are not money {#chips}

::: fact
In a cash game a chip is worth its face value: you can leave with it. In a tournament you are
paid by **finishing place**, not by your chips, and you cannot cash out. Winning every chip
pays only the first prize. A **sit & go** is a single-table tournament that starts as soon as
every seat is filled. The **bubble** is the stage where the next player to bust finishes just
outside the paid places: with four left and three paid, the next one out gets nothing.
:::

::: simple
Think of a sit & go paying 50%, 30% and 20% of the prize pool with three players left. Even the
smallest stack is sure of 20%. Even winning every chip pays only 50%. So, under the model this
lesson describes (ICM), each extra chip is worth less than the one before, and losing all your
chips costs more than doubling them gains.
:::

::: intermediate
Under ICM, prize money depends on chips in a curved way: the first chips you hold are worth the
most and each later one less. As a heuristic, that curve is why tournament play is more
cautious than chip counting suggests, above all on the bubble.
:::

## The Independent Chip Model {#icm}

::: approximation
The **Independent Chip Model** (ICM) is a model, not a law of poker. In its usual
(Malmuth–Harville) form:
- your chance of finishing **first** is your share of all the chips;
- given who finished first, the chance of each other player finishing **second** is their
  share of the chips that are left, and so on down the places;
- your ICM value is the sum, over places, of your chance of that place times its prize.

It ignores skill, position, the blinds and how many hands remain.
:::

::: math
For stacks $s_1, \dots, s_n$ with total $S$:

$$ P(i \text{ first}) = \frac{s_i}{S}, \qquad
   P(i \text{ second}) = \sum_{j \ne i} \frac{s_j}{S} \cdot \frac{s_i}{S - s_j}, \; \dots $$

$$ \text{ICM}_i = \sum_k P(i \text{ finishes } k\text{th}) \times \text{prize}_k $$

RiverLab computes this exactly over every finishing order.
:::

```riverlab:question
{"id": "big-stack-value"}
```

```riverlab:question
{"id": "second-place"}
```

```riverlab:question
{"id": "short-stack-value"}
```

Under ICM the big stack is worth less than its chip share and the short stack more. The payouts
are flatter than the chip counts: here every player is already sure of the 20% third prize, and
no stack can win more than 50%.

```riverlab:question
{"id": "winner-take-all"}
```

```riverlab:question
{"id": "when-linear"}
```

## Calling an all-in on the bubble {#bubble}

::: fact
Compare the ICM value of three outcomes: **fold**, **call and win**, **call and lose**. Ignoring
split pots, calling is worth it when equity × win + (1 − equity) × lose ≥ fold, so the equity a
call needs is

$$ e^* = \frac{\text{fold} - \text{lose}}{\text{win} - \text{lose}} $$

With chips instead of ICM values this is the familiar pot-odds price.
:::

```riverlab:question
{"id": "bubble-chip-price"}
```

```riverlab:question
{"id": "bubble-icm-price"}
```

```riverlab:question
{"id": "bubble-fold-value"}
```

```riverlab:question
{"id": "bubble-win-value"}
```

::: heuristic
Near the bubble, call all-ins with fewer hands than chip pot odds suggest, especially when
shorter stacks are likely to bust first. Calling a big stack's all-in costs a shorter stack more
prize money, relative to what it can win, than the chip count suggests, while the big stack is
not knocked out if it loses. So big stacks can move all-in with more hands and shorter stacks
must call with fewer. How far to tighten depends on the payouts and on everyone's stacks, so
check the spot rather than learning one number.
:::

## What ICM leaves out {#limits}

::: approximation
ICM sees only the stacks and the payouts. A much better player is worth more than ICM says,
and a short stack about to pay the big blind is worth less. With the blinds rising quickly or
many hands still to play, the model is rougher.
:::

```riverlab:question
{"id": "limits"}
```
