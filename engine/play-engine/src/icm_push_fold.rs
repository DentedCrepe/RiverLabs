//! ICM push/fold for 3 to 6 players: a **modelled approximation** (docs/trainer.md).
//!
//! The model:
//! - players act in order (first to act, ..., small blind, big blind); blinds 0.5/1bb and an
//!   optional ante; each stack is the player's chips at the start of the hand;
//! - the first player to put money in moves **all-in** or folds (no limps, no smaller raises);
//! - after an all-in, each later player calls all-in or folds, and the **first call ends the
//!   betting**: nobody else may join (no overcalls);
//! - every outcome is valued with ICM (Malmuth–Harville, `poker_theory::icm`) for the payouts,
//!   so a player's payoff is their share of the prize pool;
//! - all-in equities come from the exact 1,326 × 1,326 table (`ComboEquities`), with exact card
//!   removal between the player who moves in and the caller; the folded players' cards are
//!   ignored (they could hold cards either player needs). Ties count as half a win.
//!
//! Solved with CFR+ (regret floor at zero, linear averaging). With three or more players CFR has
//! no convergence guarantee, so every solve reports `epsilon`: the most any single player could
//! gain by changing their strategy, in percent of the prize pool. With two players the model is
//! heads-up push/fold, and the tests check it against `solver_validation::push_fold`.

use crate::config::{PlayError, Result};
use poker_theory::icm::icm;
use range_engine::COMBO_COUNT;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use solver_validation::push_fold::ComboEquities;

fn default_small_blind() -> u64 {
    500
}
fn default_big_blind() -> u64 {
    1_000
}
/// Combos compatible with any given combo: C(50, 2).
const OPPONENTS: f64 = 1225.0;

/// One push/fold spot: stacks in action order (the last two are the small and big blind), in
/// any chip unit: drills use mbb (big blind 1,000), sit & go reviews the table's chips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IcmSpot {
    pub stacks: Vec<u64>,
    #[serde(default = "default_small_blind")]
    pub small_blind: u64,
    #[serde(default = "default_big_blind")]
    pub big_blind: u64,
    #[serde(default)]
    pub ante: u64,
    /// Percent of the prize pool for 1st, 2nd, ... still to be won (whole numbers, at most 100
    /// in all; drills use payouts that add up to 100).
    pub payouts: Vec<u32>,
}

impl IcmSpot {
    pub fn players(&self) -> usize {
        self.stacks.len()
    }

    pub fn validate(&self) -> Result<()> {
        let bad = |m: String| Err(PlayError::InvalidConfig(m));
        let n = self.players();
        if !(2..=6).contains(&n) {
            return bad(format!("2 to 6 players, got {n}"));
        }
        if self.payouts.is_empty() || self.payouts.len() > n || self.payouts.contains(&0) {
            return bad("give 1 to one-per-player positive payouts".into());
        }
        // At most 100: in a sit & go with fewer players left than paid places, only the top
        // prizes are still in play.
        if self.payouts.windows(2).any(|w| w[1] > w[0]) || self.payouts.iter().sum::<u32>() > 100 {
            return bad("payouts are percentages of the prize pool (at most 100 in all) that never increase".into());
        }
        if self.big_blind == 0 || self.small_blind == 0 || 2 * self.small_blind != self.big_blind {
            return bad("the push/fold model uses a small blind of exactly half the big blind".into());
        }
        if self.ante > self.big_blind {
            return bad("the ante can be at most the big blind".into());
        }
        for (k, &s) in self.stacks.iter().enumerate() {
            if s <= self.post(k) {
                return bad(format!("player {}'s stack must be more than their blind and ante", k + 1));
            }
            if s > 200 * self.big_blind {
                return bad("stacks of at most 200 big blinds".into());
            }
        }
        Ok(())
    }

    /// What player `k` puts in before acting: the ante, and the blind for the last two seats.
    fn post(&self, k: usize) -> u64 {
        let n = self.players();
        let blind = if k + 1 == n {
            self.big_blind
        } else if k + 2 == n {
            self.small_blind
        } else {
            0
        };
        self.ante + blind
    }

    fn prizes(&self) -> Vec<f64> {
        self.payouts.iter().take(self.players()).map(|&p| f64::from(p) / 100.0).collect()
    }
}

