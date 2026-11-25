+++
slug = "equity"
title = "Equity"
level = 1
order = 5
version = 2
status = "published"
summary = "Your share of the pot if the hand were played out with no more betting: against one hand, against a range, and how it changes street by street."
estimated_minutes = 15
objectives = [
  "Define equity, including how ties count.",
  "Compute equity of a hand against a hand and against a range with the equity calculator.",
  "Explain why equity against a range is what matters at the table.",
]
prerequisites = ["hand-rankings"]
tags = ["math", "equity"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every equity is an exact enumeration by RiverLab's equity engine, recomputed when the content is synced; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Exact or Monte Carlo equity for hands and weighted ranges on any board."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[questions]]
id = "aa-kk"
type = "numeric"
prompt = "Before the flop the money goes all in: you hold A♠ A♥ and your opponent holds K♠ K♥. What is your equity (in %)?"
answer = 82.64
tolerance = 0.05
unit = "%"
explanation = "Enumerating all 1,712,304 possible boards: you win 82.36%, tie 0.54% (counted as half) and lose the rest, for 82.64% equity. Kings win mainly when a king comes and no ace does; with these suits, any flush the kings make the aces make too, higher, unless the kings make a straight flush."
concepts = ["equity"]
compute = { kind = "equity", params = { ranges = ["AsAh", "KsKh"], board = "" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["As", "Ah"]
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BTN = "all-in", BB = "calls all-in" }
shown = { BB = ["Ks", "Kh"] }

[[questions]]
id = "ak-qq"
type = "numeric"
prompt = "Before the flop, all in: you hold A♥ K♥ and your opponent holds Q♠ Q♣. What is your equity (in %)?"
answer = 46.21
tolerance = 0.05
unit = "%"
explanation = "Over every possible board AK suited wins about 46% against queens: it needs an ace or a king (or a straight or flush) to beat a pair it is behind before the flop."
concepts = ["equity"]
compute = { kind = "equity", params = { ranges = ["AhKh", "QsQc"], board = "" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "CO"
hero_cards = ["Ah", "Kh"]
folded = ["UTG", "HJ", "BTN", "SB"]
actions = { CO = "all-in", BB = "calls all-in" }
shown = { BB = ["Qs", "Qc"] }

[[questions]]
id = "ak-22"
type = "numeric"
prompt = "Before the flop, all in: you hold A♥ K♥ and your opponent holds 2♠ 2♣. What is your equity (in %)?"
answer = 50.08
tolerance = 0.05
unit = "%"
explanation = "Over every possible board, suited AK is very slightly ahead of the lowest pair. The deuces are ahead now, but an ace or a king usually beats them, AK's straights and flushes add more, and when the board pairs twice the deuces no longer play while AK keeps its ace kicker. Against a higher pair, such as queens, the same hand does worse: queens are rarely counterfeited and they block some of AK's straights."
concepts = ["equity"]
compute = { kind = "equity", params = { ranges = ["AhKh", "2s2c"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "draw-vs-pair"
type = "numeric"
prompt = "Flop K♠ 7♥ 2♥. You hold A♥ T♥ (a flush draw) and your opponent holds K♦ Q♣ (top pair). The money goes all in. What is your equity (in %)?"
answer = 45.35
tolerance = 0.05
unit = "%"
explanation = "Count the 990 possible turn and river pairs; there are no ties. Most runouts with a heart win with a flush, but not all: a few also pair the board so that top pair becomes a full house or four kings (for example K♥ with a 7). Without a heart, an ace gives you a pair of aces, which wins unless the other card is a king or a queen (your opponent then has trips or two pair). Without a heart or an ace you win only with two tens (three of a kind, 3 ways) or a queen and a jack (a straight to the ace, 2 × 3 = 6 ways)."
concepts = ["equity", "outs-and-draws"]
compute = { kind = "equity", params = { ranges = ["AhTh", "KdQc"], board = "Ks7h2h" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Th"]
board = ["Ks", "7h", "2h"]
pot_bb = "6.5"
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "all-in" }
shown = { BB = ["Kd", "Qc"] }

[[questions]]
id = "vs-range"
type = "numeric"
prompt = "Flop J♦ 7♣ 2♠. You hold Q♠ Q♥ in the big blind against the button. You believe the button holds AK, any of its 16 combinations. What is your equity (in %) against that range if the money goes all in?"
answer = 74.49
tolerance = 0.05
unit = "%"
explanation = "Against every AK combination, weighted equally and with all 990 turn and river pairs: you are ahead now, and AK mostly needs an ace or a king to win; runner-runner straights and flushes add a little."
concepts = ["equity"]
compute = { kind = "equity", params = { ranges = ["QsQh", "AKs,AKo"], board = "Jd7c2s" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "BB"
hero_cards = ["Qs", "Qh"]
board = ["Jd", "7c", "2s"]
pot_bb = "6.5"
folded = ["UTG", "HJ", "CO", "SB"]

[[questions]]
id = "definition"
type = "multiple_choice"
prompt = "What does 'my equity is 40%' mean?"
options = [
  "I will win 40% of the pots I play",
  "If the remaining cards were dealt with no more betting, on average I would win 40% of this pot (ties counting as half)",
  "I have put 40% of the chips into this pot",
]
answer = 1
explanation = "Equity is your average share of the pot at showdown over all the ways the remaining cards can come, assuming nobody bets or folds again."
concepts = ["equity"]
+++

## What equity is {#definition}

::: fact
Your **equity** is the share of the pot you would win on average if all the remaining cards
were dealt and nobody bet or folded again. A tie counts as half (or a third in a three-way
split).
:::

::: simple
If you would win this pot 3 times in 4 with no more betting, your equity is 75%. It is a
property of the cards still to come, not a prediction of what will happen in the hand.
:::

::: math
For one opponent, with $W$, $T$ and $L$ the probabilities of winning, tying and losing over all
remaining boards,

$$ \text{equity} = W + \tfrac12 T. $$

Before the flop there are $\binom{48}{5} = 1{,}712{,}304$ boards for two known hands; after the
flop, $\binom{45}{2} = 990$ turn and river pairs.
:::

```riverlab:question
{"id": "aa-kk"}
```

## Hand against hand {#hand-vs-hand}

::: approximation
Before the flop, a pair against a lower pair is a big favourite, roughly 4 to 1 (the A♠ A♥
against K♠ K♥ question above works one case out exactly). Two higher cards against a lower pair
are close to even: usually a little behind, and against a very small pair suited high cards can
even be very slightly ahead. Check particular hands with the calculator; the exact figure always
depends on the ranks and suits (which straights and flushes each hand can make or blocks).
:::

```riverlab:equity
{"hands": ["AhKd", "2s2c"], "board": ""}
```

```riverlab:equity
{"hands": ["AhKh", "QsQc"], "board": ""}
```

```riverlab:question
{"id": "ak-qq"}
```

```riverlab:question
{"id": "ak-22"}
```

::: intermediate
After the flop, draws can have a lot of equity even when they are behind.
:::

::: heuristic
A flush draw with an overcard against top pair is close to a coin flip with two cards to come,
usually a little behind. The question below works out one case exactly.
:::

```riverlab:question
{"id": "draw-vs-pair"}
```

## Against a range {#range}

::: fact
Your opponent's cards are hidden, so the useful number is your equity against the **range** of
hands they could hold, each combination weighted by how likely it is.
:::

::: intermediate
Your equity against a range is the weighted average of your equity against each hand in it.
Card removal matters: your own cards and the board make some combinations impossible.
:::

```riverlab:question
{"id": "vs-range"}
```

::: approximation
Equity assumes no more betting. In a real hand, later bets and folds mean you do not always get
to see the showdown: a hand can win more or less than its equity. That is **equity
realisation**, covered later.
:::

```riverlab:question
{"id": "definition"}
```
