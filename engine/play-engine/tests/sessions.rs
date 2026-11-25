//! Whole sessions in every format with a scripted "hero": chips are conserved, sessions end,
//! and reports build.
use play_engine::config::{Difficulty, Format, Leak, Level, TableConfig};
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
            leaks: vec![],
            hide_leaks: false,
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
        leaks: vec![],
        hide_leaks: false,
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
            payouts: vec![],
        },
        opponents: vec![Difficulty::Hard, Difficulty::Easy],
        seed: 3,
        leaks: vec![],
        hide_leaks: false,
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
fn sit_and_go_all_ins_are_valued_with_icm() {
    let lv = |s, b| Level { small_blind: s, big_blind: b, ante: 0 };
    let mut seen = 0;
    for seed in 0..6 {
        let cfg = TableConfig {
            format: Format::SitAndGo {
                starting_chips: 500,
                levels: vec![lv(10, 20), lv(25, 50), lv(50, 100), lv(100, 200)],
                hands_per_level: 4,
                payouts: vec![50, 30, 20],
            },
            opponents: vec![Difficulty::Easy, Difficulty::Hard, Difficulty::Easy],
            seed,
            leaks: vec![],
            hide_leaks: false,
        };
        let s = play(cfg, 200);
        let shares: f64 = s.view().seats.iter().map(|seat| seat.icm_share.expect("sit & go seats have ICM")).sum();
        assert!((shares - 1.0).abs() < 1e-9, "the prize pool is shared out exactly: {shares}");
        for r in s.records() {
            for d in &r.hero_decisions {
                let Some(icm) = &d.icm else { continue };
                seen += 1;
                assert_eq!(d.opponents, 1, "ICM is used heads-up only");
                assert!(d.to_call > 0);
                assert!(icm.lose < icm.win && (0.0..=1.0).contains(&icm.required_equity), "{icm:?}");
                assert!(icm.lose <= icm.fold + 1e-12 && icm.fold <= icm.win + 1e-12, "{icm:?}");
                // Only the places still to be paid are used.
                assert!(icm.payouts.len() <= r.seats.len());
                assert!(d.feedback.is_some());
                if d.equity_vs_ranges.is_some() && matches!(d.action, Action::Call | Action::Fold) {
                    let j = play_engine::judge::judge(d, "BB", |x| x.to_string());
                    assert!(
                        j.checks.iter().any(|c| c.reference == play_engine::judge::ICM_REFERENCE),
                        "graded against ICM: {:?}",
                        j.checks
                    );
                    assert!(j.checks.iter().all(|c| !c.key.contains("odds")), "ICM replaces chip pot odds");
                }
            }
        }
    }
    assert!(seen > 0, "short-stacked sit & goes produce heads-up all-ins");
}

