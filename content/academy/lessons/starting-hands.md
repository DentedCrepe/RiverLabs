+++
slug = "starting-hands"
title = "Starting hands: classes, combinations and matchups"
level = 1
order = 8
version = 1
status = "published"
summary = "How the 1,326 two-card hands group into 169 classes, how often each kind is dealt, and what the classic preflop matchups are worth."
estimated_minutes = 15
objectives = [
  "Count pairs, suited and offsuit combinations.",
  "Know the rough size of the main preflop matchups: a pair against two overcards, domination, suited against offsuit.",
  "Explain why position and the action before you change which hands are worth playing.",
]
prerequisites = ["terminology", "equity"]
tags = ["preflop", "combinatorics", "equity"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Combination counts come from RiverLab's range engine and every equity is an exact enumeration of all boards, recomputed on sync; awaiting review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Put any two hands or ranges against each other before the flop."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[questions]]
id = "pairs"
type = "numeric"
prompt = "How many pocket-pair combinations are there in total (all thirteen pairs, 22 to AA)?"
answer = 78
tolerance = 0
explanation = "Each pair has 6 combinations (choose 2 of 4 suits), and there are 13 pairs: 13 × 6 = 78."
concepts = ["combinatorics", "starting-hands"]
compute = { kind = "range", params = { notation = "22+", dead = "" }, field = "live_combos" }

[[questions]]
id = "suited"
type = "numeric"
prompt = "How many suited non-pair combinations are there in total (every suited hand, AKs down to 32s)?"
answer = 312
tolerance = 0
explanation = "There are 78 non-pair rank pairs, each with 4 suited combinations: 78 × 4 = 312. Each offsuit class has 12, so offsuit hands make 936, and 78 + 312 + 936 = 1,326."
concepts = ["combinatorics", "starting-hands"]
compute = { kind = "range", params = { notation = "A2s+,K2s+,Q2s+,J2s+,T2s+,92s+,82s+,72s+,62s+,52s+,42s+,32s", dead = "" }, field = "live_combos" }

[[questions]]
id = "pair-vs-overs"
type = "numeric"
prompt = "Before the flop, all-in: A♠K♦ against Q♥Q♣. What is A♠K♦'s equity (in %)?"
answer = 42.84
tolerance = 0.05
unit = "%"
explanation = "About 43%: the pair is a modest favourite. AK needs to pair (or make a straight or flush); the queens win if neither happens."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["AsKd", "QhQc"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "domination"
type = "numeric"
prompt = "Before the flop, all-in: A♠K♦ against A♥Q♣. What is A♠K♦'s equity (in %)?"
answer = 74.02
tolerance = 0.05
unit = "%"
explanation = "About 74%. Both hands share an ace, so AQ mostly needs a queen without a king. This is domination: a hand sharing a card with a better one wins far less often than two unrelated cards would."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["AsKd", "AhQc"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "small-pair"
type = "numeric"
prompt = "Before the flop, all-in: A♠K♦ against 2♣2♥. What is A♠K♦'s equity (in %)?"
answer = 46.96
tolerance = 0.05
unit = "%"
explanation = "About 47%: even the smallest pair is a slight favourite against two overcards all-in."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["AsKd", "2c2h"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "connectors"
type = "numeric"
prompt = "Before the flop, all-in: 7♥6♥ against A♠K♦. What is 7♥6♥'s equity (in %)?"
answer = 42.25
tolerance = 0.05
unit = "%"
explanation = "About 42%. Two live cards that can make straights and flushes are close to a pair-against-overcards matchup."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["7h6h", "AsKd"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "position-matters"
type = "multiple_choice"
prompt = "Why do solid players open fewer hands under the gun than on the button?"
options = [
  "More players are left to act behind, so it is likelier someone has a strong hand, and the opener plays most pots out of position",
  "Hands are worth less under the gun because of the blinds",
  "The rules limit the hands you can open early",
]
answer = 0
explanation = "Under the gun, five players can still wake up with a strong hand, and whoever calls usually has position on you. On the button only the blinds are left, and you act last after the flop."
concepts = ["positions", "starting-hands"]
+++

## 1,326 hands, 169 classes {#classes}

::: fact
There are 52 × 51 / 2 = 1,326 two-card hands. Grouped by ranks and whether the cards share a
suit, they form 169 **classes**: 13 pairs (6 combinations each), 78 suited hands (4 each) and
78 offsuit hands (12 each).
:::

```riverlab:question
{"id": "pairs"}
```

```riverlab:question
{"id": "suited"}
```

::: simple
So you are dealt a pair about one time in 17 (78 of 1,326), and any particular suited hand,
like A♥K♥ or A♠K♠, 4 times in 1,326.
:::

## Classic matchups {#matchups}

::: fact
These are exact equities over every possible board, all-in before the flop, with the exact
suits shown.
:::

```riverlab:question
{"id": "pair-vs-overs"}
```

```riverlab:question
{"id": "small-pair"}
```

```riverlab:question
{"id": "domination"}
```

```riverlab:question
{"id": "connectors"}
```

```riverlab:equity
{"hands": ["AsKd", "AhQc"], "board": ""}
```

::: approximation
All-in equity is what a hand wins with no more betting. When there is betting after the flop,
hands that make strong pairs or draws (suited and connected cards, pairs that can make sets)
can win more or less than their raw equity (equity realisation, a planned lesson).
:::

## Which hands to play {#selection}

::: heuristic
Play fewer hands the more players are left to act behind you and the more often you will be
out of position. Hands that share cards with the hands that will raise you (weak aces, weak
kings) do worst against strong ranges, because of domination. For concrete ranges, use the
Preflop page: GTOpen's preflop model shows them for your table and sizes, labelled as an
approximate model.
:::

```riverlab:question
{"id": "position-matters"}
```