/// A strategy profile: `push[i][combo]` for every player but the big blind (first in), and
/// `call[i * n + j][combo]` for player `j` facing player `i`'s all-in (j > i, nobody in between
/// called).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmProfile {
    pub players: usize,
    pub push: Vec<Vec<f64>>,
    pub call: Vec<Vec<f64>>,
}

impl IcmProfile {
    fn uniform(n: usize) -> IcmProfile {
        let call = (0..n * n).map(|ij| if ij / n < ij % n { vec![0.5; COMBO_COUNT] } else { vec![] }).collect();
        IcmProfile { players: n, push: vec![vec![0.5; COMBO_COUNT]; n - 1], call }
    }
}

/// ICM values (share of the prize pool for every player) at each kind of final outcome.
struct Terminals {
    walk: Vec<f64>,
    steal: Vec<Vec<f64>>,
    /// `win[i * n + j]`: `i` moved in, `j` called and `i` won; `lose`: `j` won.
    win: Vec<Vec<f64>>,
    lose: Vec<Vec<f64>>,
}

fn terminals(spot: &IcmSpot) -> Result<Terminals> {
    let n = spot.players();
    let prizes = spot.prizes();
    let s: Vec<u64> = spot.stacks.clone();
    let post: Vec<u64> = (0..n).map(|k| spot.post(k)).collect();
    let after_posts: Vec<u64> = (0..n).map(|k| s[k] - post[k]).collect();
    let value = |stacks: Vec<u64>| -> Result<Vec<f64>> {
        icm(&stacks, &prizes).map(|r| r.values).map_err(|e| PlayError::Engine(e.to_string()))
    };
    let all_posts: u64 = post.iter().sum();
    let mut walk = after_posts.clone();
    walk[n - 1] = s[n - 1] + all_posts - post[n - 1];
    let walk = value(walk)?;
    let mut steal = vec![];
    for i in 0..n {
        let mut st = after_posts.clone();
        st[i] = s[i] + all_posts - post[i];
        steal.push(value(st)?);
    }
    let (mut win, mut lose) = (vec![vec![]; n * n], vec![vec![]; n * n]);
    for i in 0..n {
        for j in i + 1..n {
            let e = s[i].min(s[j]);
            let dead: u64 = (0..n).filter(|&k| k != i && k != j).map(|k| post[k]).sum();
            let mut w = after_posts.clone();
            w[i] = s[i] + e + dead;
            w[j] = s[j] - e;
            let mut l = after_posts.clone();
            l[i] = s[i] - e;
            l[j] = s[j] + e + dead;
            win[i * n + j] = value(w)?;
            lose[i * n + j] = value(l)?;
        }
    }
    Ok(Terminals { walk, steal, win, lose })
}

/// The stacks after each kind of outcome (for independent checks): `None` = everyone folds to
/// the big blind; `Some((i, None))` = `i` moves in and nobody calls; `Some((i, Some((j, i_wins))))`.
pub fn outcome_stacks(spot: &IcmSpot, outcome: Option<(usize, Option<(usize, bool)>)>) -> Vec<u64> {
    let n = spot.players();
    let s = &spot.stacks;
    let post: Vec<u64> = (0..n).map(|k| spot.post(k)).collect();
    let mut out: Vec<u64> = (0..n).map(|k| s[k] - post[k]).collect();
    let all: u64 = post.iter().sum();
    match outcome {
        None => out[n - 1] = s[n - 1] + all - post[n - 1],
        Some((i, None)) => out[i] = s[i] + all - post[i],
        Some((i, Some((j, i_wins)))) => {
            let e = s[i].min(s[j]);
            let dead: u64 = (0..n).filter(|&k| k != i && k != j).map(|k| post[k]).sum();
            let (w, l) = if i_wins { (i, j) } else { (j, i) };
            out[w] = s[w] + e + dead;
            out[l] = s[l] - e;
        }
    }
    out
}

/// Everything a profile implies: each decision's counterfactual values and the game's value.
struct Analysis {
    /// `q[i * n + j][h]`: probability `j` calls `i`'s all-in when `i` holds `h` (card removal
    /// between the two); `a`: the same weighted by `i`'s equity.
    q: Vec<Vec<f64>>,
    a: Vec<Vec<f64>>,
    /// `value[k][p]`: player `p`'s value when everyone before `k` folded and `k` is to act
    /// (`k = n - 1`: the big blind's walk).
    value: Vec<Vec<f64>>,
}

