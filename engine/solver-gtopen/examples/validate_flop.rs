//! Full-range flop spot: GTOpen vs RiverLab's (fast) independent check, with timings.
//! `cargo run --release -p solver-gtopen --example validate_flop`
use poker_core::rake::RakeConfig;
use poker_core::{Board, GameConfig};
use solver_gtopen::solve;
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use solver_validation::evaluate;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

fn main() {
    for (label, board, rake, raises) in [
        ("flop Ks7d2c, no rake, no raises", "Ks7d2c", RakeConfig::none(), false),
        ("flop Ks7d2c, 5% rake, one raise size", "Ks7d2c", RakeConfig::default(), true),
        ("flop Qh8h3h (monotone, suit-isomorphic), no rake", "Qh8h3h", RakeConfig::none(), false),
    ] {
        let sz = |b: u32| PlayerSizing {
            bet_pct_pot: [vec![b], vec![b], vec![b]],
            raise_pct_pot: if raises { [vec![100], vec![100], vec![100]] } else { Default::default() },
        };
        let mut solver = SolverConfig::default();
        solver.stopping.max_iterations = 300;
        solver.stopping.target_exploitability_pct_pot = None;
        solver.stopping.exploitability_check_interval = 300;
        solver.threads = 4;
        let spec = SolveSpec {
            game: GameConfig { rake, ..GameConfig::default() },
            ranges: vec![OOP.into(), IP.into()],
            board: board.parse::<Board>().unwrap(),
            starting_pot_mbb: 6_500,
            effective_stack_mbb: 97_500,
            tree: TreeAbstraction {
                oop: sz(33),
                ip: sz(75),
                max_raises_per_street: if raises { 1 } else { 0 },
                all_in_threshold_pct_stack: 100,
                add_all_in: false,
            },
            solver,
            locks: Vec::new(),
        };
        let t = Instant::now();
        let s = solve(&spec, &AtomicBool::new(false), |_| {}).unwrap();
        let solve_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let g = s.postflop_game().unwrap();
        let ours = evaluate(&g).unwrap();
        let check_s = t.elapsed().as_secs_f64();
        println!("{label}  ({} nodes, {:.0} MB)", g.nodes.len(), s.outcome.memory_bytes as f64 / 1048576.0);
        println!(
            "  values (mbb)          GTOpen {:>10.3} {:>10.3}   RiverLab {:>10.3} {:>10.3}",
            s.outcome.values_mbb[0], s.outcome.values_mbb[1], ours.ev[0], ours.ev[1]
        );
        println!(
            "  exploitability (mbb)  GTOpen {:>10.4}              RiverLab {:>10.4}",
            s.outcome.progress.exploitability_mbb, ours.exploitability
        );
        println!("  time: solve {solve_s:.1}s (300 it), independent check {check_s:.1}s");
    }
}
