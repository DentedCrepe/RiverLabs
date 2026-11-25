+++
slug = "outs-and-draws"
title = "Outs and drawing odds"
level = 1
order = 7
version = 3
status = "published"
summary = "Count the cards that improve you, turn them into exact probabilities, and know when the rule of 2 and 4 is close enough."
estimated_minutes = 15
objectives = [
  "Count outs for common draws.",
  "Compute the exact chance of hitting on the next card and by the river.",
  "Compare the rule of 2 and 4 with the exact figures, and know its assumptions.",
]
prerequisites = ["hand-rankings", "pot-odds"]
tags = ["math", "outs", "draws", "probability"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every probability is computed by poker-theory and re-checked on sync; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Outs are a shortcut for equity: check a draw's real equity against a hand or range."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Probability foundations in Part I."

[[questions]]
id = "flush-draw-next"
type = "numeric"
prompt = "Flop. You hold A♥ T♥ on K♠ 7♥ 2♥: a flush draw with 9 outs and 47 unseen cards. What is the exact chance (in %) that the turn card completes your flush?"
answer = 19.15
tolerance = 0.1
unit = "%"
explanation = "9 of the 47 unseen cards help: 9 / 47 ≈ 19.15%."
concepts = ["outs-and-draws"]
compute = { kind = "outs", params = { outs = 9, unseen = 47 }, field = "next_card", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Th"]
board = ["Ks", "7h", "2h"]
pot_bb = "6.5"
folded = ["UTG", "HJ", "CO", "SB"]

[[questions]]
id = "flush-draw-river"
type = "numeric"
prompt = "Flop. You hold A♥ T♥ on K♠ 7♥ 2♥ (9 outs, 47 unseen cards) and the money is all in, so you will see both the turn and the river. What is the exact chance (in %) that you make the flush by the river?"
answer = 34.97
tolerance = 0.1
unit = "%"
explanation = "Miss both cards: (38/47) × (37/46) ≈ 65.03%, so you hit with 1 − 0.6503 ≈ 34.97%."
concepts = ["outs-and-draws"]
compute = { kind = "outs", params = { outs = 9, unseen = 47 }, field = "two_cards", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Th"]
board = ["Ks", "7h", "2h"]
actions = { BTN = "all-in", BB = "calls all-in" }
folded = ["UTG", "HJ", "CO", "SB"]

[[questions]]
id = "gutshot"
type = "numeric"
prompt = "Turn. You hold 9♣ 8♣ on Q♦ J♥ 5♠ 2♣: only a ten makes your straight, so, assuming every ten wins, you have 4 outs among 46 unseen cards. What is the exact chance (in %) that the river is a ten?"
answer = 8.70
tolerance = 0.1
unit = "%"
explanation = "4 / 46 ≈ 8.70%. In a real hand not every ten is a clean out: an opponent holding A K or K 9 would make a higher straight with it."
concepts = ["outs-and-draws"]
compute = { kind = "outs", params = { outs = 4, unseen = 46 }, field = "next_card", scale = 100.0 }

[questions.scene]
players = 6
hero = "CO"
hero_cards = ["9c", "8c"]
board = ["Qd", "Jh", "5s", "2c"]
pot_bb = "12"
folded = ["UTG", "HJ", "BTN", "SB"]

[[questions]]
id = "rule-of-four-when"
type = "multiple_choice"
prompt = "The 'rule of 4' estimates your chance of hitting as outs × 4%. When is it meant to be used?"
options = [
  "For the next card only",
  "On the flop, when you will see both the turn and the river (e.g. all-in)",
  "On the river",
]
answer = 1
explanation = "×4 approximates the chance over two cards. If you may face another bet on the turn, use the next-card figure (×2)."
concepts = ["outs-and-draws"]

[[questions]]
id = "rule-of-four-seven"
type = "numeric"
prompt = "Flop. You have 7 outs among 47 unseen cards and the money is all in, so you will see the turn and the river. What is the exact chance (in %) that you hit by the river? Compare it with the rule-of-4 estimate, 7 × 4 = 28%."
answer = 27.84
tolerance = 0.1
unit = "%"
explanation = "You miss both cards with (40/47) × (39/46) ≈ 72.16%, so you hit with about 27.84%: just below the rule's 28%. From 7 outs on, the rule of 4 is a little too high, and the gap grows with more outs."
concepts = ["outs-and-draws"]
compute = { kind = "outs", params = { outs = 7, unseen = 47 }, field = "two_cards", scale = 100.0 }
+++

## What an out is {#outs}

::: simple
An **out** is a card that would give you the best hand. With four hearts after the flop
(two in your hand, two on the board), any of the 9 remaining hearts makes your flush: 9 outs.
:::

::: intermediate
Count the unseen cards that turn your hand into the winner. Common draws:
a flush draw has 9 outs, an open-ended straight draw 8, a gutshot 4. Cards that help you
**and** your opponent more (for example, a heart that pairs the board, which can give an
opponent holding a set a full house) are not clean outs.
:::

::: advanced
"Unseen" means every card you have not seen: 52 − 2 (your hand) − 3 (the flop) = 47 on the
flop, 46 on the turn. The opponent's cards are among them, but because you don't know which
they are, each unseen card is equally likely to come next.
:::

::: approximation
Counting outs assumes every out wins and every other card loses. Real equity also includes
backdoor draws, ties, and cards that look like outs but lose to a better hand. The equity
calculator measures all of that against a range.
:::

## Exact probabilities {#exact}

::: simple
On the flop, your chance of hitting on the turn is **outs ÷ 47**. With 9 outs that is about 19%.
:::

::: intermediate
- Next card on the flop: $\text{outs} / 47$. On the turn: $\text{outs} / 46$.
- Both cards (flop to river, when you are all-in): $1 - $ the chance of missing twice.

For a flush draw that is $1 - \tfrac{38}{47} \cdot \tfrac{37}{46} \approx 34.97\%$.
:::

::: math
With $o$ outs among $u$ unseen cards, the chance of hitting on the next card is $o/u$. Over two
cards, you miss only if both come from the $u - o$ blanks:

$$
P(\text{hit by the river}) = 1 - \frac{\binom{u - o}{2}}{\binom{u}{2}} = 1 - \frac{(u-o)(u-o-1)}{u(u-1)}.
$$
:::

```riverlab:outs
{"outs": 9, "unseen": 47}
```

## The rule of 2 and 4 {#rule}

::: heuristic
Multiply your outs by **2** for the chance on the next card, or by **4** for the chance over two
cards (flop to river). It is a quick estimate, not an exact figure.
:::

::: intermediate
The two halves of the rule err in different ways:
- **×2** is always slightly low. On the flop each out is 1 card in 47, about 2.13% (1 ÷ 47),
  not 2%; on the turn it is 1 in 46.
- **×4** is slightly low for up to 6 outs and too high from 7 outs on, and the error grows with
  the number of outs, because the two chances overlap (you can hit on both cards).

Try 6, 7, 9 and 15 outs in the calculator above: the exact two-card figure for 15 outs is well
below 4 × 15 = 60%.
:::

::: advanced
Use the two-card figure only when you will certainly see both cards: when the money is all in
on the flop. If there may be another bet on the turn, the price you pay now buys only one card,
so compare your pot odds with the one-card figure.
:::

```riverlab:question
{"id": "rule-of-four-seven"}
```
