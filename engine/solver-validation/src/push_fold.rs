//! Exact heads-up push/fold: ground truth for checking preflop solvers.
//!
//! The small blind either moves all-in or folds; the big blind calls or folds. Unlike class-level
//! models, this works on all 1,326 two-card combos with full card removal: the big blind's hand
//! is drawn from the deck that remains after the small blind's two cards. Every pairwise all-in
//! equity is computed exactly by enumerating all C(48,5) = 1,712,304 boards with RiverLab's
//! exhaustively validated evaluator.
//!
//! Units are big blinds. Stacks are equal (`stack` each, blinds included), no ante, no rake.

use hand_evaluator::evaluate_mask;
use range_engine::{COMBO_COUNT, Combo};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

/// `eq[i * 1326 + j]`: probability-weighted pot share of combo `i` against combo `j`
/// (ties count half). `NaN` where the combos share a card.
#[derive(Debug, Clone)]
pub struct ComboEquities {
    eq: Vec<f32>,
}

const MAGIC: &[u8; 8] = b"RLHUEQ01";

fn suit_perms() -> Vec<[u8; 4]> {
    let mut v = Vec::new();
    for a in 0..4u8 {
        for b in 0..4u8 {
            for c in 0..4u8 {
                for d in 0..4u8 {
                    let p = [a, b, c, d];
                    if (0..4).all(|i| (i + 1..4).all(|j| p[i] != p[j])) {
                        v.push(p);
                    }
                }
            }
        }
    }
    v
}

fn permute(c: Combo, p: &[u8; 4]) -> Combo {
    let (a, b) = c.cards();
    let f = |x: poker_core::Card| {
        poker_core::Card::from_index(x.rank().index() * 4 + p[x.suit().index() as usize]).expect("valid card")
    };
    Combo::new(f(a), f(b)).expect("distinct")
}

/// Exact equity of hand mask `h1` against `h2` (each 2 cards, disjoint), over all boards.
pub fn exact_pair_equity(h1: u64, h2: u64) -> f64 {
    let deck: Vec<u64> = (0..52).map(|i| 1u64 << i).filter(|b| b & (h1 | h2) == 0).collect();
    let n = deck.len();
    let (mut share, mut total) = (0u64, 0u64);
    for a in 0..n {
        for b in a + 1..n {
            let ab = deck[a] | deck[b];
            for c in b + 1..n {
                let abc = ab | deck[c];
                for d in c + 1..n {
                    let abcd = abc | deck[d];
                    for &card in &deck[d + 1..] {
                        let board = abcd | card;
                        let s1 = evaluate_mask(board | h1);
                        let s2 = evaluate_mask(board | h2);
                        // Count in half-units so ties stay integral.
                        share += if s1 > s2 {
                            2
                        } else if s1 == s2 {
                            1
                        } else {
                            0
                        };
                        total += 2;
                    }
                }
            }
        }
    }
    share as f64 / total as f64
}

impl ComboEquities {
    pub fn get(&self, i: usize, j: usize) -> f32 {
        self.eq[i * COMBO_COUNT + j]
    }

    /// A synthetic table (for tests): `f(i, j)` for compatible combos, `NaN` otherwise.
    pub fn from_fn(f: impl Fn(Combo, Combo) -> f32) -> ComboEquities {
        let mut eq = vec![f32::NAN; COMBO_COUNT * COMBO_COUNT];
        for x in Combo::all() {
            for y in Combo::all() {
                if !x.card_set().intersects(y.card_set()) {
                    eq[x.index() * COMBO_COUNT + y.index()] = f(x, y);
                }
            }
        }
        ComboEquities { eq }
    }

    /// Computes the full table (one exact enumeration per suit-isomorphism class of unordered
    /// pairs, about 47k), in parallel. Takes minutes; see [`ComboEquities::load_or_compute`].
    pub fn compute() -> ComboEquities {
        Self::compute_with_progress(|_, _| {})
    }

