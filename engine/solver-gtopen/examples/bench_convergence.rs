//! Exploitability against solving time (checks excluded) for algorithms, storage modes and
//! thread counts, on a full-range flop.
//! `cargo run --release -p solver-gtopen --example bench_convergence [seconds]`
use gtopen::{Algorithm, Solver, Spot, Storage};
use poker_core::rake::RakeConfig;
use poker_core::{Board, GameConfig};
use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
use std::sync::Arc;
use std::time::Instant;

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

fn main() {
    let secs: f64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30.0);
    let board = std::env::args().nth(2).unwrap_or_else(|| "Ks7d2c".into());
    let sz = |b: u32| PlayerSizing { bet_pct_pot: [vec![b], vec![b], vec![b]], raise_pct_pot: Default::default() };
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
        solver: SolverConfig::default(),
        locks: Vec::new(),
    };
    let prepared = solver_gtopen::prepare(&spec).unwrap();
    let spot = Arc::new(Spot::new_with_limit(prepared.config.clone(), Some(50_000_000)).unwrap());
    let pot = 6500.0;
    let configs: Vec<(&str, Algorithm, Storage, usize)> = vec![
        ("dcfr f32 4t", Algorithm::Dcfr, Storage::F32, 4),
        ("pcfr+ f32 4t", Algorithm::PcfrPlus, Storage::F32, 4),
        ("cfr+ f32 4t", Algorithm::CfrPlus, Storage::F32, 4),
        ("dcfr i16 4t", Algorithm::Dcfr, Storage::Compressed, 4),
        ("dcfr f32 2t", Algorithm::Dcfr, Storage::F32, 2),
        ("dcfr f32 1t", Algorithm::Dcfr, Storage::F32, 1),
    ];
    let only = std::env::args().nth(3);
    for (label, algo, storage, threads) in configs {
        if only.as_deref().is_some_and(|o| !label.contains(o)) {
            continue;
        }
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        let mut s = Solver::with_storage(spot.clone(), storage);
        s.algo = algo;
        let mut solving = 0.0f64;
        let mut line = String::new();
        let mut next_check = 10;
        while solving < secs {
            let t = Instant::now();
            pool.install(|| s.iterate());
            solving += t.elapsed().as_secs_f64();
            if s.iteration as u64 >= next_check {
                let e = pool.install(|| s.exploitability());
                line += &format!(" [{}it {:.1}s {:.3}%]", s.iteration, solving, e / pot * 100.0);
                next_check = (next_check as f64 * 1.6) as u64;
            }
        }
        let e = pool.install(|| s.exploitability());
        println!(
            "{label:14} it/s {:.2} final {:.3}% pot at {}it |{line}",
            s.iteration as f64 / solving,
            e / pot * 100.0,
            s.iteration
        );
    }
}
