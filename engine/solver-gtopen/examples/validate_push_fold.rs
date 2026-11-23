//! GTOpen's heads-up push/fold (preflop) scored in the exact game.
//! Needs the exact equity table: `cargo run --release -p solver-validation --example build_hu_equity`.
//! `cargo run --release -p solver-gtopen --example validate_push_fold -- [table-path]`
use range_engine::{COMBO_COUNT, Combo, HandClass};
use solver_gtopen::{ClassPushFold, gtopen_hu_push_fold};
use solver_validation::push_fold::{ComboEquities, PushFold, evaluate, solve};
use std::collections::BTreeMap;

fn to_combos(c: &ClassPushFold) -> PushFold {
    let lookup = |m: &std::collections::HashMap<String, f64>, combo: Combo| {
        let label = HandClass::of(combo).to_string();
        *m.get(&label).unwrap_or_else(|| panic!("GTOpen has no class {label}"))
    };
    PushFold {
        jam: Combo::all().map(|x| lookup(&c.jam, x)).collect(),
        call: Combo::all().map(|x| lookup(&c.call, x)).collect(),
    }
}

/// Class-level average of a combo-level frequency (each combo weighted equally).
fn by_class(freq: &[f64]) -> BTreeMap<String, f64> {
    let mut acc: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for c in Combo::all() {
        let e = acc.entry(HandClass::of(c).to_string()).or_default();
        e.0 += freq[c.index()];
        e.1 += 1.0;
    }
    acc.into_iter().map(|(k, (s, n))| (k, s / n)).collect()
}

fn pct(freq: &[f64]) -> f64 {
    100.0 * freq.iter().sum::<f64>() / COMBO_COUNT as f64
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "target/hu_combo_equity.bin".into());
    let eq = ComboEquities::load_or_compute(std::path::Path::new(&path)).expect("equity table");
    for stack in [10.0, 20.0] {
        let exact = solve(&eq, stack, 3000);
        let e = evaluate(&eq, stack, &exact);
        println!("== heads-up push/fold, {stack}bb ==");
        println!(
            "exact (RiverLab, combo level):  SB jams {:5.1}%, BB calls {:5.1}%, SB EV {:+.5} bb, exploitability {:.6} bb/hand",
            pct(&exact.jam),
            pct(&exact.call),
            e.sb_ev,
            e.exploitability
        );
        let exact_jam = by_class(&exact.jam);
        let exact_call = by_class(&exact.call);
        for card_removal in [false, true] {
            let g = gtopen_hu_push_fold(stack, card_removal, 20_000, 4000).expect("GTOpen solve");
            let p = to_combos(&g);
            let ge = evaluate(&eq, stack, &p);
            let differ = |ours: &BTreeMap<String, f64>, theirs: &std::collections::HashMap<String, f64>| {
                let mut v: Vec<(String, f64, f64)> = ours
                    .iter()
                    .filter(|(k, o)| (**o - theirs[*k]).abs() > 0.25)
                    .map(|(k, o)| (k.clone(), *o, theirs[k]))
                    .collect();
                v.sort_by(|a, b| (b.1 - b.2).abs().total_cmp(&(a.1 - a.2).abs()));
                v
            };
            let dj = differ(&exact_jam, &g.jam);
            let dc = differ(&exact_call, &g.call);
            println!(
                "GTOpen (blocker model {:>3}):     SB jams {:5.1}%, BB calls {:5.1}%, SB EV {:+.5} bb, exploitability in exact game {:.5} bb/hand",
                if card_removal { "on" } else { "off" },
                pct(&p.jam),
                pct(&p.call),
                ge.sb_ev,
                ge.exploitability
            );
            let show = |v: &[(String, f64, f64)]| {
                v.iter()
                    .take(6)
                    .map(|(k, o, t)| format!("{k} {:.0}%→{:.0}%", o * 100.0, t * 100.0))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!(
                "  classes differing by >25 points: SB {} [{}]  BB {} [{}]",
                dj.len(),
                show(&dj),
                dc.len(),
                show(&dc)
            );
        }
    }
}
