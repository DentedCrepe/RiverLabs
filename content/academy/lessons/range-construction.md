+++
slug = "range-construction"
title = "Range construction: classes, weights and combinations"
level = 2
order = 8
version = 1
status = "published"
summary = "A range is a set of hand classes, each played some fraction of the time. Count its combinations, see how the board removes some of them, sort what is left by made hand, and work out a bet's value-to-bluff mix from the counts."
estimated_minutes = 20
objectives = [
  "Write a range as hand classes with weights, and count its combinations.",
  "Explain how the board removes combinations, and why it removes more from some classes than others.",
  "Sort a range on a board into made-hand categories by counting live combinations.",
  "Work out the share of bluffs in a betting range from its combination counts, and compare it with the single-bet model.",
]
prerequisites = ["combinatorics", "blockers"]
tags = ["math", "ranges", "combinatorics", "bluffing"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every combination count comes from RiverLab's range engine, the equity from exact enumeration and the bluff share from poker-theory, all recomputed at every content sync; hand categories were checked with the hand evaluator. The example ranges are illustrations, not recommended ranges. Awaiting review by a person."

[[tools]]
href = "/preflop"
label = "Range editor"
why = "Build a range from hand classes, give classes partial weights, and see its combination count."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Put a range on a board and see the combinations left and its equity against a hand or another range."

[[tools]]
href = "/solver"
label = "Solver"
why = "See which hands a solved river strategy bets, and count its value and bluff combinations yourself."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[reading]]
title = "Applications of No-Limit Hold 'em"
authors = "Matthew Janda"
year = 2013

[[questions]]
id = "classes-combos"
type = "numeric"
prompt = "Before any cards are seen, how many combinations does the range QQ+, AK hold (QQ, KK, AA, and AK suited and offsuit)?"
answer = 34
tolerance = 0
unit = "combos"
explanation = "Three pairs at 6 combinations each make 18, and AK has 4 suited plus 12 offsuit = 16: 18 + 16 = 34."
concepts = ["range-construction", "combinatorics"]
compute = { kind = "range", params = { notation = "QQ+,AK", dead = "" }, field = "combos" }

[[questions]]
id = "weighted-combos"
type = "numeric"
prompt = "Before any cards are seen, a range holds AA and KK always, QQ half the time (QQ:0.5), AK suited always and AK offsuit a quarter of the time (AKo:0.25). How many combinations does it hold, counting each combination by its weight?"
answer = 22
tolerance = 0
unit = "combos"
explanation = "6 + 6 + 0.5 × 6 + 4 + 0.25 × 12 = 6 + 6 + 3 + 4 + 3 = 22."
concepts = ["range-construction"]
compute = { kind = "range", params = { notation = "AA,KK,QQ:0.5,AKs,AKo:0.25", dead = "" }, field = "combos" }

[[questions]]
id = "what-weight-means"
type = "multiple_choice"
prompt = "A range lists KK:0.5. What does that mean?"
options = [
  "Each of the six KK combinations is played half of the time, so KK counts as 3 combinations",
  "Exactly three particular KK combinations are played and the other three never are",
  "KK wins half of the time against this range",
]
answer = 0
explanation = "A weight is a frequency for each combination of the class: the player takes this line with KK half of the time they hold it. Counted by weight, the class contributes 0.5 × 6 = 3 combinations. Choosing particular suits is also possible (it is written with the cards, e.g. KhKs), but that is not what a weight on the class says."
concepts = ["range-construction"]

[[questions]]
id = "top-pair-live"
type = "numeric"
prompt = "River board Q♠ 8♦ 3♣ 7♥ 2♠. Ignoring your own cards, how many combinations of AQ suited, AQ offsuit and KQ suited can your opponent hold? Before the board, these classes hold 4 + 12 + 4 = 20."
answer = 15
tolerance = 0
unit = "combos"
explanation = "The Q♠ is on the board, so every combination that uses it is gone: AQ suited keeps 3 of 4, AQ offsuit 9 of 12 and KQ suited 3 of 4. 3 + 9 + 3 = 15."
concepts = ["range-construction", "blockers"]
compute = { kind = "range", params = { notation = "AQs,AQo,KQs", dead = "Qs8d3c7h2s" }, field = "live_combos" }

