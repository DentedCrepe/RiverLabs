//! Builds (or loads) the exact heads-up combo-vs-combo equity table.
//! `cargo run --release -p solver-validation --example build_hu_equity -- <path>`
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "target/hu_combo_equity.bin".into());
    let t = std::time::Instant::now();
    let table = solver_validation::push_fold::ComboEquities::load_or_compute(std::path::Path::new(&path)).unwrap();
    let aa = range_engine::Combo::from("AsAh".parse::<poker_core::HoleCards>().unwrap()).index();
    let kk = range_engine::Combo::from("KsKh".parse::<poker_core::HoleCards>().unwrap()).index();
    println!("ready in {:?}; AsAh vs KsKh = {:.6}", t.elapsed(), table.get(aa, kk));
}
