//! ICM push/fold against independent references, with the real exact equity table (built by
//! `scripts/build-push-fold-table.sh`; skipped when it is missing).

use hand_evaluator::evaluate_mask;
use play_engine::config::{Difficulty, Format, Level, TableConfig};
use play_engine::icm_push_fold::{IcmDrillSpec, IcmSpot, IcmTrainer, drill_from_record, outcome_stacks, solve};
use play_engine::{ModelCache, Session, SessionStatus};
use poker_theory::icm::icm;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use range_engine::{COMBO_COUNT, Combo};
use solver_validation::push_fold::{self, ComboEquities};
use std::sync::Arc;

fn table() -> Option<ComboEquities> {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/hu_combo_equity.bin"));
    let t = ComboEquities::load(path);
    if t.is_none() {
        eprintln!("skipped: {} not built", path.display());
    }
    t
}

fn spot(stacks_bb: &[u64], payouts: &[u32]) -> IcmSpot {
    IcmSpot {
        stacks: stacks_bb.iter().map(|s| s * 1000).collect(),
        small_blind: 500,
        big_blind: 1000,
        ante: 0,
        payouts: payouts.to_vec(),
    }
}

/// Heads-up, winner takes all, ICM is linear in chips: the model is exact heads-up push/fold.
#[test]
fn heads_up_matches_the_exact_push_fold_solver() {
    let Some(eq) = table() else { return };
    let s = solve(&eq, &spot(&[10, 10], &[100]), 600).unwrap();
    let exact = push_fold::solve(&eq, 10.0, 1000);
    let ev = push_fold::evaluate(&eq, 10.0, &exact);
    // Value as a share of the 20bb in play: (10 + SB EV) / 20.
    assert!((s.values[0] - (10.0 + ev.sb_ev) / 20.0).abs() < 2e-5, "{} vs {}", s.values[0], ev.sb_ev);
    let share = |v: &[f64]| v.iter().sum::<f64>() / COMBO_COUNT as f64;
    assert!((share(&s.profile.push[0]) - share(&exact.jam)).abs() < 0.003);
    assert!((share(&s.profile.call[1]) - share(&exact.call)).abs() < 0.003);
    assert!(s.epsilon_pct < 1e-3, "{}", s.epsilon_pct);
}

/// Plays the profile out with real cards: deal every hand, sample each action from the profile,
/// deal a board, evaluate the showdown and value the stacks with ICM. Independent of the
/// solver's formulas (and it does not ignore the folded players' cards, so a small difference
/// is expected on top of the sampling error).
fn simulate(s: &play_engine::icm_push_fold::IcmSolution, samples: usize, seed: u64) -> Vec<f64> {
    let sp = &s.spot;
    let n = sp.players();
    let prizes: Vec<f64> = sp.payouts.iter().map(|&p| f64::from(p) / 100.0).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut total = vec![0.0; n];
    let mut deck: Vec<u8> = (0..52).collect();
    for _ in 0..samples {
        deck.shuffle(&mut rng);
        let hand = |k: usize| {
            let (a, b) = (deck[2 * k], deck[2 * k + 1]);
            (1u64 << a) | (1u64 << b)
        };
        let combo = |k: usize| Combo::all().find(|c| c.card_set().bits() == hand(k)).expect("combo").index();
        let mut outcome = None;
        for i in 0..n - 1 {
            if rng.random::<f64>() < s.profile.push[i][combo(i)] {
                let mut called = None;
                for j in i + 1..n {
                    if rng.random::<f64>() < s.profile.call[i * n + j][combo(j)] {
                        called = Some(j);
                        break;
                    }
                }
                outcome = Some((
                    i,
                    called.map(|j| {
                        let board: u64 = deck[2 * n..2 * n + 5].iter().map(|&c| 1u64 << c).sum();
                        let (vi, vj) = (evaluate_mask(board | hand(i)).value(), evaluate_mask(board | hand(j)).value());
                        // A tie: count it as half a win either way, as the model does.
                        let i_wins = if vi == vj { rng.random::<bool>() } else { vi > vj };
                        (j, i_wins)
                    }),
                ));
                break;
            }
        }
        let stacks = outcome_stacks(sp, outcome);
        for (t, v) in total.iter_mut().zip(icm(&stacks, &prizes).unwrap().values) {
            *t += v;
        }
    }
    total.into_iter().map(|t| t / samples as f64).collect()
}