struct Game<'a> {
    eq: &'a ComboEquities,
    t: Terminals,
    n: usize,
}

impl Game<'_> {
    fn analyse(&self, prof: &IcmProfile) -> Analysis {
        let n = self.n;
        let mut q = vec![vec![]; n * n];
        let mut a = vec![vec![]; n * n];
        let pairs: Vec<(usize, usize)> = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect();
        let rows: Vec<(Vec<f64>, Vec<f64>)> = pairs
            .par_iter()
            .map(|&(i, j)| {
                let call = &prof.call[i * n + j];
                let mut qv = vec![0.0; COMBO_COUNT];
                let mut av = vec![0.0; COMBO_COUNT];
                for h in 0..COMBO_COUNT {
                    let row = self.eq.row(h);
                    let (mut sq, mut sa) = (0.0, 0.0);
                    for (c, &e) in row.iter().enumerate() {
                        if !e.is_nan() {
                            sq += call[c];
                            sa += call[c] * f64::from(e);
                        }
                    }
                    qv[h] = sq / OPPONENTS;
                    av[h] = sa / OPPONENTS;
                }
                (qv, av)
            })
            .collect();
        for (&(i, j), (qv, av)) in pairs.iter().zip(rows) {
            q[i * n + j] = qv;
            a[i * n + j] = av;
        }
        let mut value = vec![vec![0.0; n]; n];
        value[n - 1] = self.t.walk.clone();
        for k in (0..n - 1).rev() {
            let mut v = vec![0.0; n];
            for h in 0..COMBO_COUNT {
                let push = prof.push[k][h];
                for (p, vp) in v.iter_mut().enumerate() {
                    *vp += push * self.push_outcome(&q, &a, k, h, p, k + 1) + (1.0 - push) * value[k + 1][p];
                }
            }
            value[k] = v.into_iter().map(|x| x / COMBO_COUNT as f64).collect();
        }
        Analysis { q, a, value }
    }

    /// Player `p`'s value once `i` has moved in holding `h` and players before `from` have
    /// folded to it (`from = i + 1`: the whole response).
    fn push_outcome(&self, q: &[Vec<f64>], a: &[Vec<f64>], i: usize, h: usize, p: usize, from: usize) -> f64 {
        let n = self.n;
        let mut reach = 1.0;
        let mut v = 0.0;
        for j in from..n {
            let (qj, aj) = (q[i * n + j][h], a[i * n + j][h]);
            v += reach * (aj * self.t.win[i * n + j][p] + (qj - aj) * self.t.lose[i * n + j][p]);
            reach *= 1.0 - qj;
        }
        v + reach * self.t.steal[i][p]
    }

    /// Reach of `j`'s decision after `i` moved in with `h`: everyone in between folded.
    fn between(&self, q: &[Vec<f64>], i: usize, j: usize, h: usize) -> f64 {
        (i + 1..j).map(|k| 1.0 - q[i * self.n + k][h]).product()
    }

    /// Probability that everyone before `k` folds when first to act (their hands averaged).
    fn first_in_reach(&self, prof: &IcmProfile, k: usize) -> f64 {
        (0..k).map(|m| 1.0 - prof.push[m].iter().sum::<f64>() / COMBO_COUNT as f64).product()
    }

    /// Counterfactual values (push, fold) of player `i` first in, per combo.
    fn push_values(&self, prof: &IcmProfile, an: &Analysis, i: usize) -> Vec<(f64, f64)> {
        let reach = self.first_in_reach(prof, i);
        let fold = an.value[i + 1][i];
        (0..COMBO_COUNT).map(|h| (reach * self.push_outcome(&an.q, &an.a, i, h, i, i + 1), reach * fold)).collect()
    }

    /// Counterfactual values (call, fold) of player `j` facing `i`'s all-in, per combo, and the
    /// reach weight of each combo's decision (zero when `i` never moves in with a compatible hand).
    fn call_values(&self, prof: &IcmProfile, an: &Analysis, i: usize, j: usize) -> Vec<(f64, f64, f64)> {
        let n = self.n;
        let reach = self.first_in_reach(prof, i);
        let weight: Vec<f64> = (0..COMBO_COUNT).map(|h| prof.push[i][h] * self.between(&an.q, i, j, h)).collect();
        let fold_value: Vec<f64> = (0..COMBO_COUNT).map(|h| self.push_outcome(&an.q, &an.a, i, h, j, j + 1)).collect();
        let (w, l) = (self.t.win[i * n + j][j], self.t.lose[i * n + j][j]);
        (0..COMBO_COUNT)
            .into_par_iter()
            .map(|c| {
                let row = self.eq.row(c);
                let (mut sw, mut call, mut fold) = (0.0, 0.0, 0.0);
                for (h, &e) in row.iter().enumerate() {
                    if e.is_nan() || weight[h] == 0.0 {
                        continue;
                    }
                    // `e` is the caller's equity: it wins (pot share e) or the mover does.
                    let e = f64::from(e);
                    sw += weight[h];
                    call += weight[h] * (e * l + (1.0 - e) * w);
                    fold += weight[h] * fold_value[h];
                }
                let r = reach / COMBO_COUNT as f64;
                (r * call, r * fold, r * sw)
            })
            .collect()
    }
}

