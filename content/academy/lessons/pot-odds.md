+++
slug = "pot-odds"
title = "Pot odds and break-even equity"
level = 1
order = 5
version = 2
status = "published"
summary = "How much equity a call needs, and why the answer depends only on the price you are offered."
estimated_minutes = 15
objectives = [
  "Compute the equity a call needs from the pot and the bet you face.",
  "Convert between odds ratios and percentages.",
  "Explain which assumptions make the break-even figure exact.",
]
prerequisites = ["terminology", "equity"]
tags = ["math", "pot-odds", "equity", "calling"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every number is produced by poker-theory and re-checked when the content is synced; awaiting review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Find your equity against a hand or range, then compare it with the price."

[[tools]]
href = "/play"
label = "Play against bots"
why = "The session report grades your river calls and folds against exactly this break-even figure."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Part I covers pot odds and expectation in depth."

[[questions]]
id = "half-pot"
type = "numeric"
prompt = "River. The pot is 20bb and the button bets 10bb; everyone else has folded. You are in the big blind. What equity (in %) does a call need to break even? (No more betting can follow on the river.)"
answer = 25.0
tolerance = 0.5
unit = "%"
explanation = "You call 10bb to win a final pot of 20 + 10 + 10 = 40bb, so you need 10 / 40 = 25%."
concepts = ["pot-odds"]
compute = { kind = "pot_odds", params = { pot_mbb = 30000, to_call_mbb = 10000 }, field = "required_equity", scale = 100.0 }

[questions.scene]
players = 6
hero = "BB"
hero_cards = ["Qh", "Qc"]
board = ["Ks", "7d", "2c", "8h", "3s"]
pot_bb = "20"
bets_bb = { BTN = "10" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BTN = "bets 10bb" }

[[questions]]
id = "pot-bet"
type = "numeric"
prompt = "River. The pot is 12bb and your opponent bets 12bb, the size of the pot. What equity (in %) does a call need to break even?"
answer = 33.33
tolerance = 0.5
unit = "%"
explanation = "Pot P, bet P: you call P to win a final pot of 3P, so you need 1/3 ≈ 33.3%."
concepts = ["pot-odds"]
compute = { kind = "pot_odds", params = { pot_mbb = 24000, to_call_mbb = 12000 }, field = "required_equity", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Jd", "Js"]
board = ["Ah", "9c", "6d", "4s", "2h"]
pot_bb = "12"
bets_bb = { BB = "12" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "bets 12bb" }

[[questions]]
id = "river-decision"
type = "decision"
prompt = "River. Your opponent bets half the pot. You estimate that against the hands they would bet this way your equity is 30%. Ignoring rake, should you call or fold?"
options = [
  "Fold, because 30% is less than 50%",
  "Call, because 30% is more than the 25% a half-pot bet requires",
  "It makes no difference",
]
answer = 1
explanation = "A half-pot bet needs 25% equity. With 30% the call wins more than it costs on average: EV = 0.30 × (final pot) − call > 0."
concepts = ["pot-odds", "expected-value"]

[[questions]]
id = "which-equity"
type = "multiple_choice"
prompt = "Facing a bet, which equity should you compare with the equity the price requires?"
options = [
  "Your equity against a random hand",
  "Your equity against the hands your opponent would bet this way",
  "Your equity against the hand your opponent showed last time",
]
answer = 1
explanation = "The bet tells you something: compare with your equity against the range of hands that make this bet, not against all hands."
concepts = ["pot-odds", "equity"]
+++

## The price of a call {#price}

::: simple
When someone bets, you are offered a deal: put in the call to try to win everything in the
middle. **Pot odds** compare what you must pay with what you can win. If the pot (including
their bet) is 15bb and you must call 5bb, you risk 5 to win 15: odds of **3 to 1**.
:::

::: intermediate
Pot odds are the ratio of the pot you can win to the amount you must call. With the bet
already in the middle, a pot of 15bb and a 5bb call give 15 : 5 = 3 : 1. The more the pot
pays relative to the call, the less often the call has to win.
:::

::: advanced
Write the pot before the bet as $P$ and the bet as $B$. Facing the bet, the pot is $P + B$ and
the call costs $B$, so the odds are $(P + B) : B$. The final pot after a call is $P + 2B$, and
the call is a wager of $B$ for a share of $P + 2B$.
:::

::: math
Let the pot before the bet be $P$ and the bet $B$. Calling adds $B$, making the final pot
$P + 2B$. The odds offered are

$$
\text{odds} = \frac{P + B}{B}.
$$
:::

## Break-even equity {#break-even}

::: simple
To know whether a call is good, turn the price into a percentage: **call ÷ final pot**. Calling
5 into a final pot of 20 means you need to win at least 5 ÷ 20 = **25%** of the time.
:::

::: intermediate
The equity a call needs is the call divided by the final pot:

$$\text{required equity} = \frac{\text{call}}{\text{pot after you call}}.$$

Common bet sizes give round numbers: a third-pot bet needs 20%, a half-pot bet 25%, a
two-thirds-pot bet about 28.6%, and a pot-sized bet 33.3%.
:::

::: advanced
If your share of the final pot is $e$, calling is worth $e(P + 2B) - B$ compared with folding.
Setting this to zero gives the break-even equity $e^* = B / (P + 2B)$. It depends only on the
**ratio** $B / P$, so it is the same at any stakes.
:::

::: math
The expected value of calling, relative to folding (which is worth 0 from here on), is

$$
\mathrm{EV}_{\text{call}} = e\,(P + 2B) - B .
$$

It is positive exactly when

$$
e > e^* = \frac{B}{P + 2B} = \frac{\beta}{1 + 2\beta}, \qquad \beta = \frac{B}{P}.
$$

For $\beta = \tfrac12$, $e^* = \tfrac{1/2}{2} = 25\%$; for $\beta = 1$, $e^* = \tfrac13$. As $\beta \to \infty$,
$e^* \to \tfrac12$: however large the bet, a call never needs more than 50%.
:::

```riverlab:pot-odds
{"pot_bb": "10", "bet_bb": "5"}
```

## When the figure is exact {#assumptions}

::: fact
The break-even equity is exact when **no more money can go in after your call**: on the river,
or when the call puts you or your opponent all-in.
:::

::: approximation
Earlier in the hand it is only a guide. Future bets change what you win and lose
(**implied odds** and **reverse implied odds**), and you will not always realise your equity:
you may fold the best hand later, or be outplayed. Rake also reduces the final pot, which raises
the equity you need.
:::

::: simple
So on the flop or turn, treat the number as a starting point, not the full answer.
:::

::: advanced
Equity here means your share of the pot against the **range** of hands your opponent would bet
this way, not against their exact cards and not against a random hand. A good call compares a
realistic range estimate with the price.
:::
