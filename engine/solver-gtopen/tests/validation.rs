//! GTOpen, run through the adapter, checked by RiverLab's independent validator
//! (`solver-validation`) and against hand-derived equilibria. Milestone 2 evidence.

use poker_core::rake::{RakeAmount, RakeConfig};
use poker_core::{Board, GameConfig};
use solver_gtopen::{AdapterError, StopReason, solve};
use solver_interface::{CfrVariant, PlayerSizing, SamplingScheme, SolveSpec, SolverConfig, TreeAbstraction};
use solver_validation::evaluate;
use std::sync::atomic::AtomicBool;

fn sizing(bet: &[u32]) -> PlayerSizing {
    PlayerSizing { bet_pct_pot: [vec![], vec![], bet.to_vec()], raise_pct_pot: [vec![], vec![], vec![]] }
}

fn spec(
    board: &str,
    ranges: [&str; 2],
    pot_bb: u64,
    stack_bb: u64,
    oop: &[u32],
    ip: &[u32],
    rake: RakeConfig,
) -> SolveSpec {
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = 3000;
    solver.stopping.target_exploitability_pct_pot = None;
    solver.stopping.exploitability_check_interval = 500;
    solver.threads = 2;
    SolveSpec {
        game: GameConfig { rake, ..GameConfig::default() },
        ranges: ranges.iter().map(|s| s.to_string()).collect(),
        board: board.parse::<Board>().unwrap(),
        starting_pot_mbb: pot_bb * 1000,
        effective_stack_mbb: stack_bb * 1000,
        tree: TreeAbstraction {
            oop: sizing(oop),
            ip: sizing(ip),
            max_raises_per_street: 0,
            all_in_threshold_pct_stack: 100,
            add_all_in: false,
        },
        solver,
    }
}

fn toy(rake: RakeConfig) -> SolveSpec {
    // Hand-solved spot (docs/solver-comparison.md): pot 10bb, IP may bet pot (all-in).
    spec("KsQd7h7c2s", ["AQ", "KK,43o"], 10, 10, &[], &[100], rake)
}

fn five_pct() -> RakeConfig {
    RakeConfig { no_flop_no_drop: true, ..RakeConfig::default() }
}