/// A solved spot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmSolution {
    pub spot: IcmSpot,
    pub profile: IcmProfile,
    pub iterations: usize,
    /// Each player's value of the hand under the profile (share of the prize pool), and the
    /// most each could gain by changing strategy alone.
    pub values: Vec<f64>,
    pub gains: Vec<f64>,
    /// max(gains) × 100: the profile is an ε-equilibrium of the model for this ε (% of the pool).
    pub epsilon_pct: f64,
}

fn rm(r: &[f64; 2]) -> f64 {
    let s = r[0] + r[1];
    if s > 0.0 { r[1] / s } else { 0.5 }
}

/// Solves `spot` with CFR+ for `iterations` and measures how far from equilibrium it is.
pub fn solve(eq: &ComboEquities, spot: &IcmSpot, iterations: usize) -> Result<IcmSolution> {
    spot.validate()?;
    let n = spot.players();
    let game = Game { eq, t: terminals(spot)?, n };
    let mut regret_push = vec![vec![[0.0f64; 2]; COMBO_COUNT]; n - 1];
    let mut regret_call: Vec<Vec<[f64; 2]>> =
        (0..n * n).map(|ij| if ij / n < ij % n { vec![[0.0; 2]; COMBO_COUNT] } else { vec![] }).collect();
    let mut avg = IcmProfile::uniform(n);
    for v in avg.push.iter_mut().chain(avg.call.iter_mut()) {
        v.iter_mut().for_each(|x| *x = 0.0);
    }
    let mut current = IcmProfile::uniform(n);
    let mut wsum = 0.0;
    for t in 1..=iterations {
        let an = game.analyse(&current);
        for i in 0..n - 1 {
            for (h, (vp, vf)) in game.push_values(&current, &an, i).into_iter().enumerate() {
                let s = current.push[i][h];
                let v = s * vp + (1.0 - s) * vf;
                let r = &mut regret_push[i][h];
                r[1] = (r[1] + vp - v).max(0.0);
                r[0] = (r[0] + vf - v).max(0.0);
            }
            for j in i + 1..n {
                for (c, (vc, vf, _)) in game.call_values(&current, &an, i, j).into_iter().enumerate() {
                    let s = current.call[i * n + j][c];
                    let v = s * vc + (1.0 - s) * vf;
                    let r = &mut regret_call[i * n + j][c];
                    r[1] = (r[1] + vc - v).max(0.0);
                    r[0] = (r[0] + vf - v).max(0.0);
                }
            }
        }
        for (p, r) in current.push.iter_mut().zip(&regret_push) {
            *p = r.iter().map(rm).collect();
        }
        for (ij, r) in regret_call.iter().enumerate() {
            if !r.is_empty() {
                current.call[ij] = r.iter().map(rm).collect();
            }
        }
        let w = t as f64;
        wsum += w;
        for (a, c) in avg.push.iter_mut().zip(&current.push).chain(avg.call.iter_mut().zip(&current.call)) {
            for (x, y) in a.iter_mut().zip(c) {
                *x += w * y;
            }
        }
    }
    for v in avg.push.iter_mut().chain(avg.call.iter_mut()) {
        v.iter_mut().for_each(|x| *x /= wsum.max(1.0));
    }
    let (values, gains) = exploitability(&game, &avg);
    let epsilon_pct = gains.iter().copied().fold(0.0, f64::max) * 100.0;
    Ok(IcmSolution { spot: spot.clone(), profile: avg, iterations, values, gains, epsilon_pct })
}

