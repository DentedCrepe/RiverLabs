//! Closed-form poker theory for RiverLab Academy, plus toy-game demonstrations.
//!
//! Every function states its model. These are exact results **for the stated model** (e.g. a
//! single bet on the river with no further action), not equilibria of full poker. Amounts are
//! integer milli-big-blinds; results are `f64` ratios or mbb.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TheoryError {
    #[error("{0}")]
    Invalid(String),
}

type Result<T> = std::result::Result<T, TheoryError>;

fn invalid<T>(m: impl Into<String>) -> Result<T> {
    Err(TheoryError::Invalid(m.into()))
}

/// Pot odds facing a bet. `pot` is everything in the middle **including** the bet you face;
/// `to_call` is what you must add.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PotOdds {
    /// Equity a call needs to break even when no more money goes in: to_call / (pot + to_call).
    pub required_equity: f64,
    /// The same as odds against: pot : to_call.
    pub odds_ratio: f64,
    /// Size of the final pot if you call.
    pub final_pot_mbb: u64,
}

pub fn pot_odds(pot_mbb: u64, to_call_mbb: u64) -> Result<PotOdds> {
    if to_call_mbb == 0 {
        return invalid("there is nothing to call");
    }
    let final_pot = pot_mbb + to_call_mbb;
    Ok(PotOdds {
        required_equity: to_call_mbb as f64 / final_pot as f64,
        odds_ratio: pot_mbb as f64 / to_call_mbb as f64,
        final_pot_mbb: final_pot,
    })
}

/// A single bet of `bet` into `pot` (the pot **before** the bet), with no later action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetMath {
    /// Minimum defence frequency: the share of the defender's range that must continue so a
    /// pure bluff gains nothing, pot / (pot + bet).
    pub mdf: f64,
    /// Alpha: the fold frequency a pure bluff needs to break even, bet / (pot + bet) = 1 − MDF.
    pub alpha: f64,
    /// Equity the caller needs: bet / (pot + 2·bet).
    pub caller_required_equity: f64,
    /// In a polarised betting range (nuts or air) that makes the caller indifferent, the share
    /// of bets that are bluffs: bet / (pot + 2·bet). Bluffs per value bet: bet / (pot + bet).
    pub bluff_share: f64,
    pub bluffs_per_value_bet: f64,
}

pub fn bet_math(pot_mbb: u64, bet_mbb: u64) -> Result<BetMath> {
    if pot_mbb == 0 || bet_mbb == 0 {
        return invalid("pot and bet must be positive");
    }
    let (p, b) = (pot_mbb as f64, bet_mbb as f64);
    Ok(BetMath {
        mdf: p / (p + b),
        alpha: b / (p + b),
        caller_required_equity: b / (p + 2.0 * b),
        bluff_share: b / (p + 2.0 * b),
        bluffs_per_value_bet: b / (p + b),
    })
}

/// Expected value of calling versus folding (folding = 0), in mbb.
/// `pot` includes the bet faced; `equity` is your share of the final pot at showdown.
pub fn ev_call(equity: f64, pot_mbb: u64, to_call_mbb: u64) -> Result<f64> {
    if !(0.0..=1.0).contains(&equity) {
        return invalid("equity must be between 0 and 1");
    }
    Ok(equity * (pot_mbb + to_call_mbb) as f64 - to_call_mbb as f64)
}

/// Expected value of a pure bluff of `bet` into `pot` (before the bet) that never wins when
/// called, versus checking and losing (0), in mbb.
pub fn ev_bluff(fold_frequency: f64, pot_mbb: u64, bet_mbb: u64) -> Result<f64> {
    if !(0.0..=1.0).contains(&fold_frequency) {
        return invalid("fold frequency must be between 0 and 1");
    }
    Ok(fold_frequency * pot_mbb as f64 - (1.0 - fold_frequency) * bet_mbb as f64)
}

/// Probability of hitting one of `outs` with the next card(s), with `unseen` cards left.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outs {
    /// Exact chance on the next card: outs / unseen.
    pub next_card: f64,
    /// Exact chance by the river when two cards come (flop to river, no folding in between):
    /// 1 − C(unseen − outs, 2) / C(unseen, 2).
    pub two_cards: Option<f64>,
    /// The "rule of 2 and 4" approximations (2% per out per card, 4% for two cards), shown so
    /// lessons can compare them with the exact figures.
    pub rule_of_two: f64,
    pub rule_of_four: f64,
}

pub fn outs(outs: u32, unseen: u32) -> Result<Outs> {
    if unseen == 0 || outs > unseen {
        return invalid("need 0 <= outs <= unseen cards and at least one unseen card");
    }
    let (o, u) = (f64::from(outs), f64::from(unseen));
    let two = (unseen >= 2).then(|| {
        let miss = (u - o) * (u - o - 1.0) / (u * (u - 1.0));
        1.0 - miss.max(0.0)
    });
    Ok(Outs { next_card: o / u, two_cards: two, rule_of_two: 0.02 * o, rule_of_four: 0.04 * o })
}

/// Regret matching in rock–paper–scissors self-play: a toy demonstration of the update CFR
/// uses at every decision node. Not poker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegretMatchingStep {
    pub iteration: u32,
    /// Player 1's current strategy (rock, paper, scissors).
    pub current: [f64; 3],
    /// Player 1's average strategy over iterations so far.
    pub average: [f64; 3],
    /// Player 1's cumulative regrets.
    pub regrets: [f64; 3],
    /// How much a best response gains against player 1's average strategy (0 at equilibrium).
    pub exploitability: f64,
}

