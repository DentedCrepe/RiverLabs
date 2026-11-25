//! Progress of the 6-player bot model solve: iterations, the model's own gap, time.
use play_engine::model::ModelShape;
use poker_core::rake::RakeConfig;
use solver_gtopen::preflop::{equity_table, solve_preflop};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
fn main() {
    let threads: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let interval: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(25);
    let dir = std::path::PathBuf::from("/home/user/riverlabs/backend/.cache");
    let shape =
        ModelShape { players: 6, stack_bb: 100, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::default() };
    let mut spec = shape.spec();
    spec.threads = threads;
    spec.max_seconds = Some(120);
    spec.max_iterations = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(400);
    spec.check_interval = interval;
    let eq = equity_table(Some(&dir));
    let t = Instant::now();
    let s = solve_preflop(&spec, eq, 2 << 30, Arc::new(AtomicBool::new(false)), |p| {
        println!("{:>4} it  gap {:8.2} mbb/hand  {:5.1}s", p.iterations, p.model_gap_mbb, t.elapsed().as_secs_f64());
    })
    .unwrap();
    println!("done {:?} in {:.1}s, {} nodes", s.outcome.stop_reason, t.elapsed().as_secs_f64(), s.outcome.nodes);
}