/// Each player's value under `prof`, and their best-response gain. A player's decisions sit on
/// different branches (first in, or facing one particular all-in), so the best response picks
/// the better action at each of them independently.
fn exploitability(game: &Game, prof: &IcmProfile) -> (Vec<f64>, Vec<f64>) {
    let n = game.n;
    let an = game.analyse(prof);
    let values = an.value[0].clone();
    let mut gains = vec![0.0; n];
    for (p, gain) in gains.iter_mut().enumerate() {
        let mut br = prof.clone();
        if p < n - 1 {
            br.push[p] = game.push_values(prof, &an, p).into_iter().map(|(a, f)| f64::from(u8::from(a > f))).collect();
        }
        for i in 0..p {
            br.call[i * n + p] =
                game.call_values(prof, &an, i, p).into_iter().map(|(c, f, _)| f64::from(u8::from(c > f))).collect();
        }
        *gain = (game.analyse(&br).value[0][p] - values[p]).max(0.0);
    }
    (values, gains)
}

/// A decision's values for one combo: what each action is worth (share of the prize pool,
/// given the decision is reached), for grading.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionValues {
    /// Moving in (first in) or calling (facing an all-in).
    pub aggressive: f64,
    pub fold: f64,
    /// The solution's frequency of the aggressive action for this combo.
    pub frequency: f64,
}

