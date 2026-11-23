+++
slug = "hand-rankings"
title = "Hand rankings and showdowns"
level = 1
order = 2
version = 1
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
notes = "Drafted with AI assistance. Every showdown question is checked by RiverLab's exhaustively validated hand evaluator when the content is synced; awaiting review by a person."

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
explanation = "You use the board's two pair with your ace: kings and sevens, ace kicker. Your opponent's queens replace the board's sevens: kings and queens. The higher second pair wins, so the ace kicker never comes into it."
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
explanation = "A flush needs five cards of one suit. With one heart you have only four, so your hand is a pair of deuces. Your opponent has two hearts: an ace-high flush, which beats their own three deuces and your pair."
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

::: simple
- **Straight flush:** five in a row, all one suit (the ace-high one is a royal flush).
- **Four of a kind:** four cards of one rank.
- **Full house:** three of one rank and two of another.
- **Flush:** five cards of one suit.
- **Straight:** five ranks in a row. The ace can be high (T-J-Q-K-A) or low (A-2-3-4-5).
- **Three of a kind**, **two pair**, **one pair**, and **high card** when nothing else fits.
:::

::: math
The rarer a category, the higher it ranks. Of the $\binom{52}{5} = 2{,}598{,}960$ five-card
hands, 40 are straight flushes, 624 four of a kind, 3,744 full houses, 5,108 flushes and
10,200 straights.
:::

## Your best five cards {#best-five}

::: fact
At showdown each player makes the **best five-card hand** from their two cards and the five
board cards. You may use both, one or none of your own cards.
:::

::: simple
Because the board is shared, both players often use the same board cards. What matters is
what your two cards **add**.
:::

::: intermediate
A common surprise: on a paired board, your pair can be "counterfeited". With K K 7 7 on the
board, a player holding a pair of queens has kings and queens, and beats a player whose own
cards only add a kicker to the board's kings and sevens.
:::

```riverlab:question
{"id": "counterfeit"}
```

## Kickers and split pots {#kickers}

::: fact
Hands in the same category are compared rank by rank: first the cards that make the category
(the pair, the trips, the highest straight card), then the remaining cards ("kickers") from the
highest down, **only up to five cards in total**. If all five ranks match, the pot is split.
:::

::: intermediate
Kickers beyond five cards never count. On A K 8 5 2, a player holding an ace makes A A K 8 and
then the higher of their other card and the board's 5. So A J beats A T, but A 4 and A 3 both make
A A K 8 5 and split the pot: neither low card plays.
:::

```riverlab:question
{"id": "kicker"}
```

::: simple
When the board itself is the best possible hand for everyone, "the board plays" and the pot is
split, whatever your cards are.
:::

```riverlab:question
{"id": "board-plays"}
```
