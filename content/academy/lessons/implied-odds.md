+++
slug = "implied-odds"
title = "Implied and reverse implied odds"
level = 2
order = 4
version = 2
status = "published"
summary = "When a draw can call without the direct price, because of what it wins later; and when hitting can cost you."
estimated_minutes = 15
objectives = [
  "Compute how much you must win later to justify a call the pot odds alone do not.",
  "Recognise when later winnings are unlikely (short stacks, obvious draws).",
  "Explain reverse implied odds with examples.",
]
prerequisites = ["pot-odds", "outs-and-draws", "expected-value"]
tags = ["math", "implied-odds", "draws"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every numeric answer is computed by poker-theory (pot_odds, implied_odds) and re-checked when the content is synced; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Check how often a draw really wins against a range, including the times it hits and still loses."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[questions]]
id = "direct-price"
type = "numeric"
prompt = "Turn. The pot is 10bb and your opponent bets 5bb. You have a flush draw: 9 outs among 46 unseen cards, and you assume you win if and only if the river is a heart. There is no rake. What equity (in %) does a call need on the direct price alone?"
answer = 25.0
tolerance = 0.05
unit = "%"
explanation = "You call 5bb into a pot of 15bb (the bet included) to win a final pot of 20bb: 5 / 20 = 25%. Your draw hits 9 / 46 ≈ 19.6% of the time, so the direct price is not enough."
concepts = ["implied-odds", "pot-odds"]
compute = { kind = "pot_odds", params = { pot_mbb = 15000, to_call_mbb = 5000 }, field = "required_equity", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Jh"]
board = ["Kc", "7h", "2h", "4s"]
pot_bb = "10"
bets_bb = { BB = "5" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "bets 5bb" }

[[questions]]
id = "needed-later"
type = "numeric"
prompt = "Turn. The pot is 10bb and your opponent bets 5bb. Your flush draw hits 9 times in 46, and you assume you win if and only if it hits. If you call and miss, you give up. There is no rake. How much (in bb) must you win on average from later bets when you hit, for the call to break even?"
answer = 5.56
tolerance = 0.01
unit = "bb"
explanation = "Break-even when 9/46 × (15 + W) = 37/46 × 5, so W = 37 × 5 / 9 − 15 ≈ 5.56bb. You need to win about 5.6bb more, on average, on the river when the flush comes."
concepts = ["implied-odds"]
compute = { kind = "implied_odds", params = { pot_mbb = 15000, to_call_mbb = 5000, equity = 0.1956521739130435 }, field = "needed_future_mbb", scale = 0.001 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Jh"]
board = ["Kc", "7h", "2h", "4s"]
pot_bb = "10"
bets_bb = { BB = "5" }
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "bets 5bb" }

[[questions]]
id = "short-stack"
type = "decision"
prompt = "Turn. The pot is 10bb and your opponent bets 5bb, leaving them only 3bb behind. Your flush draw hits 9 times in 46 and you assume you win if and only if it hits. There is no rake. Should you call?"
options = [
  "Call: implied odds make up the difference",
  "Fold: even winning their last 3bb every time you hit is less than the 5.56bb you would need",
  "Call: draws should always be played",
]
answer = 1
explanation = "The call needs about 5.56bb of extra winnings when you hit. Your opponent can only lose 3bb more, so even in the best case the call loses money."
concepts = ["implied-odds"]

[[questions]]
id = "reverse"
type = "multiple_choice"
prompt = "Flop K♥ 9♥ 2♣. Which of these hands has the clearest reverse implied odds?"
options = [
  "A♥ Q♥: a draw to the best possible flush",
  "6♥ 5♥: a draw to a low flush that loses to any higher flush",
  "K♠ K♦: top set",
]
answer = 1
explanation = "Reverse implied odds are what you lose when you make or hold a hand that is often second best. When the flush comes, 6♥ 5♥ can lose a big pot to any higher heart; the ace-high flush cannot be beaten by another flush, and top set is the best hand now."
concepts = ["implied-odds", "reverse-implied-odds"]
+++

## Paying for a draw {#implied}

::: fact
**Implied odds** count the money you expect to win on later streets when your draw hits. A call
that loses money on the direct price can still be profitable if hitting wins enough more later.
:::

::: math
Call $C$ into a pot $P$ (the bet included) with a draw that hits with probability $e$. If you hit
you win the pot plus an extra $W$ on later streets; if you miss you give up. With no rake, the
call breaks even when

$$ e\,(P + C + W) = C \quad\Longleftrightarrow\quad W = \frac{(1 - e)\,C}{e} - P . $$

The pot-odds figure is the special case $W = 0$.
:::

```riverlab:question
{"id": "direct-price"}
```

```riverlab:question
{"id": "needed-later"}
```

## When implied odds are small {#limits}

::: fact
You can never win more on later streets than the effective stack after your call: the smaller
of what you and your opponent have left. In a multiway pot, the limit is what your opponents have
left between them, each counted only up to what you have left.
:::

::: heuristic
Implied odds are smaller when stacks are short, when the draw is obvious (a third card of a suit
often stops the action), and against opponents who rarely pay off. They are larger when the hand
you make is well hidden and your opponent is likely to have a strong but second-best hand.
:::

```riverlab:question
{"id": "short-stack"}
```

## Reverse implied odds {#reverse}

::: fact
**Reverse implied odds** are the money you lose on later streets when you make or hold a hand
that is still beaten. They reduce what a call is worth.
:::

::: intermediate
Typical cases: drawing to a low flush when higher flushes are possible, making the low end of a
straight, or a weak kicker with top pair. In each, the biggest pots you play are often the ones
you lose.
:::

::: approximation
The break-even formula above treats "hit" as "win" and "miss" as "give up". Real hands are messier:
you sometimes win without hitting, lose after hitting, or face more bets before the hand ends. Use it
as an estimate of the size of the later winnings you need, not an exact rule.
:::

```riverlab:question
{"id": "reverse"}
```
