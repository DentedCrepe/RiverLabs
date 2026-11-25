+++
slug = "terminology"
title = "Poker terminology"
level = 1
order = 1
version = 2
status = "published"
summary = "The words the rest of the Academy uses: the streets, the actions, the blinds and the button, ranges, combinations, equity and position."
estimated_minutes = 12
objectives = [
  "Name the four betting rounds and the actions available in each.",
  "Tell a bet from a raise, and a 3-bet from an open.",
  "Use range, combination, equity and position the way the rest of the Academy does.",
]
prerequisites = []
tags = ["rules", "vocabulary"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Definitions follow the betting rules RiverLab's engine implements (docs/poker-rules.md); awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/play"
label = "Play against bots"
why = "Every term here shows up at the table: blinds, the button, bets, raises and showdowns."

[[reading]]
title = "The Theory of Poker"
authors = "David Sklansky"
year = 1999

[[questions]]
id = "streets"
type = "multiple_choice"
prompt = "In hold'em, what are the four betting rounds, in order?"
options = ["Preflop, flop, turn, river", "Flop, turn, river, showdown", "Blinds, flop, turn, river"]
answer = 0
explanation = "Betting happens before any board card (preflop), after the first three board cards (the flop), after the fourth (the turn) and after the fifth (the river). Showdown is when the remaining players show their hands; there is no betting in it."
concepts = ["terminology"]

[[questions]]
id = "bet-or-raise"
type = "multiple_choice"
prompt = "On the flop, the first player checks and you put 3bb into the pot. Is that a bet or a raise?"
options = ["A bet: nobody had put chips in on this street yet", "A raise: you are putting in more than the player before you", "A call"]
answer = 0
explanation = "The first chips put in on a street are a bet. Putting in more after someone else has bet is a raise."
concepts = ["terminology"]

[[questions]]
id = "three-bet"
type = "multiple_choice"
prompt = "Before the flop, the cutoff raises to 2.5bb and the button raises to 8bb. What is the button's raise called?"
options = ["A 3-bet", "An open", "A 4-bet"]
answer = 0
explanation = "Preflop, the big blind counts as the first bet, the first raise (the open) is the second, and a re-raise is the third: a 3-bet. A raise over a 3-bet is a 4-bet."
concepts = ["terminology"]

[[questions]]
id = "range"
type = "multiple_choice"
prompt = "What does a player's \"range\" mean?"
options = [
  "Every hand they could hold in this spot, given how they have played",
  "The single hand you think they have",
  "The difference between their biggest and smallest bets",
]
answer = 0
explanation = "Opponents rarely show their cards, so you think about every hand they would play this way. Each specific two-card hand in a range, such as A♠K♦, is a combination (combo)."
concepts = ["terminology", "ranges"]

[[questions]]
id = "equity"
type = "multiple_choice"
prompt = "What is your equity in a hand?"
options = [
  "The share of the pot you would win on average if all the remaining cards were dealt with no more betting",
  "The number of chips you have",
  "How much you have put into the pot so far",
]
answer = 0
explanation = "Equity counts wins, and an equal share of ties (half, heads-up), over every way the remaining cards can come. It ignores future betting; what you actually win also depends on how the hand is played."
concepts = ["terminology", "equity"]

[[questions]]
id = "in-position"
type = "multiple_choice"
prompt = "After the flop, the button and the big blind are left. Which player is \"in position\"?"
options = ["The button, who acts last on every street after the flop", "The big blind, who acts first", "Whoever raised before the flop"]
answer = 0
explanation = "In position means acting after your opponent, so you see what they do before you decide. The button is in position on every street after the flop."
concepts = ["terminology", "positions"]
+++

## The streets {#streets}

::: fact
Each player gets two hole cards. There are four betting rounds: **preflop**, then the
**flop** (three board cards), the **turn** (a fourth) and the **river** (a fifth). If more than
one player is left after the river, there is a **showdown**: each player makes their best
five-card hand from the board and any of their hole cards (both, one or none), and the best hand
wins.
:::

```riverlab:question
{"id": "streets"}
```

## Actions {#actions}

::: fact
- **Check:** pass without putting chips in (only when nobody has bet on this street).
- **Bet:** the first chips put in on a street.
- **Call:** match the current bet.
- **Raise:** put in more than the current bet. Preflop, the first raise when nobody has limped
  in (just called the big blind) is the **open**; a raise over it is a **3-bet**, and a raise
  over that a **4-bet**.
- **Fold:** give up the hand and anything already put in.
- **All-in:** put in every remaining chip.

The same actions are used on all four streets, with one difference preflop: the big blind counts
as the first bet, so before the flop players can fold, call or raise, but not bet. The big blind
itself may check if nobody has raised, because its blind already matches the bet (this is called
the big blind's **option**).
:::

::: simple
Think of each street as a fresh round: the first person to put chips in bets, anyone who puts in
more raises, and everyone else has to match it, fold, or raise again.
:::

```riverlab:question
{"id": "bet-or-raise"}
```

```riverlab:question
{"id": "three-bet"}
```

## Blinds, the button and the pot {#blinds}

::: fact
The **small blind** and **big blind** are forced bets posted before the cards are dealt, so
there is always something to win. Amounts in RiverLab are measured in **big blinds (bb)**. The
**button** marks the dealer seat and moves one seat to the left each hand. The **pot** is every
chip put in so far; **rake** is the share the house takes from it. **Effective stack** is the
smaller of the two stacks in a heads-up pot: nobody can win more than that from the other. With
more players, there is an effective stack between you and each opponent: the smaller of your
stack and theirs.
:::

## Ranges, combinations and equity {#ranges}

::: fact
A **range** is every hand a player could hold given their actions. Each specific two-card
hand is a **combination**; hands like "AK offsuit" are **hand classes**. **Equity** is the
share of the pot a hand or range wins on average if the remaining cards are dealt with no
more betting.
:::

```riverlab:question
{"id": "range"}
```

```riverlab:question
{"id": "equity"}
```

## Position {#position}

::: fact
**In position (IP)** means acting after your opponent; **out of position (OOP)** means acting
first. After the flop the button is in position against everyone.
:::

```riverlab:question
{"id": "in-position"}
```
