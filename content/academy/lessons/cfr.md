+++
slug = "cfr"
title = "Counterfactual regret minimisation: CFR, CFR+ and DCFR"
level = 5
order = 6
version = 2
status = "published"
summary = "How solvers apply regret matching at every information set, what counterfactual values are, and how CFR+ and Discounted CFR speed it up."
estimated_minutes = 20
objectives = [
  "Explain what CFR computes at each information set on each iteration.",
  "Define a counterfactual value.",
  "State what CFR+ and Discounted CFR change, and which variant RiverLab uses.",
]
prerequisites = ["game-trees", "regret-matching", "exploitability"]
tags = ["solvers", "cfr"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance from the cited papers. The variant RiverLab uses is taken from its solver configuration (engine/solver-interface); awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Watch exploitability fall as a Discounted CFR solve runs."

[[reading]]
title = "Regret Minimization in Games with Incomplete Information"
authors = "Martin Zinkevich, Michael Johanson, Michael Bowling and Carmelo Piccione"
year = 2007
note = "NIPS 2007. Introduces counterfactual regret minimisation."

[[reading]]
title = "Monte Carlo Sampling for Regret Minimization in Extensive Games"
authors = "Marc Lanctot, Kevin Waugh, Martin Zinkevich and Michael Bowling"
year = 2009
note = "NIPS 2009. Sampling variants of CFR."

[[reading]]
title = "Solving Large Imperfect Information Games Using CFR+"
authors = "Oskari Tammelin"
year = 2014
note = "arXiv preprint introducing CFR+."

[[reading]]
title = "Solving Imperfect-Information Games via Discounted Regret Minimization"
authors = "Noam Brown and Tuomas Sandholm"
year = 2019
note = "AAAI 2019. Discounted CFR and its recommended parameters."

[[questions]]
id = "info-set-update"
type = "numeric"
prompt = "At one information set, a player's cumulative counterfactual regrets are: call +6, fold −2, raise +2. Using regret matching, how often (in %) does their next strategy call?"
answer = 75.0
tolerance = 0.05
unit = "%"
explanation = "Only positive regrets count: call 6, fold 0, raise 2. Call gets 6 / (6 + 0 + 2) = 75%, raise 25%, fold 0%."
concepts = ["cfr", "regret-matching"]
compute = { kind = "regret_matching", params = { iterations = 1, start_regrets = [6.0, -2.0, 2.0] }, field = "0.current.0", scale = 100.0 }

[[questions]]
id = "what-cfr-does"
type = "multiple_choice"
prompt = "On each iteration, what does CFR do at every information set?"
options = [
  "It picks the single best action and plays it from then on",
  "It updates each action's cumulative regret using counterfactual values, then sets the next strategy by regret matching",
  "It samples random strategies and keeps the best one",
]
answer = 1
explanation = "Each iteration walks the tree, computes counterfactual values for each action at each information set, adds the regrets, and regret-matches to get the next strategy. The average of these strategies converges."
concepts = ["cfr"]

[[questions]]
id = "counterfactual"
type = "multiple_choice"
prompt = "What makes a counterfactual value 'counterfactual'?"
options = [
  "It is computed as if the player had played to reach this information set: only the opponent's and chance's probabilities of reaching it are used as weights",
  "It is computed as if the opponent could see the player's cards",
  "It ignores the opponent's strategy",
]
answer = 0
explanation = "The value is weighted by how likely the opponent's choices and the cards are to lead here, while the player's own probability of getting here is left out. That keeps rarely reached situations from being ignored."
concepts = ["counterfactual-values", "cfr"]

[[questions]]
id = "which-converges"
type = "multiple_choice"
prompt = "In a two-player zero-sum game, which strategy produced by CFR is guaranteed to approach an equilibrium?"
options = [
  "The strategy of the latest iteration",
  "The average strategy over all iterations",
  "The strategy of the first iteration",
]
answer = 1
explanation = "CFR's guarantee is for the average strategy: its exploitability shrinks as iterations grow. The current strategy can keep moving."
concepts = ["cfr", "convergence"]

[[questions]]
id = "cfr-plus"
type = "multiple_choice"
prompt = "What is the main change CFR+ makes to the regret update?"
options = [
  "Cumulative regrets are not allowed below zero, so an action that becomes good again is played again quickly",
  "It stops computing counterfactual values",
  "It replaces regret matching with always playing the best action",
]
answer = 0
explanation = "CFR+ floors cumulative regrets at zero (regret matching+), updates the players alternately and weights later iterations more in the average. In practice it converges much faster than CFR."
concepts = ["cfr-plus"]

[[questions]]
id = "riverlab-variant"
type = "multiple_choice"
prompt = "Which CFR variant and parameters does RiverLab's postflop solver use?"
options = [
  "Vanilla CFR with a uniform average",
  "Discounted CFR with α = 1.5, β = 0, γ = 2",
  "Monte Carlo CFR with outcome sampling",
]
answer = 1
explanation = "RiverLab's postflop solves run GTOpen with Discounted CFR and the parameters Brown and Sandholm recommend: α = 1.5, β = 0, γ = 2."
concepts = ["discounted-cfr"]
+++

## From regret matching to CFR {#cfr}

::: fact
**Counterfactual regret minimisation** (CFR; Zinkevich et al., 2007) applies regret matching at
every information set of the game tree at once. Each iteration it:
1. walks the tree with the current strategies;
2. computes, at each information set, the counterfactual value of each action;
3. adds each action's regret (its counterfactual value minus the counterfactual value of the
   current strategy) to a running total;
