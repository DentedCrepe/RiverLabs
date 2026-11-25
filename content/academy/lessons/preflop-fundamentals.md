+++
slug = "preflop-fundamentals"
title = "Preflop fundamentals"
level = 1
order = 4
version = 3
status = "published"
summary = "How a preflop hand unfolds: forced blinds, opens, 3-bets, and the price the big blind gets when defending."
estimated_minutes = 15
objectives = [
  "Follow the order of preflop action and what each seat has already put in.",
  "Work out the price of calling an open or a 3-bet.",
  "Explain why the big blind can defend more hands than other seats.",
]
prerequisites = ["terminology", "positions"]
tags = ["preflop", "pot-odds"]

[review]
status = "pending"
reviewed_on = ""
reviewer = ""
notes = "Drafted with AI assistance. Prices are computed by poker-theory (pot_odds) and the table scenes are checked against them on sync; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/preflop"
label = "Preflop"
why = "Run GTOpen's preflop model (an approximate model) for your own table and sizes."

[[tools]]
href = "/trainer"
label = "Trainer"
why = "Preflop drills from your preflop solves, and push/fold drills graded against RiverLab's computed heads-up push/fold equilibrium (blinds 0.5/1, equal stacks of 2bb to 25bb, no ante, no rake), shown with its exploitability."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[[questions]]
id = "bb-price"
type = "numeric"
prompt = "Six-max, before the flop. Everyone folds to the button, who raises to 2.5bb. The small blind folds. You are in the big blind with 1bb already in. What share of the pot after your call (in %) must you win to break even on a call, ignoring the rest of the hand?"
answer = 27.27
tolerance = 0.05
unit = "%"
explanation = "You call 1.5bb to win a pot of 0.5 + 1 + 2.5 = 4bb: 1.5 / (4 + 1.5) ≈ 27.3%. Your 1bb blind is already in the pot, which is why the price is good."
concepts = ["pot-odds", "preflop"]
compute = { kind = "pot_odds", params = { pot_mbb = 4000, to_call_mbb = 1500 }, field = "required_equity", scale = 100.0 }

[questions.scene]
players = 6
hero = "BB"
hero_cards = ["9h", "7h"]
pot_bb = "0.5"
bets_bb = { BTN = "2.5", BB = "1" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BTN = "raises to 2.5bb", SB = "folds" }

[[questions]]
id = "facing-3bet"
type = "numeric"
prompt = "Six-max, before the flop. You open to 2.5bb on the button, the small blind folds and the big blind 3-bets to 10bb. What share of the pot after your call (in %) must you win to break even on a call, ignoring the rest of the hand?"
answer = 36.59
tolerance = 0.05
unit = "%"
explanation = "You call 7.5bb more to win 0.5 + 2.5 + 10 = 13bb: 7.5 / (13 + 7.5) ≈ 36.6%. Calling this 3-bet needs more equity than defending the big blind against a 2.5bb open (about 27.3%)."
concepts = ["pot-odds", "preflop"]
compute = { kind = "pot_odds", params = { pot_mbb = 13000, to_call_mbb = 7500 }, field = "required_equity", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ad", "Jc"]
pot_bb = "0.5"
bets_bb = { BTN = "2.5", BB = "10" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BTN = "raises to 2.5bb", BB = "3-bets to 10bb" }

[[questions]]
id = "first-to-act"
type = "multiple_choice"
prompt = "Six-max, before the flop. Which player acts first, and which acts last if nobody raises?"
options = ["Under the gun first, the big blind last", "The small blind first, the button last", "The button first, the big blind last"]
answer = 0
explanation = "The blinds have already posted, so action starts to the big blind's left (under the gun) and goes around to the big blind, who can check if nobody has raised. This is the order for the first betting round only; from the flop on, the first player still in the hand to the left of the button acts first."
concepts = ["positions", "preflop"]

[[questions]]
id = "why-bb-defends"
type = "multiple_choice"
prompt = "Why does the big blind usually call opens with more hands than, say, the cutoff would?"
options = [
  "Its blind is already in the pot, so calling is cheaper relative to what it can win, and nobody acts after it preflop",
  "The big blind always has the best hand",
  "The big blind acts last after the flop",
]
answer = 0
explanation = "Part of the price is already paid, and it closes the preflop action, so no one can raise behind it. After the flop it is out of position against everyone but the small blind, which works the other way."
concepts = ["preflop", "positions"]
+++

## How a preflop hand unfolds {#order}

::: fact
Before the cards are dealt the small blind and big blind post their blinds (in RiverLab, 0.5bb
and 1bb). Action starts under the gun and moves left. Each player folds, calls the current
amount, or raises. The big blind counts as the first bet, so the first raise (the **open**) is
the second bet, a re-raise is a **3-bet**, and the next a **4-bet**. In this first betting round
the big blind acts last: if nobody has raised, it can check. From the flop on, the order changes
(see the positions lesson).
:::

```riverlab:question
{"id": "first-to-act"}
```

## The price of calling {#price}

::: fact
Say you must call $C$ and the pot already holds $P$, counting every bet made so far (the blinds
and the raise you face). If you call, the pot after your call is $P + C$, and you have paid $C$
of it. Ignoring later betting and rake, the call breaks even when your share of that pot covers
what you paid: you need $\text{equity} \ge \frac{C}{P + C}$. (Your **equity** is the share of
the pot you would win on average if all the cards were dealt out, with a split pot counting as
half; the equity lesson covers it.)
:::

::: math
With equity $e$, calling wins on average $e\,(P + C)$ and costs $C$, measured from your current
stack (folding is 0). So, with no later betting and no rake,

$$ e\,(P + C) - C \ge 0 \quad\Longleftrightarrow\quad e \ge \frac{C}{P + C}. $$
:::

```riverlab:question
{"id": "bb-price"}
```

```riverlab:question
{"id": "facing-3bet"}
```

::: approximation
With betting still to come, a hand does not simply win its preflop equity. Position, the rest of the betting and how well a hand
plays after the flop all change it (equity realisation, a later lesson). The price tells you how much
equity a call needs before those effects.
:::

## Opening rather than limping {#open}

::: heuristic
Raising first in usually does better than calling the big blind ("limping"): a raise can win the
blinds straight away, and when called it builds a pot you started with the initiative. Exact
preflop ranges depend on the table, stacks and rake; RiverLab shows them from GTOpen's preflop
model, which is an approximate model, not a solved game.
:::

```riverlab:question
{"id": "why-bb-defends"}
```
