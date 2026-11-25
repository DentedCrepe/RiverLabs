+++
slug = "population-tendencies"
title = "Tendencies and samples: how much a stat really tells you"
level = 6
order = 5
version = 1
status = "published"
summary = "What a tendency statistic such as VPIP or fold to c-bet counts, how wide the uncertainty around it is for a given sample, how many hands it takes before two players can be told apart, and how to turn a reliable read into an exploit without over-adjusting to noise."
estimated_minutes = 20
objectives = [
  "State exactly what RiverLab's tendency statistics count, including what each one's opportunities are.",
  "Compute the 95% interval around a measured frequency and see how it narrows with the sample.",
  "Judge whether two measured frequencies can be told apart from their samples.",
  "Turn a reliable read into an exploit with node locking, and explain the risk of adjusting on small samples.",
]
prerequisites = ["exploitability", "node-locking", "exploitative-adjustments"]
tags = ["exploitation", "statistics", "strategy"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every interval and standard error is computed by RiverLab's engine (poker_theory::sample_frequency, Wilson score interval) each time the content is synced. All frequencies in the lesson are hypothetical inputs to that calculation; the lesson makes no claim about how real players or player pools play. Statistic definitions follow engine/play-engine/src/report.rs and the report's glossary. Awaiting review by a person."

[[tools]]
href = "/analytics"
label = "Analytics"
why = "Your own statistics over every stored hand against the bots, shown as counts (made / opportunities) you can put an interval around."

[[tools]]
href = "/play"
label = "Play: practice leaks and spot the leak"
why = "Bots with one documented river leak, named or hidden: practise finding a tendency and exploiting it, with the report checking your river decisions against the leaky bot's actual range."

[[tools]]
href = "/solver"
label = "Solver: node locking"
why = "Lock an opponent's decision to the tendency you believe they have and see the best response to it, and what it is worth."

[[reading]]
title = "Interval Estimation for a Binomial Proportion"
authors = "Lawrence D. Brown, T. Tony Cai and Anirban DasGupta"
year = 2001
note = "Statistical Science. Compares intervals for a measured proportion, shows how badly the simple normal (Wald) interval behaves in small samples, and recommends the Wilson interval, which RiverLab uses, among its alternatives."

[[reading]]
title = "Probable Inference, the Law of Succession, and Statistical Inference"
authors = "Edwin B. Wilson"
year = 1927
note = "Journal of the American Statistical Association. The original Wilson score interval."

[[questions]]
id = "vpip-definition"
type = "multiple_choice"
prompt = "In RiverLab's Play report, which hands count towards your VPIP (voluntarily put money in pot)?"
options = [
  "Hands where you called or raised before the flop; posting a blind, or checking in the big blind, does not count",
  "Hands where you put any chips in the pot, blinds included",
  "Hands where you saw the flop",
]
answer = 0
explanation = "VPIP counts a hand when you called or raised preflop at least once. Its opportunities are all your hands, so its sample size is your hand count."
concepts = ["population-tendencies"]

[[questions]]
id = "cbet-opportunities"
type = "multiple_choice"
prompt = "In RiverLab's Play report, \"fold to c-bet\" is shown as folds out of opportunities. What is one opportunity?"
options = [
  "A flop where the last preflop raiser (a bot) made the first bet and you then acted",
  "Any hand you played",
  "Any flop you saw",
]
answer = 0
explanation = "Only flops where the preflop raiser bet first, and you then had to respond, count. That is a small part of all your hands, so this statistic's sample grows much more slowly than your hand count."
concepts = ["population-tendencies"]

[[questions]]
id = "interval-100"
type = "numeric"
prompt = "A hypothetical player has put money in voluntarily on 25 of 100 hands. Treating each hand as an independent trial with one fixed underlying frequency, what is the upper end (in %) of the 95% Wilson interval for that frequency?"
answer = 34.30
tolerance = 0.01
unit = "%"
explanation = "About 34.3%; the lower end is about 17.5%. So 25 of 100 is consistent with an underlying frequency anywhere from roughly 18% to 34%."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 25, trials = 100 }, field = "high_95", scale = 100.0 }

[[questions]]
id = "standard-error-100"
type = "numeric"
prompt = "A hypothetical player has put money in voluntarily on 25 of 100 hands. Treating each hand as an independent trial with one fixed underlying frequency, what is the standard error (in percentage points) of the measured 25%?"
answer = 4.33
tolerance = 0.01
unit = "points"
explanation = "√(0.25 × 0.75 ÷ 100) ≈ 0.0433, about 4.3 points: the typical size of the gap between a measured frequency and the true one at this sample size."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 25, trials = 100 }, field = "standard_error", scale = 100.0 }