#[test]
fn toy_spot_matches_the_hand_solution_and_independent_check() {
    let s = solve(&toy(RakeConfig::none()), &AtomicBool::new(false), |_| {}).unwrap();
    // IP pot share 3bb => net 3bb - 5bb = -2bb = -2000 mbb.
    assert!((s.outcome.values_mbb[1] + 2000.0).abs() < 1.0, "{:?}", s.outcome.values_mbb);
    let game = s.postflop_game().unwrap();
    let ours = evaluate(&game).unwrap();
    assert!((ours.ev[1] - s.outcome.values_mbb[1]).abs() < 0.5, "{ours:?}");
    assert!(ours.exploitability < 1.0, "independent exploitability {} mbb", ours.exploitability);
    // IP node after OOP's check: bluff 12.5% of air, bet all KK.
    if let solver_validation::GameNode::Decision { strategy, .. } = &game.nodes[1] {
        let n = game.ranges[1].len();
        let (mut air, mut kk) = (vec![], vec![]);
        for (i, (c, _)) in game.ranges[1].iter().enumerate() {
            let bet = strategy[n + i];
            if c.hole_cards().high().rank() == poker_core::Rank::King { kk.push(bet) } else { air.push(bet) }
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        assert!((mean(&kk) - 1.0).abs() < 1e-3 && (mean(&air) - 0.125).abs() < 2e-3, "{} {}", mean(&kk), mean(&air));
    } else {
        panic!("node 1 should be IP's decision");
    }
    assert!(s.outcome.approximations.is_empty());
    assert_eq!(s.outcome.backend_commit, solver_gtopen::GTOPEN_BUILD);
}

#[test]
fn raked_toy_spot_is_validated_with_the_rake_rule() {
    let s = solve(&toy(five_pct()), &AtomicBool::new(false), |_| {}).unwrap();
    let ours = evaluate(&s.postflop_game().unwrap()).unwrap();
    // IP pot share 2.7769bb => net -2.2231bb.
    assert!((s.outcome.values_mbb[1] + 2223.1).abs() < 1.0, "{:?}", s.outcome.values_mbb);
    assert!((ours.ev[1] - s.outcome.values_mbb[1]).abs() < 0.5, "{ours:?}");
    assert!(ours.exploitability < 1.0 && ours.exploitability > -1e-6, "{ours:?}");
    assert!((ours.exploitability - s.outcome.progress.exploitability_mbb).abs() < 0.5);
    assert_eq!(s.outcome.approximations.len(), 1, "rake rounding is reported");
}

const OOP: &str = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
const IP: &str = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

#[test]
fn wide_river_value_and_exploitability_agree_with_independent_check() {
    for rake in [RakeConfig::none(), five_pct()] {
        let raked = rake.percentage_bps > 0;
        let s = solve(&spec("Ks7d2c8h3d", [OOP, IP], 100, 1000, &[50], &[75], rake), &AtomicBool::new(false), |_| {})
            .unwrap();
        let ours = evaluate(&s.postflop_game().unwrap()).unwrap();
        if !raked {
            // Value cross-checked by three independent solvers: OOP pot share 53.6726%.
            assert!((s.outcome.values_mbb[0] - 3672.6).abs() < 1.0, "{:?}", s.outcome.values_mbb);
        }
        for p in 0..2 {
            assert!(
                (ours.ev[p] - s.outcome.values_mbb[p]).abs() < 0.5,
                "rake {raked}: {ours:?} vs {:?}",
                s.outcome.values_mbb
            );
        }
        // Same profile, two independent exploitability computations.
        assert!(
            (ours.exploitability - s.outcome.progress.exploitability_mbb).abs() < 0.5,
            "rake {raked}: ours {} vs GTOpen {}",
            ours.exploitability,
            s.outcome.progress.exploitability_mbb
        );
        assert!(ours.exploitability < 0.001 * 100_000.0, "below 0.1% of pot");
    }
}

#[test]
fn unsupported_settings_are_rejected_not_approximated() {
    let base = toy(RakeConfig::none());
    let mut mc = base.clone();
    mc.solver.variant = CfrVariant::MonteCarlo { sampling: SamplingScheme::External };
    mc.solver.seed = Some(1);
    assert!(matches!(solve(&mc, &AtomicBool::new(false), |_| {}), Err(AdapterError::Unsupported(_))));
    let mut min_pot = base.clone();
    min_pot.game.rake = RakeConfig { min_pot: Some(RakeAmount::BigBlinds { mbb: 50_000 }), ..five_pct() };
    assert!(matches!(solve(&min_pot, &AtomicBool::new(false), |_| {}), Err(AdapterError::Unsupported(_))));
    let mut cap = base.clone();
    cap.game.rake = RakeConfig { cap: Some(RakeAmount::Currency { minor_units: 300 }), ..five_pct() };
    assert!(matches!(solve(&cap, &AtomicBool::new(false), |_| {}), Err(AdapterError::InvalidSpec(_))));
}

#[test]
fn cancellation_stops_the_solve() {
    let s = solve(&toy(RakeConfig::none()), &AtomicBool::new(true), |_| {}).unwrap();
    assert_eq!(s.outcome.stop_reason, StopReason::Cancelled);
}

#[test]
fn outcome_serializes_with_provenance() {
    let s = solve(&toy(RakeConfig::none()), &AtomicBool::new(false), |_| {}).unwrap();
    let json = serde_json::to_value(&s.outcome).unwrap();
    assert_eq!(json["backend"], "gtopen");
    assert_eq!(json["algorithm"], gtopen_algorithm_name());
    assert_eq!(json["stop_reason"], "max_iterations");
}

fn gtopen_algorithm_name() -> &'static str {
    gtopen::Algorithm::Dcfr.name()
}

fn turn_spec(board: &str, rake: RakeConfig, iterations: u64) -> SolveSpec {
    let sz = |turn: u32, river: u32| PlayerSizing {
        bet_pct_pot: [vec![], vec![turn], vec![river]],
        raise_pct_pot: [vec![], vec![], vec![]],
    };
    let mut s = spec(board, [OOP, IP], 100, 1000, &[], &[], rake);
    s.tree.oop = sz(50, 50);
    s.tree.ip = sz(75, 75);
    s.solver.stopping.max_iterations = iterations;
    s.solver.stopping.exploitability_check_interval = iterations;
    s
}

