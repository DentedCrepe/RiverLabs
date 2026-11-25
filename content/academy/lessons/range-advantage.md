+++
slug = "range-advantage"
title = "Range advantage, nut advantage and board texture"
level = 4
order = 1
version = 2
status = "published"
summary = "How the same two ranges swap places from one flop to another: overall equity (range advantage), the strongest hands (nut advantage), and what each tends to do to betting."
estimated_minutes = 20
objectives = [
  "Measure range advantage as one range's equity against the other on a given board.",
  "Count nut combinations each range can hold on a board.",
  "Explain how board texture moves both, and what solvers tend to do with each.",
]
prerequisites = ["equity", "combinatorics"]
tags = ["strategy", "ranges", "board-texture"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every equity is an exact enumeration and every count comes from the range engine, recomputed when the content is synced. The two ranges are illustrative examples, not anyone's measured strategy. Awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Put any two ranges on any board and compare their equity."

[[tools]]
href = "/solver"
label = "Solver"
why = "See how a solved strategy's betting changes with the board for the same ranges."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[[reading]]
title = "Applications of No-Limit Hold'em"
authors = "Matthew Janda"
year = 2013

[[questions]]
id = "equity-dry"
type = "numeric"
prompt = "Flop A♥ K♦ 4♣. Heads-up after one raise and a call: the raiser's range is AA–TT, AK and AQ (suited and offsuit) and KQ suited; the caller's is 99–22 and the suited hands AJ, AT, KJ, QJ, JT, T9, 98, 87, 76 and 65. What is the raiser's equity (in %) against the caller's range?"
answer = 85.22
tolerance = 0.05
unit = "%"
explanation = "Exact enumeration over every turn and river and every pair of hands: about 85%. Aces and kings dominate this board, and most of the caller's hands missed."
concepts = ["range-advantage"]
compute = { kind = "equity", params = { ranges = ["AA-TT,AKs,AKo,AQs,AQo,KQs", "99-22,AJs,ATs,KJs,QJs,JTs,T9s,98s,87s,76s,65s"], board = "AhKd4c" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "UTG"
board = ["Ah", "Kd", "4c"]
pot_bb = "5.5"
folded = ["HJ", "CO", "BTN", "SB"]
actions = { UTG = "raised preflop", BB = "called" }

[[questions]]
id = "equity-wet"
type = "numeric"
prompt = "Flop 8♥ 7♥ 6♠. Heads-up after one raise and a call: the raiser's range is AA–TT, AK and AQ (suited and offsuit) and KQ suited; the caller's is 99–22 and the suited hands AJ, AT, KJ, QJ, JT, T9, 98, 87, 76 and 65. What is the raiser's equity (in %)?"
answer = 44.16
tolerance = 0.05
unit = "%"
explanation = "About 44%: below half. The caller's middle pairs (66–88), connectors and suited hands hit this board; the raiser's big cards mostly did not."
concepts = ["range-advantage", "board-textures"]
compute = { kind = "equity", params = { ranges = ["AA-TT,AKs,AKo,AQs,AQo,KQs", "99-22,AJs,ATs,KJs,QJs,JTs,T9s,98s,87s,76s,65s"], board = "8h7h6s" }, field = "equity.0", scale = 100.0 }

[questions.scene]
players = 6
hero = "UTG"
board = ["8h", "7h", "6s"]
pot_bb = "5.5"
folded = ["HJ", "CO", "BTN", "SB"]
actions = { UTG = "raised preflop", BB = "called" }

[[questions]]
id = "sets-wet"
type = "numeric"
prompt = "Flop 8♥ 7♥ 6♠. The preflop caller's range includes every pair from 99 down to 22. How many combinations of a set (88, 77 or 66) can it hold?"
answer = 9
tolerance = 0
explanation = "Each pair has 3 combinations left once one of its cards is on the board: 3 × 3 = 9. A raiser whose pairs are TT and up has no set on this board."
concepts = ["nut-advantage", "combinatorics"]
compute = { kind = "range", params = { notation = "88,77,66", dead = "8h7h6s" }, field = "live_combos" }

[[questions]]
id = "straights-wet"
type = "numeric"
prompt = "Flop 8♥ 7♥ 6♠. The preflop caller's range includes T9 suited. How many combinations of T9 suited (a straight) can it hold?"
answer = 4
tolerance = 0
explanation = "All four suited T9 combinations are live (no ten or nine is on the board). A raiser holding only AA–TT, AK, AQ and KQ suited has no straight here."
concepts = ["nut-advantage"]
compute = { kind = "range", params = { notation = "T9s", dead = "8h7h6s" }, field = "live_combos" }

[[questions]]
id = "sets-dry"
type = "numeric"
prompt = "Flop A♥ K♦ 4♣. The preflop raiser's range includes AA and KK. How many combinations of top set or middle set (AA or KK) can it hold?"
answer = 6
tolerance = 0
explanation = "AA and KK each have 3 combinations left with one of their cards on the board: 6. A caller whose only pairs are 99–22 can make just one set here, 44 (3 combinations)."
concepts = ["nut-advantage", "combinatorics"]
compute = { kind = "range", params = { notation = "AA,KK", dead = "AhKd4c" }, field = "live_combos" }

[[questions]]
id = "which-board"
type = "multiple_choice"
prompt = "Heads-up after one raise and a call: the raiser's range is AA–TT, AK and AQ (suited and offsuit) and KQ suited; the caller's is 99–22 and the suited hands AJ, AT, KJ, QJ, JT, T9, 98, 87, 76 and 65. On which flop does the caller have both more equity and more of the strongest hands?"
options = ["A♥ K♦ 4♣", "8♥ 7♥ 6♠", "Neither: the preflop raiser always has the advantage"]
answer = 1
explanation = "On 8♥ 7♥ 6♠ the raiser's equity is about 44% (exact enumeration), and the caller holds all the sets (88, 77, 66) and straights (T9 suited); the raiser holds neither. On A♥ K♦ 4♣ the raiser has about 85% and the top and middle sets. Who has the advantage depends on the board, not on who raised."
concepts = ["range-advantage", "nut-advantage", "board-textures"]

[[questions]]
id = "what-they-do"
type = "multiple_choice"
prompt = "Which statement about range and nut advantage is the better rule of thumb?"
options = [
  "A big range advantage on a dry board tends to favour betting often with small sizes; a nut advantage tends to favour larger, more polarised bets",
  "Range advantage never matters, only your own hand does",
  "Whoever raised before the flop should always bet the flop",
]
answer = 0
explanation = "It is a rule of thumb, not a result computed in this lesson. The river solve in the lesson on polarised ranges shows the nut-advantage half of it in one spot. It is a tendency, not a law: the exact strategy depends on ranges, stacks and the sizes allowed."
concepts = ["range-advantage", "nut-advantage"]
+++

## Two measures {#measures}

::: fact
- **Range advantage:** one range's equity against the other on the current board, over all their
  hands.
- **Nut advantage:** which range holds more combinations of the strongest hands possible on that
  board (sets, straights, flushes, top two pair).
:::

::: simple
Range advantage asks "who is ahead on average?". Nut advantage asks "who has more of the monsters?".
They often point the same way, but not always.
:::

::: approximation
The two ranges in this lesson are **illustrative examples** (a tight preflop raise against a
range that calls with pairs and suited connectors), chosen to show the effect clearly. They are
not measured from players or solved preflop strategies.
:::

## A dry, high board {#dry}

```riverlab:question
{"id": "equity-dry"}
```

```riverlab:question
{"id": "sets-dry"}
```

## A low, connected board {#wet}

::: fact
The same two ranges can swap places from one flop to another, as the exact equities below show.
:::

::: heuristic
A low, connected, two-suited board tends to hit middle pairs and suited connectors and to miss
big cards.
:::

```riverlab:equity
{"hands": ["AA-TT,AKs,AKo,AQs,AQo,KQs", "99-22,AJs,ATs,KJs,QJs,JTs,T9s,98s,87s,76s,65s"], "board": "8h7h6s"}
```

```riverlab:question
{"id": "equity-wet"}
```

```riverlab:question
{"id": "sets-wet"}
```

```riverlab:question
{"id": "straights-wet"}
```

```riverlab:question
{"id": "which-board"}
```

## What the advantages tend to do {#effects}

::: heuristic
- With a large **range advantage** on a board where few hands improve much (a dry, high board for
  the raiser), solved strategies often bet a large share of the range with a small size: most of
  the range is ahead, and the opponent has few strong hands to punish it.
- With a **nut advantage**, solved strategies tend to use larger bets with a polarised range: the
  strongest hands win more, and the bigger bet carries more bluffs (see the lesson on polarised
  ranges, where a solve uses only the 150% size).
- Without either, checking more often is common.
:::

::: heuristic
These are rules of thumb, not results computed in this lesson (the lesson solves no flop). Any
particular answer depends on the ranges, stack depth and bet sizes allowed; solve the spot to see
it, and read any solve as an approximate equilibrium for the sizes it was given.
:::

```riverlab:question
{"id": "what-they-do"}
```
