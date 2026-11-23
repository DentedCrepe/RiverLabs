//! The bots' preflop strategy: GTOpen's preflop model (an approximate model, see
//! `solver_gtopen::preflop`), solved once per table shape and cached in memory and on disk.
//!
//! Real hands rarely match a model tree exactly (stacks differ, sizes differ). The table's
//! effective stack is rounded to a bucket, and real bets are mapped to the nearest model action.
//! When a hand leaves the model tree (e.g. a 4-bet or a limp), the bots switch to the documented
//! fallback rules for the rest of the street.

use crate::config::{PlayError, Result};
use poker_core::GameConfig;
use poker_core::rake::RakeConfig;
use solver_gtopen::preflop::{PreflopBrowser, PreflopNode, PreflopSolved, equity_table, load_preflop, solve_preflop};
use solver_interface::{EquityRealization, PreflopSizing, PreflopSpec};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Effective stacks (bb) the model is solved at; a table uses the nearest one.
pub const STACK_BUCKETS: [u64; 11] = [10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200];

/// At or below this many bb the model is push/fold (all-in or fold first in).
pub const PUSH_FOLD_MAX_BB: u64 = 15;

/// What decides a table's model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelShape {
    pub players: u8,
    pub stack_bb: u64,
    pub small_blind_mbb: u64,
    pub ante_mbb: u64,
    pub rake: RakeConfig,
}

impl ModelShape {
    pub fn bucket(effective_bb: f64) -> u64 {
        *STACK_BUCKETS
            .iter()
            .min_by(|&&a, &&b| (effective_bb / a as f64).ln().abs().total_cmp(&(effective_bb / b as f64).ln().abs()))
            .expect("buckets")
    }

    pub fn spec(&self) -> PreflopSpec {
        let push_fold = self.stack_bb <= PUSH_FOLD_MAX_BB;
        let mut game = GameConfig {
            players: self.players,
            small_blind: poker_core::Chips(self.small_blind_mbb),
            ante: poker_core::Chips(self.ante_mbb),
            rake: self.rake,
            ..GameConfig::default()
        };
        game.stack.effective_stack = poker_core::Chips(self.stack_bb * 1000);
        let open = if self.stack_bb >= 30 { 2_500 } else { 2_000 };
        PreflopSpec {
            game,
            heads_up: false,
            sizing: PreflopSizing {
                open_raise_to_mbb: if push_fold { vec![] } else { vec![open] },
                reraise_pct_of_bet: if push_fold { vec![] } else { vec![300] },
                max_raises: if push_fold { 1 } else { 2 },
                add_all_in: true,
                all_in_threshold_pct_stack: 85,
                allow_limp: false,
            },
            realization: EquityRealization::Positional,
            max_iterations: 200,
            max_seconds: Some(45),
            check_interval: 50,
            threads: 2,
        }
    }

    fn key(&self) -> String {
        let r = &self.rake;
        let cap = serde_json::to_string(&r.cap).unwrap_or_default();
        format!(
            "p{}-s{}-sb{}-a{}-r{}-c{}-n{}",
            self.players, self.stack_bb, self.small_blind_mbb, self.ante_mbb, r.percentage_bps, cap, r.no_flop_no_drop
        )
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
    }
}

/// A solved model (fresh or loaded from disk).
pub enum Model {
    Solved(Box<PreflopSolved>),
    Loaded(Box<PreflopBrowser>),
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Model")
    }
}

impl Model {
    pub fn node(&self, path: &[usize]) -> Option<PreflopNode> {
        match self {
            Model::Solved(s) => s.node(path).ok(),
            Model::Loaded(b) => b.node(path).ok(),
        }
    }
}

/// Solved models shared by every session in the process.
#[derive(Debug)]
pub struct ModelCache {
    dir: Option<PathBuf>,
    max_bytes: u64,
    threads: u16,
    models: Mutex<HashMap<String, Arc<Model>>>,
}

impl ModelCache {
    pub fn new(dir: Option<PathBuf>, max_bytes: u64, threads: u16) -> ModelCache {
        if let Some(d) = &dir {
            let _ = std::fs::create_dir_all(d);
        }
        ModelCache { dir, max_bytes, threads: threads.max(1), models: Mutex::new(HashMap::new()) }
    }

    /// The model for `shape`, solving it (tens of seconds for six players) on first use.
    pub fn get(&self, shape: &ModelShape) -> Result<Arc<Model>> {
        let key = shape.key();
        if let Some(m) = self.models.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Ok(m.clone());
        }
        let mut spec = shape.spec();
        spec.threads = self.threads;
        let eq = equity_table(self.dir.as_deref());
        let file = self.dir.as_ref().map(|d| d.join(format!("preflop-model-{key}.gtp")));
        let model = match file.as_ref().filter(|f| f.exists()).map(|f| load_preflop(&spec, f, eq.clone())) {
            Some(Ok(b)) => Model::Loaded(Box::new(b)),
            _ => {
                let solved = solve_preflop(&spec, eq, self.max_bytes, Arc::new(AtomicBool::new(false)), |_| {})
                    .map_err(|e| PlayError::Engine(format!("preflop model: {e}")))?;
                if let Some(f) = &file {
                    // A failed cache write only costs a re-solve next time.
                    let _ = solved.save(f);
                }
                Model::Solved(Box::new(solved))
            }
        };
        let model = Arc::new(model);
        self.models.lock().unwrap_or_else(|e| e.into_inner()).insert(key, model.clone());
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_and_specs() {
        assert_eq!(ModelShape::bucket(100.0), 100);
        assert_eq!(ModelShape::bucket(87.0), 100);
        assert_eq!(ModelShape::bucket(12.0), 10);
        assert_eq!(ModelShape::bucket(13.0), 15);
        assert_eq!(ModelShape::bucket(5000.0), 200);
        let shape =
            |stack_bb| ModelShape { players: 3, stack_bb, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::none() };
        let pf = shape(10).spec();
        assert!(pf.sizing.open_raise_to_mbb.is_empty() && pf.sizing.max_raises == 1);
        pf.validate().unwrap();
        let deep = shape(100).spec();
        assert_eq!(deep.sizing.open_raise_to_mbb, [2_500]);
        deep.validate().unwrap();
        assert_ne!(shape(10).key(), shape(100).key());
    }

    #[test]
    fn cache_solves_once_and_reloads_from_disk() {
        // Kept between runs: GTOpen's equity table takes ~30 s to build.
        let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/play-test-cache"));
        let shape =
            ModelShape { players: 2, stack_bb: 10, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::none() };
        let cache = ModelCache::new(Some(dir.clone()), 1 << 30, 2);
        let a = cache.get(&shape).unwrap();
        let b = cache.get(&shape).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let root = a.node(&[]).unwrap();
        assert_eq!(root.actor.as_deref(), Some("SB"));
        let fresh = ModelCache::new(Some(dir.clone()), 1 << 30, 2);
        let c = fresh.get(&shape).unwrap();
        assert!(matches!(*c, Model::Loaded(_)));
        assert_eq!(c.node(&[]).unwrap(), root);
    }
}
