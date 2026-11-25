# Academy: AI review pass, November 2026

Every Academy lesson was read in full by independent AI reviewers, and the findings were checked
against the engine before any fix was applied. Every lesson that changed went up one `version`.
**No lesson is marked reviewed**: under the project rules only a person can do that, on the
Lesson review page (`/academy/review`).

This file lists what a person should still check. Items are grouped by lesson. Most of them need
a poker-literate reader, not a programmer. Where a claim is a number, the number comes from the
engine: check that the *wording* around it is fair.

## Every lesson
- Read each lesson at least once at each explanation level (Simple, Intermediate, Advanced,
  Mathematical). The app shows only the blocks for your level in each section, plus untagged
  text and callouts. The review moved definitions everyone needs into untagged text, but some
  explanations still exist at only one level.
- Check that each callout's label is right: fact, heuristic, approximation or solver.
- Check that every book in "Further reading" is cited for something it really covers.

## Citations to confirm
- *The Mathematics of Poker* (Chen and Ankenman, 2006) is cited by many lessons. Confirm the part
  and chapter notes, and that it really covers pot odds, expectation, the clairvoyance game and
  card removal.
- *Modern Poker Theory* (Acevedo, 2019): confirm it covers equity realisation, range and nut
  advantage, and card removal.
- *Applications of No-Limit Hold'em* (Janda, 2013): confirm it covers range advantage and c-bet
  sizing.
- Hart and Mas-Colell (2000): the lesson says it "introduces regret matching". Its main
  procedure is the conditional (correlated-equilibrium) version; the unconditional version used
  here is a variant.
- "DCFR generalises CFR and CFR+" (Brown and Sandholm 2019): is that a fair summary?
- *The Theory of Poker*: the 1999 Two Plus Two edition is cited; the first edition was 1987.
- Harville (1973, *JASA*) for ICM. *Kill Everyone* was dropped because its authors and edition
  could not be confirmed; re-add it only with a confirmed edition.

## Lesson by lesson
- **hand-rankings**: the five-card category counts (derived in the text) and the new
  counterfeiting example (3♣3♦ on K K 7 7).
- **positions**: the late-position heuristic; the "equity realisation" note now matches the new
  lesson.
- **equity**: whether "about 4 to 1" is acceptable for pair-over-pair (the engine gives 80–83%);
  the hedged wording on two overcards against a small pair.
- **pot-odds, preflop-fundamentals**: the new level-1 order. It is terminology, hand rankings,
  positions, preflop fundamentals, equity, pot odds, outs and draws, starting hands.
- **starting-hands**: the new suited-against-offsuit question (7♥6♣ against A♠K♦).
- **fold-equity**: the "free river card" comparison assumes a check really does get a free card.
- **mdf-and-bluff-ratios / polarised-ranges**: "merged" and "linear" are used as synonyms; some
  authors distinguish them.
- **range-advantage**: whether AA–TT, AK, AQ and KQ suited reads as a plausible under-the-gun
  illustration. The general c-bet "tendencies" are labelled heuristic, with no source.
- **range-checking**: the explanation of why top pair checks when raises are allowed. In the
  solve, KQ never bets, so the opponent's response to a bet is off the equilibrium path. The
  explanation now reasons only from what a bet can win.
- **check-raises**: the new questions about which hands bet.
- **node-locking**: the new `under-bluffs` question (answer 0%; the solve gives about 0.1%), and
  the reading of `values_bb` as each player's value net of their half of the starting pot.
- **exploitability**: RiverLab's figure is the average over the two players (NashConv ÷ 2); the
  lesson now says one player may be exploitable by more.
- **icm** (new): the bubble example and its wording.
- **equity-realisation** (new): the definition (EV ÷ (equity × pot), counting later bets), and
  that the causal explanations are labelled as interpretations.

## Lessons added after the review pass
Each was drafted with AI assistance and then reviewed by a second, independent AI pass. They
need a person's review like the others.
- **range-construction:** that the example range is clearly an illustration, and the common
  advice "bluff with hands that block calls" (labelled heuristic).
- **turn-and-river-strategy:** the explanation of why the solve's river bluff share (about 24%)
  is below the single-bet model's 30%. The explanation is card removal: the calling king blocks
  value hands, not bluffs. The reviewer checked it with an equity calculation outside the
  lesson. Also the reasoning for why middle pairs check the river, and that the solve does
  not explain every hand's mix (QQ never bets the blank turn while TT bets about 62%).
- **multiway:** the multiplayer game-theory statements. These cover:
  - non-unique values and non-interchangeable equilibria;
  - regret minimisation approaching only a coarse correlated equilibrium;
  - the defence arithmetic "under a stated simplification".
- **population-tendencies:** the explanation of what a 95% interval means, and the claim that a
  direct test separates 25% from 30% after 1,000 hands sooner than non-overlap suggests. Also
  confirm the Wilson (1927) and Brown, Cai and DasGupta (2001) citations.
- Some figures in these lessons appear only in explanations, as arithmetic from computed
  answers, and are not re-checked at sync. A person can decide whether to back them with
  questions.

## What the content check now enforces
- Prerequisites must be taught earlier in the curriculum. This catches loops like the one found
  between the MDF and polarised-ranges lessons.
- Question prompts must not refer to other questions ("same spot", "previous question"),
  because revision asks them out of order.
