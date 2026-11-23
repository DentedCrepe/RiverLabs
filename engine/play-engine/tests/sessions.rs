//! Whole sessions in every format with a scripted "hero": chips are conserved, sessions end,
//! and reports build.
use play_engine::config::{Difficulty, Format, Level, TableConfig};
use play_engine::{ModelCache, Session, SessionStatus};
use poker_core::GameConfig;
use poker_core::game::Action;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::PathBuf;

fn cache() -> ModelCache {
    ModelCache::new(Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/play-test-cache"))), 1 << 30, 2)
}

/// Picks a random legal action: fold 20%, passive 50%, aggressive 30%.
fn hero_action(s: &Session, rng: &mut ChaCha8Rng) -> Action {
    let l = s.view().legal.expect("hero to act");
    let r: f64 = rng.random();
    if r < 0.2 && l.fold {
        return Action::Fold;
    }
    if r < 0.7 || (l.bet.is_none() && l.raise.is_none()) {
        return if l.check { Action::Check } else { Action::Call };
    }
    let (min, max) = l.bet.or(l.raise).unwrap();
    let to = poker_core::Chips(if rng.random::<f64>() < 0.2 { max } else { min.min(max) });
    if l.bet.is_some() { Action::Bet { to } } else { Action::Raise { to } }
}

fn play(cfg: TableConfig, max_hands: u64) -> Session {
    let models = cache();
    let mut s = Session::new(cfg).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(9);
    let total = |s: &Session| s.players.iter().map(|p| p.chips).sum::<u64>();
    while s.status() != SessionStatus::SessionOver && s.hands_played() < max_hands {
        s.start_hand(&models).unwrap();
        while s.status() == SessionStatus::AwaitingHero {
            let a = hero_action(&s, &mut rng);
            s.act(a).unwrap();
        }
        let r = s.records().last().unwrap();
        let rake_free = r.net.iter().sum::<i64>();
        assert!(rake_free <= 0, "hand {} creates chips: {:?}", r.hand_no, r.net);
        let _ = total(&s);
    }
    s
}

#[test]
fn cash_heads_up_and_six_max_play_and_report() {
    for (players, opponents) in [
        (2u8, vec![Difficulty::Hard]),
        (6, vec![Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Hard, Difficulty::Easy]),
    ] {
        let cfg = TableConfig {
            format: Format::Cash { game: GameConfig { players, ..GameConfig::default() }, auto_rebuy: true },
            opponents,
            seed: 42,
        };
        let s = play(cfg, 30);
        assert_eq!(s.hands_played(), 30);
        let report = play_engine::report::build(s.records(), false);
        assert_eq!(report.hands, 30);
        assert_eq!(report.stats.vpip.opportunities, 30);
        assert!(report.model_comparison.is_some(), "hero decisions were compared with the model");
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("measured_against") || report.findings.is_empty());
    }
}

#[test]
fn fast_fold_gives_new_opponents_each_hand() {
    let cfg = TableConfig {
        format: Format::FastFold { game: GameConfig { players: 4, ..GameConfig::default() } },
        opponents: vec![Difficulty::Easy, Difficulty::Medium, Difficulty::Hard],
        seed: 5,
    };
    let s = play(cfg, 15);
    assert_eq!(s.hands_played(), 15);
    let names: std::collections::HashSet<String> =
        s.records().iter().flat_map(|r| r.seats.iter().map(|x| x.name.clone())).collect();
    assert!(names.len() > 4, "opponents are reshuffled: {names:?}");
}

#[test]
fn sit_and_go_ends_with_places() {
    let lv = |s, b| Level { small_blind: s, big_blind: b, ante: 0 };
    let cfg = TableConfig {
        format: Format::SitAndGo {
            starting_chips: 600,
            levels: vec![lv(10, 20), lv(15, 30), lv(25, 50), lv(50, 100), lv(100, 200)],
            hands_per_level: 5,
        },
        opponents: vec![Difficulty::Hard, Difficulty::Easy],
        seed: 3,
    };
    let s = play(cfg, 400);
    assert_eq!(s.status(), SessionStatus::SessionOver, "the tournament finishes");
    let total: u64 = s.players.iter().map(|p| p.chips).sum();
    assert_eq!(total, 1800, "tournament chips are conserved");
    assert!(s.players[0].finish.is_some());
    let places: Vec<u8> = s.players.iter().filter_map(|p| p.finish).collect();
    assert!(places.contains(&1) || s.players[0].busted);
    let report = play_engine::report::build(s.records(), true);
    assert!(report.caveats.iter().any(|c| c.contains("ICM")));
}

