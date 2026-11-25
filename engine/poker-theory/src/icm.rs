//! The Independent Chip Model (ICM): the prize-money value of tournament stacks.
//!
//! Model (Malmuth–Harville): a player finishes first with probability equal to their share of
//! the chips; given who finished first, the next place is drawn the same way from the remaining
//! stacks, and so on. Each player's value is the sum over places of P(place) × prize. This is a
//! model, not a property of poker: it ignores skill, position, blinds and how many hands remain.
//!
//! Computed exactly over every finishing order with a subset dynamic program (2^n × n steps).
//! Players with no chips finish below everyone with chips; several such players share their
//! places equally.

use crate::{Result, invalid};
use serde::{Deserialize, Serialize};

/// The most players accepted (2^n work).
pub const MAX_PLAYERS: usize = 16;

/// Each player's ICM value and finishing-place probabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icm {
    /// Value of each stack, in the units of `prizes`.
    pub values: Vec<f64>,
    /// `places[i][k]`: probability that player `i` finishes in place `k + 1`.
    pub places: Vec<Vec<f64>>,
}

/// ICM values of `stacks` (chips, any integer unit) for `prizes` (place 1 first; places without
/// a prize may be left out).
pub fn icm(stacks: &[u64], prizes: &[f64]) -> Result<Icm> {
    let n = stacks.len();
    if n == 0 || n > MAX_PLAYERS {
        return invalid(format!("give 1 to {MAX_PLAYERS} stacks, got {n}"));
    }
    if prizes.len() > n {
        return invalid(format!("{} prizes for {n} players", prizes.len()));
    }
    if prizes.iter().any(|p| !p.is_finite() || *p < 0.0) {
        return invalid("prizes must be finite and non-negative");
    }
    let full = (1usize << n) - 1;
    // prob[mask]: probability that the players in `mask` take the top |mask| places (any order).
    let mut prob = vec![0.0f64; 1 << n];
    prob[0] = 1.0;
    let mut places = vec![vec![0.0; n]; n];
    for mask in 0..full {
        let p = prob[mask];
        if p == 0.0 {
            continue;
        }
        let place = mask.count_ones() as usize;
        let rest: Vec<usize> = (0..n).filter(|&j| mask & (1 << j) == 0).collect();
        let chips: u128 = rest.iter().map(|&j| stacks[j] as u128).sum();
        for &j in &rest {
            let q = if chips == 0 { 1.0 / rest.len() as f64 } else { stacks[j] as f64 / chips as f64 };
            if q == 0.0 {
                continue;
            }
            prob[mask | (1 << j)] += p * q;
            places[j][place] += p * q;
        }
    }
    let values = places.iter().map(|row| row.iter().zip(prizes).map(|(p, prize)| p * prize).sum()).collect();
    Ok(Icm { values, places })
}

/// An all-in call heads-up, valued with ICM. Shares are fractions of the remaining prizes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmCall {
    /// Your ICM value if you fold, if you call and win, and if you call and lose.
    pub fold: f64,
    pub win: f64,
    pub lose: f64,
    /// The equity at which calling and folding have the same ICM value:
    /// (fold − lose) / (win − lose). Ties are not modelled separately.
    pub required_equity: f64,
}