[[questions]]
id = "sets-live"
type = "numeric"
prompt = "River board Q♠ 8♦ 3♣ 7♥ 2♠. Ignoring your own cards, how many combinations of QQ, 88 and 77 (each a set on this board) can your opponent hold?"
answer = 9
tolerance = 0
unit = "combos"
explanation = "Each pair has one of its four cards on the board, so 3 cards are left and C(3, 2) = 3 combinations: 3 × 3 = 9, down from 18 before the board."
concepts = ["range-construction", "combinatorics"]
compute = { kind = "range", params = { notation = "QQ,88,77", dead = "Qs8d3c7h2s" }, field = "live_combos" }

[[questions]]
id = "value-live"
type = "numeric"
prompt = "River board Q♠ 8♦ 3♣ 7♥ 2♠. An illustrative betting range (not a recommendation) holds the sets QQ, 88 and 77, the two pairs Q8 suited and 87 suited, and the top pairs AQ suited, AQ offsuit and KQ suited. Ignoring your own cards, how many live combinations do these hands hold in total?"
answer = 28
tolerance = 0
unit = "combos"
explanation = "Sets 9 + two pairs 4 (Q8 suited and 87 suited keep 2 suits each) + top pairs 15 = 28. Before the board the same classes held 46: every one of them uses a board rank, so the board removes 18."
concepts = ["range-construction"]
compute = { kind = "range", params = { notation = "QQ,88,77,Q8s,87s,AQs,AQo,KQs", dead = "Qs8d3c7h2s" }, field = "live_combos" }

[[questions]]
id = "bluff-share-pot"
type = "numeric"
prompt = "River. In the single-bet model (the bettor holds only hands that beat every bluff-catcher, plus pure bluffs; the caller holds only bluff-catchers), the bettor bets the size of the pot. What share (in %) of the betting range must be bluffs for a bluff-catcher to be indifferent between calling and folding?"
answer = 33.33
tolerance = 0.05
unit = "%"
explanation = "The caller risks B to win P + B, so needs B / (P + 2B) = 1/3 equity. In that model one third of the bets are bluffs: half a bluff combination per value combination."
concepts = ["bluff-to-value-ratios", "range-construction"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 10000 }, field = "bluff_share", scale = 100.0 }

[[questions]]
id = "bluffs-live"
type = "numeric"
prompt = "River board Q♠ 8♦ 3♣ 7♥ 2♠. An illustrative betting range (not a recommendation) bluffs with JT suited, KJ suited and 65 suited every time and with A4 suited half the time (A4s:0.5); each of these is only high card on this board. Ignoring your own cards, how many combinations of bluffs does it hold, counting each by its weight?"
answer = 14
tolerance = 0
unit = "combos"
explanation = "No card of these classes is on the board, so each suited class keeps all 4 combinations: 4 + 4 + 4 + 0.5 × 4 = 14. That is half of 28 value combinations, so bluffs are 14 / 42 = 1/3 of the range."
concepts = ["range-construction", "bluff-to-value-ratios"]
compute = { kind = "range", params = { notation = "JTs,KJs,65s,A4s:0.5", dead = "Qs8d3c7h2s" }, field = "live_combos" }