fn check_turn(spec: &SolveSpec) -> (f64, f64) {
    let s = solve(spec, &AtomicBool::new(false), |_| {}).unwrap();
    let g = s.postflop_game().unwrap();
    assert!(g.nodes.iter().any(|n| matches!(n, solver_validation::GameNode::Chance { .. })));
    let ours = evaluate(&g).unwrap();
    for p in 0..2 {
        assert!((ours.ev[p] - s.outcome.values_mbb[p]).abs() < 0.5, "{ours:?} vs {:?}", s.outcome.values_mbb);
    }
    assert!(
        (ours.exploitability - s.outcome.progress.exploitability_mbb).abs() < 0.5,
        "ours {} vs GTOpen {}",
        ours.exploitability,
        s.outcome.progress.exploitability_mbb
    );
    (ours.ev[0], ours.exploitability)
}

#[test]
fn turn_spot_with_river_runouts_is_validated_independently() {
    let (oop_ev, expl) = check_turn(&turn_spec("Ks7d2c8h", RakeConfig::none(), 1000));
    // Three independent solvers agreed on OOP ≈ +6.148% of the pot for this spot (solver-comparison.md).
    assert!((oop_ev - 6147.8).abs() < 5.0, "OOP value {oop_ev}");
    assert!(expl < 0.001 * 100_000.0, "exploitability {expl} mbb");
}

#[test]
fn raked_turn_spot_is_validated_independently() {
    check_turn(&turn_spec("Ks7d2c8h", five_pct(), 1000));
}

#[test]
fn suit_isomorphic_runouts_are_validated_independently() {
    // Clubs and hearts are interchangeable on this board, so GTOpen solves one of each pair of
    // runouts and copies it with suits relabelled; every copied subtree is checked here.
    check_turn(&turn_spec("Ks7s2d8d", RakeConfig::none(), 1000));
}

#[test]
fn corrupting_one_river_decision_is_detected() {
    let s = solve(&turn_spec("Ks7d2c8h", RakeConfig::none(), 1000), &AtomicBool::new(false), |_| {}).unwrap();
    let mut g = s.postflop_game().unwrap();
    let baseline = evaluate(&g).unwrap().exploitability;
    // Find the first decision below a chance node (a river decision) and make it play its
    // first action with every combo.
    let first_chance = g.nodes.iter().position(|n| matches!(n, solver_validation::GameNode::Chance { .. })).unwrap();
    let target = (first_chance + 1..g.nodes.len())
        .find(|&k| matches!(&g.nodes[k], solver_validation::GameNode::Decision { children, .. } if children.len() > 1))
        .unwrap();
    if let solver_validation::GameNode::Decision { player, children, strategy } = &mut g.nodes[target] {
        let n = g.ranges[*player].len();
        for a in 0..children.len() {
            for i in 0..n {
                strategy[a * n + i] = if a == 0 { 1.0 } else { 0.0 };
            }
        }
    }
    let corrupted = evaluate(&g).unwrap().exploitability;
    assert!(corrupted > baseline + 1.0, "baseline {baseline} mbb, corrupted {corrupted} mbb");
}

#[test]
fn flop_spot_with_turn_and_river_runouts_is_validated_independently() {
    // Small ranges keep the (deliberately simple) validator fast across ~2,200 runouts.
    let sz =
        |b: u32| PlayerSizing { bet_pct_pot: [vec![b], vec![b], vec![b]], raise_pct_pot: [vec![], vec![], vec![]] };
    let mut s = spec("Ks7d2c", ["AA,KQs,76s,A5s", "QQ,JJ,AKs,98s"], 100, 1000, &[], &[], RakeConfig::none());
    s.tree.oop = sz(50);
    s.tree.ip = sz(75);
    s.solver.stopping.max_iterations = 500;
    s.solver.stopping.exploitability_check_interval = 500;
    let solved = solve(&s, &AtomicBool::new(false), |_| {}).unwrap();
    let g = solved.postflop_game().unwrap();
    let chance = g.nodes.iter().filter(|n| matches!(n, solver_validation::GameNode::Chance { .. })).count();
    assert!(chance > 49, "turn and river chance nodes expected, found {chance}");
    let ours = evaluate(&g).unwrap();
    for p in 0..2 {
        assert!((ours.ev[p] - solved.outcome.values_mbb[p]).abs() < 0.5, "{ours:?} vs {:?}", solved.outcome.values_mbb);
    }
    assert!((ours.exploitability - solved.outcome.progress.exploitability_mbb).abs() < 0.5);
}

