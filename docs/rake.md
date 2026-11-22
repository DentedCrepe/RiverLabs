# Rake model

Source: `engine/poker-core/src/rake.rs` (unit + property tests). Rake configuration is
independent of stack configuration (ADR-0004).

| Field | Meaning | Default |
|---|---|---|
| `percentage_bps` | Percentage in basis points (500 = 5%), 0–10000 | 500 |
| `cap` | Maximum rake per hand, in bb (`mbb`) or currency (`minor_units`) | none |
| `min_pot` | Pots below this are not raked | none |
| `rounding.increment` | Rake is a multiple of this amount (bb or currency) | 0.001bb |
| `rounding.mode` | `down`, `nearest` (half up) or `up` | down |
| `no_flop_no_drop` | Hands that end before the flop is dealt are not raked | true |
| `allocation` | How rake is split over main/side pots: `proportional` or `main_pot_first` | proportional |
| `stake` (game config) | Currency and big blind in minor units; required for currency amounts | none |

## Calculation
For a final pot `P` (after uncalled bets are returned):
1. If no-flop-no-drop applies and no flop was dealt → 0.
2. If `P < min_pot` → 0.
3. `raw = P × bps / 10000`, kept as an exact rational.
4. If `raw ≥ cap` → `cap`.
5. Otherwise round `raw` to a multiple of the increment using the rounding mode.
6. Clamp to `cap` and to `P` (round-up can never take more than the pot).

The cap must be a multiple of the rounding increment (validated).

## Currency amounts
Currency amounts convert to mbb as `minor_units × 1000 / big_blind_minor_units`. If that is not
an integer (e.g. a 1-cent increment with a 3-cent big blind) the configuration is **rejected**
rather than silently rounded.

## Allocation across pots
Proportional allocation gives each pot `floor(rake × pot / total)`. Leftover mbb go to the
pots with the largest remainders, ties to the main pot first. Shares always sum exactly to
the rake and never exceed a pot.

## Known simplifications
- Caps that depend on the number of players dealt in are not modelled yet.
- Jackpot drops, time charges and rakeback are out of scope.
