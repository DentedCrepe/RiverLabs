//! All-in equity: the share of the pot each player wins if all remaining board cards are
//! dealt and nobody folds. Ties split the pot equally.
//!
//! - [`exact`]: full enumeration of every non-conflicting combo tuple × every runout.
//!   Each disjoint (combos, runout) tuple is equally likely given the weights, because every
//!   tuple removes the same number of cards from the deck; its weight is the product of the
//!   combo weights. Refuses work above a budget instead of running unbounded.
//! - [`monte_carlo`]: seeded sampling with per-player standard errors. Combos are drawn
//!   independently by weight and conflicting draws are rejected, which samples exactly the
//!   conditional joint distribution used by [`exact`].
//!
//! See `docs/equity.md` for assumptions.

use hand_evaluator::evaluate_mask;
use poker_core::{Board, CardSet};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::{Combo, Range};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default maximum number of (tuple × runout) evaluations for exact enumeration.
pub const DEFAULT_EXACT_BUDGET: u64 = 400_000_000;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum EquityError {
    #[error("equity needs between 2 and 6 players, got {0}")]
    PlayerCount(usize),
    #[error("player {0}'s range has no combos left after removing board and dead cards")]
    EmptyRange(usize),
    #[error("no valid deal exists: the ranges cannot be held simultaneously")]
    NoValidDeal,
    #[error("exact enumeration would need about {estimated} evaluations (budget {budget}); use Monte Carlo")]
    TooLarge { estimated: u64, budget: u64 },
    #[error("Monte Carlo needs at least 1 iteration")]
    NoIterations,
    #[error("Monte Carlo rejected too many conflicting samples; ranges are nearly incompatible")]
    TooManyRejections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquityMethod {
    Exact,
    MonteCarlo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityResult {
    pub method: EquityMethod,
    /// Expected pot share per player; sums to 1.
    pub equity: Vec<f64>,
    /// Probability of winning the whole pot alone.
    pub win: Vec<f64>,
    /// Probability of splitting the pot.
    pub tie: Vec<f64>,
    /// Exact: number of (tuple, runout) evaluations. Monte Carlo: accepted samples.
    pub samples: u64,
    /// Monte Carlo only: standard error of each equity estimate.
    pub std_error: Option<Vec<f64>>,
    pub seed: Option<u64>,
}

struct Prepared {
    combos: Vec<Vec<(u64, f64)>>, // per player: (card mask, weight)
    board: u64,
    deck: Vec<u8>,
    need: usize,
}

fn prepare(ranges: &[Range], board: &Board, dead: CardSet) -> Result<Prepared, EquityError> {
    if !(2..=6).contains(&ranges.len()) {
        return Err(EquityError::PlayerCount(ranges.len()));
    }
    let blocked = dead.union(board.card_set());
    let mut combos = Vec::with_capacity(ranges.len());
    for (i, r) in ranges.iter().enumerate() {
        let v: Vec<(u64, f64)> = r
            .iter()
            .filter(|(c, _)| !c.card_set().intersects(blocked))
            .map(|(c, w)| (c.card_set().bits(), w))
            .collect();
        if v.is_empty() {
            return Err(EquityError::EmptyRange(i));
        }
        combos.push(v);
    }
    let deck: Vec<u8> = (0..52u8).filter(|&i| blocked.bits() & (1 << i) == 0).collect();
    Ok(Prepared { combos, board: board.card_set().bits(), deck, need: 5 - board.cards().len() })
}

fn n_choose_k(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    (0..k).fold(1u64, |acc, i| acc * (n - i) / (i + 1))
}

/// Calls `f(mask)` for every `k`-subset of `deck`.
fn for_each_subset(deck: &[u8], k: usize, f: &mut impl FnMut(u64)) {
    fn rec(deck: &[u8], k: usize, start: usize, mask: u64, f: &mut impl FnMut(u64)) {
        if k == 0 {
            f(mask);
            return;
        }
        for i in start..=deck.len() - k {
            rec(deck, k - 1, i + 1, mask | 1 << deck[i], f);
        }
    }
    if k <= deck.len() {
        rec(deck, k, 0, 0, f);
    }
}

#[derive(Default)]
struct Acc {
    equity: Vec<f64>,
    win: Vec<f64>,
    tie: Vec<f64>,
    total: f64,
}

impl Acc {
    fn new(n: usize) -> Acc {
        Acc { equity: vec![0.0; n], win: vec![0.0; n], tie: vec![0.0; n], total: 0.0 }
    }
    /// Adds one deal with weight `w` and per-player strengths `s`; returns per-player shares.
    fn add(&mut self, w: f64, s: &[u32], shares: &mut [f64]) {
        let best = *s.iter().max().expect("players");
        let k = s.iter().filter(|&&x| x == best).count();
        for (i, &x) in s.iter().enumerate() {
            shares[i] = if x == best { 1.0 / k as f64 } else { 0.0 };
            self.equity[i] += w * shares[i];
            if x == best {
                if k == 1 {
                    self.win[i] += w;
                } else {
                    self.tie[i] += w;
                }
            }
        }
        self.total += w;
    }
    fn normalise(mut self) -> Acc {
        for v in [&mut self.equity, &mut self.win, &mut self.tie] {
            for x in v.iter_mut() {
                *x /= self.total;
            }
        }
        self
    }
}

struct Enumerator<'a> {
    p: &'a Prepared,
    strengths: Vec<Vec<u32>>,
    s: Vec<u32>,
    shares: Vec<f64>,
    acc: Acc,
    samples: u64,
}

impl Enumerator<'_> {
    fn runout(&mut self, runout: u64) {
        let full_board = self.p.board | runout;
        for (pi, cs) in self.p.combos.iter().enumerate() {
            for (ci, &(m, _)) in cs.iter().enumerate() {
                // Combos colliding with the runout are skipped during tuple enumeration.
                self.strengths[pi][ci] = if m & runout == 0 { evaluate_mask(full_board | m).value() } else { 0 };
            }
        }
        self.dfs(0, runout, 1.0);
    }

    /// Depth-first over players, choosing non-conflicting combos.
    fn dfs(&mut self, pi: usize, used: u64, w: f64) {
        if pi == self.p.combos.len() {
            self.acc.add(w, &self.s, &mut self.shares);
            self.samples += 1;
            return;
        }
        for ci in 0..self.p.combos[pi].len() {
            let (m, cw) = self.p.combos[pi][ci];
            if m & used == 0 {
                self.s[pi] = self.strengths[pi][ci];
                self.dfs(pi + 1, used | m, w * cw);
            }
        }
    }
}

