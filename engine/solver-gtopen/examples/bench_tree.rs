//! Times tree building (what `memory_needed` does) for flop trees of growing size.
use poker_core::{Board, GameConfig};
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use std::time::Instant;
const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";
fn main() {
    for (bets, raises) in [(vec![33], vec![]), (vec![33, 75], vec![100]), (vec![33, 75, 150], vec![60, 100])] {
        let sz = PlayerSizing {
            bet_pct_pot: [bets.clone(), bets.clone(), bets.clone()],
            raise_pct_pot: [raises.clone(), raises.clone(), raises.clone()],
        };
        let spec = SolveSpec {
            game: GameConfig::default(),
            ranges: vec![OOP.into(), IP.into()],
            board: "Ks7d2c".parse::<Board>().unwrap(),
            starting_pot_mbb: 6_500,
            effective_stack_mbb: 97_500,
            tree: TreeAbstraction {
                oop: sz.clone(),
                ip: sz,
                max_raises_per_street: if raises.is_empty() { 0 } else { 2 },
                all_in_threshold_pct_stack: 100,
                add_all_in: false,
            },
            solver: SolverConfig::default(),
            locks: Vec::new(),
        };
        let t = Instant::now();
        let r = solver_gtopen::memory_needed(&spec, 50_000_000);
        println!(
            "bets {bets:?} raises {raises:?}: {:.2}s -> {:?} MB",
            t.elapsed().as_secs_f64(),
            r.map(|b| b / 1_048_576)
        );
    }
}
