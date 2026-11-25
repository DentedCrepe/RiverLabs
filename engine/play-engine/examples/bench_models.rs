//! Times solving the bots' preflop models (cold, no disk cache) and loading them from disk.
//! `cargo run --release -p play-engine --example bench_models [threads] [cache-dir]`
use play_engine::model::{ModelCache, ModelShape};
use poker_core::rake::RakeConfig;
use std::time::Instant;
fn main() {
    let threads: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let eq_dir = std::env::args().nth(2).map(std::path::PathBuf::from);
    let tmp = std::env::temp_dir().join(format!("riverlab-models-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    // Reuse the class-equity table so only the model solve is timed.
    if let Some(d) = &eq_dir {
        for f in std::fs::read_dir(d).unwrap().flatten() {
            if f.file_name().to_string_lossy().starts_with("gtopen_preflop_eq") {
                std::fs::copy(f.path(), tmp.join(f.file_name())).unwrap();
            }
        }
    }
    for players in [2u8, 3, 6] {
        let shape =
            ModelShape { players, stack_bb: 100, small_blind_mbb: 500, ante_mbb: 0, rake: RakeConfig::default() };
        let t = Instant::now();
        ModelCache::new(Some(tmp.clone()), 2 << 30, threads).get(&shape).unwrap();
        let solve = t.elapsed().as_secs_f64();
        let t = Instant::now();
        ModelCache::new(Some(tmp.clone()), 2 << 30, threads).get(&shape).unwrap();
        println!("{players} players: solve {solve:.1}s, load from disk {:.2}s", t.elapsed().as_secs_f64());
    }
    std::fs::remove_dir_all(&tmp).ok();
}