/// Exact equity by enumeration. `budget` bounds the number of evaluations.
pub fn exact(ranges: &[Range], board: &Board, dead: CardSet, budget: u64) -> Result<EquityResult, EquityError> {
    let p = prepare(ranges, board, dead)?;
    let n = ranges.len();
    // Upper-bound estimate: product of range sizes × runouts.
    let runouts = n_choose_k(p.deck.len() as u64, p.need as u64);
    let tuples = p.combos.iter().fold(1u64, |a, c| a.saturating_mul(c.len() as u64));
    let estimated = tuples.saturating_mul(runouts.max(1));
    if estimated > budget {
        return Err(EquityError::TooLarge { estimated, budget });
    }
    let mut e = Enumerator {
        p: &p,
        strengths: p.combos.iter().map(|c| vec![0; c.len()]).collect(),
        s: vec![0u32; n],
        shares: vec![0.0; n],
        acc: Acc::new(n),
        samples: 0,
    };
    for_each_subset(&p.deck, p.need, &mut |runout| e.runout(runout));
    let Enumerator { acc, samples, .. } = e;
    if acc.total == 0.0 {
        return Err(EquityError::NoValidDeal);
    }
    let acc = acc.normalise();
    Ok(EquityResult {
        method: EquityMethod::Exact,
        equity: acc.equity,
        win: acc.win,
        tie: acc.tie,
        samples,
        std_error: None,
        seed: None,
    })
}