[[questions]]
id = "interval-1000"
type = "numeric"
prompt = "A hypothetical player has put money in voluntarily on 250 of 1,000 hands. Treating each hand as an independent trial with one fixed underlying frequency, what is the upper end (in %) of the 95% Wilson interval for that frequency?"
answer = 27.78
tolerance = 0.01
unit = "%"
explanation = "About 27.8%; the lower end is about 22.4%. The same 25% measured over ten times the hands gives an interval about a third as wide (5.4 points instead of 16.8): the width shrinks with the square root of the sample."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 250, trials = 1000 }, field = "high_95", scale = 100.0 }

[[questions]]
id = "compare-1000"
type = "numeric"
prompt = "Two hypothetical players: one has put money in voluntarily on 250 of 1,000 hands, the other on 300 of 1,000. Treating each hand as an independent trial with one fixed underlying frequency, what is the lower end (in %) of the 95% Wilson interval for the second player (300 of 1,000)?"
answer = 27.24
tolerance = 0.01
unit = "%"
explanation = "About 27.2%. The first player's interval (250 of 1,000) reaches up to about 27.8%, so the two intervals still overlap slightly: after 1,000 hands each, measured frequencies of 25% and 30% are not separated by this conservative check (a direct test of the difference is more sensitive)."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 300, trials = 1000 }, field = "low_95", scale = 100.0 }

[[questions]]
id = "compare-2000"
type = "numeric"
prompt = "Two hypothetical players: one has put money in voluntarily on 500 of 2,000 hands, the other on 600 of 2,000. Treating each hand as an independent trial with one fixed underlying frequency, what is the lower end (in %) of the 95% Wilson interval for the second player (600 of 2,000)?"
answer = 28.03
tolerance = 0.01
unit = "%"
explanation = "About 28.0%. The first player's interval (500 of 2,000) reaches only about 26.9%, so with 2,000 hands each the intervals no longer overlap: if the two players really had the same underlying frequency, a gap this large would very rarely come from sampling noise alone."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 600, trials = 2000 }, field = "low_95", scale = 100.0 }

[[questions]]
id = "small-cbet-sample"
type = "numeric"
prompt = "A hypothetical opponent has folded to 6 of the 10 flop continuation bets they faced. Treating each c-bet as an independent trial with one fixed underlying frequency, what is the lower end (in %) of the 95% Wilson interval for how often they fold to a c-bet?"
answer = 31.27
tolerance = 0.01
unit = "%"
explanation = "About 31.3%, and the upper end is about 83.2%. Ten opportunities say almost nothing: the opponent could fold far less than half the time or far more."
concepts = ["population-tendencies"]
compute = { kind = "sample_frequency", params = { successes = 6, trials = 10 }, field = "low_95", scale = 100.0 }

[[questions]]
id = "over-adjust"
type = "multiple_choice"
prompt = "An opponent has folded to 6 of the 10 pot-size river bets they faced. A pure pot-size bluff profits only if they fold more than half the time. Treating each bet as an independent trial with one fixed underlying fold frequency, what is the sound conclusion?"
options = [
  "The sample cannot tell whether they fold more or less than half the time; a large adjustment on it risks exploiting noise",
  "They fold 60% of the time, so bluff every hand on the river against them",
  "They fold too little, so never bluff them",
]
answer = 0
explanation = "Six of ten is consistent with an underlying fold frequency from about 31% to 83% (95% Wilson interval), which includes values well below the 50% a pure bluff needs. As a heuristic, adjust a little, or not at all, until the sample can separate the read from the break-even point."
concepts = ["population-tendencies", "exploitative-adjustments"]
+++

## What a tendency statistic counts {#definitions}

::: fact
A **tendency statistic** is a count: how many times a player did something, out of the times
they had the chance (its **opportunities**). RiverLab's Play report and Analytics show your own
statistics as a percentage with the count beside it, for example 10/40. They include:
- **VPIP** (voluntarily put money in pot): hands where you called or raised preflop. Posting a
  blind, or checking in the big blind, does not count. Opportunities: every hand.
- **PFR** (preflop raise): hands where you raised preflop, all-in raises included (calling
  all-in does not count). Opportunities: every hand.
- **3-bet:** how often you re-raised when facing exactly one raise preflop, out of the times
  you had that chance.
- **Fold to 3-bet:** after you opened with the first raise, how often you folded when a bot
  re-raised.
- **C-bet:** as the last preflop raiser, how often you bet the flop when nobody had bet before
  you.
- **Fold to c-bet:** when the preflop raiser (a bot) bet the flop first, how often you folded.

A statistic whose opportunities are rare (fold to c-bet, fold to 3-bet) builds up a sample much
more slowly than VPIP or PFR.
:::

::: simple
A stat like "VPIP 25%" is a summary of what someone did in the hands you saw. It is only as
good as the number of hands behind it: 1 hand of 4 and 250 hands of 1,000 are both 25%, but
the second tells you far more.
:::

```riverlab:question
{"id": "vpip-definition"}
```

```riverlab:question
{"id": "cbet-opportunities"}
```

::: fact
RiverLab measures **your** tendencies, from your own stored hands. It does not compute tendency
statistics such as VPIP on the bots: their play is a set of documented rules (`docs/play.md`), and it makes
no claims about how real players or player pools play. Every frequency in this lesson is a
hypothetical input to the sample calculation, not a statement about any real population.
:::

