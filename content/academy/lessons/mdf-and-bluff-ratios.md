+++
slug = "mdf-and-bluff-ratios"
title = "Minimum defence frequency and bluff-to-value ratios"
level = 3
order = 7
version = 2
status = "published"
summary = "The indifference arithmetic of a single river bet: how often a defender must continue, and how often a polarised bettor should bluff."
estimated_minutes = 25
objectives = [
  "Derive alpha (a bluff's break-even fold frequency) and minimum defence frequency.",
  "Derive the bluff share that makes a bluff-catcher indifferent.",
  "State the model these results come from, and where real poker departs from it.",
]
prerequisites = ["pot-odds", "expected-value", "indifference", "polarised-ranges"]
tags = ["gto", "indifference", "mdf", "bluffing", "bet-sizing"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Formulas are implemented in poker-theory with tests; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Solve a river spot and compare real frequencies with this toy model."

[[tools]]
href = "/play"
label = "Play against bots"
why = "The hard bots bet with exactly this bluff ratio and defend by this frequency."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "The clairvoyance game and its solution (Part III)."

[[questions]]
id = "mdf-half"
type = "numeric"
prompt = "River. Your opponent bets half the pot. In the single-bet model, what share of your range (in %) must you continue with so that a pure bluff cannot profit? (This is the minimum defence frequency.)"
answer = 66.67
tolerance = 0.5
unit = "%"
explanation = "MDF = P / (P + B) = 1 / 1.5 ≈ 66.7%."
concepts = ["minimum-defence-frequency"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 5000 }, field = "mdf", scale = 100.0 }

[[questions]]
id = "alpha-pot"
type = "numeric"
prompt = "River. You bet the size of the pot with a hand that always loses if called. How often (in %) must your opponent fold for the bluff to break even?"
answer = 50.0
tolerance = 0.5
unit = "%"
explanation = "Risk B to win P: alpha = B / (P + B) = 1/2."
concepts = ["minimum-defence-frequency", "fold-equity"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 10000 }, field = "alpha", scale = 100.0 }

[[questions]]
id = "bluff-share-pot"
type = "numeric"
prompt = "River. A player whose range holds only very strong hands and bluffs bets the size of the pot. What share of their bets (in %) must be bluffs for a bluff-catcher to be indifferent between calling and folding?"
answer = 33.33
tolerance = 0.5
unit = "%"
explanation = "The caller risks B to win P + B, so needs B / (P + 2B) = 1/3 equity: bluffs must be one third of bets."
concepts = ["bluff-to-value-ratios", "indifference"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 10000 }, field = "bluff_share", scale = 100.0 }

[[questions]]
id = "overfold"
type = "decision"
prompt = "Facing pot-sized river bets, a player folds 70% of the time. In the single-bet model, what does that mean for the bettor's pure bluffs?"
options = [
  "They lose money: 70% folds is too few",
  "They make money: any fold rate above 50% makes a pot-sized bluff profitable",
  "They break even",
]
answer = 1
explanation = "A pot-sized bluff needs folds more than alpha = 50% of the time. At 70%: EV = 0.7 × P − 0.3 × P = 0.4P > 0."
concepts = ["minimum-defence-frequency", "exploitative-adjustments"]

[[questions]]
id = "mdf-limits"
type = "multiple_choice"
prompt = "Why is MDF not a rule to defend exactly that share of your range in every real spot?"
options = [
  "It is a rule: always defend exactly MDF",
  "It comes from a model where the defender holds only bluff-catchers and there is no later action; real ranges and later streets change the answer",
  "Because MDF only applies preflop",
]
answer = 1
explanation = "MDF guarantees that pure bluffs can't profit in the single-bet model. In real spots part of the range beats value bets or loses to bluffs, and equilibrium defence can be higher or lower."
concepts = ["minimum-defence-frequency"]
+++

## The model {#model}

::: fact
Everything in this lesson is exact **for this model**: the river, one bet of size $B$ into a pot
of $P$, and the defender can only call or fold. The bettor's hands are either the **nuts** or
**air** (a polarised range), and the defender's hands are **bluff-catchers**: they beat every
bluff and lose to every value bet.
:::

::: simple
Think of a player who either has the best hand or nothing, betting into an opponent who can only
beat a bluff. The question is how often each should bluff or call so that neither can do better.
:::

::: advanced
This is a version of the classic "clairvoyance" game: the bettor knows who wins, and the
defender only knows the bettor's range. Its solution has the indifference structure below.
Real spots differ: ranges contain medium-strength hands, there can be raises, and earlier streets
shape which hands arrive.
:::

## Alpha and minimum defence frequency {#mdf}

::: simple
A bluff risks the bet to win the pot. A pot-sized bluff wins 1 and loses 1, so it breaks even if
the opponent folds half the time. If they fold more than that, bluffing with any two cards
makes money. So the defender must **continue at least half the time** against a pot-sized bet.
:::

::: intermediate
- **Alpha**, the fold frequency a bluff needs: $\alpha = B / (P + B)$.
- **Minimum defence frequency**, the share the defender must continue with so bluffs can't
  profit: $\text{MDF} = 1 - \alpha = P / (P + B)$.

Half pot: alpha 33.3%, MDF 66.7%. Pot: 50% and 50%. Double pot: 66.7% and 33.3%.
:::

::: math
A pure bluff that is called always loses $B$ and wins $P$ when the defender folds with
probability $f$:

$$
\mathrm{EV}_{\text{bluff}} = f P - (1 - f) B = 0 \iff f = \alpha = \frac{B}{P + B}.
$$

The defender continues with probability $1 - f$, so bluffs gain nothing exactly when

$$
1 - f \ge \text{MDF} = \frac{P}{P + B}.
$$
:::

```riverlab:bet-math
{"pot_bb": "10", "bet_bb": "10"}
```

## Bluff-to-value ratio {#ratio}

::: simple
The caller needs to be right often enough. Calling a pot bet risks 1 to win 2, so the caller
needs to win a third of the time. A bettor who wants to make calling and folding equal bluffs
with **one hand for every two value hands**.
:::

::: intermediate
A call risks $B$ to win $P + B$, so the caller needs equity $B / (P + 2B)$. Against a polarised
range the caller's equity is the share of bluffs, so the indifferent bluff share is
$B / (P + 2B)$, or $B / (P + B)$ bluffs per value bet.

- Half pot: bluffs are 25% of bets (1 bluff per 3 value bets).
- Pot: 33.3% (1 per 2).
- Double pot: 40% (2 per 3).
:::

::: math
With bluff share $s$ the caller wins $P + B$ with probability $s$ and loses $B$ otherwise:

$$
\mathrm{EV}_{\text{call}} = s (P + B) - (1 - s) B = 0 \iff s = \frac{B}{P + 2B}.
$$

So bluffs per value bet are $\frac{s}{1 - s} = \frac{B}{P + B}$. Larger bets allow more bluffs,
and require the defender to fold more, which is why polarised ranges can use large sizes.
:::

## Reading the results {#reading}

::: heuristic
If an opponent folds more than alpha, bluff more; if they call more than MDF, bluff less and
value-bet thinner. That is an **exploitative** adjustment. The equilibrium of the model is the
point where neither side can gain by changing.
:::

::: approximation
Do not defend exactly MDF in every spot. MDF only stops **pure bluffs** from profiting in the
single-bet model. With real ranges, some of the defender's hands beat value bets, others lose to
bluffs, and later streets exist. Solver results often defend more or less than MDF for good
reasons. Use the Solver to compare a real river spot with these numbers.
:::
