+++
slug = "positions"
title = "Positions and the order of action"
level = 1
order = 3
version = 2
status = "published"
summary = "The six seats of a six-max table, who acts first before and after the flop, and why acting last is an advantage."
estimated_minutes = 12
objectives = [
  "Name the six positions at a six-max table.",
  "Say who acts first before the flop and after the flop, including heads-up.",
  "Explain what information acting last gives you.",
]
prerequisites = ["terminology"]
tags = ["rules", "positions"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. The order of action matches RiverLab's betting engine (poker-core positions and its rules tests); awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/play"
label = "Play against bots"
why = "Every table shows positions, the dealer button and whose turn it is."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Its models show why the player who acts last has an advantage."

[[questions]]
id = "first-preflop"
type = "multiple_choice"
prompt = "Six players, before the flop. The blinds are posted and nobody has acted yet. You are in the cutoff. Who acts first?"
options = ["Under the gun (UTG)", "The small blind", "The big blind"]
answer = 0
explanation = "Before the flop the blinds have already put chips in, so action starts with the player to the big blind's left: under the gun. The big blind acts last."
concepts = ["positions"]

[questions.scene]
players = 6
hero = "CO"
hero_cards = ["Kc", "Qc"]
bets_bb = { SB = "0.5", BB = "1" }
actions = { SB = "posts 0.5bb", BB = "posts 1bb" }

[[questions]]
id = "first-postflop"
type = "multiple_choice"
prompt = "Flop, six players. You raised to 2.5bb on the button and the big blind called; everyone else folded. Who acts first on the flop?"
options = ["You, on the button", "The big blind", "Whoever raised before the flop"]
answer = 1
explanation = "After the flop the first player still in the hand to the dealer button's left acts first: the small blind, then the big blind. With the small blind folded, the big blind acts first and you act last on every street."
concepts = ["positions"]

[questions.scene]
players = 6
hero = "BTN"
hero_cards = ["Ah", "Jd"]
board = ["Ts", "6h", "2c"]
pot_bb = "5.5"
folded = ["UTG", "HJ", "CO", "SB"]

[[questions]]
id = "heads-up"
type = "multiple_choice"
prompt = "Heads-up (two players), the small blind is also the dealer button. Who acts first before the flop, and who acts first after it?"
options = [
  "The small blind acts first before the flop; the big blind acts first after it",
  "The big blind acts first both before and after the flop",
  "The small blind acts first both before and after the flop",
]
answer = 0
explanation = "Heads-up the button posts the small blind and acts first before the flop. After the flop the order follows the button: the big blind acts first and the button acts last."
concepts = ["positions"]

[questions.scene]
players = 2
hero = "SB"
hero_cards = ["9s", "9d"]
bets_bb = { SB = "0.5", BB = "1" }
actions = { SB = "posts 0.5bb", BB = "posts 1bb" }

[[questions]]
id = "why-last"
type = "multiple_choice"
prompt = "What does acting last after the flop give you on every street?"
options = [
  "Better cards on average",
  "You see what your opponent did before you choose",
  "The right to act first on the next street",
]
answer = 1
explanation = "Position does not change the cards you are dealt. Acting last means every decision you make is made with one more piece of information: whether your opponent checked or bet, and how much."
concepts = ["positions"]
+++

## The six seats {#seats}

::: fact
At a six-max table the seats are named by their place relative to the dealer button. In the
order they act before the flop: **UTG** (under the gun), **HJ** (hijack), **CO** (cutoff),
**BTN** (button), **SB** (small blind) and **BB** (big blind).
:::

::: simple
The button moves one seat to the left after every hand, so everyone plays every position in
turn. The two players to its left post the blinds before the cards are dealt.
:::

## Who acts first {#order}

::: fact
- **Before the flop:** UTG acts first and the big blind last, because the blinds have already
  put money in.
- **After the flop:** the first player still in the hand to the button's left acts first. The
  button, if still in, acts last on the flop, turn and river.
- **Heads-up:** the button is the small blind. It acts first before the flop and last after it.
:::

```riverlab:question
{"id": "first-preflop"}
```

```riverlab:question
{"id": "first-postflop"}
```

## Why position matters {#why}

::: fact
The player who acts last sees the other player's action before choosing their own on every
street after the flop.
:::

::: heuristic
Late positions (cutoff and button) can profitably play more hands than early ones for two
reasons: they will usually act last after the flop, and fewer players are left to act behind
them, so a raise more often wins the blinds straight away. The blinds are forced to put money
in and will usually be out of position after the flop, which is why they tend to lose money over
time even with good play. Exact amounts depend on the players and the game.
:::

::: intermediate
Information is worth something concrete: in position you can check behind to see a free card,
bet when your opponent shows weakness, and control the size of the pot. Out of position you
must commit to an action first, and your opponent gets to respond.
:::

::: advanced
Players call this **equity realisation**. As a rule of thumb (a heuristic, not an exact law), a
range in position tends to win a larger share of the pot than its raw all-in equity, because it
reacts last on every street; out of position it tends to win less.
:::

::: heuristic
The same range is usually worth more in position than out of position. How much more depends
on the ranges, the stacks and the bet sizes available, so treat this as a tendency, not a fixed
amount.
:::