#[test]
fn every_decision_gets_feedback_and_reveal_is_flagged() {
    let models = cache();
    let cfg = TableConfig {
        format: Format::Cash { game: GameConfig { players: 3, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Hard, Difficulty::Easy],
        seed: 21,
    };
    let mut s = Session::new(cfg).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    let mut graded = 0;
    for hand in 0..12 {
        s.set_reveal(hand >= 10);
        s.start_hand(&models).unwrap();
        if s.reveal() && s.status() == SessionStatus::AwaitingHero {
            let v = s.view();
            assert!(v.seats.iter().all(|seat| !seat.in_hand || seat.cards.is_some()), "all cards shown in reveal mode");
        }
        while s.status() == SessionStatus::AwaitingHero {
            let v = s.view();
            let eq = v.hero_equity.expect("your equity is shown on your turn");
            assert!((0.0..=1.0).contains(&eq));
            assert_eq!(v.required_equity.is_some(), v.legal.as_ref().unwrap().call.is_some());
            let before = v.feedback.len();
            let a = hero_action(&s, &mut rng);
            s.act(a).unwrap();
            let fb = s.view().feedback;
            assert_eq!(fb.len(), before + 1, "feedback appears straight after each decision");
            let f = &fb.last().unwrap().feedback;
            assert!(["mistake", "rare", "ok", "not_graded"].contains(&f.verdict.as_str()));
            graded += usize::from(f.verdict != "not_graded");
            if f.verdict != "not_graded" {
                assert!(f.reference.is_some(), "graded feedback names its reference");
            }
        }
        let r = s.records().last().unwrap();
        assert!(!r.summary.is_empty(), "the hand has a result line");
        assert!(r.summary.iter().all(|l| l.contains(" wins ") || l.starts_with("You win ")));
        assert_eq!(r.revealed, hand >= 10);
        let v = s.view();
        // After the hand, cards nobody showed can still be revealed on request.
        assert!(v.seats.iter().filter(|x| x.in_hand).all(|x| x.cards.is_some() || x.hidden_cards.is_some()));
    }
    assert!(graded > 0);
    let report = play_engine::report::build(s.records(), false);
    assert!(report.caveats.iter().any(|c| c.contains("revealed")));
}

/// Index of the solver action matching `step` (same kind, nearest amount).
fn matching_action(labels: &[String], step: &play_engine::spot::ExportStep) -> Option<usize> {
    let amount = |l: &str| l.rsplit(' ').next().and_then(|a| a.strip_suffix("bb")).and_then(|a| a.parse::<f64>().ok());
    fn kind(l: &str) -> &str {
        if l.starts_with("raise") { "raise" } else { l.split(' ').next().unwrap_or("") }
    }
    labels
        .iter()
        .enumerate()
        .filter(|(_, l)| kind(l) == step.kind)
        .min_by(|(_, a), (_, b)| {
            let d = |l: &str| amount(l).map_or(0.0, |x| (x * 1000.0 - step.to_mbb as f64).abs());
            d(a).total_cmp(&d(b))
        })
        .map(|(i, _)| i)
}

#[test]
fn heads_up_river_spot_exports_a_solvable_tree_containing_the_line() {
    let cfg = TableConfig {
        format: Format::Cash { game: GameConfig { players: 2, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Medium],
        seed: 11,
    };
    let models = cache();
    let mut s = Session::new(cfg.clone()).unwrap();
    // A calling station reaches the river often; bet the river sometimes so lines vary.
    let mut found = None;
    for _ in 0..200 {
        s.start_hand(&models).unwrap();
        while s.status() == SessionStatus::AwaitingHero {
            let v = s.view();
            let l = v.legal.clone().unwrap();
            let a = match (v.board.len(), l.bet) {
                (5, Some((min, _))) => Action::Bet { to: poker_core::Chips(min.max(v.pot / 2)) },
                _ if l.check => Action::Check,
                _ => Action::Call,
            };
            s.act(a).unwrap();
        }
        let r = s.records().last().unwrap();
        if let Some(i) = r.hero_decisions.iter().position(|d| d.spot.as_ref().is_some_and(|p| p.street == "river")) {
            found = Some((r.clone(), i));
            break;
        }
    }
    let (record, i) = found.expect("a heads-up river decision within 200 hands");
    let spot = record.hero_decisions[i].spot.clone().unwrap();
    assert!(spot.hero_range.is_some(), "the hand stayed in the preflop model");
    let mut ex = play_engine::spot::export(&record, i, &cfg, 2 << 30).unwrap();
    assert_eq!(ex.spec.board.cards().len(), 5);
    assert_eq!(ex.line.len(), spot.line.len());
    assert!(ex.notes.iter().any(|n| n.contains("preflop model")));
    // Every decision before ours was taken, so the line's sizes are in the tree.
    ex.spec.solver.stopping.max_iterations = 20;
    let solved = solver_gtopen::solve(&ex.spec, &std::sync::atomic::AtomicBool::new(false), |_| {}).unwrap();
    let mut path = Vec::new();
    for step in &ex.line {
        let node = solved.node_view(&path).unwrap();
        assert_eq!(node.player, Some(step.player as usize));
        let a = matching_action(&node.actions, step).unwrap_or_else(|| panic!("{step:?} in {:?}", node.actions));
        path.push(solver_gtopen::PathStep::Action(a));
    }
    let node = solved.node_view(&path).unwrap();
    assert_eq!(node.kind, "decision");
    assert_eq!(node.player, Some(ex.hero_player as usize));
    assert!(matching_action(&node.actions, &ex.hero_action).is_some(), "{:?}", node.actions);
    assert!(node.by_class.iter().any(|c| c.label == ex.hero_class), "your hand is in the solved range");
}