#[test]
fn memory_estimate_is_exact_and_limits_refuse_before_allocating() {
    let s = solve(&turn_spec("Ks7d2c8h", RakeConfig::none(), 10), &AtomicBool::new(false), |_| {}).unwrap();
    let actual = s.solver().arena_bytes() + s.solver().spot.tree_bytes();
    assert_eq!(s.outcome.memory_bytes, actual);
    let tight = solver_gtopen::Limits { max_nodes: 50_000_000, max_bytes: actual - 1 };
    let r = solver_gtopen::solve_with_limits(
        &turn_spec("Ks7d2c8h", RakeConfig::none(), 10),
        tight,
        &AtomicBool::new(false),
        |_| {},
    );
    assert!(matches!(r, Err(AdapterError::TooLarge { .. })), "{r:?}");
    let few_nodes = solver_gtopen::Limits { max_nodes: 100, max_bytes: u64::MAX };
    let r = solver_gtopen::solve_with_limits(
        &turn_spec("Ks7d2c8h", RakeConfig::none(), 10),
        few_nodes,
        &AtomicBool::new(false),
        |_| {},
    );
    assert!(matches!(&r, Err(AdapterError::Backend(m)) if m.contains("tree too large")), "{r:?}");
}

#[test]
fn class_strategies_are_distributions_and_checks_run_where_supported() {
    let s = solve(&toy(RakeConfig::none()), &AtomicBool::new(false), |_| {}).unwrap();
    assert!(s.outcome.independent_check.performed);
    assert!(s.outcome.independent_check.exploitability_mbb.unwrap() < 1.0);
    let t = solve(&turn_spec("Ks7d2c8h", RakeConfig::none(), 200), &AtomicBool::new(false), |_| {}).unwrap();
    assert!(t.outcome.independent_check.performed);
    let total: f64 = t.outcome.root_by_class.iter().map(|c| c.weight).sum();
    assert!((total - t.postflop_game().unwrap().ranges[0].iter().map(|(_, w)| w).sum::<f64>()).abs() < 1e-9);
    for c in &t.outcome.root_by_class {
        assert!((c.frequencies.iter().sum::<f64>() - 1.0).abs() < 1e-4, "{c:?}");
        assert_eq!(range_engine::HandClass::at(c.row, c.col).to_string(), c.label);
    }
    let sz = |b: u32| PlayerSizing { bet_pct_pot: [vec![b], vec![], vec![]], raise_pct_pot: [vec![], vec![], vec![]] };
    let mut f = spec("Ks7d2c", ["AA,KK", "QQ,JJ"], 100, 1000, &[], &[], RakeConfig::none());
    f.tree.oop = sz(50);
    f.tree.ip = sz(50);
    f.solver.stopping.max_iterations = 10;
    let flop = solve(&f, &AtomicBool::new(false), |_| {}).unwrap();
    assert!(flop.outcome.independent_check.performed, "flop spots are checked too");
    // Without memory headroom for the export, the check is skipped and says why.
    let tight = solver_gtopen::Limits { max_nodes: 50_000_000, max_bytes: flop.outcome.memory_bytes };
    let skipped = solver_gtopen::solve_with_limits(&f, tight, &AtomicBool::new(false), |_| {}).unwrap();
    assert!(!skipped.outcome.independent_check.performed);
    assert!(skipped.outcome.independent_check.note.contains("Skipped"), "{}", skipped.outcome.independent_check.note);
}

