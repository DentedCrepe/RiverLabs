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
use std::collections::{HashMap, HashSet};
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
///
/// A table whose stacks drift into a new bucket (common in a sit & go, as blinds rise) needs a
/// new model, which takes tens of seconds to solve. [`ModelCache::get_or_nearest`] avoids making
/// the player wait for it: it plays the hand with the nearest stack depth already prepared for
/// that table and prepares the exact model in the background.
#[derive(Debug, Clone)]
pub struct ModelCache {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    dir: Option<PathBuf>,
    max_bytes: u64,
    threads: u16,
    models: Mutex<HashMap<String, Arc<Model>>>,
    /// One lock per model key, held while it is solved, so a model is never solved twice at once.
    solving: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// Keys being prepared in the background.
    pending: Mutex<HashSet<String>>,
}

fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Inner {
    /// The model if it is in memory or on disk (loading is fast); never solves.
    fn ready(&self, shape: &ModelShape) -> Option<Arc<Model>> {
        let key = shape.key();
        if let Some(m) = locked(&self.models).get(&key) {
            return Some(m.clone());
        }
        let file = self.dir.as_ref()?.join(format!("preflop-model-{key}.gtp"));
        if !file.exists() {
            return None;
        }
        let mut spec = shape.spec();
        spec.threads = self.threads;
        let browser = load_preflop(&spec, &file, equity_table(self.dir.as_deref())).ok()?;
        let model = Arc::new(Model::Loaded(Box::new(browser)));
        locked(&self.models).insert(key, model.clone());
        Some(model)
    }

    fn get(&self, shape: &ModelShape) -> Result<Arc<Model>> {
        if let Some(m) = self.ready(shape) {
            return Ok(m);
        }
        let key = shape.key();
        let gate = locked(&self.solving).entry(key.clone()).or_default().clone();
        let _solving = locked(&gate);
        if let Some(m) = self.ready(shape) {
            return Ok(m); // solved by someone else while this waited
        }
        let mut spec = shape.spec();
        spec.threads = self.threads;
        let eq = equity_table(self.dir.as_deref());
        let solved = solve_preflop(&spec, eq, self.max_bytes, Arc::new(AtomicBool::new(false)), |_| {})
            .map_err(|e| PlayError::Engine(format!("preflop model: {e}")))?;
        if let Some(d) = &self.dir {
            // A failed cache write only costs a re-solve next time.
            let _ = solved.save(&d.join(format!("preflop-model-{key}.gtp")));
        }
        let model = Arc::new(Model::Solved(Box::new(solved)));
        locked(&self.models).insert(key, model.clone());
        Ok(model)
    }
}

impl ModelCache {
    pub fn new(dir: Option<PathBuf>, max_bytes: u64, threads: u16) -> ModelCache {
        if let Some(d) = &dir {
            let _ = std::fs::create_dir_all(d);
        }
        ModelCache {
            inner: Arc::new(Inner {
                dir,
                max_bytes,
                threads: threads.max(1),
                models: Mutex::new(HashMap::new()),
                solving: Mutex::new(HashMap::new()),
                pending: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// The model for `shape`, solving it (tens of seconds for six players) on first use.
    pub fn get(&self, shape: &ModelShape) -> Result<Arc<Model>> {
        self.inner.get(shape)
    }

    /// Whether the model for `shape` is ready without solving.
    pub fn is_ready(&self, shape: &ModelShape) -> bool {
        self.inner.ready(shape).is_some()
    }

    /// Starts preparing `shape`'s model on a background thread, unless it is ready or already
    /// being prepared.
    pub fn prepare_in_background(&self, shape: &ModelShape) {
        if self.inner.ready(shape).is_some() {
            return;
        }
        let key = shape.key();
        if !locked(&self.inner.pending).insert(key.clone()) {
            return;
        }
        let inner = self.inner.clone();
        let shape = shape.clone();
        std::thread::spawn(move || {
            // An error leaves the model unprepared; `get` reports it if the model is needed.
            let _ = inner.get(&shape);
            locked(&inner.pending).remove(&key);
        });
    }

    /// The model for `shape` if it is ready. Otherwise the model for the nearest stack depth
    /// already prepared for the same table (same players, blinds, ante and rake), with the exact
    /// model prepared in the background; only a table with no model at all is solved while the
    /// caller waits. Returns the model and the stack bucket (bb) it was solved for.
    pub fn get_or_nearest(&self, shape: &ModelShape) -> Result<(Arc<Model>, u64)> {
        if let Some(m) = self.inner.ready(shape) {
            return Ok((m, shape.stack_bb));
        }
        let mut others: Vec<u64> = STACK_BUCKETS.iter().copied().filter(|&b| b != shape.stack_bb).collect();
        let distance = |b: u64| (b as f64 / shape.stack_bb as f64).ln().abs();
        others.sort_by(|&a, &b| distance(a).total_cmp(&distance(b)));
        for b in others {
            let near = ModelShape { stack_bb: b, ..shape.clone() };
            if let Some(m) = self.inner.ready(&near) {
                self.prepare_in_background(shape);
                return Ok((m, b));
            }
        }
        Ok((self.inner.get(shape)?, shape.stack_bb))
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

    #[test]
    fn a_missing_stack_depth_uses_the_nearest_ready_model_and_prepares_the_exact_one() {
        // A private model directory with the shared equity table, so the 15bb model starts unsolved.
        let shared = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/play-test-cache"));
        let dir = std::env::temp_dir().join(format!("riverlab-nearest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        ModelCache::new(Some(shared.clone()), 1 << 30, 2)
            .get(&ModelShape { players: 2, stack_bb: 10, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::none() })
            .unwrap();
        for f in std::fs::read_dir(&shared).unwrap().flatten() {
            let name = f.file_name().to_string_lossy().to_string();
            if name.starts_with("gtopen_preflop_eq") || name.contains("p2-s10-sb500-a0-r0") {
                std::fs::copy(f.path(), dir.join(&name)).unwrap();
            }
        }
        let cache = ModelCache::new(Some(dir.clone()), 1 << 30, 2);
        let wanted =
            ModelShape { players: 2, stack_bb: 15, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::none() };
        assert!(!cache.is_ready(&wanted));
        let (_, used) = cache.get_or_nearest(&wanted).unwrap();
        assert_eq!(used, 10, "the nearest prepared depth is used straight away");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while !cache.is_ready(&wanted) {
            assert!(std::time::Instant::now() < deadline, "the 15bb model was not prepared in the background");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert_eq!(cache.get_or_nearest(&wanted).unwrap().1, 15, "then the exact model is used");
        // A table with no model at all is still solved while the caller waits.
        let other =
            ModelShape { players: 2, stack_bb: 20, small_blind_mbb: 400, ante_mbb: 0, rake: RakeConfig::none() };
        assert_eq!(cache.get_or_nearest(&other).unwrap().1, 20);
        std::fs::remove_dir_all(&dir).ok();
    }
}
