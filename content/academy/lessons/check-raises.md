+++
slug = "check-raises"
title = "Check-raises: value, bluffs and blockers"
level = 4
order = 8
version = 2
status = "published"
summary = "A river solve where the first player checks, the second bets small, and the first check-raises with its strongest hands and some bluffs, with blockers deciding which bluff-catchers call."
estimated_minutes = 20
objectives = [
  "Explain what a check-raise is for, for value and as a bluff.",
  "Check a solver's check-raise bluff share against the indifference formula.",
  "See how blockers make one bluff-catcher better than another.",
]
prerequisites = ["polarised-ranges", "blockers", "bluff-to-value-ratios"]
tags = ["strategy", "solver", "blockers"]

[review]
status = "pending"
reviewer = ""
reviewed_on = ""
notes = "Drafted with AI assistance. Every figure comes from a river solve run by RiverLab's solver each time the content is synced (within 0.05% of the pot of an equilibrium of its tree), or from poker-theory; awaiting review by a person. AI review pass (2026-11): checked prose, labels, numbers and questions; fixes applied. Still needs review by a person."

[[tools]]
href = "/solver"
label = "Solver"
why = "Add raise sizes to your own spots and see who check-raises."

[[reading]]
title = "The Mathematics of Poker"
authors = "Bill Chen and Jerrod Ankenman"
year = 2006

[[reading]]
title = "Modern Poker Theory"
authors = "Michael Acevedo"
year = 2019

[solves.river]
board = "Ks9d5c3h2s"
ranges = ["55,KTs,KTo,K8s,K7s,76s", "AA,KQs,KQo,KJs,KJo,QJs,JTs"]
pot_bb = "10"
stack_bb = "100"
oop_bets = []
ip_bets = [33]
oop_raises = [50]
iterations = 2000

[[questions]]
id = "ip-bets"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. Your opponent checked to you (in this tree they cannot bet). You are in position with AA, KQ, KJ, or the busted draws QJ suited and JT suited; they hold 55 (a set), KT, K8 suited, K7 suited, or 76 suited (nothing). You can check or bet 33% of the pot (3.3bb); after a bet they can fold, call or check-raise to 11.6bb, and you cannot re-raise. In the solved strategy, what share (in %) of your range bets?"
answer = 94.59
tolerance = 0.1
unit = "%"
explanation = "Almost all of it. Your aces and kings beat every king your opponent holds (only 55 beats you), so they all bet for value, together with most of your QJ and JT as bluffs. 33% is the only size this tree allows."
concepts = ["bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }], max_exploitability_pct = 0.05 }, field = "share.a1", scale = 100.0 }

[questions.scene]
players = 2
hero = "SB"
board = ["Ks", "9d", "5c", "3h", "2s"]
pot_bb = "10"
stacks_bb = { SB = "100", BB = "100" }
actions = { BB = "checks" }

[[questions]]
id = "ip-value-bets"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. Your opponent checked to you (in this tree they cannot bet). You are in position with AA, KQ, KJ, or the busted draws QJ suited and JT suited; they hold 55 (a set), KT, K8 suited, K7 suited, or 76 suited (nothing). You can check or bet 33% of the pot (3.3bb); after a bet they can fold, call or check-raise to 11.6bb, and you cannot re-raise. In the solved strategy, what share (in %) of your aces and kings (AA, KQ, KJ) bet?"
answer = 100.0
tolerance = 0.1
unit = "%"
explanation = "All of them. AA, KQ and KJ beat every king your opponent holds and lose only to 55, so they bet for value every time."
concepts = ["bet-sizing-theory"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }], subset = ["AA", "KQs", "KQo", "KJs", "KJo"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[[questions]]
id = "ip-bluffs-bet"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. Your opponent checked to you (in this tree they cannot bet). You are in position with AA, KQ, KJ, or the busted draws QJ suited and JT suited; they hold 55 (a set), KT, K8 suited, K7 suited, or 76 suited (nothing). You can check or bet 33% of the pot (3.3bb); after a bet they can fold, call or check-raise to 11.6bb, and you cannot re-raise. In the solved strategy, what share (in %) of your busted draws (QJ suited and JT suited) bet as bluffs?"
answer = 74.31
tolerance = 0.1
unit = "%"
explanation = "About three quarters. The rest check and give up. The checks in your range all come from these bluffs."
concepts = ["bet-sizing-theory", "bluff-to-value-ratios"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }], subset = ["QJs", "JTs"], max_exploitability_pct = 0.05 }, field = "subset_share.bet", scale = 100.0 }