/// Payoff to the row player: rock 0, paper 1, scissors 2.
const RPS: [[f64; 3]; 3] = [[0.0, -1.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 1.0, 0.0]];

fn matched(regrets: &[f64; 3]) -> [f64; 3] {
    let pos: Vec<f64> = regrets.iter().map(|r| r.max(0.0)).collect();
    let total: f64 = pos.iter().sum();
    if total > 0.0 { [pos[0] / total, pos[1] / total, pos[2] / total] } else { [1.0 / 3.0; 3] }
}

/// Simultaneous self-play from the given starting regrets (both players start the same).
/// `plus` floors cumulative regrets at zero after each update (the "+" of CFR+).
pub fn regret_matching_rps(iterations: u32, start_regrets: [f64; 3], plus: bool) -> Result<Vec<RegretMatchingStep>> {
    if iterations == 0 || iterations > 10_000 {
        return invalid("iterations must be between 1 and 10,000");
    }
    if start_regrets.iter().any(|r| !r.is_finite()) {
        return invalid("starting regrets must be finite");
    }
    let mut r = [start_regrets, start_regrets];
    let mut sum = [[0.0; 3]; 2];
    let mut out = Vec::with_capacity(iterations as usize);
    for t in 1..=iterations {
        let s = [matched(&r[0]), matched(&r[1])];
        for p in 0..2 {
            let opp = s[1 - p];
            // Value of each pure action against the opponent's current strategy (symmetric game).
            let value: Vec<f64> = (0..3).map(|a| (0..3).map(|b| RPS[a][b] * opp[b]).sum()).collect();
            let node: f64 = (0..3).map(|a| s[p][a] * value[a]).sum();
            for a in 0..3 {
                r[p][a] += value[a] - node;
                if plus {
                    r[p][a] = r[p][a].max(0.0);
                }
                sum[p][a] += s[p][a];
            }
        }
        let total: f64 = sum[0].iter().sum();
        let avg = [sum[0][0] / total, sum[0][1] / total, sum[0][2] / total];
        let br = (0..3).map(|b| (0..3).map(|a| -RPS[a][b] * avg[a]).sum::<f64>()).fold(f64::MIN, f64::max);
        out.push(RegretMatchingStep { iteration: t, current: s[0], average: avg, regrets: r[0], exploitability: br });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn pot_odds_and_bet_math_match_hand_calculations() {
        // Facing a half-pot bet: pot 10 + bet 5 = 15, call 5 -> need 5/20 = 25%.
        let po = pot_odds(15_000, 5_000).unwrap();
        assert!(close(po.required_equity, 0.25) && close(po.odds_ratio, 3.0) && po.final_pot_mbb == 20_000);
        assert!(pot_odds(10_000, 0).is_err());
        // Pot-sized bet: MDF 50%, alpha 50%, caller needs 1/3, bluffs are 1/3 of bets.
        let m = bet_math(10_000, 10_000).unwrap();
        assert!(close(m.mdf, 0.5) && close(m.alpha, 0.5));
        assert!(close(m.caller_required_equity, 1.0 / 3.0) && close(m.bluff_share, 1.0 / 3.0));
        assert!(close(m.bluffs_per_value_bet, 0.5));
        // Half pot: MDF 2/3, bluffs 25% of bets.
        let h = bet_math(10_000, 5_000).unwrap();
        assert!(close(h.mdf, 2.0 / 3.0) && close(h.bluff_share, 0.25));
    }

    #[test]
    fn ev_is_zero_exactly_at_break_even() {
        // At the required equity, calling and folding are worth the same.
        let po = pot_odds(15_000, 5_000).unwrap();
        assert!(ev_call(po.required_equity, 15_000, 5_000).unwrap().abs() < 1e-9);
        let m = bet_math(10_000, 10_000).unwrap();
        assert!(ev_bluff(m.alpha, 10_000, 10_000).unwrap().abs() < 1e-9);
        assert!(ev_call(1.5, 1, 1).is_err());
    }

    #[test]
    fn outs_are_exact_and_rules_of_thumb_are_labelled_approximations() {
        // A flush draw on the flop: 9 outs, 47 unseen cards.
        let o = outs(9, 47).unwrap();
        assert!(close(o.next_card, 9.0 / 47.0));
        assert!(close(o.two_cards.unwrap(), 1.0 - (38.0 * 37.0) / (47.0 * 46.0))); // ≈ 34.97%
        assert!(close(o.rule_of_four, 0.36));
        assert!(outs(48, 47).is_err());
    }

    #[test]
    fn regret_matching_average_converges_to_the_uniform_equilibrium() {
        let steps = regret_matching_rps(5_000, [3.0, 0.0, 0.0], false).unwrap();
        let last = steps.last().unwrap();
        assert!(last.exploitability < 0.02, "{last:?}");
        for x in last.average {
            assert!((x - 1.0 / 3.0).abs() < 0.02);
        }
        // The current strategy cycles instead of settling: it is the average that converges.
        let spread = |s: &RegretMatchingStep| {
            s.current.iter().cloned().fold(0.0, f64::max) - s.current.iter().cloned().fold(1.0, f64::min)
        };
        assert!(steps.iter().skip(1000).any(|s| spread(s) > 0.2));
        let plus = regret_matching_rps(5_000, [3.0, 0.0, 0.0], true).unwrap();
        assert!(plus.last().unwrap().regrets.iter().all(|r| *r >= 0.0));
        assert!(regret_matching_rps(0, [0.0; 3], false).is_err());
    }
}