    /// Like [`ComboEquities::compute`], calling `progress(done, total)` as exact enumerations finish.
    pub fn compute_with_progress(progress: impl Fn(usize, usize) + Sync) -> ComboEquities {
        let perms = suit_perms();
        let combos: Vec<Combo> = Combo::all().collect();
        // Canonical representative of each unordered pair.
        let canon = |x: Combo, y: Combo| -> (usize, usize) {
            perms
                .iter()
                .map(|p| {
                    let (a, b) = (permute(x, p).index(), permute(y, p).index());
                    (a.min(b), a.max(b))
                })
                .min()
                .expect("24 permutations")
        };
        let mut reps: HashMap<(usize, usize), f64> = HashMap::new();
        for &x in &combos {
            for &y in &combos {
                if x < y && !x.card_set().intersects(y.card_set()) {
                    reps.entry(canon(x, y)).or_insert(f64::NAN);
                }
            }
        }
        let keys: Vec<(usize, usize)> = reps.keys().copied().collect();
        let done = std::sync::atomic::AtomicUsize::new(0);
        let values: Vec<f64> = keys
            .par_iter()
            .map(|&(a, b)| {
                let (ca, cb) = (combos[a], combos[b]);
                let v = exact_pair_equity(ca.card_set().bits(), cb.card_set().bits());
                let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if n % 256 == 0 || n == keys.len() {
                    progress(n, keys.len());
                }
                v
            })
            .collect();
        for (k, v) in keys.into_iter().zip(values) {
            reps.insert(k, v);
        }
        let mut eq = vec![f32::NAN; COMBO_COUNT * COMBO_COUNT];
        for &x in &combos {
            for &y in &combos {
                if x == y || x.card_set().intersects(y.card_set()) {
                    continue;
                }
                // equity(x vs y): find the permutation that maps {x, y} onto its representative.
                let key = canon(x, y);
                let v = reps[&key];
                let p = perms
                    .iter()
                    .find(|p| {
                        let (a, b) = (permute(x, p).index(), permute(y, p).index());
                        (a.min(b), a.max(b)) == key
                    })
                    .expect("representative is reachable");
                let first_is_x = permute(x, p).index() == key.0;
                eq[x.index() * COMBO_COUNT + y.index()] = if first_is_x { v as f32 } else { (1.0 - v) as f32 };
            }
        }
        ComboEquities { eq }
    }

    /// Loads a cached table; `None` if the file is missing or not a table.
    pub fn load(path: &Path) -> Option<ComboEquities> {
        let mut f = std::fs::File::open(path).ok()?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).ok()?;
        (buf.len() == 8 + 4 * COMBO_COUNT * COMBO_COUNT && &buf[..8] == MAGIC).then(|| ComboEquities {
            eq: buf[8..].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect(),
        })
    }

    /// Re-computes `samples` pseudo-random entries exactly and compares them (a check that a
    /// loaded or downloaded table is intact). Also checks every entry is in [0, 1] or `NaN`
    /// exactly where the combos share a card, and that `eq(i, j) + eq(j, i) = 1`.
    pub fn verify(&self, samples: usize, seed: u64) -> Result<(), String> {
        let combos: Vec<Combo> = Combo::all().collect();
        for (i, x) in combos.iter().enumerate() {
            for (j, y) in combos.iter().enumerate() {
                let v = self.get(i, j);
                let shared = i == j || x.card_set().intersects(y.card_set());
                if shared != v.is_nan() || (!shared && !(0.0..=1.0).contains(&v)) {
                    return Err(format!("entry {x} v {y} is {v}"));
                }
                if !shared && j > i && (v + self.get(j, i) - 1.0).abs() > 1e-5 {
                    return Err(format!("{x} v {y} and {y} v {x} do not sum to 1"));
                }
            }
        }
        let mut state = seed | 1;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % COMBO_COUNT as u64) as usize
        };
        let mut checked = 0;
        while checked < samples {
            let (i, j) = (next(), next());
            if self.get(i, j).is_nan() {
                continue;
            }
            let exact = exact_pair_equity(combos[i].card_set().bits(), combos[j].card_set().bits());
            if (exact - self.get(i, j) as f64).abs() > 1e-5 {
                return Err(format!("{} v {}: table {} but exact {exact}", combos[i], combos[j], self.get(i, j)));
            }
            checked += 1;
        }
        Ok(())
    }

    /// Loads a cached table, or computes and caches it.
    pub fn load_or_compute(path: &Path) -> std::io::Result<ComboEquities> {
        if let Some(t) = Self::load(path) {
            return Ok(t);
        }
        let t = ComboEquities::compute();
        t.save(path)?;
        Ok(t)
    }

    /// Writes the table (an 8-byte header and little-endian f32s).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let t = self;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("partial");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(MAGIC)?;
        let bytes: Vec<u8> = t.eq.iter().flat_map(|v| v.to_le_bytes()).collect();
        // Write then rename, so an interrupted save never leaves a truncated table.
        f.write_all(&bytes)?;
        drop(f);
        std::fs::rename(&tmp, path)
    }
}

/// A push/fold profile at combo level: `jam[i]` for the small blind, `call[j]` for the big blind.
#[derive(Debug, Clone, PartialEq)]
pub struct PushFold {
    pub jam: Vec<f64>,
    pub call: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PushFoldEvaluation {
    /// Small blind's expected value per hand (bb); the big blind's is the negative.
    pub sb_ev: f64,
    /// Best-response value per hand for each seat against the other's profile strategy.
    pub sb_best: f64,
    pub bb_best: f64,
    /// Average gain available to a deviating player (bb per hand).
    pub exploitability: f64,
}

struct Game<'a> {
    eq: &'a ComboEquities,
    stack: f64,
}

