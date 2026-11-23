//! RiverLabs independent check of GTOpen on the same spots used for b-inary and ucsandman
//! (docs/solver-comparison.md). Not part of GTOpen.
use solver::cards::{rank, RANK_CHARS};
use solver::game::Dealt;
use solver::tree::{parse_sizes, StreetSizing, TreeConfig, KIND_ACTION};
use solver::{Solver, Spot, SpotConfig};
use std::collections::BTreeMap;
use std::sync::Arc;

fn sz(bet: &str, donk: &str) -> StreetSizing {
    StreetSizing { bet: parse_sizes(bet).unwrap(), raise: vec![], donk: parse_sizes(donk).unwrap() }
}

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

/// Average per-hand value for each player (chips, GTOpen's convention) and exploitability.
fn values(s: &Solver) -> ([f64; 2], f64) {
    let spot = &*s.spot;
    let denom = s.pair_weight_sum();
    let mut v = [0f64; 2];
    for p in 0..2 {
        let cfv = s.traverse_avg(0, p, &spot.weights[1 - p], Dealt::default());
        v[p] = spot.weights[p].iter().zip(&cfv).map(|(&w, &x)| w as f64 * x as f64).sum::<f64>() / denom;
    }
    (v, s.exploitability())
}

fn class(c1: u8, c2: u8) -> String {
    let (a, b) = (rank(c1), rank(c2));
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    format!("{}{}", RANK_CHARS[hi as usize], RANK_CHARS[lo as usize])
}

fn walk(s: &Solver, idx: u32, path: &str, depth: usize) {
    let tree = &s.spot.tree;
    let node = &tree.nodes[idx as usize];
    if node.kind != KIND_ACTION || depth > 3 {
        return;
    }
    let p = node.player as usize;
    let na = node.num_children as usize;
    let sigma = s.average_strategy(idx, node);
    let hands = &s.spot.hands[p];
    let nh = hands.len();
    let mut agg: BTreeMap<String, (f64, Vec<f64>)> = BTreeMap::new();
    for (i, h) in hands.iter().enumerate() {
        let e = agg.entry(class(h.c1, h.c2)).or_insert((0.0, vec![0.0; na]));
        e.0 += 1.0;
        for a in 0..na {
            e.1[a] += sigma[a * nh + i] as f64;
        }
    }
    for (c, (n, v)) in &agg {
        let f: Vec<String> = v.iter().map(|x| format!("{:.4}", x / n)).collect();
        println!("{path:<10} player {p} {c} ({n} combos): {f:?}");
    }
    for a in 0..na {
        let child = tree.children[node.children_start as usize + a];
        walk(s, child, &format!("{path}/{a}"), depth + 1);
    }
}

fn toy(rake: f64) {
    let none = sz("", "");
    let cfg = SpotConfig {
        board: "KsQd7h7c2s".into(),
        range_oop: "AQ".into(),
        range_ip: "KK,43o".into(),
        tree: TreeConfig {
            starting_pot: 10.0,
            effective_stack: 10.0,
            rake_pct: rake,
            rake_cap: 0.0,
            oop: [none.clone(), none.clone(), none.clone()],
            ip: [none.clone(), none.clone(), sz("100", "")],
            ..Default::default()
        },
    };
    let mut s = Solver::new(Arc::new(Spot::new(cfg).unwrap()));
    for _ in 0..4000 {
        s.iterate();
    }
    println!("== toy river, rake {}% ==", rake * 100.0);
    walk(&s, 0, "root", 0);
    let (v, e) = values(&s);
    println!("values OOP {:.4} IP {:.4}   exploitability {e:.5}", v[0], v[1]);
}

fn spot(board: &str, pot: f64, stack: f64, turn_bets: bool, iters: usize, label: &str) {
    let none = sz("", "");
    let (oop_turn, ip_turn) = if turn_bets { (sz("50", ""), sz("75", "")) } else { (none.clone(), none.clone()) };
    let cfg = SpotConfig {
        board: board.into(),
        range_oop: OOP.into(),
        range_ip: IP.into(),
        tree: TreeConfig {
            starting_pot: pot,
            effective_stack: stack,
            oop: [none.clone(), oop_turn, sz("50", "50")],
            ip: [none.clone(), ip_turn, sz("75", "")],
            allin_threshold: 1.0,
            add_allin: false,
            max_raises: 0,
            ..Default::default()
        },
    };
    let mut s = Solver::new(Arc::new(Spot::new(cfg).unwrap()));
    let t = std::time::Instant::now();
    for _ in 0..iters {
        s.iterate();
    }
    let secs = t.elapsed().as_secs_f64();
    let (v, e) = values(&s);
    println!(
        "== {label} ==  combos OOP {} IP {}  arena {:.1} MB\nvalues OOP {:.4} IP {:.4}   exploitability (avg) {e:.5}   {iters} iters {secs:.2}s",
        s.spot.hands[0].len(),
        s.spot.hands[1].len(),
        s.arena_bytes() as f64 / 1_048_576.0,
        v[0],
        v[1]
    );
}

fn main() {
    toy(0.0);
    toy(0.05);
    spot("Ks7d2c8h3d", 100.0, 1000.0, false, 2000, "wide river");
    spot("Ks7d2c8h", 1000.0, 10000.0, false, 500, "turn (no turn betting)");
    spot("Ks7d2c8h", 1000.0, 10000.0, true, 500, "turn");
}
