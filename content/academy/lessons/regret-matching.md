+++
slug = "regret-matching"
title = "Regret matching and why solvers average"
level = 5
order = 5
version = 1
status = "published"
summary = "The update rule inside CFR, shown on rock–paper–scissors: why the current strategy keeps moving and the average strategy converges."
estimated_minutes = 25
objectives = [
  "Define regret and cumulative regret for a decision.",
  "Apply regret matching to turn regrets into a strategy.",
  "Explain why CFR reports the average strategy, and what exploitability measures.",
]
prerequisites = ["nash-equilibrium", "mixed-strategies", "exploitability", "game-trees"]
tags = ["solver", "cfr", "regret-matching", "convergence"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. The demonstration is implemented and tested in poker-theory; references were checked for existence; awaiting review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Watch exploitability fall as GTOpen runs Discounted CFR on a real spot."

[[reading]]
title = "A Simple Adaptive Procedure Leading to Correlated Equilibrium"
authors = "Sergiu Hart and Andreu Mas-Colell"
year = 2000
note = "Econometrica 68(5). Introduces regret matching."

[[reading]]
title = "Regret Minimization in Games with Incomplete Information"
authors = "Martin Zinkevich, Michael Johanson, Michael Bowling and Carmelo Piccione"
year = 2007
note = "NIPS 2007. Introduces counterfactual regret minimisation (CFR)."

[[reading]]
title = "Solving Large Imperfect Information Games Using CFR+"
authors = "Oskari Tammelin"
year = 2014
note = "arXiv:1407.5042. Regret matching+ and CFR+."

[[reading]]
title = "Solving Imperfect-Information Games via Discounted Regret Minimization"
authors = "Noam Brown and Tuomas Sandholm"
year = 2019
note = "AAAI 2019. Discounted CFR, the variant GTOpen uses."

[[questions]]
id = "match"
type = "numeric"
prompt = "Cumulative regrets for rock, paper and scissors are 3, −1 and 1. With regret matching, how often (in %) is rock played next?"
answer = 75.0
tolerance = 0.5
unit = "%"
explanation = "Only positive regrets count: 3 and 1 (paper's −1 becomes 0). Rock gets 3 / (3 + 0 + 1) = 75%."
concepts = ["regret-matching"]
compute = { kind = "regret_matching", params = { iterations = 1, start_regrets = [3.0, -1.0, 1.0] }, field = "0.current.0", scale = 100.0 }

[[questions]]
id = "average"
type = "multiple_choice"
prompt = "In self-play regret matching on rock–paper–scissors, which strategy converges to the equilibrium?"
options = [
  "The current strategy",
  "The average strategy over all iterations",
  "Neither: regret matching does not converge",
]
answer = 1
explanation = "The current strategy keeps cycling; the average strategy's exploitability goes to zero. Solvers like CFR report the average for the same reason."
concepts = ["strategy-averaging", "regret-matching"]

[[questions]]
id = "exploitability-meaning"
type = "multiple_choice"
prompt = "What does an exploitability of 0 mean for a strategy in a two-player zero-sum game?"
options = [
  "It wins against every opponent",
  "No opponent strategy can win more against it than the game's value: it is an equilibrium strategy",
  "It has never lost a hand",
]
answer = 1
explanation = "Exploitability measures how much a best response gains over the game value. At zero, the strategy is part of a Nash equilibrium. It does not mean it beats every opponent by the most possible."
concepts = ["exploitability", "nash-equilibrium"]
+++

## Regret {#regret}

::: simple
After each round, ask: "How much better would I have done if I had always picked this action?"
That difference is the action's **regret**. Add it up over many rounds, and actions you keep
wishing you had played build up high regret.
:::

::: intermediate
At a decision with actions $a$, after a round where you played strategy $\sigma$ and each action
would have been worth $u(a)$, the regret of $a$ is $u(a) - u(\sigma)$. **Cumulative regret** $R(a)$
adds these up over rounds. Positive $R(a)$ means you would have done better playing $a$ more.
:::

::: math
Let $u_t(a)$ be the value of action $a$ in round $t$ given the opponent's strategy, and
$u_t(\sigma_t) = \sum_a \sigma_t(a)\,u_t(a)$. Cumulative regret after $T$ rounds is

$$
R_T(a) = \sum_{t=1}^{T} \big(u_t(a) - u_t(\sigma_t)\big).
$$
:::

## Regret matching {#matching}

::: simple
**Regret matching** plays each action in proportion to its positive regret. Actions with no
positive regret are not played, and if nothing has positive regret, every action is played
equally often.
:::

::: math
$$
\sigma_{T+1}(a) =
\begin{cases}
\dfrac{R_T^+(a)}{\sum_b R_T^+(b)} & \text{if } \sum_b R_T^+(b) > 0,\\[2ex]
\dfrac{1}{|A|} & \text{otherwise,}
\end{cases}
\qquad R^+ = \max(R, 0).
$$

Regret matching keeps cumulative regret growing no faster than $\sqrt{T}$, so the **average**
regret per round goes to zero (Hart and Mas-Colell, 2000).
:::

::: fact
In a two-player zero-sum game, if both players' average regret goes to zero, their **average
strategies** converge to a Nash equilibrium. CFR (Zinkevich et al., 2007) runs regret matching at
every decision point of a game tree, using **counterfactual values** so that each decision's regret
can be minimised on its own.
:::

## See it converge {#demo}

::: fact
The demonstration below is rock–paper–scissors, not poker. It is the smallest game where you
can watch the mechanism. Its unique equilibrium is to play each action 1/3 of the time.
:::

```riverlab:regret-matching
{"iterations": 300, "start_regrets": [3, 0, 0]}
```

::: intermediate
Watch two things. The **current** strategy keeps swinging: when rock is over-played, paper's
regret grows, then scissors', then rock's again. The **average** strategy settles at 1/3 each,
and its exploitability (how much a best response gains against it) shrinks towards 0.
:::

::: advanced
Switch on **regret matching+** (CFR+'s rule: regrets are floored at zero after every update).
It forgets negative regret, so it reacts faster. CFR+ also changes how the average is
weighted (later iterations count more). Discounted CFR, which GTOpen uses, generalises both by
discounting positive and negative regrets and the average at different rates.
:::

## What this means for solver output {#solvers}

::: fact
A solver result is an **approximate** equilibrium of the game it was given: the bet sizes you
allowed, the ranges, the rake. Its quality is measured by exploitability. RiverLab shows that
figure with every result and re-checks it with independent best-response code.
:::

::: approximation
Exploitability near zero means the strategy is close to an equilibrium **of that abstracted
game**. It does not mean the strategy is optimal against sizes the tree left out, or against a
real opponent. That is a property of the abstraction, not of the algorithm.
:::
