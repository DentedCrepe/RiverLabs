//! Independent check of ucsandman/postflop on an analytically solvable river spot.
use engine::cards::{rank, RANK_CHARS};
use engine::cfr::{DcfrParams, Solver};
use engine::config::SolveConfig;
use engine::game::{Game, NodeInfo};
use engine::nlhe::NlheGame;
use std::collections::BTreeMap;

fn class(h: (u8, u8)) -> String {
    let (a, b) = (rank(h.0), rank(h.1));
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    format!("{}{}", RANK_CHARS[hi as usize], RANK_CHARS[lo as usize])
}

fn config(rake: f64) -> SolveConfig {
    let toml = format!(
        r#"
board = "Ks Qd 7h 7c 2s"
oop_range = "AQ"
ip_range = "KK,43o"
effective_stack = 10.0
starting_pot = 10.0
raise_cap = 0
max_iterations = 4000
target_exploitability = 0.0
[rake]
percent = {rake}
cap = 0.0
[sizings.ip.river]
bet = {{ percents = [100.0] }}
"#
    );
    SolveConfig::from_toml_str(&toml).expect("config")
}

fn run(rake: f64) {
    let cfg = config(rake);
    let game = NlheGame::new(&cfg).expect("game");
    let mut s = Solver::new(game);
    s.run(4000, &DcfrParams::default(), 0, |_, _, _| {});
    let g = s.game();
    println!("== rake {rake}% ==");
    // Walk the tree, report average strategy per hand class at each decision node.
    let mut stack = vec![(g.root(), String::from("root"))];
    while let Some((n, path)) = stack.pop() {
        if let NodeInfo::Decision { player, num_actions } = g.node(n) {
            let strat = s.average_strategy(n);
            let hands = g.live_combos(n, player);
            let k = hands.len();
            let mut agg: BTreeMap<String, (f64, Vec<f64>)> = BTreeMap::new();
            for (i, h) in hands.iter().enumerate() {
                let e = agg.entry(class((h.0, h.1))).or_insert((0.0, vec![0.0; num_actions]));
                e.0 += 1.0;
                for a in 0..num_actions {
                    e.1[a] += strat[a * k + i] as f64;
                }
            }
            for (c, (cnt, v)) in &agg {
                let freqs: Vec<String> = v.iter().map(|x| format!("{:.4}", x / cnt)).collect();
                println!("{path:<12} player {player} {c} ({cnt} combos): actions {freqs:?}");
            }
            for a in 0..num_actions {
                stack.push((g.child(n, a), format!("{path}/{a}")));
            }
        }
    }
    let r = s.exploitability();
    let ev = [s.expected_value(0), s.expected_value(1)];
    let nashconv = (r.br[0] - ev[0]) + (r.br[1] - ev[1]);
    println!("EV net chips  OOP {:.4}  IP {:.4}", ev[0], ev[1]);
    println!("IP pot share  {:.4}", g.ev_pot_share(1, ev[1]));
    println!("reported exploitability (chips) {:.5}", r.chips);
    println!("true NashConv (gain0 + gain1)   {:.5}", nashconv);
}

fn wide() {
    let toml = r#"
board = "Ks 7d 2c 8h 3d"
oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s"
ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+"
effective_stack = 1000.0
starting_pot = 100.0
raise_cap = 0
allin_threshold = 100.0
max_iterations = 2000
target_exploitability = 0.0
[sizings.oop.river]
bet = { percents = [50.0] }
[sizings.ip.river]
bet = { percents = [75.0] }
"#;
    let cfg = SolveConfig::from_toml_str(toml).expect("config");
    let game = NlheGame::new(&cfg).expect("game");
    let t = std::time::Instant::now();
    let mut s = Solver::new(game);
    s.run(2000, &DcfrParams::default(), 0, |_, _, _| {});
    let secs = t.elapsed().as_secs_f64();
    let r = s.exploitability();
    let ev0 = s.expected_value(0);
    println!("== wide river ==");
    let g = s.game();
    let w0: f32 = g.root_weights(0).iter().sum();
    let w1: f32 = g.root_weights(1).iter().sum();
    println!("live combos OOP {} (weight {w0})  IP {} (weight {w1})", g.combo_count(g.root(), 0), g.combo_count(g.root(), 1));
    println!("IP pot share {:.4}", g.ev_pot_share(1, s.expected_value(1)));
    println!("OOP pot share {:.4}   exploitability (sum, chips) {:.5}   {secs:.2}s", s.game().ev_pot_share(0, ev0), r.chips);
}

fn turn() {
    let toml = r#"
board = "Ks 7d 2c 8h"
oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s"
ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+"
effective_stack = 10000.0
starting_pot = 1000.0
raise_cap = 0
allin_threshold = 100.0
max_iterations = 500
target_exploitability = 0.0
[sizings.oop.turn]
bet = { percents = [50.0] }
[sizings.ip.turn]
bet = { percents = [75.0] }
[sizings.oop.river]
bet = { percents = [50.0] }
donk = { percents = [50.0] }
[sizings.ip.river]
bet = { percents = [75.0] }
"#;
    let cfg = SolveConfig::from_toml_str(toml).expect("config");
    let game = NlheGame::new(&cfg).expect("game");
    let mut s = Solver::new(game);
    let mb = s.storage_entries() as f64 * 8.0 / 1_048_576.0;
    let t = std::time::Instant::now();
    s.run(500, &DcfrParams::default(), 0, |_, _, _| {});
    let secs = t.elapsed().as_secs_f64();
    let r = s.exploitability();
    let ev0 = s.expected_value(0);
    println!("== turn ==  storage {mb:.1} MB");
    println!("OOP pot share {:.4}   exploitability (sum) {:.4}   500 iters {secs:.2}s", s.game().ev_pot_share(0, ev0), r.chips);
}

fn turn_nobet() {
    let toml = r#"
board = "Ks 7d 2c 8h"
oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s"
ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+"
effective_stack = 10000.0
starting_pot = 1000.0
raise_cap = 0
allin_threshold = 100.0
max_iterations = 500
target_exploitability = 0.0
[sizings.oop.river]
bet = { percents = [50.0] }
[sizings.ip.river]
bet = { percents = [75.0] }
"#;
    let cfg = SolveConfig::from_toml_str(toml).expect("config");
    let game = NlheGame::new(&cfg).expect("game");
    let mut s = Solver::new(game);
    let mb = s.storage_entries() as f64 * 8.0 / 1_048_576.0;
    let t = std::time::Instant::now();
    s.run(500, &DcfrParams::default(), 0, |_, _, _| {});
    let secs = t.elapsed().as_secs_f64();
    let r = s.exploitability();
    let ev0 = s.expected_value(0);
    println!("== turn (no turn betting) ==  storage {mb:.1} MB");
    println!("OOP pot share {:.4}   exploitability (sum) {:.4}   500 iters {secs:.2}s", s.game().ev_pot_share(0, ev0), r.chips);
}

fn main() {
    if std::env::args().any(|a| a == "turn_nobet") {
        turn_nobet();
        return;
    }
    if std::env::args().any(|a| a == "turn") {
        turn();
        return;
    }
    if std::env::args().any(|a| a == "wide") {
        wide();
        return;
    }
    run(0.0);
    run(5.0);
}
