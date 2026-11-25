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
why = "Set the payouts for a sit & go: each seat shows its ICM share, and heads-up all-in calls and folds are graded against the equity ICM requires."

[[reading]]
title = "Assigning Probabilities to the Outcomes of Multi-Entry Competitions"
authors = "David A. Harville"
year = 1973
note = "Journal of the American Statistical Association. The finishing-order model ICM is built on, first written for horse races."

[[reading]]
title = "Kill Everyone"
authors = "Lee Nelson, Tysen Streib and Steven Heston"
year = 2007
note = "Tournament strategy, including ICM and its use in push/fold decisions."

[[questions]]
id = "big-stack-value"
type = "numeric"
prompt = "Three players are left in a sit & go paying 50%, 30% and 20% of the prize pool. The stacks are 5,000, 3,000 and 2,000 chips. Under ICM (the Malmuth–Harville model), what share of the prize pool (in %) is the 5,000-chip stack worth?"
answer = 38.39
tolerance = 0.01
unit = "%"
explanation = "About 38.4%, although that stack holds 50% of the chips. It can win at most the 50% first prize, and the other two are already sure of at least 20% each."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm", params = { stacks = [5000, 3000, 2000], prizes = [50, 30, 20] }, field = "values.0" }

[[questions]]
id = "short-stack-value"
type = "numeric"
prompt = "Three players are left in a sit & go paying 50%, 30% and 20% of the prize pool. The stacks are 5,000, 3,000 and 2,000 chips. Under ICM, what share of the prize pool (in %) is the 2,000-chip stack worth?"
answer = 28.86
tolerance = 0.01
unit = "%"
explanation = "About 28.9%, with only 20% of the chips: it is already guaranteed the 20% third prize, and every place above that is a bonus."
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
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips behind. A player with a bigger stack moves all-in; everyone else folds, and 500 chips of blinds are already in the pot. Calling risks your 2,000 chips to win their matching 2,000 plus the 500 blinds. Counting chips only, what equity (in %) does a call need?"
answer = 44.44
tolerance = 0.01
unit = "%"
explanation = "2,000 ÷ (2,500 + 2,000) ≈ 44.4%: the chip pot-odds price, risk ÷ (pot you win + risk)."
concepts = ["tournament-theory-icm", "pot-odds"]
compute = { kind = "pot_odds", params = { pot_mbb = 2500, to_call_mbb = 2000 }, field = "required_equity", scale = 100.0 }

[[questions]]
id = "bubble-icm-price"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips behind; the two players who folded have 1,000 each. A player who started the hand with 6,000 moves all-in, and 500 chips of blinds are already in the pot. Under ICM, what equity (in %) does calling need?"
answer = 73.18
tolerance = 0.01
unit = "%"
explanation = "About 73%, far above the 44% chips alone need. Losing knocks you out in 4th with nothing, while folding keeps you in with two short stacks who may bust before you; winning adds less prize money than losing costs."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm_call", params = { stacks = [2000, 0, 1000, 1000], hero = 0, villain = 1, pot = 6500, to_call = 6000, prizes = [50, 30, 20] }, field = "required_equity", scale = 100.0 }

[[questions]]
id = "bubble-fold-value"
type = "numeric"
prompt = "Four players are left in a sit & go that pays three places (50%, 30%, 20%). You have 2,000 chips behind; the two players who folded have 1,000 each; a player who started the hand with 6,000 moves all-in, with 500 chips of blinds in the pot. If you fold, what share of the prize pool (in %) is your stack worth under ICM?"
answer = 25.97
tolerance = 0.01
unit = "%"
explanation = "About 26%. Calling and losing is worth 0 (4th place pays nothing); calling and winning, about 35.5%. A call risks about 26 points to gain about 9.5, which is why it needs about 73% equity."
concepts = ["tournament-theory-icm"]
compute = { kind = "icm_call", params = { stacks = [2000, 0, 1000, 1000], hero = 0, villain = 1, pot = 6500, to_call = 6000, prizes = [50, 30, 20] }, field = "fold", scale = 100.0 }

[[questions]]
id = "when-linear"
type = "multiple_choice"
prompt = "In which situation does ICM give the same answer as counting chips?"
options = [
  "Winner takes all, or only two players are left",
  "Whenever you have the biggest stack",
  "On the bubble, when one more player will bust before the money",
]
answer = 0
explanation = "With one prize, or two players left, a player's ICM value is a straight line in their chips (second prize plus their chip share of the difference), so chip EV and ICM agree. On the bubble they differ the most."
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
pays only the first prize.
:::

::: simple
Think of a sit & go paying 50%, 30% and 20% of the prize pool with three players left. Even the
smallest stack is sure of 20%. Doubling the biggest stack cannot pay more than 50%. So each
extra chip is worth less than the one before, and losing all your chips costs more than
doubling them gains.
:::

::: intermediate
Prize money depends on chips in a curved way: the first chips you add are worth the most and
each later one less. That curve is why tournament play is more cautious than chip counting
suggests, above all when a bust-out gets nothing (the **bubble**).
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

::: intermediate
The big stack's value is below its chip share and the short stack's is above it. Chips move
prize money towards the small stacks, because everyone is already guaranteed the smallest prize.
:::

```riverlab:question
{"id": "winner-take-all"}
```

```riverlab:question
{"id": "when-linear"}
```

## Calling an all-in on the bubble {#bubble}

::: fact
Compare the ICM value of three outcomes: **fold**, **call and win**, **call and lose**. Calling
is worth it when equity × win + (1 − equity) × lose ≥ fold, so the equity a call needs is

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

::: heuristic
Near the bubble, call all-ins with fewer hands than chip pot odds suggest, especially when
shorter stacks are likely to bust first. A big stack can push smaller ones around for the same
reason: they must risk more prize money to call than the big stack risks to bet. How far to
tighten depends on the payouts and on everyone's stacks, so check the spot rather than
learning one number.
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
