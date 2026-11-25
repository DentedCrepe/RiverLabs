+++
slug = "hand-rankings"
title = "Hand rankings and showdowns"
level = 1
order = 2
version = 2
status = "published"
summary = "Which five-card hands beat which, how kickers decide ties, and how to read a showdown when the board is shared."
estimated_minutes = 15
objectives = [
  "Order the nine hand categories from high card to straight flush.",
  "Find each player's best five cards from their two cards and the five on the board.",
  "Decide ties with kickers, and recognise when the board plays and the pot is split.",
]
prerequisites = ["terminology"]
tags = ["rules", "showdown"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every showdown question is checked by RiverLab's exhaustively validated hand evaluator when the content is synced; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/hand-analysis"
label = "Showdown evaluator"
why = "Enter a board and any hands to see each player's best five cards and who wins."

[[questions]]
id = "order"
type = "multiple_choice"
prompt = "Which of these hands ranks highest?"
options = ["A straight", "A flush", "Three of a kind"]
answer = 1
explanation = "From the top: straight flush, four of a kind, full house, flush, straight, three of a kind, two pair, one pair, high card. A flush beats a straight."
concepts = ["hand-rankings"]

[[questions]]
id = "counterfeit"
type = "decision"
prompt = "River. The board is K♠ K♦ 7♣ 7♥ 2♠. You hold A♣ 3♦ and your opponent shows Q♠ Q♦. Who wins the pot?"
options = ["You win", "Your opponent wins", "Split pot"]
answer = 1
explanation = "Your cards add only a kicker: your best five are the board's kings and sevens with your ace. Your opponent's pair of queens is higher than the board's sevens, so their best five are kings and queens with a seven. Two pair is compared by the higher pair, then the lower pair, then the kicker: queens beat sevens, so the ace kicker never comes into it."
concepts = ["hand-rankings"]
showdown_check = { board = "KsKd7c7h2s", you = "Ac3d", opponent = "QsQd" }

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ac", "3d"]
board = ["Ks", "Kd", "7c", "7h", "2s"]
folded = ["UTG", "HJ", "CO", "SB"]
actions = { BB = "shows" }
shown = { BB = ["Qs", "Qd"] }

[[questions]]
id = "kicker"
type = "decision"
prompt = "River. The board is A♥ K♦ 8♣ 5♠ 2♥. You hold A♦ J♣ and your opponent shows A♠ T♣. Who wins the pot?"
options = ["You win", "Your opponent wins", "Split pot"]
answer = 0
explanation = "Both of you have a pair of aces. The best five cards are A A K J 8 for you and A A K T 8 for your opponent: after the pair and the king, your jack beats their ten."
concepts = ["hand-rankings"]
showdown_check = { board = "AhKd8c5s2h", you = "AdJc", opponent = "AsTc" }

[questions.scene]
players = 6
hero = "CO"
hero_cards = ["Ad", "Jc"]
board = ["Ah", "Kd", "8c", "5s", "2h"]
folded = ["UTG", "HJ", "SB", "BB"]
actions = { BTN = "shows" }
shown = { BTN = ["As", "Tc"] }

[[questions]]
id = "board-plays"
type = "decision"
prompt = "River. The board is 5♥ 6♦ 7♣ 8♠ 9♥. You hold A♣ 2♦ and your opponent shows K♥ 3♠. Who wins the pot?"
options = ["You win", "Your opponent wins", "Split pot"]
answer = 2
explanation = "The board is a nine-high straight and neither of you can improve it: your cards do not play. Both best hands are 5-6-7-8-9, so the pot is split. A player holding a ten would have won with a ten-high straight."
concepts = ["hand-rankings"]
showdown_check = { board = "5h6d7c8s9h", you = "Ac2d", opponent = "Kh3s" }

[[questions]]
id = "five-suited"
type = "decision"
prompt = "River. The board is A♥ 9♥ 4♥ 2♣ 2♦. You hold K♥ 3♣ and your opponent shows Q♥ 2♥. Who wins the pot?"
options = ["You win", "Your opponent wins", "Split pot"]
answer = 1
explanation = "A flush needs five cards of one suit. With one heart you have only four, so your hand is a pair of deuces. Your opponent has two hearts: an ace-high flush (better than the three deuces they also make), which beats your pair."
concepts = ["hand-rankings"]
showdown_check = { board = "Ah9h4h2c2d", you = "Kh3c", opponent = "Qh2h" }

[[questions]]
id = "suits"
type = "multiple_choice"
prompt = "In hold'em, can the suit of a card ever decide which of two hands wins?"
options = [
  "Yes: spades are the highest suit",
  "No: suits never break ties; hands of equal rank split the pot",
  "Only when both players have a flush",
]
answer = 1
explanation = "Hands are compared by the ranks of their best five cards only. Two flushes are compared card by card from the top; if every rank matches, the pot is split."
concepts = ["hand-rankings"]
+++

## The nine categories {#categories}

::: fact
From highest to lowest: **straight flush**, **four of a kind**, **full house**, **flush**,
**straight**, **three of a kind**, **two pair**, **one pair**, **high card**. Any hand in a
higher category beats every hand in a lower one.
:::

- **Straight flush:** five in a row, all one suit (the ace-high one is a royal flush).
- **Four of a kind:** four cards of one rank.
- **Full house:** three of one rank and two of another.
- **Flush:** five cards of one suit.
- **Straight:** five ranks in a row. The ace can be high (T-J-Q-K-A, the highest straight) or
  low (A-2-3-4-5, the lowest straight, which counts as five-high). Straights do not wrap
  around the ace: Q-K-A-2-3 is not a straight.
- **Three of a kind:** three cards of one rank.
- **Two pair:** two cards of one rank and two of another.
- **One pair:** two cards of one rank.
- **High card:** when nothing else fits.

::: simple
You do not need to memorise how often each hand comes up: learn the order above. The maths view
shows how the categories are counted.
:::

::: math
Of the $\binom{52}{5} = 2{,}598{,}960$ five-card hands:
- straight flushes: 10 high cards (five to ace) in each of 4 suits, $10 \times 4 = 40$;
- four of a kind: 13 ranks, times 48 possible fifth cards, $13 \times 48 = 624$;
- full house: a rank for the three ($13 \times \binom43 = 13 \times 4$) and another for the pair
  ($12 \times \binom42 = 12 \times 6$), $13 \cdot 4 \cdot 12 \cdot 6 = 3{,}744$;
- flush: 5 of the 13 ranks in one of 4 suits, minus the 10 straight flushes in each suit,
  $4 \times (\binom{13}{5} - 10) = 4 \times (1{,}287 - 10) = 5{,}108$;
- straight: 10 high cards, any suit for each of the five cards, minus the 40 straight flushes,
  $10 \times 4^5 - 40 = 10{,}240 - 40 = 10{,}200$.

For five-card hands, a rarer category always ranks higher. That stops being true when you take
the best five of seven cards, as in hold'em: there, ending up with only high card is rarer than
ending up with one pair, or with two pair. (RiverLab's hand evaluator is checked against all
$\binom{52}{7} = 133{,}784{,}560$ seven-card hands, which confirms this.) The ranking stays the
same; it is fixed by the five-card counts.
:::

## Your best five cards {#best-five}

::: fact
At showdown each player makes the **best five-card hand** from their two cards and the five
board cards. You may use both, one or none of your own cards.
:::

Because the board is shared, both players often use the same board cards. What matters is
what your two cards **add**.

A pair in your hand can be **counterfeited**: the board pairs in a way that makes it useless.
With K K 7 7 on the board, a player holding 3 3 has only the board's kings and sevens plus a
kicker; the threes do not play. A player holding Q Q is different: queens are higher than the
board's sevens, so they make kings and queens, which beats the board's two pair.

```riverlab:question
{"id": "counterfeit"}
```

## Kickers and split pots {#kickers}

::: fact
Hands in the same category are compared rank by rank: first the cards that make the category,
then the remaining cards ("kickers") from the highest down, **only up to five cards in total**.
- **One pair, three of a kind, four of a kind:** the pair, trips or quads first, then the kickers.
- **Two pair:** the higher pair, then the lower pair, then the kicker.
- **Full house:** the three of a kind first, then the pair.
- **Straight:** the highest card of the straight.
- **Flush and high card:** card by card from the highest down.

If all five ranks match, the pot is split.
:::

::: intermediate
Kickers beyond five cards never count. On A K 8 5 2, a player holding an ace makes A A K 8 and
then the higher of their other card and the board's 5, as long as that other card does not pair
the board (A K, for example, makes two pair: aces and kings). So A J beats A T, but A 4 and A 3
both make A A K 8 5 and split the pot: neither low card plays.
:::

```riverlab:question
{"id": "kicker"}
```

When the board itself is the best possible hand for everyone, "the board plays" and the pot is
split, whatever your cards are.

```riverlab:question
{"id": "board-plays"}
```
