+++
slug = "fold-equity"
title = "Fold equity and semi-bluffs"
level = 2
order = 3
version = 1
status = "published"
summary = "How much a bet gains when your opponent folds, and why a draw needs fewer folds than a pure bluff."
estimated_minutes = 15
objectives = [
  "Compute how often a pure bluff must work to break even.",
  "Compute the EV of a semi-bluff from fold frequency and equity when called.",
  "State the assumptions of the single-bet model and where it stops being exact.",
]
prerequisites = ["expected-value", "equity"]
tags = ["math", "bluffing", "fold-equity"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every numeric answer is computed by poker-theory (bet_math, semi_bluff) and re-checked when the content is synced; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "See how often an approximate equilibrium bets each draw, and how often the opponent folds."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[questions]]
id = "pure-bluff"
type = "numeric"
prompt = "Turn. The pot is 10bb and you bet 7.5bb with a hand that can never win if called. How often (in %) must your opponent fold for the bet to break even?"
answer = 42.86
tolerance = 0.05
unit = "%"
explanation = "You risk 7.5bb to win 10bb: break-even fold frequency = 7.5 / (10 + 7.5) ≈ 42.86%."
concepts = ["fold-equity"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 7500 }, field = "alpha", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["7d", "6c"]
board = ["As", "Kc", "9h", "2s"]
pot_bb = "10"
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "checks" }

[[questions]]
id = "semi-break-even"
type = "numeric"
prompt = "Turn. The pot is 10bb and you bet 7.5bb, all in, with a draw that wins 20% of the time when called (no more betting is possible). How often (in %) must your opponent fold for the bet to break even?"
answer = 20.0
tolerance = 0.05
unit = "%"
explanation = "When called you put in 7.5bb and win 20% of a 25bb pot: 0.2 × 25 − 7.5 = −2.5bb. When they fold you win 10bb. Break even: f × 10 = (1 − f) × 2.5, so f = 2.5 / 12.5 = 20%. A pure bluff needed 42.86%."
concepts = ["fold-equity"]
compute = { kind = "semi_bluff", params = { pot_mbb = 10000, bet_mbb = 7500, fold_frequency = 0.0, equity = 0.2 }, field = "break_even_fold", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Qs", "Js"]
board = ["As", "8c", "4s", "2h"]
pot_bb = "10"
stacks_bb = { BTN = "7.5" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "checks" }

[[questions]]
id = "semi-ev"
type = "numeric"
prompt = "Turn. The pot is 10bb and you bet 7.5bb, all in, with a draw that wins 20% of the time when called. Your opponent folds 40% of the time. What is the bet's expected value (in bb), compared with checking and giving up?"
answer = 2.5
tolerance = 0.01
unit = "bb"
explanation = "EV = 0.40 × 10 + 0.60 × (0.20 × 25 − 7.5) = 4 − 1.5 = +2.5bb."
concepts = ["fold-equity", "expected-value"]
compute = { kind = "semi_bluff", params = { pot_mbb = 10000, bet_mbb = 7500, fold_frequency = 0.4, equity = 0.2 }, field = "ev_mbb", scale = 0.001 }

[[questions]]
id = "why-fewer"
type = "multiple_choice"
prompt = "Why does a semi-bluff with a draw need fewer folds to break even than a pure bluff of the same size?"
options = [
  "Because opponents fold more often against draws",
  "Because when it is called it still wins part of the time, so being called costs less",
  "Because draws are always ahead",
]
answer = 1
explanation = "The bet's EV is folds × pot plus calls × (EV when called). With equity, the second term is less negative, so fewer folds are needed to make the total positive."
concepts = ["fold-equity"]
+++

## Winning without a showdown {#fold-equity}

::: fact
When you bet and your opponent folds, you win the pot whatever your cards. **Fold equity** is
the part of a bet's value that comes from those folds.
:::

::: simple
A bet that never wins when called can still make money if it gets enough folds: you risk the
bet to win the pot.
:::

::: math
A pure bluff of $B$ into a pot $P$ that is called with probability $1 - f$ is worth
$f P - (1 - f) B$. It breaks even at

$$ f^* = \alpha = \frac{B}{P + B}. $$
:::

```riverlab:question
{"id": "pure-bluff"}
```

## Semi-bluffs {#semi-bluff}

::: fact
A **semi-bluff** is a bet with a hand that is probably behind when called but can still
improve to win. When called it wins its equity of the final pot instead of nothing.
:::

::: math
With equity $e$ when called and no later betting, the bet is worth

$$ \mathrm{EV} = f P + (1 - f)\,\big(e (P + 2B) - B\big). $$

If $e(P + 2B) - B \ge 0$ the bet is profitable even when always called. Otherwise it breaks
even at

$$ f^* = \frac{B - e(P + 2B)}{P + B - e(P + 2B)} . $$
:::

```riverlab:question
{"id": "semi-break-even"}
```

```riverlab:question
{"id": "semi-ev"}
```

## Where the model stops {#limits}

::: approximation
These formulas are exact only when the bet ends the betting (an all-in, or the river with no
raise). Earlier in the hand a semi-bluff that is called can still win more on later streets
when it hits, or lose more when it misses, and the opponent may raise instead of calling.
:::

::: heuristic
Draws with good equity are often good bets because both parts of the formula work for them. How
often to bet them, and with which size, is a strategy question that solvers answer for each
spot.
:::

```riverlab:question
{"id": "why-fewer"}
```
