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
    let game = s.river_game().unwrap();
    let ours = evaluate(&game).unwrap();
    assert!((ours.ev[1] - s.outcome.values_mbb[1]).abs() < 0.5, "{ours:?}");
    assert!(ours.exploitability < 1.0, "independent exploitability {} mbb", ours.exploitability);
    // IP node after OOP's check: bluff 12.5% of air, bet all KK.
    if let solver_validation::RiverNode::Decision { strategy, .. } = &game.nodes[1] {
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
    assert_eq!(s.outcome.backend_commit, solver_gtopen::GTOPEN_COMMIT);
}

#[test]
fn raked_toy_spot_is_validated_with_the_rake_rule() {
    let s = solve(&toy(five_pct()), &AtomicBool::new(false), |_| {}).unwrap();
    let ours = evaluate(&s.river_game().unwrap()).unwrap();
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
        let ours = evaluate(&s.river_game().unwrap()).unwrap();
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