[[questions]]
id = "oop-raises"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. You checked (in this tree you cannot bet) with 55 (a set), KT, K8 suited, K7 suited or 76 suited (nothing). Your opponent, who holds AA, KQ, KJ, QJ suited or JT suited, bets 3.3bb (33% of the pot). You may fold, call or raise to 11.6bb, and they cannot re-raise. In the solved strategy, what share (in %) of your range check-raises?"
answer = 16.0
tolerance = 0.1
unit = "%"
explanation = "Your set raises every time, and some of your 76s raises as a bluff. Your kings never raise: they lose to every value hand that would call a raise."
concepts = ["check-raises"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], max_exploitability_pct = 0.05 }, field = "share.a2", scale = 100.0 }

[questions.scene]
players = 2
hero = "BB"
board = ["Ks", "9d", "5c", "3h", "2s"]
pot_bb = "10"
bets_bb = { SB = "3.3" }
stacks_bb = { SB = "96.7", BB = "100" }
actions = { BB = "checks", SB = "bets 3.3bb" }

[[questions]]
id = "raise-bluff-share"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. You checked (in this tree you cannot bet) with 55 (a set), KT, K8 suited, K7 suited or 76 suited (nothing). Your opponent, who holds AA, KQ, KJ, QJ suited or JT suited, bets 3.3bb (33% of the pot). You may fold, call or raise to 11.6bb, and they cannot re-raise. The solved strategy check-raises with 55 and some 76s. What share (in %) of those check-raises are bluffs (76s)?"
answer = 24.99
tolerance = 0.1
unit = "%"
explanation = "About a quarter: enough bluffs that your opponent's aces and kings are indifferent between calling the raise and folding. The raise adds 8.3bb to a 16.6bb pot, and the indifference formula gives 8.3 / (16.6 + 16.6) = 25%."
concepts = ["check-raises", "bluff-to-value-ratios"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], subset = ["76s"], max_exploitability_pct = 0.05 }, field = "subset_of_action.a2", scale = 100.0 }

[[questions]]
id = "raise-formula"
type = "numeric"
prompt = "A check-raise to 11.6bb over a 3.3bb bet into 10bb adds 8.3bb to a pot of 16.6bb (the 10bb pot, the 3.3bb bet and your matching 3.3bb). By the indifference formula B / (P + 2B), with B = 8.3bb and P = 16.6bb, what share (in %) of the raises should be bluffs?"
answer = 25.0
tolerance = 0.05
unit = "%"
explanation = "8.3 / (16.6 + 16.6) = 25%. In this lesson's solved river spot (K♠ 9♦ 5♣ 3♥ 2♠, 55, kings and 76s check-raising against AA, KQ, KJ, QJs and JTs), 24.99% of the check-raises are bluffs: the raise works like a new polarised bet of 8.3bb into 16.6bb."
concepts = ["bluff-to-value-ratios"]
compute = { kind = "bet_math", params = { pot_mbb = 16600, bet_mbb = 8300 }, field = "bluff_share", scale = 100.0 }

[[questions]]
id = "kt-folds"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. You checked (in this tree you cannot bet) with 55 (a set), KT, K8 suited, K7 suited or 76 suited (nothing). Your opponent, who holds AA, KQ, KJ, QJ suited or JT suited, bets 3.3bb (33% of the pot). You may fold, call or raise to 11.6bb, and they cannot re-raise. How often (in %) does KT offsuit fold in the solved strategy?"
answer = 60.74
tolerance = 0.1
unit = "%"
explanation = "KT folds much more often than K8 suited and K7 suited, which in the same solve never fold. On this board all three kings beat and lose to the same hands. The difference is the ten: holding it removes some of your opponent's JT bluffs, so KT catches fewer bluffs."
concepts = ["blockers", "check-raises"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], subset = ["KTo"], max_exploitability_pct = 0.05 }, field = "subset_share.fold", scale = 100.0 }

