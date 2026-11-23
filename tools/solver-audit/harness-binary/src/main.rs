//! Independent check of b-inary/postflop-solver on the same analytically solvable river spot.
use postflop_solver::*;
use std::collections::BTreeMap;

fn class(s: &str) -> String {
    let b = s.as_bytes();
    format!("{}{}", b[0] as char, b[2] as char)
}

fn report(game: &PostFlopGame, label: &str) {
    let player = game.current_player();
    let hands = holes_to_strings(game.private_cards(player)).unwrap();
    let strat = game.strategy();
    let k = hands.len();
    let n_act = game.available_actions().len();
    let mut agg: BTreeMap<String, (f64, Vec<f64>)> = BTreeMap::new();
    for (i, h) in hands.iter().enumerate() {
        let e = agg.entry(class(h)).or_insert((0.0, vec![0.0; n_act]));
        e.0 += 1.0;
        for a in 0..n_act {
            e.1[a] += strat[a * k + i] as f64;
        }
    }
    for (c, (cnt, v)) in &agg {
        let f: Vec<String> = v.iter().map(|x| format!("{:.4}", x / cnt)).collect();
        println!("{label:<10} player {player} {c} ({cnt} combos) {:?}: {f:?}", game.available_actions());
    }
}

fn run(rake_rate: f64) {
    let card_config = CardConfig {
        range: ["AQs,AQo".parse().unwrap(), "KK,43o".parse().unwrap()],
        flop: flop_from_str("KsQd7h").unwrap(),
        turn: card_from_str("7c").unwrap(),
        river: card_from_str("2s").unwrap(),
    };
    let none = BetSizeOptions::try_from(("", "")).unwrap();
    let ip_bet = BetSizeOptions::try_from(("100%", "")).unwrap();
    let tree_config = TreeConfig {
        initial_state: BoardState::River,
        starting_pot: 10,
        effective_stack: 10,
        rake_rate,
        rake_cap: if rake_rate > 0.0 { 1000.0 } else { 0.0 }, // b-inary: cap 0 means "not raked"
        flop_bet_sizes: [none.clone(), none.clone()],
        turn_bet_sizes: [none.clone(), none.clone()],
        river_bet_sizes: [none.clone(), ip_bet],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    let tree = ActionTree::new(tree_config).unwrap();
    let mut game = PostFlopGame::with_config(card_config, tree).unwrap();
    game.allocate_memory(false);
    let expl = solve(&mut game, 4000, 0.0, false);
    println!("== rake {}% ==", rake_rate * 100.0);
    println!("reported exploitability (per-player average, chips) {expl:.5}");
    game.cache_normalized_weights();
    for p in 0..2 {
        let ev = compute_average(&game.expected_values(p), game.normalized_weights(p));
        println!("EV player {p} (pot share) {ev:.4}");
    }
    report(&game, "root");
    game.play(0); // OOP check
    report(&game, "root/0");
    let bet = game.available_actions().iter().position(|a| !matches!(a, Action::Check)).unwrap();
    game.play(bet);
    report(&game, "root/0/bet");
}

fn wide() {
    let card_config = CardConfig {
        range: ["66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".parse().unwrap(), "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".parse().unwrap()],
        flop: flop_from_str("Ks7d2c").unwrap(),
        turn: card_from_str("8h").unwrap(),
        river: card_from_str("3d").unwrap(),
    };
    let none = BetSizeOptions::try_from(("", "")).unwrap();
    let tree_config = TreeConfig {
        initial_state: BoardState::River,
        starting_pot: 100,
        effective_stack: 1000,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [none.clone(), none.clone()],
        turn_bet_sizes: [none.clone(), none.clone()],
        river_bet_sizes: [
            BetSizeOptions::try_from(("50%", "")).unwrap(),
            BetSizeOptions::try_from(("75%", "")).unwrap(),
        ],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    let mut game = PostFlopGame::with_config(card_config, ActionTree::new(tree_config).unwrap()).unwrap();
    game.allocate_memory(false);
    let t = std::time::Instant::now();
    let expl = solve(&mut game, 2000, 0.0, false);
    let secs = t.elapsed().as_secs_f64();
    game.cache_normalized_weights();
    let ev0 = compute_average(&game.expected_values(0), game.normalized_weights(0));
    println!("== wide river ==");
    println!("OOP pot share {ev0:.4}   exploitability (per-player avg, chips) {expl:.5}   {secs:.2}s");
    println!("root actions {:?}", game.available_actions());
    let ev1 = compute_average(&game.expected_values(1), game.normalized_weights(1));
    println!("IP pot share {ev1:.4}");
    println!("live combos OOP {}  IP {}", game.private_cards(0).len(), game.private_cards(1).len());
    for p in 0..2 {
        let hs = holes_to_strings(game.private_cards(p)).unwrap();
        std::fs::write(format!("target/binary_p{p}.txt"), hs.join("\n")).unwrap();
    }
}

fn turn() {
    let card_config = CardConfig {
        range: ["66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".parse().unwrap(), "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".parse().unwrap()],
        flop: flop_from_str("Ks7d2c").unwrap(),
        turn: card_from_str("8h").unwrap(),
        river: NOT_DEALT,
    };
    let none = BetSizeOptions::try_from(("", "")).unwrap();
    let o = BetSizeOptions::try_from(("50%", "")).unwrap();
    let i = BetSizeOptions::try_from(("75%", "")).unwrap();
    let tree_config = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: 1000,
        effective_stack: 10000,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [none.clone(), none],
        turn_bet_sizes: [o.clone(), i.clone()],
        river_bet_sizes: [o, i],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    let mut game = PostFlopGame::with_config(card_config, ActionTree::new(tree_config).unwrap()).unwrap();
    let (mem, _) = game.memory_usage();
    game.allocate_memory(false);
    let t = std::time::Instant::now();
    for it in 0..500 {
        solve_step(&game, it);
    }
    finalize(&mut game);
    let secs = t.elapsed().as_secs_f64();
    let expl = compute_exploitability(&game);
    game.cache_normalized_weights();
    let ev0 = compute_average(&game.expected_values(0), game.normalized_weights(0));
    println!("== turn ==  memory {:.1} MB", mem as f64 / 1_048_576.0);
    println!("OOP pot share {ev0:.4}   exploitability (per-player avg) {expl:.4}   500 iters {secs:.2}s");
}

fn turn_nobet() {
    let card_config = CardConfig {
        range: ["66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".parse().unwrap(), "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".parse().unwrap()],
        flop: flop_from_str("Ks7d2c").unwrap(),
        turn: card_from_str("8h").unwrap(),
        river: NOT_DEALT,
    };
    let none = BetSizeOptions::try_from(("", "")).unwrap();
    let o = BetSizeOptions::try_from(("50%", "")).unwrap();
    let i = BetSizeOptions::try_from(("75%", "")).unwrap();
    let tree_config = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: 1000,
        effective_stack: 10000,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [none.clone(), none],
        turn_bet_sizes: [BetSizeOptions::try_from(("", "")).unwrap(), BetSizeOptions::try_from(("", "")).unwrap()],
        river_bet_sizes: [o, i],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 0.0,
        force_allin_threshold: 0.0,
        merging_threshold: 0.0,
    };
    let mut game = PostFlopGame::with_config(card_config, ActionTree::new(tree_config).unwrap()).unwrap();
    let (mem, _) = game.memory_usage();
    game.allocate_memory(false);
    let t = std::time::Instant::now();
    for it in 0..500 {
        solve_step(&game, it);
    }
    finalize(&mut game);
    let secs = t.elapsed().as_secs_f64();
    let expl = compute_exploitability(&game);
    game.cache_normalized_weights();
    let ev0 = compute_average(&game.expected_values(0), game.normalized_weights(0));
    println!("== turn (no turn betting) ==  memory {:.1} MB", mem as f64 / 1_048_576.0);
    println!("OOP pot share {ev0:.4}   exploitability (per-player avg) {expl:.4}   500 iters {secs:.2}s");
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
    run(0.05);
}
