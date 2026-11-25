+++
slug = "expected-value"
title = "Expected value"
level = 2
order = 1
version = 2
status = "published"
summary = "Judging a decision by its average result over all the ways it can turn out, measured from the moment you make it."
estimated_minutes = 15
objectives = [
  "Compute the expected value of a call from equity, pot and price.",
  "Measure EV from the decision point, treating chips already in the pot as no longer yours.",
  "Separate the quality of a decision from the result of one hand.",
]
prerequisites = ["pot-odds", "equity"]
tags = ["math", "expected-value"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every numeric answer is computed by poker-theory (ev_call) and re-checked when the content is synced; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/play"
label = "Play against bots"
why = "The session report's all-in line compares what you won with what you were expected to win."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Expectation is the foundation of Part I."

[[questions]]
id = "ev-call-plus"
type = "numeric"
prompt = "River. The pot is 20bb and the button bets 10bb; you are in the big blind. Your equity against the hands that bet like this is 30%. What is the expected value (in bb) of calling, compared with folding?"
answer = 2.0
tolerance = 0.01
unit = "bb"
explanation = "Calling costs 10bb and wins a 40bb pot 30% of the time: EV = 0.30 × 40 − 10 = +2bb. Folding is worth 0 from this point."
concepts = ["expected-value", "pot-odds"]
compute = { kind = "ev_call", params = { equity = 0.3, pot_mbb = 30000, to_call_mbb = 10000 }, field = "", scale = 0.001 }

[questions.scene]
players = 6
hero = "BB"
hero_cards = ["Jc", "Jd"]
board = ["Qs", "8h", "4d", "3c", "Kh"]
pot_bb = "20"
bets_bb = { BTN = "10" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BTN = "bets 10bb" }

[[questions]]
id = "ev-call-minus"
type = "numeric"
prompt = "River. The pot is 20bb and the button bets 10bb; you are in the big blind. Your equity against the hands that bet like this is only 20%. What is the expected value (in bb) of calling, compared with folding?"
answer = -2.0
tolerance = 0.01
unit = "bb"
explanation = "EV = 0.20 × 40 − 10 = −2bb. The call needs 25% equity; at 20% it loses 2bb on average, so folding (0) is better."
concepts = ["expected-value", "pot-odds"]
compute = { kind = "ev_call", params = { equity = 0.2, pot_mbb = 30000, to_call_mbb = 10000 }, field = "", scale = 0.001 }

[[questions]]
id = "simple-bet"
type = "numeric"
prompt = "A proposition pays you 3bb when you win and costs you 1bb when you lose. You win 30% of the time. What is its expected value (in bb) per play?"
answer = 0.2
tolerance = 0.001
unit = "bb"
explanation = "EV = 0.30 × 3 − 0.70 × 1 = 0.9 − 0.7 = +0.2bb. Winning less than half the time can still be profitable when the wins are bigger than the losses."
concepts = ["expected-value"]
compute = { kind = "ev_call", params = { equity = 0.3, pot_mbb = 3000, to_call_mbb = 1000 }, field = "", scale = 0.001 }

[[questions]]
id = "sunk"
type = "multiple_choice"
prompt = "You raised before the flop and have 12bb in the pot. On the river you face a bet. Why is folding worth 0, not −12bb, when you compare your options?"
options = [
  "Because folding gets your 12bb back",
  "Because the 12bb has already left your stack whatever you do now; you compare options by what each one changes from here",
  "Because the blinds are refunded when you fold",
]
answer = 1
explanation = "The 12bb left your stack before this decision, and no choice you make now changes that. So measure each option from the stack you have now: folding changes nothing (0), and calling is worth what you win from the pot on average minus what the call costs. The 12bb still matters, but only as part of the pot a call can win."
concepts = ["expected-value"]

[[questions]]
id = "result-vs-decision"
type = "decision"
prompt = "On the river you call a bet getting a price that needs 25% equity, with 40% equity against the betting range. There is no rake. Your opponent shows a better hand and you lose the pot. Was the call a mistake?"
options = [
  "Yes: you lost, so you should have folded",
  "No: the call was worth more than folding on average; with 40% equity, losing is a common outcome",
  "It is impossible to say",
]
answer = 1
explanation = "Decisions are judged by their expected value with the information you had. A call with less than 50% equity fails to win the whole pot more often than not, even when it is positive EV; over many such spots it gains."
concepts = ["expected-value"]
+++

## Averages over outcomes {#definition}

::: fact
The **expected value** (EV) of a choice is the average of its results over all the ways the
hand can turn out, each weighted by its probability.
:::

::: simple
If a bet wins 3 when it wins and loses 1 when it loses, and it wins 30% of the time, then on
average it earns 0.3 × 3 − 0.7 × 1 = 0.2 per play. You will never win exactly 0.2 in one
play; it is what you earn per play over many.
:::

::: math
For outcomes $i$ with probabilities $p_i$ and results $x_i$,

$$ \mathrm{EV} = \sum_i p_i\, x_i . $$
:::

```riverlab:question
{"id": "simple-bet"}
```

## The EV of a call {#call}

::: fact
Measured from the decision point, folding is worth 0. Calling $C$ into a pot of $P$ (the bet
included) with equity $e$, with no more betting to come and no rake, is worth
$e\,(P + C) - C$. Equity counts a split pot as half a win.
:::

::: intermediate
The call is better than folding exactly when this is positive, which is the pot-odds rule from
the pot-odds lesson: the equity must exceed $C / (P + C)$.
:::

```riverlab:question
{"id": "ev-call-plus"}
```

```riverlab:question
{"id": "ev-call-minus"}
```

## What is not part of EV {#sunk}

::: fact
Chips you have already put in the pot are not yours any more: they left your stack whatever
you choose now. So measure every option from your current stack. Folding is worth 0; calling is
worth what you win from the pot on average minus what you pay now. The chips you put in earlier
count only as part of the pot you might win.
:::

```riverlab:question
{"id": "sunk"}
```

## Decisions and results {#results}

::: fact
A single result says little about a decision. With no rake, a call with 40% equity at a price
needing 25% is worth more than folding on average, yet it fails to win the whole pot more often
than not.
:::

::: heuristic
Review hands by asking what the decision was worth with the information you had, not whether
it won. RiverLab's session report separates the two for all-ins: what you won against what you
were expected to win.
:::

```riverlab:question
{"id": "result-vs-decision"}
```
