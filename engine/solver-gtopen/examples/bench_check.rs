//! Times the independent check's two parts (export, evaluate) on a full-range flop.
use poker_core::rake::RakeConfig;
use poker_core::{Board, GameConfig};
use solver_gtopen::solve;
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use std::sync::atomic::AtomicBool;
use std::time::Instant;
const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";
fn main() {
    let board = std::env::args().nth(1).unwrap_or_else(|| "Ks7d2c".into());
    let iters: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let sz = |b: u32| PlayerSizing { bet_pct_pot: [vec![b], vec![b], vec![b]], raise_pct_pot: Default::default() };
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = iters;
    solver.stopping.target_exploitability_pct_pot = None;
    solver.stopping.exploitability_check_interval = iters;
    solver.threads = 4;
    let spec = SolveSpec {
        game: GameConfig { rake: RakeConfig::default(), ..GameConfig::default() },
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
    };
    let s = solve(&spec, &AtomicBool::new(false), |_| {}).unwrap();
    for _ in 0..2 {
        let t = Instant::now();
        let g = s.postflop_game().unwrap();
        let export = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let e = solver_validation::evaluate(&g).unwrap();
        let eval = t.elapsed().as_secs_f64();
        println!(
            "export {export:.3}s evaluate {eval:.3}s nodes {} | ev {:?} expl {:.6}",
            g.nodes.len(),
            e.ev,
            e.exploitability
        );
    }
}
