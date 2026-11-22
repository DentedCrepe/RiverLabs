# Hand evaluator

Source: `engine/hand-evaluator/src/lib.rs`. Decision: ADR-0003.

## Encoding
`HandStrength(u32)`: bits 20–23 hold the category (0 high card … 8 straight flush) and bits
0–19 hold up to five 4-bit rank indices, most significant first. Unused nibbles are zero, so
integer comparison is exactly poker order and equality is an exact tie. Royal flush is reported
as its own category for display; in the ordering it is the top straight flush.

| Category | Significant ranks |
|---|---|
| Straight flush / straight | top card (wheel = five) |
| Four of a kind | quad rank, kicker |
| Full house | trips rank, pair rank (a second set counts as the pair) |
| Flush / high card | five cards, descending |
| Three of a kind | trips rank, two kickers |
| Two pair | high pair, low pair, kicker (may come from a third pair) |
| One pair | pair rank, three kickers |

## Validation (all automated)
| Check | Where |
|---|---|
| All 2,598,960 five-card hands: textbook category counts and exactly 7,462 distinct values (1277/2860/858/858/10/1277/156/156/9/1 per category) | `tests/exhaustive.rs::all_five_card_hands` |
| All 133,784,560 seven-card hands: textbook category counts and 4,824 distinct values | `all_seven_card_hands` (`scripts/validate-engine.sh`) |
| 7-card value = best of its 21 five-card subsets (20,000 random hands) | property test |
| Input order never matters | property test |
| Pairwise order agrees with an independently written naive evaluator | property test |
| Value ↔ class bijection with `phevaluator` over 150,000 random hands; category agreement | `tests/validation/` |
| Showdown order agrees with `pokerkit` | `tests/validation/` |

Seven-card textbook counts used: straight flush 41,584 (royal 4,324), quads 224,848, full house
3,473,184, flush 4,047,644, straight 6,180,020, trips 6,461,620, two pair 31,433,400, pair
58,627,800, high card 23,294,460.
