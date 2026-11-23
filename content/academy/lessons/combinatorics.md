+++
slug = "combinatorics"
title = "Combinatorics: counting hands"
level = 2
order = 6
version = 2
status = "published"
summary = "Why there are 1,326 starting hands but 169 hand classes, and how the cards you see remove combinations from a range."
estimated_minutes = 20
objectives = [
  "Count combinations for pairs, suited and offsuit hands.",
  "Explain the difference between 1,326 combinations and 169 classes.",
  "Apply card removal: count what is left of a range given the cards you can see.",
]
prerequisites = ["terminology", "starting-hands"]
tags = ["math", "combinatorics", "ranges", "blockers"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Combo counts are produced by RiverLab's range engine and re-checked on sync; awaiting review by a person."

[[tools]]
href = "/preflop"
label = "Range editor"
why = "Build a range and see its exact combo count and card removal."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Hand-reading with distributions of hands."

[[questions]]
id = "pairs"
type = "numeric"
prompt = "How many combinations of pocket kings (KK) are there before any cards are seen?"
answer = 6
tolerance = 0
explanation = "Choose 2 of the 4 kings: C(4, 2) = 6."
concepts = ["combinatorics"]
compute = { kind = "range", params = { notation = "KK", dead = "" }, field = "live_combos" }

[[questions]]
id = "ak-offsuit"
type = "numeric"
prompt = "How many offsuit AK combinations (AKo) are there?"
answer = 12
tolerance = 0
explanation = "4 aces × 4 kings = 16 AK combinations, of which 4 are suited, so 12 are offsuit."
concepts = ["combinatorics"]
compute = { kind = "range", params = { notation = "AKo", dead = "" }, field = "live_combos" }

[[questions]]
id = "removal-ak"
type = "numeric"
prompt = "The board is K♥ 7♦ 2♣. Ignoring your own cards, how many AK combinations (suited and offsuit) can your opponent still hold?"
answer = 12
tolerance = 0
explanation = "Only 3 kings are left: 4 aces × 3 kings = 12."
concepts = ["combinatorics", "blockers"]
compute = { kind = "range", params = { notation = "AK", dead = "Kh7d2c" }, field = "live_combos" }

[[questions]]
id = "removal-sets"
type = "numeric"
prompt = "The board is K♥ 7♦ 2♣. Ignoring your own cards, how many combinations of pocket kings (a set of kings) can your opponent still hold?"
answer = 3
tolerance = 0
explanation = "Choose 2 of the 3 remaining kings: C(3, 2) = 3."
concepts = ["combinatorics", "blockers"]
compute = { kind = "range", params = { notation = "KK", dead = "Kh7d2c" }, field = "live_combos" }

[[questions]]
id = "which-more"
type = "multiple_choice"
prompt = "Before any cards are seen, is a player more likely to be dealt a pocket pair (any pair) or AK (suited or offsuit)?"
options = ["Any pocket pair (78 combinations)", "AK (16 combinations)", "They are equally likely"]
answer = 0
explanation = "13 pairs × 6 = 78 combinations against 16 for AK."
concepts = ["combinatorics"]
+++

## Combinations and classes {#combos}

::: simple
A deck has 52 cards, so there are **1,326** different two-card starting hands. Poker players
group them into **169** types: 13 pairs (like KK), 78 suited hands (like AKs) and 78 offsuit
hands (like AKo).
:::

::: intermediate
Each type has a fixed number of **combinations** (combos):

| Type | Example | Combos |
|---|---|---|
| Pair | KK | 6 |
| Suited | AKs | 4 |
| Offsuit | AKo | 12 |

So AK (suited or offsuit) has 16 combos, while any single pair has only 6.
:::

::: math
Choosing 2 cards from 52 gives $\binom{52}{2} = 1326$. A pair uses 2 of the 4 cards of one rank:
$\binom{4}{2} = 6$. Two different ranks give $4 \times 4 = 16$ combos: 4 suited (same suit) and 12
offsuit. Summing over all types,

$$
13 \cdot 6 + 78 \cdot 4 + 78 \cdot 12 = 78 + 312 + 936 = 1326 .
$$
:::

## Card removal {#removal}

::: simple
Cards you can see (your hand, the board) can't be in your opponent's hand. On a board with a
king, there are fewer ways for them to hold a king.
:::

::: intermediate
Count the cards that are left. On K♥ 7♦ 2♣:
- KK: only 3 kings remain, so $\binom{3}{2} = 3$ combos (instead of 6).
- AK: 4 aces × 3 kings = 12 combos (instead of 16).

The same works for your own cards: holding an ace makes every ace-x hand less likely for your
opponent. These are **blockers**.
:::

::: advanced
Card removal reshapes ranges, not just single hands. A range's live combo count, weighted by
how often each hand is played, is what equity calculations and solvers work with. Two ranges
can look the same as a 13×13 chart and still differ in combos once the board removes cards.
:::

```riverlab:combos
{"range": "AK,KK,77,22", "dead": "Kh7d2c"}
```

::: fact
These counts are exact. RiverLab's range engine counts combinations directly, and the numbers in
this lesson's questions are checked against it every time the content is published.
:::