/// Monte Carlo equity with a caller-supplied seed (results are reproducible).
pub fn monte_carlo(
    ranges: &[Range],
    board: &Board,
    dead: CardSet,
    iterations: u64,
    seed: u64,
) -> Result<EquityResult, EquityError> {
    if iterations == 0 {
        return Err(EquityError::NoIterations);
    }
    let p = prepare(ranges, board, dead)?;
    let n = ranges.len();
    let cumulative: Vec<Vec<f64>> = p
        .combos
        .iter()
        .map(|cs| {
            let mut acc = 0.0;
            cs.iter()
                .map(|&(_, w)| {
                    acc += w;
                    acc
                })
                .collect()
        })
        .collect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut acc = Acc::new(n);
    let mut sum_sq = vec![0.0; n];
    let mut s = vec![0u32; n];
    let mut shares = vec![0.0; n];
    let mut hands = vec![0u64; n];
    let mut rejections = 0u64;
    let max_rejections = iterations.saturating_mul(1000);
    let mut deck = p.deck.clone();
    let mut accepted = 0u64;
    while accepted < iterations {
        let mut used = 0u64;
        let mut ok = true;
        for pi in 0..n {
            let cum = &cumulative[pi];
            let x = rng.random::<f64>() * cum[cum.len() - 1];
            let idx = cum.partition_point(|&c| c <= x).min(cum.len() - 1);
            let m = p.combos[pi][idx].0;
            if m & used != 0 {
                ok = false;
                break;
            }
            used |= m;
            hands[pi] = m;
        }
        if !ok {
            rejections += 1;
            if rejections > max_rejections {
                return Err(EquityError::TooManyRejections);
            }
            continue;
        }
        // Partial Fisher–Yates over the live deck for the remaining board cards.
        let mut runout = 0u64;
        let mut drawn = 0;
        let mut end = deck.len();
        while drawn < p.need {
            let j = rng.random_range(0..end);
            let c = deck[j];
            end -= 1;
            deck.swap(j, end);
            if used & (1 << c) == 0 {
                runout |= 1 << c;
                drawn += 1;
            }
        }
        let full = p.board | runout;
        for pi in 0..n {
            s[pi] = evaluate_mask(full | hands[pi]).value();
        }
        acc.add(1.0, &s, &mut shares);
        for pi in 0..n {
            sum_sq[pi] += shares[pi] * shares[pi];
        }
        accepted += 1;
    }
    let total = acc.total;
    let acc = acc.normalise();
    let std_error = acc
        .equity
        .iter()
        .zip(&sum_sq)
        .map(|(&mean, &sq)| {
            let var = (sq / total - mean * mean).max(0.0);
            (var / total).sqrt()
        })
        .collect();
    Ok(EquityResult {
        method: EquityMethod::MonteCarlo,
        equity: acc.equity,
        win: acc.win,
        tie: acc.tie,
        samples: accepted,
        std_error: Some(std_error),
        seed: Some(seed),
    })
}