impl Game<'_> {
    fn compatible(&self, i: usize, j: usize) -> bool {
        !self.eq.get(i, j).is_nan()
    }
    /// SB's value of jamming with combo `i`, summed over BB's compatible combos.
    fn jam_value(&self, i: usize, call: &[f64]) -> f64 {
        let s = self.stack;
        (0..COMBO_COUNT)
            .filter(|&j| self.compatible(i, j))
            .map(|j| (1.0 - call[j]) * 1.0 + call[j] * (2.0 * s * self.eq.get(i, j) as f64 - s))
            .sum()
    }
    /// BB's values with combo `j` (call, fold), summed over SB's compatible combos weighted by jam.
    fn bb_values(&self, j: usize, jam: &[f64]) -> (f64, f64, f64) {
        let s = self.stack;
        let (mut call, mut fold, mut no_jam) = (0.0, 0.0, 0.0);
        // Row `j` holds the big blind's own equities (stored, like every entry, from the exact f64
        // value), so the table is read contiguously instead of down a column.
        for i in (0..COMBO_COUNT).filter(|&i| self.compatible(j, i)) {
            call += jam[i] * (2.0 * s * self.eq.get(j, i) as f64 - s);
            fold -= jam[i];
            no_jam += (1.0 - jam[i]) * 0.5;
        }
        (call, fold, no_jam)
    }
}

/// Number of BB combos compatible with any given SB combo (and vice versa): C(50, 2).
const OPPONENTS: f64 = 1225.0;

/// Exact value and exploitability of a push/fold profile.
pub fn evaluate(eq: &ComboEquities, stack: f64, profile: &PushFold) -> PushFoldEvaluation {
    let g = Game { eq, stack };
    let n = COMBO_COUNT as f64;
    let (mut ev, mut sb_best) = (0.0, 0.0);
    for i in 0..COMBO_COUNT {
        let jam = g.jam_value(i, &profile.call) / OPPONENTS;
        ev += (profile.jam[i] * jam + (1.0 - profile.jam[i]) * -0.5) / n;
        sb_best += jam.max(-0.5) / n;
    }
    let mut bb_best = 0.0;
    for j in 0..COMBO_COUNT {
        let (call, fold, no_jam) = g.bb_values(j, &profile.jam);
        bb_best += (call.max(fold) + no_jam) / OPPONENTS / n;
    }
    let bb_ev = -ev;
    let nash_conv = (sb_best - ev) + (bb_best - bb_ev);
    PushFoldEvaluation { sb_ev: ev, sb_best, bb_best, exploitability: nash_conv / 2.0 }
}

/// Each combo's expected value (bb, from the start of the hand, blinds included) of the
/// non-fold action against the other seat's `profile`: the small blind jamming, and the big blind
/// calling a jam. Folding is worth −0.5 (small blind) and −1 (big blind facing a jam).
#[derive(Debug, Clone, PartialEq)]
pub struct ComboEvs {
    pub jam: Vec<f64>,
    /// `NaN` where the small blind never jams a hand compatible with this combo.
    pub call: Vec<f64>,
}

pub fn combo_evs(eq: &ComboEquities, stack: f64, profile: &PushFold) -> ComboEvs {
    let g = Game { eq, stack };
    let jam = (0..COMBO_COUNT).into_par_iter().map(|i| g.jam_value(i, &profile.call) / OPPONENTS).collect();
    let call = (0..COMBO_COUNT)
        .into_par_iter()
        .map(|j| {
            let (call, fold, _) = g.bb_values(j, &profile.jam);
            // `fold` is minus the jam weight facing this combo; normalise to "given a jam".
            if fold < 0.0 { call / -fold } else { f64::NAN }
        })
        .collect();
    ComboEvs { jam, call }
}

