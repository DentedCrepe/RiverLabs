+++
slug = "turn-and-river-strategy"
title = "Turn and river: barrelling, turn cards and polarised rivers"
level = 4
order = 10
version = 1
status = "published"
summary = "Three solves of one turn spot with different turn cards: a card that helps the bettor's range makes it bet more, a card that helps the caller makes it bet less and lets the caller bet. Then the river after a turn bet and call, where the bettor's strategy splits into strong hands and bluffs that bet, and middle hands that check."
estimated_minutes = 25
objectives = [
  "Explain, from solved strategies, how the turn card changes how often the turn bettor bets.",
  "See which parts of a range bet the turn and which check.",
  "Recognise a polarised river strategy: strong hands and bluffs bet, middle-strength hands check.",
  "Compare a solved river bluff share with the single-bet model, and say why they need not match.",
]
prerequisites = ["polarised-ranges", "range-checking", "check-raises"]
tags = ["strategy", "solver", "turn", "river"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every frequency comes from three turn solves run by RiverLab's solver each time the content is synced (each within 0.05% of the pot of an equilibrium of its tree; measured about 0.002–0.006% at 1000 iterations), equities from exact enumeration, hand strengths checked with the hand evaluator. The ranges are illustrations, not recommended ranges. Interpretations are labelled heuristic. Awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Solve a turn spot with your own ranges and sizes, and step through turn cards and rivers to see how the strategy changes."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Check how a turn card moves each range's equity before you solve."

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[solves.blank]
board = "Kh9d4c2s"
ranges = ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"]
pot_bb = "10"
stack_bb = "30"
oop_bets = [75]
ip_bets = [75]
oop_raises = [100]
ip_raises = [100]
iterations = 1000

[solves.ace]
board = "Kh9d4cAs"
ranges = ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"]
pot_bb = "10"
stack_bb = "30"
oop_bets = [75]
ip_bets = [75]
oop_raises = [100]
ip_raises = [100]
iterations = 1000

[solves.queen]
board = "Kh9d4cQs"
ranges = ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"]
pot_bb = "10"
stack_bb = "30"
oop_bets = [75]
ip_bets = [75]
oop_raises = [100]
ip_raises = [100]
iterations = 1000

[[questions]]
id = "blank-ip-bets"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ 2♠, pot 10bb, 30bb behind, no rake. Your opponent (out of position) holds KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44, and checks. You hold AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On the turn and the river either player may check or bet 75% of the pot, and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your range bets?"
answer = 67.36
tolerance = 0.1
unit = "%"
explanation = "About two thirds. All of AA and AK bet; most of the hands without a pair (AQ suited, AJ suited, A5 suited, 76 suited) bet too; the middle hands (QQ, JJ, TT, and part of KQ and KJ) check more often. 75% of the pot is the only bet size in this tree."
concepts = ["turn-and-river-strategy", "range-advantage"]
compute = { kind = "solve", params = { solve = "blank", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[questions.scene]
players = 2
hero = "SB"
board = ["Kh", "9d", "4c", "2s"]
pot_bb = "10"
stacks_bb = { SB = "30", BB = "30" }
actions = { BB = "checks" }

[[questions]]
id = "ace-ip-bets"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ A♠, pot 10bb, 30bb behind, no rake. Your opponent (out of position) holds KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44, and checks. You hold AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On the turn and the river either player may check or bet 75% of the pot, and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your range bets?"
answer = 91.31
tolerance = 0.1
unit = "%"
explanation = "About 91%, up from about 67% when the turn is the 2♠ with the same ranges and tree. The ace improves five of your classes (AQ, AJ and A5 suited make a pair of aces, AK two pair, AA a set) and none of your opponent's."
concepts = ["turn-and-river-strategy", "range-advantage"]
compute = { kind = "solve", params = { solve = "ace", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[questions.scene]
players = 2
hero = "SB"
board = ["Kh", "9d", "4c", "As"]
pot_bb = "10"
stacks_bb = { SB = "30", BB = "30" }
actions = { BB = "checks" }

[[questions]]
id = "queen-ip-bets"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ Q♠, pot 10bb, 30bb behind, no rake. Your opponent (out of position) holds KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44, and checks. You hold AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On the turn and the river either player may check or bet 75% of the pot, and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your range bets?"
answer = 43.57
tolerance = 0.1
unit = "%"
explanation = "About 44%, down from about 67% when the turn is the 2♠ with the same ranges and tree. The queen gives your opponent's JT suited a straight and pairs their QJ and QT, and their equity rises to about half."
concepts = ["turn-and-river-strategy", "range-advantage"]
compute = { kind = "solve", params = { solve = "queen", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[[questions]]
id = "queen-oop-bets"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ Q♠, pot 10bb, 30bb behind, no rake. You are out of position with KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44; your opponent holds AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On the turn and the river either player may check or bet 75% of the pot, and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your range bets the turn first?"
answer = 11.84
tolerance = 0.1
unit = "%"
explanation = "About 12%. On the 2♠ and A♠ turns the same range almost never bets first; on the queen it bets part of nearly every class (all but 87 suited) and checks the rest."
concepts = ["turn-and-river-strategy"]
compute = { kind = "solve", params = { solve = "queen", path = [], max_exploitability_pct = 0.05 }, field = "share.bet", scale = 100.0 }

[[questions]]
id = "queen-equity"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ Q♠. One player holds KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44; the other holds AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. What is the exact equity (in %) of the first range if both players checked to showdown?"
answer = 49.94
tolerance = 0.05
unit = "%"
explanation = "Almost exactly half. On the 2♠ turn the same range has about 37%, and on the A♠ about 30% (the equity widgets in this lesson show both)."
concepts = ["equity", "turn-and-river-strategy"]
compute = { kind = "equity", params = { ranges = ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"], board = "Kh9d4cQs" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "turn-middle-bets"
type = "numeric"
prompt = "Turn K♥ 9♦ 4♣ 2♠, pot 10bb, 30bb behind, no rake. Your opponent (out of position) holds KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44, and checks. You hold AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On the turn and the river either player may check or bet 75% of the pot, and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your QQ, JJ and TT (pairs below the king) bets?"
answer = 29.82
tolerance = 0.1
unit = "%"
explanation = "About 30%, against 100% of AA and AK. QQ never bets; JJ and TT bet part of the time."
concepts = ["turn-and-river-strategy", "range-checking"]
compute = { kind = "solve", params = { solve = "blank", path = [{ type = "action", value = 0 }], subset = ["QQ", "JJ", "TT"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[[questions]]
id = "river-strong-bets"
type = "numeric"
prompt = "River K♥ 9♦ 4♣ 2♠ 6♥, pot 25bb, 22.5bb behind, no rake. On the turn (pot 10bb) your opponent checked, you bet 7.5bb and they called; they now check again. They started the turn with KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44; you with AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On each street either player may check or bet 75% of the pot (here 18.75bb), and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your AA, AK, KQ and KJ suited bets?"
answer = 99.12
tolerance = 0.1
unit = "%"
explanation = "Nearly all of them. AA is an overpair and AK, KQ and KJ are a pair of kings with a better kicker than KT, so each beats your opponent's KT and 98 suited and loses only to 44."
concepts = ["turn-and-river-strategy", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "blank", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }, { type = "action", value = 1 }, { type = "card", value = "6h" }, { type = "action", value = 0 }], subset = ["AA", "AKs", "AKo", "KQs", "KQo", "KJs"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[questions.scene]
players = 2
hero = "SB"
board = ["Kh", "9d", "4c", "2s", "6h"]
pot_bb = "25"
stacks_bb = { SB = "22.5", BB = "22.5" }
actions = { BB = "checks" }

[[questions]]
id = "river-middle-bets"
type = "numeric"
prompt = "River K♥ 9♦ 4♣ 2♠ 6♥, pot 25bb, 22.5bb behind, no rake. On the turn (pot 10bb) your opponent checked, you bet 7.5bb and they called; they now check again. They started the turn with KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44; you with AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On each street either player may check or bet 75% of the pot (here 18.75bb), and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your JJ and TT bets? (QQ never bets the turn in this solve, so it does not reach this river.)"
answer = 0.0
tolerance = 0.1
unit = "%"
explanation = "None: JJ and TT always check. The same pairs bet part of the time on the turn; on the river they stop betting entirely."
concepts = ["turn-and-river-strategy", "polarised-ranges"]
compute = { kind = "solve", params = { solve = "blank", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }, { type = "action", value = 1 }, { type = "card", value = "6h" }, { type = "action", value = 0 }], subset = ["QQ", "JJ", "TT"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[[questions]]
id = "river-bluff-share"
type = "numeric"
prompt = "River K♥ 9♦ 4♣ 2♠ 6♥, pot 25bb, 22.5bb behind, no rake. On the turn (pot 10bb) your opponent checked, you bet 7.5bb and they called; they now check again. They started the turn with KT, QJ suited, QT suited, JT suited, T9 suited, 98 suited, 87 suited or 44; you with AA, AK, KQ, KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited or 76 suited. On each street either player may check or bet 75% of the pot (here 18.75bb), and after a bet the other may fold, call or raise all-in once. In RiverLab's solve of this tree, what share (in %) of your 18.75bb bets are AQ suited, AJ suited, A5 suited and 76 suited (ace high or a pair of sixes, which lose to every hand your opponent still holds)?"
answer = 24.19
tolerance = 0.1
unit = "%"
explanation = "About 24%: the rest of the bets are AA, AK, KQ and KJ suited. The single-bet model's bluff share for this size is 18.75 / (25 + 2 × 18.75) = 30%; the solve's tree is not that model (the caller can raise all-in, and holds 44, which beats every hand that bets), so the two need not agree."
concepts = ["turn-and-river-strategy", "bluff-to-value-ratios"]
compute = { kind = "solve", params = { solve = "blank", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }, { type = "action", value = 1 }, { type = "card", value = "6h" }, { type = "action", value = 0 }], subset = ["AQs", "AJs", "A5s", "76s"], max_exploitability_pct = 0.05 }, field = "subset_of_action.bet", scale = 100.0 }

[[questions]]
id = "why-middle-checks"
type = "multiple_choice"
prompt = "River K♥ 9♦ 4♣ 2♠ 6♥ after a turn bet and call. The caller holds KT, 98 suited or 44; in RiverLab's solve, facing an 18.75bb bet into 25bb they fold all of 98 suited, call or fold KT about half of the time each (raising it rarely), and raise all-in with 44. Why does the solved strategy check JJ and TT rather than bet them?"
options = [
  "A bet with JJ or TT is called or raised only by hands that beat it (KT, 44), while the one hand it beats (98 suited) folds",
  "JJ and TT are the strongest hands in the bettor's range",
  "Pocket pairs should never be bet on the river",
]
answer = 0
explanation = "Betting a middle hand gains only if worse hands call or better hands fold. Here every worse hand folds and every better hand continues, so the bet gains nothing that checking does not, and loses more when it meets KT or 44. Checking keeps the showdown win against 98 suited. (This is a heuristic reading of the solved strategy for this spot; in other spots middle hands may bet for thin value or to deny equity.)"
concepts = ["turn-and-river-strategy", "polarised-ranges"]
+++

## The spot {#spot}

::: solver
A turn spot, solved by RiverLab each time the content is synced: pot 10bb, 30bb behind, no
rake. The **out-of-position** player holds KT (suited and offsuit), QJ suited, QT suited, JT
suited, T9 suited, 98 suited, 87 suited and 44. The **in-position** player holds AA, AK, KQ,
KJ suited, QQ, JJ, TT, AQ suited, AJ suited, A5 suited and 76 suited. The flop is K♥ 9♦ 4♣ and
the turn is the 2♠, the A♠ or the Q♠ (three separate solves). On the turn and the river either
player may check or bet **75% of the pot**; after a bet the other may fold, call or raise once,
and every raise is all-in (a pot-sized raise would be more than the stacks). Each solve is
within 0.05% of the pot of an equilibrium of its tree: an **approximate equilibrium for these
sizes only**.
:::

::: heuristic
The ranges are an illustration of a common shape, not ranges anyone should play: the in-position
player has the strongest hands (AA, AK) and some hands with nothing, as a flop bettor might; the
out-of-position player has a king with a weak kicker, draws and middle pairs, as a flop caller
might.
:::

::: simple
The **turn** is the fourth board card and the **river** the fifth. Betting the turn after
betting the flop is often called **barrelling**. This lesson looks at when a solver bets the turn,
and what its river bets look like after the turn bet is called.
:::

## A blank turn {#blank}

```riverlab:equity
{"hands": ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"], "board": "Kh9d4c2s"}
```

```riverlab:question
{"id": "blank-ip-bets"}
```

```riverlab:question
{"id": "turn-middle-bets"}
```

::: intermediate
On the 2♠ the bettor's range splits three ways. AA and AK bet every time; the hands without a pair
(AQ, AJ and A5 suited, and 76 suited) mostly bet; the middle hands (QQ, JJ, TT and some of KQ and KJ) check
more often. As a heuristic reading: the strongest hands want to build the pot, the weakest can
win it only if the opponent folds, and the middle hands do well enough at showdown without a
bet.
:::

## A turn card that helps one range {#turn-card}

::: fact
The A♠ improves five of the in-position player's classes (AQ, AJ and A5 suited make a pair of
aces, AK two pair, AA a set) and none of the out-of-position player's. The Q♠ gives JT suited a
straight (K Q J T 9) and pairs QJ and QT for the out-of-position player, and gives the
in-position player's QQ a set, KQ two pair and AQ suited a pair of queens. The exact equities on each turn:
:::

```riverlab:equity
{"hands": ["KTs,KTo,QJs,QTs,JTs,T9s,98s,87s,44", "AA,AK,KQ,KJs,QQ,JJ,TT,AQs,AJs,A5s,76s"], "board": "Kh9d4cAs"}
```

```riverlab:question
{"id": "queen-equity"}
```

```riverlab:question
{"id": "ace-ip-bets"}
```

```riverlab:question
{"id": "queen-ip-bets"}
```

```riverlab:question
{"id": "queen-oop-bets"}
```

::: intermediate
With the same ranges and the same tree, the turn card alone moves the in-position player's
betting from about 67% (2♠) to about 91% (A♠) or about 44% (Q♠), and on the Q♠ the
out-of-position player starts betting part of its range. In these three solves the bettor bets
more often on the card that raises its equity and less often on the card that lowers it.
:::

::: heuristic
A common rule of thumb follows the same direction: keep betting on turn cards that improve your
range more than your opponent's, and slow down on cards that improve theirs. Three solves of one
spot are consistent with it; they do not prove it in general.
:::

## The river after a turn bet is called {#river}

::: solver
In the 2♠ solve, follow the line where the out-of-position player checks the turn, the
in-position player bets 7.5bb and is called, the river is the 6♥ and the out-of-position player
checks. The pot is 25bb with 22.5bb behind; a 75% bet is 18.75bb. At this point the caller
holds only KT, 98 suited and 44 (in the solve the rest of its range folds to the turn bet), so
its range is capped: its best hand is a set of fours.
:::

```riverlab:question
{"id": "river-strong-bets"}
```

```riverlab:question
{"id": "river-middle-bets"}
```

```riverlab:question
{"id": "river-bluff-share"}
```

```riverlab:question
{"id": "why-middle-checks"}
```

::: intermediate
On the turn, JJ and TT bet part of the time. On this river they never do, while the kings bet
almost always and the hands with no showdown value bet part of the time. The river bet is
**polarised**: strong hands and bluffs, with the middle checking. On the river no card is left
to come, so a hand's value at showdown is fixed and a bet only gains from what the opponent does
next.
:::

::: advanced
The solve's bluff share (about 24% of the 18.75bb bets) is not the single-bet model's 30%
(18.75 / (25 + 37.5)). That model assumes the bettor's value hands beat every hand that calls
and the caller cannot raise. Here the caller can raise all-in, and its 44 beats every hand the
bettor bets with, so the conditions of the model do not hold and the solve need not match it.
Which of these differences moves the number, and by how much, would take further solves to
show.
:::

::: approximation
Real turn and river play has more sizes, more raises, wider ranges and earlier streets that shape
both ranges. The frequencies here belong to this tree. What carries over is the pattern to look
for, not the numbers.
:::
