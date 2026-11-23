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