/// Solves exact push/fold with CFR+ (regret floor at zero, linear averaging).
pub fn solve(eq: &ComboEquities, stack: f64, iterations: usize) -> PushFold {
    let g = Game { eq, stack };
    let (mut r_sb, mut r_bb) = (vec![[0.0f64; 2]; COMBO_COUNT], vec![[0.0f64; 2]; COMBO_COUNT]);
    let (mut avg_jam, mut avg_call) = (vec![0.0; COMBO_COUNT], vec![0.0; COMBO_COUNT]);
    let rm = |r: &[f64; 2]| {
        let s = r[0] + r[1];
        if s > 0.0 { r[1] / s } else { 0.5 }
    };
    let mut wsum = 0.0;
    for t in 1..=iterations {
        let call: Vec<f64> = r_bb.iter().map(rm).collect();
        let jam: Vec<f64> = r_sb.iter().map(rm).collect();
        // Alternating updates: SB against the current BB, then BB against the updated SB.
        let new_jam: Vec<f64> = (0..COMBO_COUNT)
            .into_par_iter()
            .map(|i| {
                let v_jam = g.jam_value(i, &call) / OPPONENTS;
                let v = jam[i] * v_jam + (1.0 - jam[i]) * -0.5;
                (v_jam - v, -0.5 - v)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, (rj, rf))| {
                r_sb[i][1] = (r_sb[i][1] + rj).max(0.0);
                r_sb[i][0] = (r_sb[i][0] + rf).max(0.0);
                rm(&r_sb[i])
            })
            .collect();
        let updates: Vec<(f64, f64)> = (0..COMBO_COUNT)
            .into_par_iter()
            .map(|j| {
                let (c, f, _) = g.bb_values(j, &new_jam);
                let v = call[j] * c + (1.0 - call[j]) * f;
                (c - v, f - v)
            })
            .collect();
        for (j, (rc, rf)) in updates.into_iter().enumerate() {
            r_bb[j][1] = (r_bb[j][1] + rc).max(0.0);
            r_bb[j][0] = (r_bb[j][0] + rf).max(0.0);
        }
        let w = t as f64;
        wsum += w;
        for i in 0..COMBO_COUNT {
            avg_jam[i] += w * new_jam[i];
            avg_call[i] += w * rm(&r_bb[i]);
        }
    }
    PushFold {
        jam: avg_jam.into_iter().map(|x| x / wsum).collect(),
        call: avg_call.into_iter().map(|x| x / wsum).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poker_core::{Board, CardSet, HoleCards};

    fn combo(s: &str) -> Combo {
        Combo::from(s.parse::<HoleCards>().unwrap())
    }

    #[test]
    fn fast_enumerator_matches_the_general_equity_engine() {
        for (a, b) in [("AsAh", "KsKh"), ("AsKd", "AhKc"), ("7c2d", "8h8s"), ("JsTs", "AdKd")] {
            let (x, y) = (combo(a), combo(b));
            let fast = exact_pair_equity(x.card_set().bits(), y.card_set().bits());
            let reference = equity_engine::exact(
                &[equity_engine::single_combo_range(x), equity_engine::single_combo_range(y)],
                &Board::empty(),
                CardSet::EMPTY,
                u64::MAX,
            )
            .unwrap();
            assert!((fast - reference.equity[0]).abs() < 1e-12, "{a} v {b}: {fast} vs {}", reference.equity[0]);
        }
    }

    #[test]
    fn coin_flip_equities_give_jam_and_call_everything() {
        let eq = ComboEquities::from_fn(|_, _| 0.5);
        let sol = solve(&eq, 10.0, 200);
        let e = evaluate(&eq, 10.0, &sol);
        assert!(sol.jam.iter().all(|&j| j > 0.99) && sol.call.iter().all(|&c| c > 0.99));
        assert!(e.sb_ev.abs() < 1e-9 && e.exploitability < 1e-6, "{e:?}");
        // A profile that never calls is exploitable: SB jams everything and wins the 1bb blind.
        let lazy = PushFold { jam: vec![1.0; COMBO_COUNT], call: vec![0.0; COMBO_COUNT] };
        let e = evaluate(&eq, 10.0, &lazy);
        assert!((e.sb_ev - 1.0).abs() < 1e-9 && (e.bb_best - 0.0).abs() < 1e-9, "{e:?}");
    }

    #[test]
    fn verification_rejects_tables_that_are_not_exact() {
        // Symmetric and well-formed, but not the real equities: caught by exact re-computation.
        let fake = ComboEquities::from_fn(|_, _| 0.5);
        assert!(fake.verify(0, 1).is_ok());
        assert!(fake.verify(3, 1).unwrap_err().contains("exact"));
        // Not symmetric.
        let skewed = ComboEquities::from_fn(|_, _| 0.6);
        assert!(skewed.verify(0, 1).unwrap_err().contains("sum to 1"));
        // A real table round-trips through save and load.
        let dir = std::env::temp_dir().join(format!("riverlab-eq-{}", std::process::id()));
        let path = dir.join("t.bin");
        fake.save(&path).unwrap();
        assert_eq!(ComboEquities::load(&path).unwrap().get(0, 100), 0.5);
        std::fs::write(&path, b"short").unwrap();
        assert!(ComboEquities::load(&path).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn suit_permutations_are_the_24_relabelings() {
        assert_eq!(suit_perms().len(), 24);
        let x = combo("AsKs");
        assert_eq!(suit_perms().iter().map(|p| permute(x, p)).collect::<std::collections::HashSet<_>>().len(), 4);
    }
}
