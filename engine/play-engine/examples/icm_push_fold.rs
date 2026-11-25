//! Times ICM push/fold solves and checks the heads-up case against the exact solver.
//! `cargo run --release -p play-engine --example icm_push_fold -- target/hu_combo_equity.bin`
use play_engine::icm_push_fold::{IcmSpot, solve};
use solver_validation::push_fold::{self, ComboEquities};
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "target/hu_combo_equity.bin".into());
    let eq = ComboEquities::load(std::path::Path::new(&path)).expect("equity table");
    // Heads-up, winner takes all: ICM is linear, so this is chip-EV push/fold.
    let hu = IcmSpot { stacks: vec![10_000, 10_000], small_blind: 500, big_blind: 1000, ante: 0, payouts: vec![100] };
    let t = Instant::now();
    let s = solve(&eq, &hu, 1000).unwrap();
    let exact = push_fold::solve(&eq, 10.0, 1000);
    let diff = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max);
    println!(
        "HU 10bb: {:.1}s eps {:.5}% | max |jam diff| {:.4} max |call diff| {:.4}",
        t.elapsed().as_secs_f64(),
        s.epsilon_pct,
        diff(&s.profile.push[0], &exact.jam),
        diff(&s.profile.call[1], &exact.call)
    );
    for (stacks, payouts, iters) in [
        (vec![10u64, 10, 10], vec![50u32, 30, 20], 300usize),
        (vec![15, 5, 10, 10], vec![50, 30, 20], 300),
        (vec![10, 10, 10, 10, 10, 10], vec![65, 35], 200),
    ] {
        let spot = IcmSpot {
            stacks: stacks.iter().map(|s| s * 1000).collect(),
            small_blind: 500,
            big_blind: 1000,
            ante: 0,
            payouts,
        };
        for it in [iters / 2, iters] {
            let t = Instant::now();
            let s = solve(&eq, &spot, it).unwrap();
            let shares: Vec<String> =
                s.profile.push.iter().map(|p| format!("{:.2}", p.iter().sum::<f64>() / 1326.0)).collect();
            println!(
                "{:?} {:?} it {it}: {:.1}s eps {:.4}% push shares {:?}",
                stacks,
                spot.payouts,
                t.elapsed().as_secs_f64(),
                s.epsilon_pct,
                shares
            );
        }
    }
}