4. sets the next strategy at each information set by regret matching on those totals;
5. adds the current strategy at each information set, weighted by the player's own probability of
   reaching it, to a running average. That **average strategy** is what converges.
:::

```riverlab:question
{"id": "info-set-update"}
```

```riverlab:question
{"id": "what-cfr-does"}
```

## Counterfactual values {#counterfactual}

::: fact
The **counterfactual value** of an information set for a player is its expected value weighted by
the probability that the **opponent and chance** lead there, as if the player had played to reach
it. The player's own probability of reaching it is left out.
:::

::: intermediate
Leaving out your own reach probability keeps the update meaningful at information sets your current
strategy rarely reaches: they are still improved, so if your strategy changes and starts reaching
them, it plays well there. The deeper reason is a theorem of Zinkevich et al.: with these
values, the player's overall regret is at most the sum of the regrets at the separate information
sets, so keeping each local regret small keeps the whole strategy's regret small.
:::

```riverlab:question
{"id": "counterfactual"}
```

## Convergence {#convergence}

::: fact
In a two-player zero-sum game with perfect recall (each player remembers everything they have
seen and done, as in poker), the **average** of CFR's strategies converges to an equilibrium: its
exploitability is bounded by a term that shrinks like $1/\sqrt{T}$ after $T$ iterations. The
current strategy carries no such guarantee, and with three or more players CFR is not
guaranteed to approach a Nash equilibrium at all.
:::

::: solver
In practice solvers stop at an iteration limit or when exploitability falls below a target (for
example 0.5% of the pot) and report the exploitability they reached. RiverLab shows it with every
result.
:::

```riverlab:question
{"id": "which-converges"}
```

## CFR+ and Discounted CFR {#variants}

::: fact
- **CFR+** (Tammelin, 2014) floors cumulative regrets at zero, updates the two players in turn and
  weights later iterations more in the average.
- **Discounted CFR** (Brown and Sandholm, 2019) multiplies accumulated positive regrets by
  $t^\alpha/(t^\alpha + 1)$, negative regrets by $t^\beta/(t^\beta + 1)$ and the contributions to
  the average strategy by $\big(t/(t + 1)\big)^\gamma$ on iteration $t$. The authors recommend
  $\alpha = 1.5$, $\beta = 0$, $\gamma = 2$.
- **Monte Carlo CFR** (Lanctot et al., 2009) samples parts of the tree on each iteration instead of
  walking all of it.
:::

::: heuristic
Discounting lets the solver forget the poor strategies of its early iterations faster. In the
experiments reported by Tammelin (2014) and Brown and Sandholm (2019), CFR+ and Discounted CFR
usually reach a given exploitability in far fewer iterations than vanilla CFR; for CFR+ part of
the gain also comes from flooring regrets and from updating the players in turn.
:::

```riverlab:question
{"id": "cfr-plus"}
```

```riverlab:question
{"id": "riverlab-variant"}
```
