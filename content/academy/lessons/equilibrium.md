+++
slug = "equilibrium"
title = "Zero-sum games, equilibrium and mixed strategies"
level = 3
order = 1
version = 1
status = "published"
summary = "What a Nash equilibrium is, why a two-player zero-sum game has a value, and why good strategies mix by making the opponent indifferent."
estimated_minutes = 20
objectives = [
  "Define a best response and a Nash equilibrium.",
  "Explain what the value of a two-player zero-sum game guarantees.",
  "Solve a 2×2 zero-sum game with the indifference principle.",
]
prerequisites = ["expected-value"]
tags = ["game-theory", "equilibrium", "mixed-strategies"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every numeric answer is computed by poker-theory (zero_sum_2x2) and re-checked when the content is synced; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Approximate equilibria of heads-up postflop spots, with how far each is from an exact one."

[[reading]]
title = "Zur Theorie der Gesellschaftsspiele"
authors = "John von Neumann"
year = 1928
note = "The minimax theorem for two-player zero-sum games."

[[reading]]
title = "Equilibrium points in n-person games"
authors = "John F. Nash"
year = 1950
note = "Proceedings of the National Academy of Sciences: every finite game has an equilibrium in mixed strategies."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006
note = "Solves many small poker games with exactly these methods."

[[questions]]
id = "pennies"
type = "numeric"
prompt = "Matching pennies: you and your opponent each show heads or tails at the same time. If they match you win 1, otherwise you lose 1. At equilibrium, how often (in %) do you show heads?"
answer = 50.0
tolerance = 0.05
unit = "%"
explanation = "Any other mix can be exploited: if you showed heads more than half the time, your opponent would always show tails. At 50% both of their options win the same, 0 on average."
concepts = ["mixed-strategies", "nash-equilibrium"]
compute = { kind = "zero_sum_2x2", params = { payoff = [[1.0, -1.0], [-1.0, 1.0]] }, field = "row_first", scale = 100.0 }

[[questions]]
id = "mix-2x2"
type = "numeric"
prompt = "In a zero-sum game you choose Top or Bottom and your opponent chooses Left or Right. You win 4 with (Top, Left), −2 with (Top, Right), −1 with (Bottom, Left) and 3 with (Bottom, Right); your opponent loses what you win. At equilibrium, how often (in %) do you play Top?"
answer = 40.0
tolerance = 0.05
unit = "%"
explanation = "Choose p (Top) to make the opponent indifferent: against Left you get 4p − (1 − p), against Right −2p + 3(1 − p). Equal when 5p − 1 = 3 − 5p, so p = 0.4."
concepts = ["indifference", "mixed-strategies"]
compute = { kind = "zero_sum_2x2", params = { payoff = [[4.0, -2.0], [-1.0, 3.0]] }, field = "row_first", scale = 100.0 }

[[questions]]
id = "value-2x2"
type = "numeric"
prompt = "In a zero-sum game you choose Top or Bottom and your opponent chooses Left or Right. You win 4 with (Top, Left), −2 with (Top, Right), −1 with (Bottom, Left) and 3 with (Bottom, Right). What is the value of the game to you, per play, when both players use their equilibrium strategies?"
answer = 1.0
tolerance = 0.001
explanation = "With Top 40%: against Left 0.4 × 4 + 0.6 × (−1) = 1, and against Right 0.4 × (−2) + 0.6 × 3 = 1. You win 1 per play whatever your opponent does."
concepts = ["nash-equilibrium", "zero-sum-games"]
compute = { kind = "zero_sum_2x2", params = { payoff = [[4.0, -2.0], [-1.0, 3.0]] }, field = "value" }

[[questions]]
id = "saddle"
type = "multiple_choice"
prompt = "You win 3 with (Top, Left), 2 with (Top, Right), 1 with (Bottom, Left) and 0 with (Bottom, Right). Should you mix between Top and Bottom?"
options = [
  "Yes, always mix 50/50 in a 2×2 game",
  "No: Top wins more than Bottom against both of your opponent's choices, so you always play Top",
  "Yes, play Top 75% of the time",
]
answer = 1
explanation = "Top dominates Bottom (3 > 1 and 2 > 0). Your opponent then plays Right to hold you to 2: this pure equilibrium has value 2. Mixing only happens when no option is best against everything."
concepts = ["mixed-strategies", "nash-equilibrium"]

[[questions]]
id = "definition"
type = "multiple_choice"
prompt = "What makes a pair of strategies a Nash equilibrium?"
options = [
  "Both players win the same amount",
  "Neither player can gain by changing their own strategy while the other keeps theirs",
  "Both players play every action equally often",
]
answer = 1
explanation = "Each strategy is a best response to the other. In a two-player zero-sum game that also means each guarantees the game's value against anything the other does."
concepts = ["nash-equilibrium"]

[[questions]]
id = "indifferent"
type = "multiple_choice"
prompt = "At a mixed equilibrium you play two actions, each some of the time. How do their expected values compare against your opponent's equilibrium strategy?"
options = [
  "They are equal",
  "The one you play more often is worth more",
  "The one you play less often is worth more",
]
answer = 0
explanation = "If one were worth more, you would gain by always playing it, so the mix would not be an equilibrium. Every action played with positive frequency has the same EV."
concepts = ["indifference"]
+++

## Zero-sum games {#zero-sum}

::: fact
A game is **zero-sum** when whatever one player wins the others lose. Heads-up poker without rake
is a two-player zero-sum game: the chips just move between the players.
:::

::: approximation
Rake takes chips out of the pot, so a raked game is not exactly zero-sum. Solvers that model rake
handle that by subtracting it from the pot.
:::

## Best responses and equilibrium {#equilibrium}

::: fact
A **best response** is a strategy that earns the most against a given opponent strategy. A **Nash
equilibrium** is a set of strategies where each one is a best response to the others: nobody can
gain by changing only their own strategy.
:::

::: intermediate
In a **two-player zero-sum game**, equilibrium strategies have a stronger property (von Neumann's
minimax theorem). There is a number, the **value** of the game, such that your equilibrium
strategy wins at least the value on average against **every** opponent strategy, and your
opponent's guarantees you win no more than it.
:::

::: math
With payoff matrix $A$ to the row player and mixed strategies $x$, $y$,

$$ \max_x \min_y\, x^{\mathsf T} A\, y \;=\; \min_y \max_x\, x^{\mathsf T} A\, y \;=\; v . $$
:::

```riverlab:question
{"id": "definition"}
```

## Why mixing happens {#mixing}

::: simple
In rock–paper–scissors any predictable choice loses to the right reply. The only way not to be
exploited is to be unpredictable: a third each. Poker is full of the same tension: a player who
always bets their strong hands and checks their weak ones is easy to read.
:::

::: fact
A **mixed strategy** chooses between actions at random with fixed probabilities. Nash proved that
every finite game has an equilibrium if mixed strategies are allowed.
:::

```riverlab:question
{"id": "pennies"}
```

::: intermediate
Mixing is not needed when one action is better against everything the opponent can do. Then the
equilibrium is pure.
:::

```riverlab:question
{"id": "saddle"}
```

## The indifference principle {#indifference}

::: fact
At a mixed equilibrium, every action a player uses with positive probability has the same expected
value against the opponent's strategy. Each player mixes in exactly the proportions that make the
**other** player indifferent between their choices.
:::

::: math
For a 2×2 game with payoffs $a, b$ (Top against Left, Right) and $c, d$ (Bottom against Left,
Right) and no dominant row or column, the row player plays Top with probability

$$ p = \frac{d - c}{a - b - c + d}, \qquad v = \frac{ad - bc}{a - b - c + d}. $$
:::

```riverlab:question
{"id": "mix-2x2"}
```

```riverlab:question
{"id": "value-2x2"}
```

::: intermediate
The minimum defence frequency and bluff ratios from the earlier lesson are this principle in a
poker setting: the bettor bluffs just often enough to make a bluff-catcher indifferent, and the
caller defends just often enough to make a bluff indifferent.
:::

```riverlab:question
{"id": "indifferent"}
```
