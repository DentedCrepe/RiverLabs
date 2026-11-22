use crate::chips::Chips;
use serde::{Deserialize, Serialize};

/// A main or side pot and the seats eligible to win it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pot {
    pub amount: Chips,
    pub eligible: Vec<usize>,
}

/// Splits total contributions into a main pot and side pots.
///
/// Layers are cut at every distinct contribution level of a non-folded player. Each layer
/// collects, from every player (folded or not), the part of their contribution inside the
/// layer; only non-folded players who contributed up to the top of the layer are eligible.
/// Adjacent layers with identical eligibility are merged. Chips in a layer nobody live
/// reached (only possible from dead money above every live contribution) are added to the
/// previous pot, so no chip is ever lost.
pub fn compute_pots(contributions: &[Chips], folded: &[bool]) -> Vec<Pot> {
    assert_eq!(contributions.len(), folded.len());
    let mut levels: Vec<Chips> =
        contributions.iter().zip(folded).filter(|(c, f)| !**f && !c.is_zero()).map(|(c, _)| *c).collect();
    let max_all = contributions.iter().copied().max().unwrap_or(Chips::ZERO);
    levels.push(max_all);
    levels.sort();
    levels.dedup();

    let mut pots: Vec<Pot> = Vec::new();
    let mut prev = Chips::ZERO;
    for level in levels {
        if level <= prev {
            continue;
        }
        let amount: Chips = contributions.iter().map(|&c| c.min(level).checked_sub(prev).unwrap_or(Chips::ZERO)).sum();
        let eligible: Vec<usize> =
            (0..contributions.len()).filter(|&i| !folded[i] && contributions[i] >= level).collect();
        prev = level;
        if amount.is_zero() {
            continue;
        }
        match pots.last_mut() {
            Some(last) if last.eligible == eligible || eligible.is_empty() => last.amount += amount,
            _ if eligible.is_empty() => {
                // Dead money with no live contributor at all: give it to every live player.
                let live: Vec<usize> = (0..contributions.len()).filter(|&i| !folded[i]).collect();
                pots.push(Pot { amount, eligible: live });
            }
            _ => pots.push(Pot { amount, eligible }),
        }
    }
    pots
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn c(v: &[u64]) -> Vec<Chips> {
        v.iter().copied().map(Chips).collect()
    }

    #[test]
    fn single_pot_when_all_equal() {
        let p = compute_pots(&c(&[100, 100, 100]), &[false, false, false]);
        assert_eq!(p, vec![Pot { amount: Chips(300), eligible: vec![0, 1, 2] }]);
    }

    #[test]
    fn side_pot_with_short_all_in_and_folded_dead_money() {
        // A all-in 50, B 100, C folded after putting in 80, D 100.
        let p = compute_pots(&c(&[50, 100, 80, 100]), &[false, false, true, false]);
        assert_eq!(
            p,
            vec![Pot { amount: Chips(200), eligible: vec![0, 1, 3] }, Pot { amount: Chips(130), eligible: vec![1, 3] },]
        );
    }

    #[test]
    fn three_way_all_in_levels() {
        let p = compute_pots(&c(&[10, 20, 30]), &[false, false, false]);
        assert_eq!(
            p,
            vec![
                Pot { amount: Chips(30), eligible: vec![0, 1, 2] },
                Pot { amount: Chips(20), eligible: vec![1, 2] },
                Pot { amount: Chips(10), eligible: vec![2] },
            ]
        );
    }

    proptest! {
        #[test]
        fn pots_conserve_chips_and_only_live_players_are_eligible(
            v in proptest::collection::vec((0u64..1000, any::<bool>()), 2..7)
        ) {
            let contributions: Vec<Chips> = v.iter().map(|x| Chips(x.0)).collect();
            let mut folded: Vec<bool> = v.iter().map(|x| x.1).collect();
            folded[0] = false; // at least one live player
            let pots = compute_pots(&contributions, &folded);
            let total: Chips = contributions.iter().copied().sum();
            prop_assert_eq!(pots.iter().map(|p| p.amount).sum::<Chips>(), total);
            for p in &pots {
                prop_assert!(!p.eligible.is_empty());
                for &i in &p.eligible { prop_assert!(!folded[i]); }
            }
        }
    }
}
