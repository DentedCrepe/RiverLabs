//! Times each stage of a solve on realistic spots: iterations, exploitability checks, the
//! independent check (export + evaluate), saving and loading.
//! `cargo run --release -p solver-gtopen --example bench_pipeline [threads]`
use poker_core::rake::RakeConfig;
use poker_core::{Board, GameConfig};
use solver_gtopen::{load, solve};
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

fn spec(board: &str, iters: u64, check: u64, threads: u16, rake: RakeConfig) -> SolveSpec {
    let sz = |b: u32| PlayerSizing { bet_pct_pot: [vec![b], vec![b], vec![b]], raise_pct_pot: Default::default() };
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = iters;
    solver.stopping.target_exploitability_pct_pot = None;
    solver.stopping.exploitability_check_interval = check;
    solver.threads = threads;
    SolveSpec {
        game: GameConfig { rake, ..GameConfig::default() },
        ranges: vec![OOP.into(), IP.into()],
        board: board.parse::<Board>().unwrap(),
        starting_pot_mbb: 6_500,
        effective_stack_mbb: 97_500,
        tree: TreeAbstraction {
            oop: sz(33),
            ip: sz(75),
            max_raises_per_street: 0,
            all_in_threshold_pct_stack: 100,
            add_all_in: false,
        },
        solver,
        locks: Vec::new(),
    }
}

fn main() {
    let threads: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    for (label, board, iters) in [("river", "Ks7d2c8h3s", 1000u64), ("turn", "Ks7d2c8h", 300), ("flop", "Ks7d2c", 100)]
    {
        let cancel = AtomicBool::new(false);
        // Iterations only (one check at the end).
        let t = Instant::now();
        let s = solve(&spec(board, iters, iters, threads, RakeConfig::default()), &cancel, |_| {}).unwrap();
        let total_one_check = t.elapsed().as_secs_f64();
        let indep = {
            let t = Instant::now();
            let c = s.independent_check();
            (t.elapsed().as_secs_f64(), c.performed)
        };
        // The same with the app's default check frequency (every iters/20, at least 10).
        let every = (iters / 20).max(10);
        let t = Instant::now();
        let _ = solve(&spec(board, iters, every, threads, RakeConfig::default()), &cancel, |_| {}).unwrap();
        let total_checks = t.elapsed().as_secs_f64();
        let checks = iters / every;
        let mut s = s;
        let dir = std::env::temp_dir().join("riverlab-bench");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{label}.gto"));
        let t = Instant::now();
        s.save(path.to_str().unwrap()).unwrap();
        let save = t.elapsed().as_secs_f64();
        let size = std::fs::metadata(&path).unwrap().len();
        let t = Instant::now();
        let _loaded = load(&spec(board, iters, iters, threads, RakeConfig::default()), path.to_str().unwrap()).unwrap();
        let load_s = t.elapsed().as_secs_f64();
        println!(
            "{label:5} {iters} it: solve+1 check {total_one_check:.2}s (includes independent check {:.2}s, done={}) | \
             with {checks} checks {total_checks:.2}s => ~{:.3}s per check | save {save:.2}s ({:.1} MB) | load {load_s:.2}s | mem {:.0} MB",
            indep.0,
            indep.1,
            (total_checks - total_one_check) / (checks.saturating_sub(1).max(1)) as f64,
            size as f64 / 1e6,
            s.outcome.memory_bytes as f64 / 1e6,
        );
    }
}
