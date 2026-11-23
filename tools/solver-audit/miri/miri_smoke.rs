//! RiverLab test added to a scratch copy of GTOpen (not upstream, not shipped). It exercises the
//! paths RiverLab uses (postflop DCFR/CFR+ with rake and chance nodes, exploitability, save/load,
//! and the preflop solver with its blocker model) on games small enough for Miri.
//! Run by `tools/solver-audit/miri/run-miri.sh`.
use solver::tree::{StreetSizing, TreeConfig};
use solver::{Algorithm, Solver, Spot, SpotConfig};
use std::sync::Arc;

fn tree(oop_bet: Vec<solver::tree::BetSize>, ip_bet: Vec<solver::tree::BetSize>, rake: f64) -> TreeConfig {
    let st = |b: Vec<solver::tree::BetSize>, donk: bool| StreetSizing {
        bet: b.clone(),
        raise: b.clone(),
        donk: if donk { b } else { Vec::new() },
    };
    TreeConfig {
        starting_pot: 10.0,
        effective_stack: 10.0,
        rake_pct: rake,
        rake_cap: 0.0,
        oop: [st(oop_bet.clone(), true), st(oop_bet.clone(), true), st(oop_bet, true)],
        ip: [st(ip_bet.clone(), false), st(ip_bet.clone(), false), st(ip_bet, false)],
        allin_threshold: 1.0,
        add_allin: false,
        max_raises: 1,
        carry_aggressor_through_checks: None,
        icm: None,
    }
}

fn run(board: &str, oop: &str, ip: &str, rake: f64, algo: Algorithm, iters: u32) {
    let pot = solver::tree::BetSize::PotPct(100.0);
    let cfg = SpotConfig {
        board: board.into(),
        range_oop: oop.into(),
        range_ip: ip.into(),
        tree: tree(vec![pot.clone()], vec![pot], rake),
    };
    let spot = Spot::new(cfg).expect("spot");
    let mut s = Solver::new(Arc::new(spot));
    s.algo = algo;
    for _ in 0..iters {
        s.iterate();
    }
    let e = s.exploitability();
    assert!(e.is_finite());
    let dir = std::env::temp_dir().join(format!("miri-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{board}.gto"));
    s.save(path.to_str().unwrap()).expect("save");
    let loaded = Solver::load(path.to_str().unwrap()).expect("load");
    assert_eq!(loaded.iteration, s.iteration);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn river_dcfr_raked() {
    run("KsQd7h7c2s", "AhQh,AcQc", "KhKd,4c3d", 0.05, Algorithm::Dcfr, 3);
}

#[test]
fn turn_with_chance_nodes() {
    run("KsQd7h2s", "AhQh", "KhKd,4c3d", 0.0, Algorithm::CfrPlus, 2);
}

#[test]
fn preflop_heads_up_with_blockers() {
    use solver::preflop::equity::{EquityTable, NUM_CLASSES};
    use solver::preflop::{PreflopConfig, PreflopSolver};
    // A synthetic, antisymmetric equity table keeps Miri fast; values are irrelevant here.
    let mut t = vec![0.5f32; NUM_CLASSES * NUM_CLASSES];
    for i in 0..NUM_CLASSES {
        for j in 0..NUM_CLASSES {
            t[i * NUM_CLASSES + j] = 0.5 + (j as f32 - i as f32) / (4.0 * NUM_CLASSES as f32);
        }
    }
    let dir = std::env::temp_dir().join(format!("miri-smoke-pf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let table = dir.join("eq.bin");
    let mut bytes = 1u32.to_le_bytes().to_vec();
    t.iter().for_each(|v| bytes.extend_from_slice(&v.to_le_bytes()));
    std::fs::write(&table, bytes).unwrap();
    let eq = Arc::new(EquityTable::load_or_build(table.to_str().unwrap(), 1));
    let cfg = PreflopConfig {
        bb_ante: 0.0, leaf_net: None, card_removal: true, limper_extra: 0.0, caller_extra: 0.0,
        raise_mults_oop: None, fourbet_mults_oop: None, utg_straddle: false,
        positions: vec!["SB".into(), "BB".into()], stack: 20.0, stacks: None, posts: vec![0.5, 1.0],
        ante: 0.0, limp: false, open_raises: vec![], raise_mults: vec![], max_raises: 1,
        add_allin: true, allin_threshold: 0.85, rake_pct: 5.0, rake_cap: 3.0, no_flop_no_drop: true,
        realization: "static".into(), call_only_seats: vec![], open_raises_by_seat: None,
        fourbet_mults: None, fourbet_mults_by_seat: None, raise_mults_by_seat: None, icm: None,
    };
    let mut s = PreflopSolver::new(cfg, eq.clone()).expect("preflop");
    s.iterate();
    let v = s.node_view(&[1]).expect("view");
    assert!(!v.actions.is_empty());
    let path = dir.join("pf.gtp");
    s.save_game(path.to_str().unwrap()).expect("save");
    let back = PreflopSolver::load_game(path.to_str().unwrap(), eq).expect("load");
    assert_eq!(back.average_strategy(0), s.average_strategy(0));
    let _ = std::fs::remove_dir_all(&dir);
}
