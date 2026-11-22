# No-limit hold'em rules implemented by the engine

Source: `engine/poker-core/src/game/`. Tests: `engine/poker-core/tests/game_rules.rs`
(scenario tests + a property test that plays 2,000 random legal hands per run and checks
chip conservation and termination). Units: integer milli-big-blinds (ADR-0002).

## Table and positions
- 3 to 6 players. Positions in preflop action order: UTG, HJ, CO, BTN, SB, BB. Short-handed
  tables drop positions from the front (5-handed starts at HJ; 3-handed is BTN/SB/BB).
- **Heads-up is not supported yet** (its blind/action order differs).
- Preflop action starts with the first seat after the BB; postflop action starts with the SB
  (first live seat left of the button) and ends with the button.

## Forced bets
- Every player posts the ante (default 0) as dead money: it counts toward the pot and side
  pots but is not part of the current bet.
- SB posts the configured small blind (default 0.5bb), BB posts 1bb. The current bet
  preflop is 1bb and the minimum raise increment is 1bb.
- Every stack must exceed ante + blind owed; posting all-in is rejected, not modelled.

## Actions
Bet and raise amounts are *raise-to* amounts (total committed on the street).
- **Check**: only when nothing is owed.
- **Fold**: only when facing a bet. Folding when checking is free is rejected (it is never
  correct and would pollute training data).
- **Call**: pays the amount owed, or the whole stack (all-in for less).
- **Bet** (no bet on the street yet): at least 1bb, or all-in for less.
- **Raise**: to at least `current bet + last full raise increment`, or all-in for less.
- No limit on the number of raises. Bets/raises are unavailable when no opponent could
  respond (everyone else folded or all-in); the player may only call or fold.

## Re-opening the action (short all-ins)
A raise of less than the minimum increment (only possible all-in) does not change the
minimum increment and does not re-open the betting for players who already acted.
Implementation: when a player acts, the engine records the minimum raise-to at that moment;
the player may raise again only once the current bet reaches that level. This also captures
the cumulative rule: several short all-ins that together amount to a full raise re-open the
action (tested in `two_short_all_ins_totalling_a_full_raise_reopen_action`). An opening bet
re-opens the action for everyone.

Opening all-in bets smaller than 1bb set the current bet without changing the 1bb minimum
increment, so the next minimum raise is to `bet + 1bb`. Card rooms differ on this edge case.

## End of a betting round
A round ends when every live player who still has chips has acted and matched the current
bet. The BB therefore gets its option after limps. Then:
1. The **uncalled part** of the largest bet (excess over the second-largest street
   commitment, folded players included) returns to its owner.
2. One live player left → the hand is won uncontested.
3. After the river → showdown. Otherwise the next street's cards must be dealt.
4. If fewer than two live players can still act, later streets are dealt without betting.

## Cards
`deal_board` requires exactly 3 cards for the flop and 1 each for turn and river. Board and
known hole cards must all be distinct. Folded players' known cards stay dead.

## Pots and settlement
- Side pots are cut at each distinct total contribution of a live player. Folded players'
  chips go into the pots they reached but they are not eligible.
- Rake (see `rake.md`) is computed once on the final pot and split across pots.
- Each pot goes to the best eligible hand; ties split evenly. **Odd chips** (1 mbb units) go
  one at a time to the tied winners in postflop order, starting left of the button.
- Invariant (property-tested): `sum(final stacks) + rake = sum(starting stacks)`.