## How much a sample can tell you {#samples}

::: approximation
**The model.** Treat each opportunity as an independent trial, and the player as having one
fixed underlying frequency. The measured share then estimates that frequency, with a
**standard error** of $\sqrt{p(1-p)/n}$ and a **95% interval**: the underlying frequencies the
sample does not rule out at the 5% level. Built this way from sample after sample, the interval
contains the true frequency about 95% of the time; it is not a 95% chance for any one interval.
RiverLab uses the Wilson score interval, which behaves well for small samples and stays between
0% and 100%; its actual coverage is usually close to 95%, not exactly 95% (Brown, Cai and DasGupta 2001). Real players are not this
simple: they play differently by position, stack depth and opponent, and change over time.
:::

```riverlab:question
{"id": "interval-100"}
```

```riverlab:question
{"id": "standard-error-100"}
```

```riverlab:question
{"id": "interval-1000"}
```

::: intermediate
The standard error falls with the **square root** of the sample: four times as many hands
halve it, and ten times as many cut it to about a third. So the first hundred hands narrow a
read a lot, and each later hundred narrows it less.
:::

::: math
With $\hat{p}$ the share measured over $n$ trials and $z = 1.96$, the Wilson interval is

$$ \frac{\hat{p} + \frac{z^2}{2n} \pm z\sqrt{\frac{\hat{p}(1-\hat{p})}{n} + \frac{z^2}{4n^2}}}{1 + \frac{z^2}{n}}. $$

For 25 of 100: $z^2 \approx 3.84$, so the centre is $(0.25 + 0.0192) / 1.0384 \approx 0.259$
and the half-width $1.96\sqrt{0.001875 + 0.000096} / 1.0384 \approx 0.084$, giving about
17.5% to 34.3%.
:::

## Telling two players apart {#comparing}

::: heuristic
A quick, conservative check: if the two players' 95% intervals **do not overlap**, the
difference is very unlikely to be sampling noise alone. Overlap does not prove they play the
same; a direct test of the difference is more sensitive. Either way, the check below needs
both samples, not just the two percentages.
:::

```riverlab:question
{"id": "compare-1000"}
```

```riverlab:question
{"id": "compare-2000"}
```

::: simple
Measured frequencies of 25% and 30% look different on paper. With 100 hands each, their
intervals (about 17.5–34.3% and 21.9–39.6%) overlap heavily. At 1,000 hands each they still
just touch; at 2,000 they separate. Overlap is a cautious check, so a difference can be real
before the intervals separate.
:::

::: math
A direct test compares the gap with its own standard error. For 250 of 1,000 against 300 of
1,000, that standard error is $\sqrt{0.25 \times 0.75 / 1000 + 0.30 \times 0.70 / 1000} \approx 0.020$,
so the 5-point gap is about 2.5 standard errors, beyond the 1.96 of a 5% test, although the
two 95% intervals still overlap.
:::

```riverlab:question
{"id": "small-cbet-sample"}
```

## From a read to an exploit {#exploit}

::: fact
Once you have a read you trust, **node locking** turns it into a strategy: in the Solver, fix
the opponent's decision at the frequency you believe and solve again. Your strategy in the result
answers that lock, with the locked player's other decisions re-solved around it (an exact best
response when the locked decision is their only one), and its values can be compared with the
unlocked solve's (see the node-locking lesson). The
answer is only as good as the read you locked.
:::

::: fact
**Practice in Play.** Each bot can be given one documented river leak (over-bluffs, never
bluffs, folds too much or calls too much on the river). With **Spot the leak** each bot draws
one of the leaks, or none, at random, and its name does not say which. At the end the report
asks you to guess each bot's leak, then, for each leak, shows how often your river calls and
folds facing those bots' bets heads-up matched the best response against their actual range,
and whether you matched it more often in the second half of those decisions than in the first.
:::

::: heuristic
A practical order: first see how much the tendency would be worth (lock it in the Solver),
then check whether your sample can actually separate it from the break-even point, and only
then adjust, by more as the evidence grows.
:::

## The risk of over-adjusting {#over-adjusting}

::: heuristic
Adjusting hard to a small sample means exploiting noise. A full exploit is usually a pure
strategy (always bluff, never bluff, always call), so if the read is wrong it can cost more than
the read would have gained if right, and an observant opponent can switch against it. Rules of
thumb: move part of the way from a balanced strategy towards the exploit; move further as the
interval narrows; and prefer reads with many opportunities over rare ones.
:::

::: advanced
The same caution applies to your own statistics. When Analytics shows, say, a fold-to-c-bet
count, put an interval around it before deciding you have a leak: the report's preflop and
pot-odds findings compare each decision with a stated reference, but a frequency on its own,
over a small number of opportunities, says little.
:::

```riverlab:question
{"id": "over-adjust"}
```