#[test]
fn three_and_four_handed_values_agree_with_a_simulation() {
    let Some(eq) = table() else { return };
    for (stacks, payouts) in [(vec![10u64, 10, 10], vec![50u32, 30, 20]), (vec![15, 5, 10, 10], vec![50, 30, 20])] {
        let s = solve(&eq, &spot(&stacks, &payouts), 150).unwrap();
        assert!((s.values.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(s.epsilon_pct < 0.01, "{:?}: {}", stacks, s.epsilon_pct);
        let sim = simulate(&s, 60_000, 11);
        for (p, (a, b)) in s.values.iter().zip(&sim).enumerate() {
            eprintln!("{stacks:?} player {p}: model {a:.4} vs simulated {b:.4}");
            assert!((a - b).abs() < 0.002, "{stacks:?} player {p}: model {a:.4} vs simulated {b:.4}");
        }
    }
}

#[test]
fn drills_and_sit_and_go_decisions_are_graded() {
    let Some(eq) = table() else { return };
    let trainer = Arc::new(IcmTrainer::new(Arc::new(eq)));
    let spec = IcmDrillSpec { players: 3, min_stack_mbb: 5_000, max_stack_mbb: 12_000, payouts: vec![], ante_mbb: 0 };
    let d = trainer.drill(&spec, 7).unwrap();
    assert_eq!(d.spot.payouts, vec![100], "default payouts for three players");
    let g = trainer.grade(&d, play_engine::push_fold::Choice::Play).unwrap();
    assert!(["correct", "close", "mistake"].contains(&g.verdict.as_str()));
    assert!(g.loss_pct >= 0.0 && g.epsilon_pct < 0.05);
    assert!((g.frequencies[0] + g.frequencies[1] - 1.0).abs() < 1e-9);

    // A short-stacked four-player sit & go paying three places: grade every decision that fits.
    let lv = |s, b| Level { small_blind: s, big_blind: b, ante: 0 };
    let cfg = TableConfig {
        format: Format::SitAndGo {
            starting_chips: 400,
            levels: vec![lv(20, 40), lv(30, 60), lv(50, 100)],
            hands_per_level: 6,
            payouts: vec![50, 30, 20],
        },
        opponents: vec![Difficulty::Hard; 3],
        seed: 2,
        leaks: vec![],
        hide_leaks: false,
    };
    let models = ModelCache::new(
        Some(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/play-test-cache"))),
        1 << 30,
        2,
    );
    let mut s = Session::new(cfg).unwrap();
    let mut graded = 0;
    while s.status() != SessionStatus::SessionOver && s.hands_played() < 60 && graded < 3 {
        s.start_hand(&models).unwrap();
        while s.status() == SessionStatus::AwaitingHero {
            let l = s.view().legal.unwrap();
            // Move in or fold, like a push/fold player.
            let a = match l.raise.or(l.bet) {
                Some((_, max)) if s.hands_played() % 2 == 0 => {
                    if l.bet.is_some() {
                        poker_core::game::Action::Bet { to: poker_core::Chips(max) }
                    } else {
                        poker_core::game::Action::Raise { to: poker_core::Chips(max) }
                    }
                }
                _ if l.check => poker_core::game::Action::Check,
                _ => poker_core::game::Action::Fold,
            };
            s.act(a).unwrap();
        }
        let r = s.records().last().unwrap().clone();
        for k in 0..r.hero_decisions.len() {
            if let Ok((drill, choice)) = drill_from_record(&r, k, &[50, 30, 20]) {
                assert_eq!(drill.spot.stacks.len(), r.seats.len());
                let g = trainer.grade(&drill, choice).unwrap();
                assert!(g.epsilon_pct < 0.05, "{}", g.epsilon_pct);
                graded += 1;
            }
        }
    }
    assert!(graded > 0, "some sit & go decisions fit the push/fold model");
}