/// The values of `combo`'s choices for player `p`, first in (`facing = None`) or facing
/// player `i`'s all-in (`facing = Some(i)`).
pub fn action_values(
    eq: &ComboEquities,
    sol: &IcmSolution,
    p: usize,
    facing: Option<usize>,
    combo: usize,
) -> Result<ActionValues> {
    let n = sol.spot.players();
    let game = Game { eq, t: terminals(&sol.spot)?, n };
    let an = game.analyse(&sol.profile);
    match facing {
        None if p + 1 < n => {
            let reach = game.first_in_reach(&sol.profile, p).max(1e-300);
            let (a, f) = game.push_values(&sol.profile, &an, p)[combo];
            Ok(ActionValues { aggressive: a / reach, fold: f / reach, frequency: sol.profile.push[p][combo] })
        }
        Some(i) if i < p && p < n => {
            let (c, f, w) = game.call_values(&sol.profile, &an, i, p)[combo];
            if w <= 0.0 {
                return Err(PlayError::State("that all-in never happens with a hand compatible with yours".into()));
            }
            Ok(ActionValues { aggressive: c / w, fold: f / w, frequency: sol.profile.call[i * n + p][combo] })
        }
        _ => Err(PlayError::State("no such decision in this spot".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny synthetic table (rank-based equities) keeps unit tests fast; the real table is
    /// used in tests/icm_push_fold.rs.
    fn toy() -> ComboEquities {
        ComboEquities::from_fn(|x, y| {
            let s = |c: range_engine::Combo| {
                let (a, b) = c.cards();
                f32::from(a.rank().index() + b.rank().index()) + if a.rank() == b.rank() { 12.0 } else { 0.0 }
            };
            let (a, b) = (s(x), s(y));
            if a == b { 0.5 } else { 0.5 + 0.3 * (a - b).signum() * ((a - b).abs() / 36.0).min(1.0) }
        })
    }

    fn spot(stacks_bb: &[u64], payouts: &[u32]) -> IcmSpot {
        IcmSpot {
            stacks: stacks_bb.iter().map(|s| s * 1000).collect(),
            small_blind: 500,
            big_blind: 1000,
            ante: 0,
            payouts: payouts.to_vec(),
        }
    }

    #[test]
    fn values_share_out_the_whole_prize_pool_and_cfr_approaches_equilibrium() {
        let eq = toy();
        let sp = spot(&[10, 8, 12], &[50, 30, 20]);
        let short = solve(&eq, &sp, 20).unwrap();
        let long = solve(&eq, &sp, 300).unwrap();
        for s in [&short, &long] {
            assert!((s.values.iter().sum::<f64>() - 1.0).abs() < 1e-9, "{:?}", s.values);
        }
        assert!(long.epsilon_pct < short.epsilon_pct, "{} vs {}", long.epsilon_pct, short.epsilon_pct);
        assert!(long.epsilon_pct < 0.1, "within 0.1% of the pool: {}", long.epsilon_pct);
    }

    #[test]
    fn action_values_agree_with_the_profile() {
        let eq = toy();
        let sol = solve(&eq, &spot(&[10, 8, 12], &[50, 30, 20]), 300).unwrap();
        // A strong hand moves in first in; the values say so.
        let aa = range_engine::Combo::new("As".parse().unwrap(), "Ah".parse().unwrap()).unwrap().index();
        let v = action_values(&eq, &sol, 0, None, aa).unwrap();
        assert!(v.aggressive > v.fold && v.frequency > 0.9, "{v:?}");
        assert!(action_values(&eq, &sol, 2, None, aa).is_err(), "the big blind is never first in");
        assert!(action_values(&eq, &sol, 0, Some(1), aa).is_err(), "player 0 never faces player 1");
    }

    #[test]
    fn invalid_spots_are_rejected() {
        assert!(spot(&[10], &[100]).validate().is_err());
        assert!(spot(&[10, 10, 10], &[70, 40]).validate().is_err(), "more than the whole pool");
        spot(&[10, 10], &[50, 30]).validate().unwrap(); // two left of three paid
        assert!(spot(&[10, 10, 10], &[30, 70]).validate().is_err());
        assert!(
            IcmSpot {
                stacks: vec![10_000, 10_000, 1_000],
                small_blind: 500,
                big_blind: 1000,
                ante: 0,
                payouts: vec![100]
            }
            .validate()
            .is_err()
        );
    }
}

// ---- drills ----

/// CFR+ iterations per drill spot (the measured epsilon is shown with every grade).
pub const ITERATIONS: usize = 150;
/// Choices whose values differ by at most this share of the prize pool count as equally good…
pub const EQUAL_SHARE: f64 = 0.000_05;
/// …and a wrong choice costing at most this share is "close" rather than a mistake.
pub const CLOSE_SHARE: f64 = 0.002_5;
/// Solutions kept in memory (by spot).
const CACHE: usize = 64;

/// What kind of spots to deal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IcmDrillSpec {
    pub players: usize,
    pub min_stack_mbb: u64,
    pub max_stack_mbb: u64,
    /// Empty: `config::default_payouts(players)`.
    #[serde(default)]
    pub payouts: Vec<u32>,
    #[serde(default)]
    pub ante_mbb: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmDrill {
    pub spot: IcmSpot,
    /// Position names in action order (e.g. CO, BTN, SB, BB).
    pub positions: Vec<String>,
    pub hero: usize,
    /// `None`: everyone before you folded (move in or fold); `Some(i)`: player `i` moved in and
    /// everyone in between folded (call or fold).
    pub facing: Option<usize>,
    pub cards: Vec<String>,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcmGrade {
    pub drill: IcmDrill,
    pub choice: crate::push_fold::Choice,
    /// ["fold", "jam"] or ["fold", "call"].
    pub actions: Vec<String>,
    /// The solution's frequency of each action with this combo.
    pub frequencies: [f64; 2],
    /// Your share of the prize pool (%) after each action, given this decision is reached.
    pub values_pct: [f64; 2],
    pub loss_pct: f64,
    /// "correct", "close" or "mistake" (thresholds above, docs/trainer.md).
    pub verdict: String,
    pub epsilon_pct: f64,
    pub iterations: usize,
    pub approximations: Vec<String>,
}

pub const APPROXIMATIONS: [&str; 4] = [
    "ICM (Malmuth–Harville) values each outcome: a model that ignores skill, position and future hands.",
    "Push/fold only: the first player in moves all-in or folds; the first call ends the betting (no overcalls).",
    "Card removal is exact between the player who moves in and the caller; the folded players' cards are ignored.",
    "Solved with CFR+, which has no convergence guarantee with three or more players: epsilon is measured, not assumed.",
];

pub fn position_names(n: usize) -> Vec<String> {
    let all = ["UTG", "HJ", "CO", "BTN", "SB", "BB"];
    if n == 2 {
        return vec!["SB".into(), "BB".into()];
    }
    all[6 - n..].iter().map(|s| s.to_string()).collect()
}

/// Deals ICM push/fold drills and grades them; solutions are cached and the next spot of each
/// kind is solved in the background while you think.
pub struct IcmTrainer {
    eq: std::sync::Arc<ComboEquities>,
    solved: std::sync::Mutex<Vec<(IcmSpot, std::sync::Arc<IcmSolution>)>>,
    /// A spot already solved for each drill spec, ready for the next deal.
    prepared: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<IcmDrillSpec, IcmSpot>>>,
}

impl IcmTrainer {
    pub fn new(eq: std::sync::Arc<ComboEquities>) -> IcmTrainer {
        IcmTrainer { eq, solved: Default::default(), prepared: Default::default() }
    }

    fn cached(&self, spot: &IcmSpot) -> Option<std::sync::Arc<IcmSolution>> {
        self.solved.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(s, _)| s == spot).map(|(_, x)| x.clone())
    }

    fn solution(&self, spot: &IcmSpot) -> Result<std::sync::Arc<IcmSolution>> {
        if let Some(s) = self.cached(spot) {
            return Ok(s);
        }
        let s = std::sync::Arc::new(solve(&self.eq, spot, ITERATIONS)?);
        let mut cache = self.solved.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= CACHE {
            cache.remove(0);
        }
        cache.push((spot.clone(), s.clone()));
        Ok(s)
    }

    fn random_spot(spec: &IcmDrillSpec, rng: &mut impl rand::RngExt) -> Result<IcmSpot> {
        let bad = |m: &str| Err(PlayError::InvalidConfig(m.into()));
        if !(3..=6).contains(&spec.players) {
            return bad("ICM drills are for 3 to 6 players");
        }
        if spec.min_stack_mbb < 2_000 || spec.max_stack_mbb > 30_000 || spec.min_stack_mbb > spec.max_stack_mbb {
            return bad("stacks between 2bb and 30bb, smallest first");
        }
        let steps = (spec.max_stack_mbb - spec.min_stack_mbb) / 500;
        let stacks = (0..spec.players).map(|_| spec.min_stack_mbb + rng.random_range(0..=steps) * 500).collect();
        let payouts =
            if spec.payouts.is_empty() { crate::config::default_payouts(spec.players) } else { spec.payouts.clone() };
        if payouts.iter().sum::<u32>() != 100 {
            return bad("drill payouts must add up to 100");
        }
        let spot = IcmSpot { stacks, small_blind: 500, big_blind: 1_000, ante: spec.ante_mbb, payouts };
        spot.validate()?;
        Ok(spot)
    }

    /// Deals a decision from a solved spot of this kind (solving one first if none is ready),
    /// then starts solving the next spot in the background.
    pub fn drill(self: &std::sync::Arc<Self>, spec: &IcmDrillSpec, seed: u64) -> Result<IcmDrill> {
        use rand::{RngExt, SeedableRng};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let ready = self.prepared.lock().unwrap_or_else(|e| e.into_inner()).remove(spec);
        let spot = match ready {
            Some(s) => s,
            None => Self::random_spot(spec, &mut rng)?,
        };
        let sol = self.solution(&spot)?;
        let me = self.clone();
        let (next_spec, next_seed) = (spec.clone(), seed ^ 0x9E37_79B9_7F4A_7C15);
        std::thread::spawn(move || {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(next_seed);
            if let Ok(next) = Self::random_spot(&next_spec, &mut rng) {
                if me.solution(&next).is_ok() {
                    me.prepared.lock().unwrap_or_else(|e| e.into_inner()).insert(next_spec, next);
                }
            }
        });
        let n = spot.players();
        // Decisions that happen often enough to be worth drilling.
        let share = |i: usize| sol.profile.push[i].iter().sum::<f64>() / COMBO_COUNT as f64;
        let mut options: Vec<(usize, Option<usize>)> =
            (0..n - 1).filter(|&p| (0..p).map(|m| 1.0 - share(m)).product::<f64>() > 0.02).map(|p| (p, None)).collect();
        for i in 0..n - 1 {
            if share(i) > 0.02 {
                options.extend((i + 1..n).map(|j| (j, Some(i))));
            }
        }
        let (hero, facing) = options[rng.random_range(0..options.len())];
        let combo = range_engine::Combo::from_index(rng.random_range(0..COMBO_COUNT)).expect("combo");
        let (a, b) = combo.cards();
        Ok(IcmDrill {
            positions: position_names(n),
            spot,
            hero,
            facing,
            cards: vec![a.to_string(), b.to_string()],
            class: range_engine::HandClass::of(combo).to_string(),
        })
    }

    /// Grades a choice against the solved spot (re-solving it if it was evicted).
    pub fn grade(&self, drill: &IcmDrill, choice: crate::push_fold::Choice) -> Result<IcmGrade> {
        drill.spot.validate()?;
        let combo = crate::push_fold::parse_combo(&drill.cards)?;
        let sol = self.solution(&drill.spot)?;
        let v = action_values(&self.eq, &sol, drill.hero, drill.facing, combo.index())?;
        let values = [v.fold, v.aggressive];
        let mine = values[usize::from(choice == crate::push_fold::Choice::Play)];
        let loss = (values[0].max(values[1]) - mine).max(0.0);
        let verdict = if loss <= EQUAL_SHARE {
            "correct"
        } else if loss <= CLOSE_SHARE {
            "close"
        } else {
            "mistake"
        };
        Ok(IcmGrade {
            drill: IcmDrill { class: range_engine::HandClass::of(combo).to_string(), ..drill.clone() },
            choice,
            actions: vec!["fold".into(), if drill.facing.is_some() { "call" } else { "jam" }.into()],
            frequencies: [1.0 - v.frequency, v.frequency],
            values_pct: [values[0] * 100.0, values[1] * 100.0],
            loss_pct: loss * 100.0,
            verdict: verdict.into(),
            epsilon_pct: sol.epsilon_pct,
            iterations: sol.iterations,
            approximations: APPROXIMATIONS.iter().map(|s| s.to_string()).collect(),
        })
    }
}

impl std::fmt::Debug for IcmTrainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IcmTrainer").finish_non_exhaustive()
    }
}

/// A sit & go decision you played, as an ICM push/fold drill, when it fits the model: preflop,
/// and either everyone before you folded and you moved all-in or folded, or exactly one player
/// moved all-in, everyone else before you folded, and you called or folded. `payouts` are the
/// tournament's; only the places still to be paid count. Returns the drill and your choice.
pub fn drill_from_record(
    record: &crate::record::HandRecord,
    decision: usize,
    payouts: &[u32],
) -> Result<(IcmDrill, crate::push_fold::Choice)> {
    use crate::push_fold::Choice;
    use poker_core::game::Action;
    let no = |m: &str| Err(PlayError::State(format!("not an ICM push/fold spot: {m}")));
    if !record.tournament {
        return no("only sit & go hands are valued with ICM");
    }
    let hero = record.hero_seat();
    let at = record
        .actions
        .iter()
        .enumerate()
        .filter(|(_, a)| a.seat == hero)
        .nth(decision)
        .map(|(k, _)| k)
        .ok_or_else(|| PlayError::State("no such decision in this hand".into()))?;
    let mine = &record.actions[at];
    if mine.street != "preflop" || record.actions[..at].iter().any(|a| a.seat == hero) {
        return no("only your first preflop action fits the model");
    }
    let before: Vec<_> = record.actions[..at].iter().filter(|a| a.action != Action::Fold).collect();
    let facing = match before.as_slice() {
        [] => None,
        [a] if a.all_in && matches!(a.action, Action::Raise { .. } | Action::Bet { .. }) => Some(a.seat),
        _ => return no("someone limped, raised less than all-in, or more than one player is in"),
    };
    let choice = match (&mine.action, facing) {
        (Action::Fold, _) => Choice::Fold,
        (Action::Raise { .. } | Action::Bet { .. }, None) if mine.all_in => Choice::Play,
        (Action::Call, Some(_)) => Choice::Play,
        _ => return no("your action was not all-in or fold (or call or fold facing an all-in)"),
    };
    let n = record.seats.len();
    let spot = IcmSpot {
        stacks: record.seats.iter().map(|s| s.starting_stack).collect(),
        small_blind: record.small_blind,
        big_blind: record.big_blind,
        ante: record.ante,
        payouts: payouts.iter().copied().take(n).collect(),
    };
    // With fewer players left than paid places, the lower places are already decided: only the
    // top `n` prizes are still in play (they add up to less than 100%).
    spot.validate()?;
    let cards = record.seats[hero].hole_cards.clone();
    let combo = crate::push_fold::parse_combo(&cards)?;
    Ok((
        IcmDrill {
            positions: record.seats.iter().map(|s| s.position.clone()).collect(),
            spot,
            hero,
            facing,
            cards,
            class: range_engine::HandClass::of(combo).to_string(),
        },
        choice,
    ))
}
