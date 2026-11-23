//! Prints GTOpen's results next to RiverLab's independent validation for the reference river
//! spots. `cargo run --release -p solver-gtopen --example validate_river`
use poker_core::rake::RakeConfig;
use poker_core::{Board, GameConfig};
use solver_gtopen::solve;
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use solver_validation::evaluate;
use std::sync::atomic::AtomicBool;

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

struct Spot<'a> {
    label: &'a str,
    board: &'a str,
    ranges: [&'a str; 2],
    pot_bb: u64,
    stack_bb: u64,
    oop: &'a [u32],
    ip: &'a [u32],
    rake: RakeConfig,
}

fn run(spot: Spot) {
    let Spot { label, board, ranges, pot_bb, stack_bb, oop, ip, rake } = spot;
    let sz = |b: &[u32]| PlayerSizing { bet_pct_pot: [vec![], vec![], b.to_vec()], raise_pct_pot: Default::default() };
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = 3000;
    solver.stopping.target_exploitability_pct_pot = None;
    solver.stopping.exploitability_check_interval = 3000;
    let spec = SolveSpec {
        game: GameConfig { rake, ..GameConfig::default() },
        ranges: ranges.iter().map(|s| s.to_string()).collect(),
        board: board.parse::<Board>().unwrap(),
        starting_pot_mbb: pot_bb * 1000,
        effective_stack_mbb: stack_bb * 1000,
        tree: TreeAbstraction {
            oop: sz(oop),
            ip: sz(ip),
            max_raises_per_street: 0,
            all_in_threshold_pct_stack: 100,
            add_all_in: false,
        },
        solver,
    };
    let s = solve(&spec, &AtomicBool::new(false), |_| {}).unwrap();
    let ours = evaluate(&s.river_game().unwrap()).unwrap();
    println!("{label}");
    println!(
        "  values (mbb, OOP/IP)    GTOpen {:>10.3} {:>10.3}   RiverLab {:>10.3} {:>10.3}",
        s.outcome.values_mbb[0], s.outcome.values_mbb[1], ours.ev[0], ours.ev[1]
    );
    println!(
        "  exploitability (mbb)    GTOpen {:>10.4}              RiverLab {:>10.4}   ({:.5}% of pot)",
        s.outcome.progress.exploitability_mbb,
        ours.exploitability,
        ours.exploitability / (pot_bb as f64 * 1000.0) * 100.0
    );
}

fn main() {
    let five = RakeConfig::default();
    let toy = |label, rake| Spot {
        label,
        board: "KsQd7h7c2s",
        ranges: ["AQ", "KK,43o"],
        pot_bb: 10,
        stack_bb: 10,
        oop: &[],
        ip: &[100],
        rake,
    };
    let wide = |label, rake| Spot {
        label,
        board: "Ks7d2c8h3d",
        ranges: [OOP, IP],
        pot_bb: 100,
        stack_bb: 1000,
        oop: &[50],
        ip: &[75],
        rake,
    };
    run(toy("toy river, no rake", RakeConfig::none()));
    run(toy("toy river, 5% rake", five));
    run(wide("wide river, no rake", RakeConfig::none()));
    run(wide("wide river, 5% rake", five));
}