/// Values calling an all-in heads-up (everyone else has folded) against folding.
///
/// `stacks` are chips behind for every player still in the tournament (including those who
/// folded this hand); `pot` is everything in the middle including the bet you face; `to_call`
/// is the full amount the bet asks of you (it may exceed your stack: the unmatched part goes
/// back to the bettor). Dead money in the pot goes to whoever wins.
pub fn icm_call(
    stacks: &[u64],
    hero: usize,
    villain: usize,
    pot: u64,
    to_call: u64,
    prizes: &[f64],
) -> Result<IcmCall> {
    let n = stacks.len();
    if hero >= n || villain >= n || hero == villain {
        return invalid("hero and villain must be two different players");
    }
    if to_call == 0 || to_call > pot {
        return invalid("to_call must be positive and part of the pot");
    }
    let called = to_call.min(stacks[hero]);
    let refund = to_call - called;
    let value = |hero_chips: u64, villain_chips: u64| -> Result<f64> {
        let mut s = stacks.to_vec();
        s[hero] = hero_chips;
        s[villain] = villain_chips;
        Ok(icm(&s, prizes)?.values[hero])
    };
    let total = prizes.iter().sum::<f64>();
    if total <= 0.0 {
        return invalid("the prizes must add up to more than zero");
    }
    let fold = value(stacks[hero], stacks[villain] + pot)? / total;
    let win = value(stacks[hero] + pot - refund, stacks[villain] + refund)? / total;
    let lose = value(stacks[hero] - called, stacks[villain] + pot + called)? / total;
    if win <= lose {
        return invalid("winning the pot must be worth more than losing it");
    }
    Ok(IcmCall { fold, win, lose, required_equity: ((fold - lose) / (win - lose)).clamp(0.0, 1.0) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Every finishing order, weighted by the Malmuth–Harville probability (reference).
    fn brute(stacks: &[u64], prizes: &[f64]) -> Vec<f64> {
        fn go(stacks: &[u64], prizes: &[f64], left: &mut Vec<usize>, place: usize, p: f64, out: &mut [f64]) {
            if left.is_empty() || p == 0.0 {
                return;
            }
            let chips: u64 = left.iter().map(|&j| stacks[j]).sum();
            for k in 0..left.len() {
                let j = left[k];
                let q = if chips == 0 { 1.0 / left.len() as f64 } else { stacks[j] as f64 / chips as f64 };
                out[j] += p * q * prizes.get(place).copied().unwrap_or(0.0);
                left.remove(k);
                go(stacks, prizes, left, place + 1, p * q, out);
                left.insert(k, j);
            }
        }
        let mut out = vec![0.0; stacks.len()];
        go(stacks, prizes, &mut (0..stacks.len()).collect(), 0, 1.0, &mut out);
        out
    }

    #[test]
    fn three_players_by_hand() {
        // 5000/3000/2000 chips, prizes 50/30/20. Worked by hand:
        // A: 1st 0.5; 2nd 0.3·5/7 + 0.2·5/8 = 0.339286; 3rd 0.160714 → 25 + 10.178571 + 3.214286.
        // C: 1st 0.2; 2nd 0.5·2/5 + 0.3·2/7 = 0.285714; 3rd 0.514286 → 10 + 8.571429 + 10.285714.
        let r = icm(&[5000, 3000, 2000], &[50.0, 30.0, 20.0]).unwrap();
        assert!((r.values[0] - 38.392857).abs() < 1e-5, "{:?}", r.values);
        assert!((r.values[2] - 28.857143).abs() < 1e-5, "{:?}", r.values);
        assert!((r.values[1] - 32.75).abs() < 1e-9, "{:?}", r.values);
        assert!((r.values.iter().sum::<f64>() - 100.0).abs() < 1e-9);
        assert!((r.places[0][1] - 0.339286).abs() < 1e-6);
    }

    #[test]
    fn winner_take_all_and_heads_up_are_chip_shares() {
        let r = icm(&[1000, 3000, 6000], &[100.0]).unwrap();
        for (v, s) in r.values.iter().zip([0.1, 0.3, 0.6]) {
            assert!((v - 100.0 * s).abs() < 1e-9);
        }
        // Two players: 2nd prize + chip share × (1st − 2nd), linear in chips.
        let r = icm(&[2500, 7500], &[65.0, 35.0]).unwrap();
        assert!((r.values[0] - (35.0 + 0.25 * 30.0)).abs() < 1e-9);
    }

    #[test]
    fn busted_players_take_the_bottom_places() {
        let r = icm(&[4000, 0, 6000], &[50.0, 30.0, 20.0]).unwrap();
        assert!((r.values[1] - 20.0).abs() < 1e-9);
        let r = icm(&[10_000, 0, 0], &[50.0, 30.0, 20.0]).unwrap();
        assert!((r.values[0] - 50.0).abs() < 1e-9);
        assert!((r.values[1] - 25.0).abs() < 1e-9 && (r.values[2] - 25.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert!(icm(&[], &[]).is_err());
        assert!(icm(&[1, 2], &[1.0, 1.0, 1.0]).is_err());
        assert!(icm(&[1, 2], &[f64::NAN]).is_err());
        assert!(icm(&[1; 17], &[1.0]).is_err());
        assert!(icm_call(&[10, 10], 0, 0, 5, 2, &[1.0]).is_err());
        assert!(icm_call(&[10, 10], 0, 1, 5, 0, &[1.0]).is_err());
    }

    #[test]
    fn icm_needs_more_equity_than_chips_on_the_bubble() {
        // Four left, three paid 50/30/20: you have 2000 behind and face a 6000 shove, with 500
        // of blinds in the pot; the two others sit on 1000 each. You risk 2000 to win 2500.
        let stacks = [2000u64, 0, 1000, 1000];
        let c = icm_call(&stacks, 0, 1, 6500, 6000, &[50.0, 30.0, 20.0]).unwrap();
        let chip = 2000.0 / 4500.0;
        assert!(c.required_equity > chip + 0.05, "ICM {} vs chip {}", c.required_equity, chip);
        // Winner take all: ICM is linear, so it matches chip pot odds.
        let c = icm_call(&[3000, 4000, 1000], 0, 1, 2500, 2000, &[100.0]).unwrap();
        assert!((c.required_equity - 2000.0 / 4500.0).abs() < 1e-9);
    }

    proptest! {
        #[test]
        fn matches_every_finishing_order(
            stacks in prop::collection::vec(0u64..10_000, 1..7),
            raw in prop::collection::vec(0u32..100, 0..7),
        ) {
            let mut prizes: Vec<f64> = raw.into_iter().take(stacks.len()).map(f64::from).collect();
            prizes.sort_by(|a, b| b.total_cmp(a));
            let r = icm(&stacks, &prizes).unwrap();
            let b = brute(&stacks, &prizes);
            for (x, y) in r.values.iter().zip(&b) {
                prop_assert!((x - y).abs() < 1e-7, "{:?} vs {:?}", r.values, b);
            }
            // Conservation: every prize is paid exactly once.
            prop_assert!((r.values.iter().sum::<f64>() - prizes.iter().sum::<f64>()).abs() < 1e-7);
            // Every player finishes somewhere.
            for row in &r.places {
                prop_assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-9);
            }
            // With prizes that never increase, more chips are never worth less.
            for i in 0..stacks.len() {
                for j in 0..stacks.len() {
                    if stacks[i] > stacks[j] {
                        prop_assert!(r.values[i] >= r.values[j] - 1e-9);
                    }
                }
            }
        }
    }
}