[[questions]]
id = "catcher-equity"
type = "numeric"
prompt = "River board Q♠ 8♦ 3♣ 7♥ 2♠. You hold 9♥ 9♣. Your opponent's illustrative betting range holds QQ, 88, 77, Q8 suited, 87 suited, AQ suited, AQ offsuit and KQ suited (all beat your nines), plus JT suited, KJ suited, 65 suited and half of A4 suited (A4s:0.5), which are high card and lose to your nines. What is your exact equity (in %) against this range?"
answer = 33.33
tolerance = 0.05
unit = "%"
explanation = "You win against exactly the bluffs: 14 of the 42 weighted combinations, 1/3. Your nines remove none of these combinations, so the count is the same as without your hand. Against a pot-sized bet you need 1/3 equity to call, so this range leaves the nines indifferent, as the single-bet model predicts."
concepts = ["range-construction", "equity", "bluff-to-value-ratios"]
compute = { kind = "equity", params = { ranges = ["9h9c", "QQ,88,77,Q8s,87s,AQs,AQo,KQs,JTs,KJs,65s,A4s:0.5"], board = "Qs8d3c7h2s" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "jacks-vs-queens"
type = "multiple_choice"
prompt = "Before any cards are seen, QQ and JJ each have 6 combinations. Ignoring your own cards, on the board Q♠ 8♦ 3♣ 7♥ 2♠, which can your opponent hold more combinations of?"
options = [
  "JJ: it keeps all 6, while QQ keeps 3 because the Q♠ is on the board",
  "QQ: a set is more likely than a pair of jacks",
  "They still have the same number",
]
answer = 0
explanation = "Only QQ loses combinations: three queens remain, so C(3, 2) = 3, while no jack is on the board and JJ keeps C(4, 2) = 6. Hands that connect with the board lose combinations to it."
concepts = ["range-construction", "blockers"]
+++

## A range is a list of classes with weights {#classes}

::: fact
A **range** is the set of hands a player can hold in a spot. It is written as **hand classes**
(AA, AKs, AKo, …), and each class can carry a **weight**: the share of its combinations that
take this line. `KK:0.5` means each KK combination is in the range half of the time. A class
written without a weight has weight 1. Short forms: `QQ+` is QQ, KK and AA; `AK` is AKs and
AKo together.
:::

::: simple
Think of a range as a shopping list. Each line is a hand type, and a weight says how often you
take it: always (1), half the time (0.5) or never. To count the range, add up each hand type's
combinations times its weight.
:::

::: intermediate
Counting a range is adding combinations class by class: 6 for a pair, 4 for a suited hand, 12
for an offsuit hand, each multiplied by its weight. QQ+ with AK is 3 × 6 + 4 + 12 = 34. Weights
let a range take a line with part of a class, and the combinations are counted by weight: half
of KK counts as 3.
:::

::: math
For a range with weights $w_c$ on classes $c$ holding $n_c$ combinations,

$$ \text{combos} = \sum_c w_c \, n_c . $$

With AA, KK, QQ:0.5, AKs and AKo:0.25: $6 + 6 + 0.5 \cdot 6 + 4 + 0.25 \cdot 12 = 22$.
:::

```riverlab:question
{"id": "classes-combos"}
```

```riverlab:question
{"id": "weighted-combos"}
```

```riverlab:question
{"id": "what-weight-means"}
```

::: heuristic
Every range in this lesson is an **illustration** chosen to make the counting clear. None of
them is a recommended range for any position or spot.
:::

## The board removes combinations {#board}

::: fact
A card on the board cannot be in anyone's hand, so every combination that uses it is gone (card
removal, from the combinatorics and blockers lessons). The **live combinations** are those
still possible. Classes that share a rank with the board lose the most: a pair whose rank is on
the board keeps 3 of its 6 combinations, and a suited hand one of whose cards is on the board
keeps 3 of its 4.
:::

```riverlab:question
{"id": "top-pair-live"}
```

```riverlab:question
{"id": "sets-live"}
```

```riverlab:question
{"id": "jacks-vs-queens"}
```

::: intermediate
This is why the same range can look very different on different boards. The hands that
connect with the board (sets, two pairs, top pairs) are exactly the ones whose cards the board
uses, so they lose combinations, while hands that missed keep all of theirs.
:::

::: advanced
What a range is made of on a board is a matter of combination counts, and the board changes
them unevenly. Before the board, the illustrative value classes in the next section hold 46
combinations; on Q♠ 8♦ 3♣ 7♥ 2♠ they hold 28. Its bluff classes hold 14 either way. Counting
classes ("eight value classes, four bluff classes") without counting live combinations
misjudges the mix.
:::

```riverlab:combos
{"range": "QQ,88,77,Q8s,87s,AQs,AQo,KQs", "dead": "Qs8d3c7h2s"}
```

## Sorting a range on a board {#composition}

::: fact
On the river Q♠ 8♦ 3♣ 7♥ 2♠ no flush or straight is possible: no suit appears three times, and
no five-card straight uses three of these ranks. Checked with RiverLab's hand evaluator, the
classes of this lesson's illustrative range make:

| Made hand | Classes | Live combos |
|---|---|---|
| Set | QQ, 88, 77 | 9 |
| Two pair | Q8s, 87s | 4 |
| Top pair (queens) | AQs, AQo, KQs | 15 |
| High card | JTs, KJs, 65s, A4s:0.5 | 14 |
:::

::: simple
Put each hand type in a bucket by what it makes on this board, then count each bucket. The
counts, not the number of hand types, tell you what the range is made of.
:::

::: intermediate
Sorting by made hand is the first step in thinking about a range on a board. Here 28 of the 42
weighted combinations are a pair of queens or better and 14 are high card. Which of the made
hands are "value" depends on what the opponent calls with; this lesson simply calls the queens
or better the value hands and the high-card hands bluffs, for the illustration.
:::

```riverlab:question
{"id": "value-live"}
```

## Value and bluffs for one river bet {#value-bluff}

::: approximation
The [MDF and bluff-ratio lesson](/academy/mdf-and-bluff-ratios) (Level 3) studies a
**single-bet model**: one river bet, a bettor holding only hands that beat every bluff-catcher
plus pure bluffs, and a caller holding only bluff-catchers. In that model a bluff-catcher is
indifferent when bluffs are $B / (P + 2B)$ of the bets. Real ranges have hands in between, raises
and other sizes, so the model is a reference point, not a poker equilibrium.
:::

```riverlab:question
{"id": "bluff-share-pot"}
```

::: intermediate
Range construction turns that share into a count. With 28 value combinations, a pot-sized bet
whose bluffs are one third of the range needs half as many bluffs as value hands: 28 ÷ 2 = 14.
The illustrative range gets there with three suited classes (12 combinations) and half of A4
suited (2 combinations), which is one use of weights: reaching a count that whole classes
cannot.
:::

```riverlab:question
{"id": "bluffs-live"}
```

::: math
With $V$ value combinations, $X$ bluff combinations and a bet of $B$ into $P$, the bluff share is
$X / (V + X)$. Setting it equal to $B / (P + 2B)$ gives

$$ X = V \cdot \frac{B}{P + B} . $$

For a pot-sized bet $B = P$: $X = V / 2 = 28 / 2 = 14$.
:::

```riverlab:question
{"id": "catcher-equity"}
```

::: advanced
A bluff-catcher's equity against a polarised range is the bluffs' share of the live
combinations **after removing the bluff-catcher's own cards**. 9♥ 9♣ blocks none of the
illustrative range, so its equity is exactly 14 / 42. A bluff-catcher that holds a card of the
bluffs (a jack against JT suited and KJ suited, say) sees fewer bluffs and has less equity, which
is the blocker effect from the blockers lesson, applied to a whole range.
:::

::: heuristic
The arithmetic says how many bluffs a target mix needs, not which hands to choose. Common
advice is to bluff with hands that cannot win at showdown and that block the opponent's calls
rather than their folds; a solver's choice depends on the whole game tree.
:::
