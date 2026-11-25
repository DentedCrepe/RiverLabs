+++
slug = "starting-hands"
title = "Starting hands: classes, combinations and matchups"
level = 1
order = 8
version = 3
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
notes = "Drafted with AI assistance. Combination counts come from RiverLab's range engine and every equity is an exact enumeration of all boards, recomputed on sync; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Put any two hands or ranges against each other before the flop."

[[tools]]
href = "/preflop"
label = "Preflop"
why = "Run GTOpen's preflop model (an approximate model) to see opening and defending ranges for your own table and sizes."

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
explanation = "About 74%, so A♥Q♣ keeps only about 26% (100 − 74). Both hands share an ace, so AQ mostly needs a queen without a king. Compare 7♥6♥, which shares no rank with A♠K♦ and keeps about 42% against it. This is domination: a hand that shares a rank with a better one does much worse than two cards that share nothing with it."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["AsKd", "AhQc"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "small-pair"
type = "numeric"
prompt = "Before the flop, all-in: A♠K♦ against 2♣2♥. What is A♠K♦'s equity (in %)?"
answer = 46.96
tolerance = 0.05
unit = "%"
explanation = "About 47%, so against A♠K♦ even the smallest pair is a slight favourite all-in. That is not true against every pair of overcards: suited, connected overcards such as J♠T♠ make more straights and flushes, and the equity calculator in this lesson shows 2♣2♥ behind J♠T♠."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["AsKd", "2c2h"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "connectors"
type = "numeric"
prompt = "Before the flop, all-in: 7♥6♥ against A♠K♦. What is 7♥6♥'s equity (in %)?"
answer = 42.25
tolerance = 0.05
unit = "%"
explanation = "About 42%, close to what A♠K♦ itself has against Q♥Q♣ (about 43%). Neither of 7♥6♥'s ranks is in A♠K♦, so every seven or six pairs it without helping AK, and the suited, connected cards can also make flushes and straights."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["7h6h", "AsKd"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "offsuit-connectors"
type = "numeric"
prompt = "Before the flop, all-in: 7♥6♣ (the same ranks as 7♥6♥, but offsuit) against A♠K♦. What is 7♥6♣'s equity (in %)?"
answer = 38.50
tolerance = 0.05
unit = "%"
explanation = "About 38.5%. 7♥6♥ has about 42.25% against the same hand, so here being suited is worth about 3.75 percentage points (42.25 − 38.5): useful, but it does not turn a weak hand into a strong one."
concepts = ["equity", "starting-hands"]
compute = { kind = "equity", params = { ranges = ["7h6c", "AsKd"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "position-matters"
type = "multiple_choice"
prompt = "In six-max, why do solid players open fewer hands under the gun than on the button?"
options = [
  "More players are left to act behind, so it is likelier someone has a strong hand, and the opener plays most pots out of position",
  "Hands are worth less under the gun because of the blinds",
  "The rules limit the hands you can open early",
]
answer = 0
explanation = "Under the gun in six-max, five players can still wake up with a strong hand, and if the hijack, cutoff or button calls, they act after you on every later street; only the blinds would be out of position against you. On the button only the blinds are left, and you act last after the flop."
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
So you are dealt a pair about one time in 17 (78 of 1,326). One exact hand, like A♥K♥, comes
1 time in 1,326; a suited class, like AKs (any suited ace-king), 4 times in 1,326.
:::

## Classic matchups {#matchups}

::: fact
These are exact equities over every possible board, all-in before the flop, with the exact
suits shown. Equity counts a split pot as half a win.
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

```riverlab:question
{"id": "offsuit-connectors"}
```

```riverlab:equity
{"hands": ["AsKd", "AhQc"], "board": ""}
```

A small pair is not a favourite against every two overcards. Here is 2♣2♥ against suited,
connected overcards:

```riverlab:equity
{"hands": ["2c2h", "JsTs"], "board": ""}
```

::: approximation
All-in equity is what a hand wins with no more betting. When there is betting after the flop,
hands that make strong pairs or draws (suited and connected cards, pairs that can make sets)
can win more or less than their raw equity (equity realisation, a later lesson).
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