/// Convenience: a range containing exactly one combo.
pub fn single_combo_range(c: Combo) -> Range {
    let mut r = Range::empty();
    r.set_weight(c, 1.0).expect("1.0 is a valid weight");
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use poker_core::{Card, HoleCards};

    fn hand(s: &str) -> Range {
        single_combo_range(Combo::from(s.parse::<HoleCards>().unwrap()))
    }
    fn board(s: &str) -> Board {
        s.parse().unwrap()
    }
    fn eq(a: &str, b: &str, bd: &str) -> EquityResult {
        exact(&[hand(a), hand(b)], &board(bd), CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap()
    }

    #[test]
    fn river_is_deterministic() {
        let r = eq("AsAh", "KsKh", "2c7d9hTs3c");
        assert_eq!(r.equity, vec![1.0, 0.0]);
        assert_eq!(r.samples, 1);
        let r = eq("AsKd", "AhKc", "2c7d9hTs3c");
        assert_eq!(r.equity, vec![0.5, 0.5]);
        assert_eq!(r.tie, vec![1.0, 1.0]);
    }

    #[test]
    fn turn_counts_outs_exactly() {
        // KhQh (top pair + flush draw) vs 7c7d (set) on Kc7h2h3s, 44 unseen river cards.
        // 9 hearts remain, but 3h pairs the board and fills up the set; a king also gives
        // villain sevens full. Hero's outs: Ah Jh Th 9h 8h 6h 5h 4h = 8.
        let r = eq("KhQh", "7c7d", "Kc7h2h3s");
        assert!((r.equity[0] - 8.0 / 44.0).abs() < 1e-12, "{:?}", r.equity);
        assert_eq!(r.samples, 44);
    }

    #[test]
    fn symmetric_spots_are_exactly_even() {
        let r = eq("AsKs", "AhKh", "");
        assert!((r.equity[0] - 0.5).abs() < 1e-12);
        assert!((r.equity.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert_eq!(r.samples, 1_712_304); // C(48, 5)
    }

    #[test]
    fn ranges_respect_card_removal() {
        // AA vs a range of {KK} on a K-high flop with Kc on board: 3 KK combos remain.
        let aa = hand("AsAh");
        let kk = Range::parse("KK").unwrap();
        let r = exact(&[aa, kk], &board("Kc7d2h"), CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap();
        // Villain has a set; hero needs runner-runner. Verify against per-combo average.
        let mut sum = 0.0;
        for k in ["KsKd", "KsKh", "KdKh"] {
            sum += eq("AsAh", k, "Kc7d2h").equity[0];
        }
        assert!((r.equity[0] - sum / 3.0).abs() < 1e-12);
    }

    #[test]
    fn weighted_ranges_mix_linearly() {
        let b = board("Ts9s2d");
        let hero = hand("AsAh");
        let mixed = Range::parse("KsKh:0.25,QdQc:0.75").unwrap();
        let r = exact(&[hero.clone(), mixed], &b, CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap();
        let e1 = eq("AsAh", "KsKh", "Ts9s2d").equity[0];
        let e2 = eq("AsAh", "QdQc", "Ts9s2d").equity[0];
        assert!((r.equity[0] - (0.25 * e1 + 0.75 * e2)).abs() < 1e-12);
    }

    #[test]
    fn three_way_sums_to_one() {
        let r =
            exact(&[hand("AsAh"), hand("KsKh"), hand("QsQh")], &board("2c7d9h"), CardSet::EMPTY, DEFAULT_EXACT_BUDGET)
                .unwrap();
        assert!((r.equity.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(r.equity[0] > r.equity[1] && r.equity[1] > r.equity[2]);
    }

    #[test]
    fn errors() {
        let full = Range::full();
        assert!(matches!(
            exact(&[full.clone(), full.clone()], &Board::empty(), CardSet::EMPTY, DEFAULT_EXACT_BUDGET),
            Err(EquityError::TooLarge { .. })
        ));
        assert_eq!(
            exact(&[hand("AsAh")], &Board::empty(), CardSet::EMPTY, 1).unwrap_err(),
            EquityError::PlayerCount(1)
        );
        assert_eq!(
            exact(&[hand("AsAh"), hand("KsKh")], &board("As2c3d"), CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap_err(),
            EquityError::EmptyRange(0)
        );
        assert_eq!(
            exact(&[hand("AsAh"), hand("AsKh")], &board("2c3d4h"), CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap_err(),
            EquityError::NoValidDeal
        );
        let dead = CardSet::from_cards(Card::parse_many("Kh").unwrap());
        assert_eq!(
            exact(&[hand("AsAh"), hand("KsKh")], &Board::empty(), dead, DEFAULT_EXACT_BUDGET).unwrap_err(),
            EquityError::EmptyRange(1)
        );
    }

    #[test]
    fn monte_carlo_agrees_with_exact_within_error_and_is_reproducible() {
        let ranges = [Range::parse("TT+,AK").unwrap(), Range::parse("22+,AT+,KQ").unwrap()];
        let b = board("Ks8d4c");
        let ex = exact(&ranges, &b, CardSet::EMPTY, DEFAULT_EXACT_BUDGET).unwrap();
        let mc = monte_carlo(&ranges, &b, CardSet::EMPTY, 200_000, 7).unwrap();
        let se = mc.std_error.as_ref().unwrap()[0];
        assert!((mc.equity[0] - ex.equity[0]).abs() < 5.0 * se, "mc {} exact {} se {se}", mc.equity[0], ex.equity[0]);
        let again = monte_carlo(&ranges, &b, CardSet::EMPTY, 200_000, 7).unwrap();
        assert_eq!(mc, again);
    }
}