#[test]
fn saved_trees_reload_and_can_be_browsed() {
    use solver_gtopen::{PathStep, load};
    let spec = turn_spec("Ks7d2c8h", RakeConfig::none(), 200);
    let mut s = solve(&spec, &AtomicBool::new(false), |_| {}).unwrap();
    let dir = std::env::temp_dir().join(format!("riverlab-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("spot.gto");
    s.save(path.to_str().unwrap()).unwrap();
    let loaded = load(&spec, path.to_str().unwrap()).unwrap();
    for p in 0..2 {
        assert!((loaded.outcome.values_mbb[p] - s.outcome.values_mbb[p]).abs() < 1e-6);
    }
    // Root view = stored root strategy (reach = range weights).
    let root = loaded.node_view(&[]).unwrap();
    assert_eq!(root.kind, "decision");
    assert_eq!(root.by_class, s.outcome.root_by_class);
    assert_eq!(root.reach_fraction, [1.0, 1.0]);
    // OOP checks, IP checks, then the river is dealt.
    let check = root.actions.iter().position(|a| a == "check").unwrap();
    let ip = loaded.node_view(&[PathStep::Action(check)]).unwrap();
    assert_eq!(ip.player, Some(1));
    let ip_check = ip.actions.iter().position(|a| a == "check").unwrap();
    let chance = loaded.node_view(&[PathStep::Action(check), PathStep::Action(ip_check)]).unwrap();
    assert_eq!(chance.kind, "chance");
    assert_eq!(chance.cards.len(), 48);
    let ah: poker_core::Card = "Ah".parse().unwrap();
    let river = loaded.node_view(&[PathStep::Action(check), PathStep::Action(ip_check), PathStep::Card(ah)]).unwrap();
    assert_eq!(river.street, "river");
    assert_eq!(river.board.last().unwrap(), "Ah");
    assert!(river.reach_fraction[0] < 1.0 && river.reach_fraction[1] < 1.0);
    // No class in the river view may contain the dealt card's combos only: AA loses Ah combos.
    let aa = river.by_class.iter().find(|c| c.label == "AA").unwrap();
    let aa_root = root.by_class.iter().find(|c| c.label == "AA").unwrap();
    assert!(aa.weight < aa_root.weight);
    for c in &river.by_class {
        assert!((c.frequencies.iter().sum::<f64>() - 1.0).abs() < 1e-4);
    }
    // Invalid paths are rejected.
    assert!(loaded.node_view(&[PathStep::Card(ah)]).is_err());
    assert!(loaded.node_view(&[PathStep::Action(99)]).is_err());
    let ks: poker_core::Card = "Ks".parse().unwrap();
    assert!(loaded.node_view(&[PathStep::Action(check), PathStep::Action(ip_check), PathStep::Card(ks)]).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

/// Per-hand EVs along a line, on the hand-solved toy spot (pot 10bb, IP may bet the pot, all-in):
/// at equilibrium IP's bluffs and OOP's bluff-catchers are indifferent, so their two actions are
/// worth the same, and the value bets strictly prefer betting.
#[test]
fn hand_at_gives_per_hand_action_evs_that_satisfy_indifference() {
    use range_engine::Combo;
    use solver_gtopen::PathStep;
    let s = solve(&toy(RakeConfig::none()), &AtomicBool::new(false), |_| {}).unwrap();
    let c = |x: &str| Combo::from(x.parse::<poker_core::HoleCards>().unwrap());
    let root = s.hand_at(&[], 0, c("AhQh")).unwrap();
    assert_eq!(root.actions, ["check"]);
    let after_check = [PathStep::Action(0)];
    let bluff = s.hand_at(&after_check, 1, c("4h3c")).unwrap();
    assert_eq!(bluff.actions, ["check", "bet 10bb"]);
    let ev = bluff.action_evs_mbb.unwrap();
    // Net of IP's half of the starting pot: checking 4-high loses 5bb; bluffing is indifferent.
    assert!((ev[0] + 5000.0).abs() < 1.0 && (ev[1] - ev[0]).abs() < 30.0, "{ev:?}");
    let value = s.hand_at(&after_check, 1, c("KhKd")).unwrap();
    let v = value.action_evs_mbb.unwrap();
    assert!(v[1] > v[0] + 1000.0, "kings gain by betting: {v:?}");
    assert!(value.strategy.unwrap()[1] > 0.99);
    // Facing the bet, the bluff-catcher folds or calls for the same EV; folding loses 5bb for every hand.
    let facing = [PathStep::Action(0), PathStep::Action(1)];
    for hand in ["AhQh", "AcQc"] {
        let h = s.hand_at(&facing, 0, c(hand)).unwrap();
        assert_eq!(h.actions, ["fold", "call 10bb"]);
        let e = h.action_evs_mbb.unwrap();
        assert!((e[0] + 5000.0).abs() < 1.0 && (e[1] - e[0]).abs() < 30.0, "{hand}: {e:?}");
    }
    // The opponent's hand is not given a strategy, and ranges are reported with their reach.
    let other = s.hand_at(&facing, 1, c("KhKd")).unwrap();
    assert!(other.strategy.is_none() && other.action_evs_mbb.is_none());
    assert!(other.ranges[1].iter().all(|(_, w)| *w > 0.0));
    assert!(other.reach_fraction[1] < 1.0, "IP's checks are no longer in its range after a bet");
}
