//! Wait for a grade at a new stack depth: graded straight after dealing, and after 1.5 s of
//! "thinking" (the depth is solved in the background from the moment the drill is dealt).
use play_engine::push_fold::{Choice, PushFoldTrainer};
use solver_validation::push_fold::ComboEquities;
use std::sync::Arc;
use std::time::{Duration, Instant};
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "target/hu_combo_equity.bin".into());
    let eq = Arc::new(ComboEquities::load_or_compute(std::path::Path::new(&path)).unwrap());
    let trainer = PushFoldTrainer::new(eq);
    for (think, stack) in [(0u64, 10_000u64), (1500, 15_000), (1500, 20_000)] {
        let d = trainer.drill(stack, stack, None, 7).unwrap();
        std::thread::sleep(Duration::from_millis(think));
        let t = Instant::now();
        trainer.grade(&d, Choice::Play).unwrap();
        println!(
            "{}bb, answered after {think} ms: waited {:.2}s for the grade",
            stack / 1000,
            t.elapsed().as_secs_f64()
        );
    }
}
