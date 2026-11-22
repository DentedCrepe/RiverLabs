# Equity

Source: `engine/equity-engine/src/lib.rs`. **Equity is not strategy**: it is the share of
the pot each player wins if all remaining cards are dealt and nobody folds.

## Exact enumeration
Each player has a weighted range (a hand is a one-combo range). Combos that use a board or
dead card are removed. The engine enumerates every runout and every tuple of mutually
disjoint combos. A tuple's weight is the product of its combo weights. Every disjoint tuple
removes the same number of cards from the deck, so every (tuple, runout) pair is equally likely
given the weights. Ties split the pot equally among the tied players.

The work estimate is `product(range sizes) × runouts`. Above the budget the engine refuses and
suggests Monte Carlo (API default budget: 5×10⁷ evaluations).

## Monte Carlo
Each iteration draws one combo per player by weight. If the combos conflict, the whole draw
is rejected and redrawn, which samples exactly the exact method's distribution. Then the
missing board cards are drawn uniformly from the remaining deck. The seed is supplied by the
caller, so results are reproducible. The reported standard error is `sqrt(var(share) / n)` per
player.

## Validation
- Exact results equal hand-counted outs (turn spots), symmetric spots give exactly 50%, and
  weighted ranges mix linearly (unit tests).
- AA vs KK preflop over all 1,712,304 boards matches an independent `phevaluator`
  enumeration to 1e-12 (`tests/validation/`).
- Monte Carlo lands within 5 standard errors of the exact result and is reproducible per seed.