#[test]
fn every_decision_gets_feedback_and_reveal_is_flagged() {
    let models = cache();
    let cfg = TableConfig {
        format: Format::Cash { game: GameConfig { players: 3, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Hard, Difficulty::Easy],
        seed: 21,
        leaks: vec![],
        hide_leaks: false,
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
        leaks: vec![],
        hide_leaks: false,
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

#[test]
fn whole_hand_review_chains_street_solves() {
    use play_engine::review;
    let cfg = TableConfig {
        format: Format::Cash { game: GameConfig { players: 2, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Medium],
        seed: 23,
        leaks: vec![],
        hide_leaks: false,
    };
    let models = cache();
    let mut s = Session::new(cfg.clone()).unwrap();
    let mut found = None;
    for _ in 0..300 {
        s.start_hand(&models).unwrap();
        while s.status() == SessionStatus::AwaitingHero {
            let v = s.view();
            let l = v.legal.clone().unwrap();
            let a = match (v.board.len(), l.bet) {
                (4, Some((min, _))) => Action::Bet { to: poker_core::Chips(min.max(v.pot / 2)) },
                _ if l.check => Action::Check,
                _ => Action::Call,
            };
            s.act(a).unwrap();
        }
        let r = s.records().last().unwrap();
        let st = review::streets(r);
        let ok = |name: &str| st.iter().any(|x| x.street == name && x.solvable);
        if ok("turn") && ok("river") {
            found = Some(r.clone());
            break;
        }
    }
    let record = found.expect("a hand with heads-up turn and river decisions");
    let quick = |mut spec: review::StreetSpec| {
        spec.spec.solver.stopping.max_iterations = 40;
        spec.spec.solver.stopping.exploitability_check_interval = 40;
        spec
    };
    let turn = quick(review::street_spec(&record, "turn", &cfg, 300 << 20, None).unwrap());
    assert_eq!(turn.spec.board.cards().len(), 4);
    assert!(turn.next_card.is_some(), "the hand reached the river");
    let solved = solver_gtopen::solve(&turn.spec, &std::sync::atomic::AtomicBool::new(false), |_| {}).unwrap();
    let a = review::analyze(&solved, &turn).unwrap();
    assert!(a.followed, "{:?}", a.notes);
    let turn_decisions = review::streets(&record).into_iter().find(|x| x.street == "turn").unwrap().decisions;
    assert_eq!(a.decisions.iter().map(|d| d.decision).collect::<Vec<_>>(), turn_decisions);
    for d in &a.decisions {
        let mix = d.strategy.as_ref().expect("your hand is in your range");
        assert!((mix.iter().sum::<f64>() - 1.0).abs() < 1e-3);
        assert!(d.ev_loss_bb.unwrap() >= 0.0);
        assert_eq!(d.action_evs_bb.as_ref().unwrap().len(), d.actions.len());
    }
    // The river starts from the turn solution's ranges along the real line.
    let carried = a.next_ranges.clone().expect("ranges carried to the river");
    let river = quick(review::street_spec(&record, "river", &cfg, 300 << 20, Some(carried.clone())).unwrap());
    assert!(river.notes.iter().any(|n| n.contains("subgame re-solving")));
    assert_eq!(river.spec.ranges[1 - river.hero_player as usize], carried[1 - river.hero_player as usize]);
    let solved = solver_gtopen::solve(&river.spec, &std::sync::atomic::AtomicBool::new(false), |_| {}).unwrap();
    let r = review::analyze(&solved, &river).unwrap();
    assert!(r.followed, "{:?}", r.notes);
    assert!(r.next_ranges.is_none(), "nothing after the river");
    assert!(!r.decisions.is_empty());
    // Preflop decisions are not part of the solver review.
    assert!(review::streets(&record).iter().all(|x| x.street != "preflop"));
}

#[test]
fn postflop_drills_are_dealt_from_a_solve_and_graded_by_ev() {
    use play_engine::drill;
    use solver_interface::{PlayerSizing, SolveSpec, SolverConfig, TreeAbstraction};
    let sizing =
        |b: Vec<u32>| PlayerSizing { bet_pct_pot: [vec![], vec![], b], raise_pct_pot: [vec![], vec![], vec![100]] };
    let mut solver = SolverConfig::default();
    solver.stopping.max_iterations = 300;
    let spec = SolveSpec {
        game: GameConfig::default(),
        ranges: vec!["AA,KK,QQ,AKs,AKo,JJ,TT,98s".into(), "AQs,AQo,KQs,JJ,TT,99,88,76s".into()],
        board: "Kh9c4d2s7h".parse().unwrap(),
        starting_pot_mbb: 10_000,
        effective_stack_mbb: 50_000,
        tree: TreeAbstraction {
            oop: sizing(vec![50]),
            ip: sizing(vec![50, 100]),
            max_raises_per_street: 1,
            all_in_threshold_pct_stack: 100,
            add_all_in: false,
        },
        solver,
        locks: Vec::new(),
    };
    let solved = solver_gtopen::solve(&spec, &std::sync::atomic::AtomicBool::new(false), |_| {}).unwrap();
    let a = drill::deal(&solved, &spec, 7, None).unwrap();
    assert_eq!(a, drill::deal(&solved, &spec, 7, None).unwrap(), "seeded");
    let mut seen_players = std::collections::HashSet::new();
    for seed in 0..40 {
        let d = drill::deal(&solved, &spec, seed, None).unwrap();
        seen_players.insert(d.hero_player);
        assert!(!d.actions.is_empty());
        assert_eq!(d.board.len(), 5);
        assert!((d.pot_mbb - 10_000.0).abs() < 1e-6, "river root: the pot is the starting pot");
        assert!(d.stacks_mbb.iter().all(|s| (0.0..=50_000.0).contains(s)));
        let grades: Vec<_> = (0..d.actions.len()).map(|c| drill::grade(&solved, &spec, &d, c).unwrap()).collect();
        let best = grades[0].best;
        assert_eq!(grades[best].verdict, "correct");
        assert_eq!(grades[best].ev_loss_bb, 0.0);
        for g in &grades {
            assert!(g.ev_loss_bb >= 0.0);
            assert!((g.strategy.iter().sum::<f64>() - 1.0).abs() < 1e-3);
            assert!((g.ev_loss_pct_pot - g.ev_loss_bb * 10.0).abs() < 1e-9, "1bb is 10% of a 10bb pot");
        }
        assert!(drill::grade(&solved, &spec, &d, d.actions.len()).is_err());
    }
    assert_eq!(seen_players.len(), 2, "both positions are drilled");
    assert!(drill::deal(&solved, &spec, 3, Some(1)).unwrap().hero_player == 1);
}

#[test]
fn equity_breakdown_explains_each_tracked_range() {
    let cfg = TableConfig {
        format: Format::Cash { game: GameConfig { players: 3, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Hard, Difficulty::Medium],
        seed: 5,
        leaks: vec![],
        hide_leaks: false,
    };
    let models = cache();
    let mut s = Session::new(cfg).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let (mut checked, mut postflop, mut with_steps, mut saved) = (0, 0, 0, 0);
    while s.hands_played() < 25 {
        s.start_hand(&models).unwrap();
        while s.status() == SessionStatus::AwaitingHero {
            let b = s.equity_breakdown().unwrap();
            checked += 1;
            let eq = b.equity.expect("equity against the live ranges");
            assert!((0.0..=1.0).contains(&eq));
            assert_eq!(b.hero_cards.len(), 2);
            assert!(!b.opponents.is_empty());
            for o in &b.opponents {
                let grid: f64 = o.classes.iter().map(|c| c.weight).sum();
                assert!((grid - o.combos).abs() < 1e-6, "the grid adds up to the range");
                assert!(o.classes.iter().all(|c| (0.0..=1.0 + 1e-9).contains(&c.frequencies[0])));
                assert!((0.0..=1.0 + 1e-9).contains(&o.share_of_all));
                for st in &o.steps {
                    assert!((0.0..=1.0 + 1e-9).contains(&st.kept), "{st:?}");
                }
                with_steps += usize::from(!o.steps.is_empty());
                if b.board.len() >= 3 && o.combos > 0.0 {
                    let total: f64 = o.categories.iter().map(|c| c.share).sum();
                    assert!((total - 1.0).abs() < 1e-6, "categories cover the range: {total}");
                    assert!(o.categories.iter().all(|c| (0.0..=1.0).contains(&c.hero_equity)));
                }
            }
            if b.board.len() >= 3 {
                postflop += 1;
                assert!(b.hero_hand.is_some());
                if b.opponents.len() == 1 {
                    assert_eq!(b.method, "exact");
                    let alone = b.opponents[0].hero_equity.unwrap();
                    assert!((alone - eq).abs() < 1e-9, "heads-up, the total is the equity against that range");
                }
            }
            assert!(b.opponents.iter().all(|o| o.held.is_none()), "live, the bots' cards stay hidden");
            let a = hero_action(&s, &mut rng);
            s.act(a).unwrap();
        }
        assert!(s.equity_breakdown().is_err(), "only while you are to act");
        // The saved copy adds each bot's real cards, which the tracked range must contain:
        // the bot chose every action with those cards, so their weight is never 0.
        let r = s.records().last().unwrap();
        for d in &r.hero_decisions {
            let b = d.breakdown.as_ref().expect("saved with every decision");
            for o in &b.opponents {
                let held = o.held.as_ref().expect("the review shows what they held");
                let seat = r.seats.iter().find(|seat| seat.name == o.name).expect("seat");
                assert_eq!(held.cards, seat.hole_cards, "the cards they were dealt");
                let grid: f64 = o.grid.iter().map(|g| g.1).sum();
                assert!(o.classes.is_empty());
                if !o.steps.is_empty() {
                    assert!((grid - o.combos).abs() < 0.2, "packed grid {grid} vs {}", o.combos);
                }
                assert!(
                    held.weight > 0.0 && held.weight <= 1.0,
                    "{} held {:?} at weight {}",
                    o.name,
                    held.cards,
                    held.weight
                );
                saved += 1;
            }
        }
    }
    assert!(checked > 10 && postflop > 0 && with_steps > 0, "{checked} {postflop} {with_steps}");
    assert!(saved > 10);
}

#[test]
fn practice_leaks_are_recorded_and_graded() {
    for leak in [Leak::OverbluffsRiver, Leak::OverfoldsRiver] {
        let cfg = TableConfig {
            format: Format::Cash { game: GameConfig { players: 2, ..GameConfig::default() }, auto_rebuy: true },
            opponents: vec![Difficulty::Hard],
            seed: 8,
            leaks: vec![leak],
            hide_leaks: false,
        };
        let s = play(cfg, 150);
        assert!(s.players[1].name.contains(leak.label()), "{}", s.players[1].name);
        assert!(s.records().iter().all(|r| r.seats.iter().any(|seat| seat.leak == leak)));
        for r in s.records() {
            for a in &r.actions {
                if let (Some(f), Some(be)) = (a.fold_share, a.bluff_break_even) {
                    assert!((0.0..=1.0 + 1e-9).contains(&f) && (0.0..=1.0).contains(&be), "{f} {be}");
                }
            }
        }
        let report = play_engine::report::build(s.records(), false);
        let p = report.leak_practice.iter().find(|p| p.leak == leak).expect("a section for the leak");
        assert!(p.decisions + p.bets > 0, "river spots against the leak: {p:?}");
        assert!(p.matched <= p.decisions && p.bets_profitable_as_bluffs <= p.bets);
        assert!(!p.best_response.is_empty());
    }
    let bad = TableConfig {
        format: Format::Cash { game: GameConfig { players: 3, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Hard, Difficulty::Easy],
        seed: 1,
        leaks: vec![Leak::OverfoldsRiver],
        hide_leaks: false,
    };
    assert!(bad.validate().is_err(), "one leak per opponent");

    // Spot the leak: names stay plain, the report lists every bot's real leak.
    let hidden = TableConfig {
        format: Format::Cash { game: GameConfig { players: 3, ..GameConfig::default() }, auto_rebuy: true },
        opponents: vec![Difficulty::Hard, Difficulty::Hard],
        seed: 4,
        leaks: vec![Leak::NeverBluffsRiver, Leak::None],
        hide_leaks: true,
    };
    let s = play(hidden, 40);
    assert!(s.players.iter().all(|p| !p.name.contains("river")), "names give nothing away");
    let report = play_engine::report::build(s.records(), false);
    let leaks: Vec<Leak> = report.hidden_leaks.iter().map(|h| h.leak).collect();
    assert_eq!(leaks, vec![Leak::NeverBluffsRiver, Leak::None]);
    let ff = TableConfig {
        format: Format::FastFold { game: GameConfig { players: 3, ..GameConfig::default() } },
        opponents: vec![Difficulty::Hard, Difficulty::Hard],
        seed: 4,
        leaks: vec![Leak::NeverBluffsRiver, Leak::None],
        hide_leaks: true,
    };
    assert!(ff.validate().is_err(), "fast fold reshuffles the bots");
}