[[questions]]
id = "k7-folds"
type = "numeric"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind. You checked (in this tree you cannot bet) with 55 (a set), KT, K8 suited, K7 suited or 76 suited (nothing). Your opponent, who holds AA, KQ, KJ, QJ suited or JT suited, bets 3.3bb (33% of the pot). You may fold, call or raise to 11.6bb, and they cannot re-raise. How often (in %) do K8 suited and K7 suited fold in the solved strategy?"
answer = 0.0
tolerance = 0.1
unit = "%"
explanation = "Never. K8 and K7 block none of your opponent's bluffs (QJ, JT), so they are the best bluff-catchers among your kings even though KT has the better kicker."
concepts = ["blockers"]
compute = { kind = "solve", params = { solve = "river", path = [{ type = "action", value = 0 }, { type = "action", value = 1 }], subset = ["K8s", "K7s"], max_exploitability_pct = 0.05 }, field = "subset_share.fold", scale = 100.0 }

[[questions]]
id = "why-raise-76"
type = "multiple_choice"
prompt = "River K♠ 9♦ 5♣ 3♥ 2♠. You checked with 55, KT, K8 suited, K7 suited or 76 suited, and your opponent, holding AA, KQ, KJ, QJ suited or JT suited, bets a third of the pot. Why does the solved strategy check-raise 76 suited as a bluff, rather than KT?"
options = [
  "76s can never win at showdown, so it gains most from making better hands fold; KT can win by calling",
  "76s is the strongest hand in the range",
  "Bluffs should always be the lowest cards",
]
answer = 0
explanation = "A bluff gains only when the opponent folds. Hands with some showdown value lose that value by raising, so the hands with none make the best bluffs."
concepts = ["check-raises"]
+++

## What a check-raise does {#what}

::: fact
A **check-raise** is checking, then raising after your opponent bets.
:::

::: heuristic
On the river a check-raise is usually polarised, like most river raises: very strong hands
raising for value, and hands that cannot win at showdown raising as bluffs.
:::

## The spot {#spot}

::: solver
River K♠ 9♦ 5♣ 3♥ 2♠, pot 10bb, 100bb behind, no rake. The first player holds 55, KT, K8
suited, K7 suited or 76 suited, and must check. The second holds AA, KQ, KJ, QJ suited or JT
suited, and may check or bet 33% of the pot (3.3bb). After a bet, the first player may fold,
call or raise by 50% of the pot after calling: to 3.3 + 0.5 × (10 + 3.3 + 3.3) = 11.6bb. The
second player cannot re-raise. Solved by RiverLab each time the content is synced, within 0.05%
of the pot of an equilibrium of this tree: an approximate equilibrium for these sizes only.
:::

```riverlab:question
{"id": "ip-bets"}
```

```riverlab:question
{"id": "ip-value-bets"}
```

```riverlab:question
{"id": "ip-bluffs-bet"}
```

## Who check-raises {#who}

```riverlab:question
{"id": "oop-raises"}
```

```riverlab:question
{"id": "raise-bluff-share"}
```

```riverlab:question
{"id": "raise-formula"}
```

```riverlab:question
{"id": "why-raise-76"}
```

## Blockers decide which kings call {#blockers}

```riverlab:question
{"id": "kt-folds"}
```

```riverlab:question
{"id": "k7-folds"}
```

::: intermediate
Facing a bet, the kings are pure bluff-catchers: they beat QJ and JT and lose to AA, KQ and KJ.
The solution calls with just enough of them to make the bluffs indifferent between betting and
checking (QJ and JT beat 76s at showdown, so they are not pure bluffs, and the single-bet MDF
does not apply exactly), and it picks the kings that block the fewest bluffs. Holding a ten removes JT combinations from your opponent's
range, so KT is called with less often.
:::

::: approximation
Real spots are messier: more hand types, more sizes, earlier streets. The patterns shown here
(strong hands and blocker-free bluffs raise, bluff-catchers that block bluffs fold) are what to
look for, but the exact frequencies come from the tree and ranges given.
:::
