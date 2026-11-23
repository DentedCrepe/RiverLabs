+++
slug = "blockers"
title = "Blockers and card removal"
level = 2
order = 7
version = 1
status = "published"
summary = "How the cards you hold change which hands your opponent can have, counted exactly."
estimated_minutes = 15
objectives = [
  "Count how many combinations of a hand remain once your cards and the board are removed.",
  "Explain why one card can remove a large share of a strong range.",
  "Use blockers sensibly when choosing which hands to bluff or call with.",
]
prerequisites = ["combinatorics"]
tags = ["math", "combinatorics", "blockers"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every count is produced by RiverLab's range engine and re-checked when the content is synced; awaiting review by a person."

[[tools]]
href = "/preflop"
label = "Range editor"
why = "Enter a range and dead cards to see how many combinations remain."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019
note = "A study of no-limit hold'em strategy based on solver output."

[[questions]]
id = "ak-with-ace"
type = "numeric"
prompt = "Before the flop. You hold A♠ Q♦ in the big blind and the cutoff has raised. How many combinations of AK (suited and offsuit) can the cutoff hold?"
answer = 12
tolerance = 0
explanation = "Normally there are 4 × 4 = 16. With the ace of spades in your hand, only 3 aces remain: 3 × 4 = 12."
concepts = ["blockers", "combinatorics"]
compute = { kind = "range", params = { notation = "AKs,AKo", dead = "AsQd" }, field = "live_combos" }

[questions.scene]
players = 6
hero = "BB"
hero_cards = ["As", "Qd"]
pot_bb = "0.5"
bets_bb = { CO = "2.5", BB = "1" }
folded = ["UTG", "HJ", "BTN", "SB"]
actions = { CO = "raises to 2.5bb" }

[[questions]]
id = "aa-with-ace"
type = "numeric"
prompt = "You hold A♥ 7♣. How many combinations of pocket aces can your opponent hold? (No board yet.)"
answer = 3
tolerance = 0
explanation = "Three aces are left, and C(3, 2) = 3 pairs can be made from them, half of the usual 6."
concepts = ["blockers", "combinatorics"]
compute = { kind = "range", params = { notation = "AA", dead = "Ah7c" }, field = "live_combos" }

[[questions]]
id = "share-blocked"
type = "numeric"
prompt = "Your opponent's range is AA, KK and AK (28 combinations). You hold A♠ 5♦. What share (in %) of those combinations does your hand remove?"
answer = 25.0
tolerance = 0.05
unit = "%"
explanation = "With one ace gone: AA 3 (of 6), KK 6 (of 6), AK 12 (of 16): 21 of 28 remain, so 7 / 28 = 25% are removed."
concepts = ["blockers"]
compute = { kind = "range", params = { notation = "AA,KK,AKs,AKo", dead = "As5d" }, field = "blocked_fraction", scale = 100.0 }

[[questions]]
id = "flush-combos"
type = "numeric"
prompt = "River. The board is K♠ 8♠ 3♠ 7♦ 2♣ and you hold A♠ 5♦. How many combinations of two spades (a flush) can your opponent hold?"
answer = 36
tolerance = 0
explanation = "Ten spades are not on the board. Without your hand that would be C(10, 2) = 45 two-spade combinations. You hold the A♠, so 9 spades are left: C(9, 2) = 36, and none of them is the ace-high (nut) flush."
concepts = ["blockers", "combinatorics"]
compute = { kind = "range", params = { notation = "AsQs,AsJs,AsTs,As9s,As7s,As6s,As5s,As4s,As2s,QsJs,QsTs,Qs9s,Qs7s,Qs6s,Qs5s,Qs4s,Qs2s,JsTs,Js9s,Js7s,Js6s,Js5s,Js4s,Js2s,Ts9s,Ts7s,Ts6s,Ts5s,Ts4s,Ts2s,9s7s,9s6s,9s5s,9s4s,9s2s,7s6s,7s5s,7s4s,7s2s,6s5s,6s4s,6s2s,5s4s,5s2s,4s2s", dead = "Ks8s3s7d2cAs5d" }, field = "live_combos" }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["As", "5d"]
board = ["Ks", "8s", "3s", "7d", "2c"]
pot_bb = "15"
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "checks" }

[[questions]]
id = "bluff-candidate"
type = "multiple_choice"
prompt = "River. The board is K♠ 8♠ 3♠ 7♦ 2♣. You missed your draw and are considering a bluff. Other things equal, which card in your hand makes a bluff more attractive?"
options = [
  "The A♠, because your opponent cannot hold the ace-high flush",
  "The 2♠, because it makes a small flush",
  "It never matters which cards you hold when you bluff",
]
answer = 0
explanation = "Holding the A♠ removes every ace-high flush from your opponent's range, so they are less likely to hold a hand that will never fold. The 2♠ would give you a flush, which is not a bluff. How much a blocker matters depends on the whole ranges."
concepts = ["blockers"]
+++

## Card removal {#removal}

::: fact
Every card you can see (your own two and the board) cannot be in your opponent's hand. Removing
a card removes every combination that contains it. This is **card removal**; a card that removes
combinations of a particular hand is said to **block** it.
:::

::: simple
There are 6 ways to be dealt pocket aces. If you hold one ace, only 3 are left. If you hold two,
there are none.
:::

```riverlab:combos
{"range": "AA,KK,AKs,AKo", "dead": "As5d"}
```

```riverlab:question
{"id": "ak-with-ace"}
```

```riverlab:question
{"id": "aa-with-ace"}
```

## Strong ranges are small {#strong}

::: fact
Ranges of very strong hands contain few combinations, so a single card can remove a large share
of them.
:::

::: math
A pair has $\binom{4}{2} = 6$ combinations; with one of its cards seen, $\binom{3}{2} = 3$. A
non-pair hand has $4 \times 4 = 16$; with one of its cards seen, $3 \times 4 = 12$.
:::

```riverlab:question
{"id": "share-blocked"}
```

```riverlab:question
{"id": "flush-combos"}
```

## Using blockers {#using}

::: heuristic
When choosing which hands to bluff with, hands that block your opponent's strongest calls (and
do not block their folds) are better candidates. When choosing which hands to call with, hands
that block their value bets are better candidates.
:::

::: approximation
The effect of one card is real but usually modest: it changes a few combinations in ranges of
hundreds. It decides close choices between similar hands; it does not turn a bad play into a good
one.
:::

```riverlab:question
{"id": "bluff-candidate"}
```
