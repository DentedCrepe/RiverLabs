+++
slug = "multiway"
title = "Multiway pots: more players, less equity, weaker guarantees"
level = 6
order = 1
version = 1
status = "published"
summary = "With three or more players in the pot, every hand's equity shrinks, defending against a bet becomes a shared job, and game theory stops promising what it promises heads-up. Exact equities from the engine, the defence arithmetic under a stated simplification, and what RiverLab can and cannot solve."
estimated_minutes = 20
objectives = [
  "Compute exact three-way equities and see how a hand's equity falls as players are added.",
  "Explain why weaker made hands lose more equity than strong ones when opponents are added.",
  "Work out, under a stated simplification, how the minimum defence against a bet is shared between two defenders.",
  "State what game theory does and does not guarantee with three or more players, and what RiverLab's solver and bots do in multiway pots.",
]
prerequisites = ["equity", "minimum-defence-frequency", "exploitability", "nash-equilibrium"]
tags = ["multiway", "math", "game-theory"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every equity is an exact enumeration by RiverLab's equity engine and every defence figure comes from the engine's bet math (poker_theory), recomputed each time the content is synced; the independent-defenders figure (29.3%) is plain arithmetic shown in the text. No solver output is used: RiverLab's solver is heads-up only. Awaiting review by a person."

[[tools]]
href = "/postflop"
label = "Equity calculator"
why = "Exact equity for up to six hands or ranges at once: add players one at a time and watch each hand's share fall."

[[tools]]
href = "/play"
label = "Play"
why = "Multiway pots against the bots, which follow simple documented multiway rules (practice opponents, not solvers)."

[[tools]]
href = "/solver"
label = "Solver"
why = "Heads-up postflop solves only: a multiway spot has to be reduced to two players before it can be solved."

[[reading]]
title = "Non-Cooperative Games"
authors = "John Nash"
year = 1951
note = "Annals of Mathematics. Proves that every finite game with any number of players has an equilibrium in mixed strategies."

[[reading]]
title = "Superhuman AI for multiplayer poker"
authors = "Noam Brown and Tuomas Sandholm"
year = 2019
note = "Science. A six-player no-limit hold'em program; the paper discusses why the guarantees of two-player zero-sum games do not carry over to six players."

[[questions]]
id = "aks-vs-qq"
type = "numeric"
prompt = "Preflop, all-in with no more betting: A♠K♠ against Q♥Q♦, two players only. What is the exact equity (in %) of A♠K♠?"
answer = 46.21
tolerance = 0.01
unit = "%"
explanation = "About 46.2%, and Q♥Q♦ has about 53.8%: an exact enumeration of every five-card board, with ties split."
concepts = ["equity", "multiway-game-theory"]
compute = { kind = "equity", params = { ranges = ["AsKs", "QhQd"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "aks-three-way"
type = "numeric"
prompt = "Preflop, all-in with no more betting, three players: A♠K♠, Q♥Q♦ and 7♣6♣. What is the exact equity (in %) of A♠K♠?"
answer = 37.79
tolerance = 0.01
unit = "%"
explanation = "About 37.8%, down from about 46.2% against Q♥Q♦ alone. Q♥Q♦ has about 39.6% and 7♣6♣ about 22.6%; the three add up to 100%."
concepts = ["equity", "multiway-game-theory"]
compute = { kind = "equity", params = { ranges = ["AsKs", "QhQd", "7c6c"], board = "" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "qq-three-way"
type = "numeric"
prompt = "Preflop, all-in with no more betting, three players: A♠K♠, Q♥Q♦ and 7♣6♣. What is the exact equity (in %) of Q♥Q♦?"
answer = 39.57
tolerance = 0.01
unit = "%"
explanation = "About 39.6%, down from about 53.8% against A♠K♠ alone: Q♥Q♦ loses about 14.2 points while A♠K♠ loses about 8.4. Still the favourite, but now well under half the pot. With three players the even share is 33.3%."
concepts = ["equity", "multiway-game-theory"]
compute = { kind = "equity", params = { ranges = ["AsKs", "QhQd", "7c6c"], board = "" }, field = "equity.1", scale = 100.0 }

[[questions]]
id = "aces-two-random"
type = "numeric"
prompt = "The turn is K♣ 7♦ 2♠ 4♥ and you hold A♥A♦. Two opponents each hold a random hand (any two cards not on the board or in your hand, every combination equally likely), and the river is dealt with no more betting. What is your exact equity (in %)?"
answer = 79.03
tolerance = 0.01
unit = "%"
explanation = "About 79.0%. Against one random hand on this turn A♥A♦ has about 89.0%; the second opponent takes about 10 points more."
concepts = ["equity", "multiway-game-theory"]
compute = { kind = "equity", params = { ranges = ["AhAd", "22+,A2s+,K2s+,Q2s+,J2s+,T2s+,92s+,82s+,72s+,62s+,52s+,42s+,32s,A2o+,K2o+,Q2o+,J2o+,T2o+,92o+,82o+,72o+,62o+,52o+,42o+,32o", "22+,A2s+,K2s+,Q2s+,J2s+,T2s+,92s+,82s+,72s+,62s+,52s+,42s+,32s,A2o+,K2o+,Q2o+,J2o+,T2o+,92o+,82o+,72o+,62o+,52o+,42o+,32o"], board = "Kc7d2s4h" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "eights-two-random"
type = "numeric"
prompt = "The turn is K♣ 7♦ 2♠ 4♥ and you hold 8♥8♣. Two opponents each hold a random hand (any two cards not on the board or in your hand, every combination equally likely), and the river is dealt with no more betting. What is your exact equity (in %)?"
answer = 54.24
tolerance = 0.01
unit = "%"
explanation = "About 54.2%, down from about 73.7% against one random hand on this turn: a loss of about 19.5 points, against about 10 for A♥A♦. Each extra opponent is another chance that someone holds a king, a seven or better."
concepts = ["equity", "multiway-game-theory"]
compute = { kind = "equity", params = { ranges = ["8h8c", "22+,A2s+,K2s+,Q2s+,J2s+,T2s+,92s+,82s+,72s+,62s+,52s+,42s+,32s,A2o+,K2o+,Q2o+,J2o+,T2o+,92o+,82o+,72o+,62o+,52o+,42o+,32o", "22+,A2s+,K2s+,Q2s+,J2s+,T2s+,92s+,82s+,72s+,62s+,52s+,42s+,32s,A2o+,K2o+,Q2o+,J2o+,T2o+,92o+,82o+,72o+,62o+,52o+,42o+,32o"], board = "Kc7d2s4h" }, field = "equity.0", scale = 100.0 }

[[questions]]
id = "mdf-one-defender"
type = "numeric"
prompt = "On the river a player bets 10bb into a 10bb pot and one opponent is left to act. In the bluff-catching model (the bettor's bet is either the best hand or a bluff that always loses when called), what share of their range (in %) must the defender continue with so that a bluff cannot profit?"
answer = 50.0
tolerance = 0.01
unit = "%"
explanation = "50%: the minimum defence frequency P ÷ (P + B) = 10 ÷ 20. A bluff risks 10bb to win 10bb, so it breaks even when it gets a fold half the time."
concepts = ["minimum-defence-frequency"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 10000 }, field = "mdf", scale = 100.0 }

[[questions]]
id = "split-defence"
type = "numeric"
prompt = "On the river a player bets 10bb into a 10bb pot and two opponents are left to act. Simplify: a bluff wins the pot only if both opponents fold and always loses when anyone calls, and the bluff cannot profit as long as someone continues at least 50% of the time. If the two opponents share that job equally and never both continue with the same deal, what is the least share of their range (in %) each must continue with?"
answer = 25.0
tolerance = 0.01
unit = "%"
explanation = "25% each: the single-defender minimum, 50%, divided between two defenders. If their continuing hands never coincide, someone continues 25% + 25% = 50% of the time. Any overlap (both holding a hand they continue with) means each needs more."
concepts = ["minimum-defence-frequency", "multiway-game-theory"]
compute = { kind = "bet_math", params = { pot_mbb = 10000, bet_mbb = 10000 }, field = "mdf", scale = 50.0 }

[[questions]]
id = "no-guarantee"
type = "multiple_choice"
prompt = "In a poker game with three or more players, what does playing your part of a Nash equilibrium guarantee you?"
options = [
  "No minimum result: if the other players change their strategies, even without colluding, you can do worse than the equilibrium value",
  "At least the equilibrium value against any strategies the others play, as in a two-player zero-sum game",
  "The highest possible result against any opponents",
]
answer = 0
explanation = "Heads-up and zero-sum, an equilibrium strategy guarantees at least the game's value whatever the opponent does. With three or more players nothing like that holds: one opponent's change can move money from you to a third player, and different equilibria can give you different values."
concepts = ["multiway-game-theory", "nash-equilibrium"]

[[questions]]
id = "riverlab-multiway"
type = "multiple_choice"
prompt = "You are reviewing a hand from Play in which three players saw the river. What can RiverLab's solver do with that river decision?"
options = [
  "Nothing directly: it solves heads-up postflop spots only, so a multiway street is marked not solvable",
  "Solve it three-way and show an approximate equilibrium with its exploitability",
  "Solve it three-way, but without an exploitability figure",
]
answer = 0
explanation = "RiverLab's solver is heads-up only. In the hand review, streets that began with more than two players are marked \"not solvable\"; only heads-up streets get a solve."
concepts = ["multiway-game-theory"]

[[questions]]
id = "bot-multiway"
type = "multiple_choice"
prompt = "In RiverLab's Play mode, how do the bots change their postflop play in a pot with three or more players?"
options = [
  "By a fixed practice rule: they continue with 20% less of the share they would defend heads-up, and value-bet 40% less",
  "They switch to a three-way solver strategy",
  "They play exactly as they would heads-up",
]
answer = 0
explanation = "The bots are practice opponents, not solvers. Their multiway adjustment is a simple documented rule (docs/play.md): the defending share is multiplied by 0.8 and the value-betting share by 0.6. It is a design choice, not a claim about correct multiway play."
concepts = ["multiway-game-theory"]
+++

## What changes with a third player {#what}

::: fact
A pot is **multiway** when three or more players are still in it. Each player's **equity** is
still their share of the pot if the remaining cards are dealt with no more betting, and the
equities of all the players add up to 100%. With three players the even share is 33.3%, with
four 25%.
:::

::: simple
More players means more hands that can beat yours. A hand that is a big favourite against one
opponent can be only a small favourite against two, because each extra player is another chance
that someone has you beaten or catches up.
:::

::: intermediate
Three things change at once, and this lesson takes them in turn: every hand's equity shrinks;
defending against a bet becomes a job shared between several players; and the game-theory
guarantees of heads-up play no longer hold.
:::

## Exact equity three ways {#three-way}

::: fact
The equity engine enumerates every remaining board exactly, for two or more hands. Here are
A♠K♠ and Q♥Q♦ heads-up preflop, and then with 7♣6♣ added.
:::

```riverlab:equity
{"hands": ["AsKs", "QhQd"], "board": ""}
```

```riverlab:question
{"id": "aks-vs-qq"}
```

```riverlab:question
{"id": "aks-three-way"}
```

```riverlab:question
{"id": "qq-three-way"}
```

::: intermediate
The third hand takes equity from both, but not equally: here Q♥Q♦ loses about 14.2 points and
A♠K♠ about 8.4. The engine counts every board, so these numbers are exact for these three
hands; which hand loses more changes with the hands, so try others in the Equity calculator
(it takes up to six).
:::

## Equity falls as players are added {#falling}

::: fact
On the turn K♣ 7♦ 2♠ 4♥, against **random hands** (every two-card combination equally likely,
with card removal), exact enumeration gives A♥A♦ about 89.0% against one opponent and 8♥8♣
about 73.7%. With a second opponent:
:::

```riverlab:question
{"id": "aces-two-random"}
```

```riverlab:question
{"id": "eights-two-random"}
```

::: approximation
A quick estimate: if each opponent were an independent heads-up contest, your chance against
two would be your heads-up chance squared. For A♥A♦, 0.890 × 0.890 ≈ 0.792, close to the exact
79.0%; for 8♥8♣, 0.737 × 0.737 ≈ 0.543, close to 54.2%. It is only an estimate: the opponents'
cards come from the same deck, ties are split three ways, and it says nothing about real ranges.
:::

::: heuristic
Random hands are a model, not real opponents. In a real multiway pot the players still in have
chosen to continue, so their ranges are usually stronger than random and your equity lower
still. The pattern the numbers show is the usual rule of thumb: **strong hands keep more of
their equity multiway, and medium-strength made hands lose the most**, so medium hands are
worth less, relative to heads-up, the more players there are.
:::

::: math
For two players the equities are $e_1 + e_2 = 1$. For $n$ players, with ties split evenly
among the hands that tie,

$$ \sum_{i=1}^{n} e_i = 1, $$

so equity one player gains must come out of the others' shares.
:::

## Defending against a bet when others are left to act {#defence}

::: fact
Heads-up, in the bluff-catching model of the MDF lesson, a bet of $B$ into a pot of $P$ needs
the defender to continue with at least $\text{MDF} = P / (P + B)$ of their range, or a bluff
that always loses when called makes a profit.
:::

```riverlab:question
{"id": "mdf-one-defender"}
```

::: approximation
**The simplification.** With two players left to act, treat a bluff as winning the pot only
when **both** fold and losing the bet whenever **anyone** calls, and ignore raises and the
better price a second caller gets after a first call. Then the bluff cannot profit as long as
*someone* continues at least MDF of the time. That condition is now shared between two
players. It is a simplification of real multiway play, not an equilibrium of it.
:::

```riverlab:question
{"id": "split-defence"}
```

::: math
Let $c_1, c_2$ be the defenders' continuing shares. The chance that someone continues is at
most $c_1 + c_2$ (equality when they never both continue), so the defence needs

$$ c_1 + c_2 \ge \text{MDF}, \qquad \text{so with equal shares } c \ge \tfrac{\text{MDF}}{2}. $$

If instead each defender's hand is independent of the other's and each folds with probability
$f$, both fold with probability $f^2$, and the condition is $f^2 \le 1 - \text{MDF}$. For a pot-size
bet, $f \le \sqrt{0.5} \approx 0.707$, so each continues with at least about 29.3%. Either way,
each defender needs well under the heads-up 50%.
:::

::: heuristic
Two practical consequences, as rules of thumb, not solver results: a bluff into several players
needs all of them to fold, so it works less often; and a defender facing a bet with another
player still to act can fold more than the heads-up MDF, because they are not the only one
keeping the bettor honest. Who should do the defending (the player with the better hands, or
the last to act) is not settled by this arithmetic.
:::

## What game theory says multiway {#theory}

::: fact
- **Equilibria exist.** Nash (1951) proved that every finite game, with any number of
  players, has at least one equilibrium in mixed strategies: a strategy for each player such
  that no single player can gain by changing only their own.
- **Heads-up zero-sum is special.** With two players and zero sum (no rake), every equilibrium
  gives each player the same value, and an equilibrium strategy guarantees at least that value
  whatever the opponent does. That guarantee is what "exploitability" measures distance from.
- **Three or more players lose that guarantee.** Different equilibria can give a player
  different values, mixing your part of one equilibrium with others' parts of another need not
  be an equilibrium, and even when you play your part, another player's change can move money
  from you to a third player without anyone colluding.
- **Algorithms.** The convergence guarantees of CFR-style algorithms are proven for two-player
  zero-sum games; with three or more players there is no such guarantee in general.
:::

```riverlab:question
{"id": "no-guarantee"}
```

::: advanced
Because no player can be guaranteed a value, "unexploitable" has no clean meaning multiway, and
a three-way strategy cannot be scored by one exploitability number the way a heads-up solve is.
Brown and Sandholm (2019) built a six-player program that beat strong human players; they judged
it by results in play, not by distance from an equilibrium.
:::

## Multiway in RiverLab {#riverlab}

::: fact
- **The solver is heads-up only.** It solves postflop spots between two players. In Play, a
  street that began with three or more players is marked "not solvable" in the hand review.
- **The bots' multiway play is a simple practice rule.** In pots with three or more players a
  bot continues with 20% less of the share it would defend heads-up and value-bets 40% less
  (`docs/play.md`). These are design choices for practice opponents, not solver output and not
  a claim about correct multiway play.
- **The equity calculator handles up to six hands or ranges**, exactly or by Monte Carlo.
:::

```riverlab:question
{"id": "riverlab-multiway"}
```

```riverlab:question
{"id": "bot-multiway"}
```
