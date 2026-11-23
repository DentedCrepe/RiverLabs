+++
slug = "game-trees"
title = "Game trees, information sets and abstraction"
level = 5
order = 1
version = 1
status = "published"
summary = "How a solver represents a poker hand: decision, chance and terminal nodes, information sets, and the bet-size abstraction that keeps the tree small enough to solve."
estimated_minutes = 15
objectives = [
  "Name the three kinds of node in a game tree.",
  "Explain what an information set is and why poker needs them.",
  "Explain why solvers restrict bet sizes, and what that means for their results.",
]
prerequisites = ["nash-equilibrium"]
tags = ["solvers", "game-trees", "abstraction"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. The tree description matches RiverLab's postflop solver (GTOpen) and its node browser; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Browse a solved tree node by node: decisions, dealt cards and the end of each line."

[[reading]]
title = "Regret Minimization in Games with Incomplete Information"
authors = "Martin Zinkevich, Michael Johanson, Michael Bowling and Carmelo Piccione"
year = 2007
note = "NIPS 2007. Sets out extensive games with information sets as used by CFR."

[[questions]]
id = "chance-node"
type = "multiple_choice"
prompt = "In a solver's game tree, what kind of node deals the turn card?"
options = ["A decision node", "A chance node", "A terminal node"]
answer = 1
explanation = "Chance nodes are where cards are dealt: each child is one possible card, weighted by its probability. Decision nodes are a player's choices; terminal nodes are folds and showdowns."
concepts = ["game-trees"]

[[questions]]
id = "info-set"
type = "multiple_choice"
prompt = "Why must a poker strategy be defined per information set rather than per node of the full game tree?"
options = [
  "Because information sets are smaller and faster to store",
  "Because a player cannot see the opponent's cards, so they cannot tell apart the nodes that differ only in those cards and must act the same way in all of them",
  "Because every node is its own information set in poker",
]
answer = 1
explanation = "An information set groups the nodes a player cannot distinguish. Holding A♠ K♠ after the same actions on the same board, you cannot know which hand your opponent holds, so one strategy must cover all of those nodes."
concepts = ["game-trees"]

[[questions]]
id = "count-terminals"
type = "numeric"
prompt = "River. You face a bet and may fold, call, or raise to one fixed size. If you raise, your opponent may only fold or call. How many terminal nodes (ends of the hand) does this small tree have?"
answer = 4
tolerance = 0
explanation = "You fold (1 end), you call and go to showdown (2), you raise and they fold (3), you raise and they call (4). On the river, a call always ends the betting."
concepts = ["game-trees"]

[[questions]]
id = "abstraction"
type = "multiple_choice"
prompt = "A solve allows bets of 33% and 75% of the pot only. A player in a real hand bets 50% of the pot. What does the solution say about that bet?"
options = [
  "It is a mistake, because the solver never bets 50%",
  "Nothing directly: the solution is an approximate equilibrium only for the sizes in its tree; to study the 50% bet you must add that size and solve again",
  "The 50% bet should be treated exactly like the 33% bet",
]
answer = 1
explanation = "A solution is an equilibrium of the abstracted game. Sizes outside the tree were never considered, so the solve cannot judge them; mapping them to nearby sizes is an approximation."
concepts = ["abstraction", "game-trees"]
+++

## Nodes {#nodes}

::: fact
A **game tree** describes every way a hand can unfold. It has three kinds of node:
- **decision nodes**, where a player chooses an action (fold, check, call, bet or raise a size);
- **chance nodes**, where cards are dealt;
- **terminal nodes**, where the hand ends with a fold or a showdown, and the payoffs are known.
:::

::: simple
Think of a map of every possible line, from the current point to the end of the hand. Each
branch is a choice or a card.
:::

```riverlab:question
{"id": "chance-node"}
```

```riverlab:question
{"id": "count-terminals"}
```

## Information sets {#information-sets}

::: fact
In poker you cannot see your opponent's cards. An **information set** is the set of nodes a player
cannot tell apart: the same public actions and board, and the same own cards, but different hidden
cards for the opponent. A strategy chooses one action distribution per information set.
:::

::: intermediate
This is what makes poker a game of **imperfect information**, and why its equilibria involve whole
ranges: your choice with one hand has to work against every hand your opponent could hold, and
your opponent reasons about every hand you could hold.
:::

```riverlab:question
{"id": "info-set"}
```

## Abstraction {#abstraction}

::: fact
No-limit hold'em allows any bet size, so its real tree is far too large to solve. Solvers allow a
small set of sizes (for example 33% and 75% of the pot) and sometimes an all-in. This is a
**bet-size abstraction**.
:::

::: solver
A solver result is an approximate equilibrium of the abstracted game: the best play given that
both players may only use those sizes. It says nothing directly about sizes outside the tree.
:::

::: intermediate
Tree size grows quickly. Every extra size adds branches at every decision, and each branch has its
own subtree with its own turn and river cards. Adding one size on every street can multiply the
memory a solve needs several times over; RiverLab's "Solve this spot" chooses smaller trees when a
full one would not fit.
:::

```riverlab:question
{"id": "abstraction"}
```
