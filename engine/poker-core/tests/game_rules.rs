//! Rule-level tests for the no-limit hold'em state machine (see docs/poker-rules.md).

use poker_core::game::{Action, GameState, Phase};
use poker_core::rake::{RakeAmount, RakeConfig};
use poker_core::{Card, Chips, GameConfig, HoleCards, Position, Street};
use proptest::prelude::*;

fn bb(x: f64) -> Chips {
    Chips((x * 1000.0).round() as u64)
}
fn no_rake() -> GameConfig {
    GameConfig { rake: RakeConfig::none(), ..GameConfig::default() }
}
fn cards(s: &str) -> Vec<Card> {
    Card::parse_many(s).unwrap()
}
fn fold_to(g: &mut GameState, seat: usize) {
    while g.to_act() != Some(seat) {
        g.apply(Action::Fold).unwrap();
    }
}
fn total_chips(g: &GameState) -> Chips {
    g.players().iter().map(|p| p.stack).sum::<Chips>() + g.pot_total()
}

#[test]
fn blinds_posted_and_utg_opens() {
    let g = GameState::new(GameConfig::default()).unwrap();
    assert_eq!(g.pot_total(), bb(1.5));
    assert_eq!(g.to_act(), Some(0));
    assert_eq!(g.players()[0].position, Position::Utg);
    let la = g.legal_actions().unwrap();
    assert!(la.fold && !la.check);
    assert_eq!(la.call, Some(bb(1.0)));
    assert_eq!(la.raise, Some((bb(2.0), bb(100.0))));
    assert_eq!(la.bet, None);
}

#[test]
fn antes_are_dead_money_not_part_of_the_bet() {
    let cfg = GameConfig { ante: bb(0.1), ..no_rake() };
    let g = GameState::new(cfg).unwrap();
    assert_eq!(g.pot_total(), bb(2.1));
    assert_eq!(g.legal_actions().unwrap().call, Some(bb(1.0)));
}

#[test]
fn fold_to_big_blind_is_uncontested_and_unraked_with_nfnd() {
    let mut g = GameState::new(GameConfig::default()).unwrap();
    for _ in 0..5 {
        g.apply(Action::Fold).unwrap();
    }
    assert_eq!(g.phase(), Phase::Uncontested { winner: 5 });
    let s = g.settle_uncontested().unwrap();
    assert_eq!(s.rake.rake, Chips::ZERO);
    assert_eq!(s.net[5], 500);
    assert_eq!(s.net[4], -500);
    assert_eq!(s.final_stacks.iter().copied().sum::<Chips>(), bb(600.0));
}

#[test]
fn big_blind_gets_option_after_limps() {
    let mut g = GameState::new(no_rake()).unwrap();
    for _ in 0..4 {
        g.apply(Action::Call).unwrap(); // UTG, HJ, CO, BTN limp
    }
    g.apply(Action::Call).unwrap(); // SB completes
    assert_eq!(g.to_act(), Some(5));
    let la = g.legal_actions().unwrap();
    assert!(la.check && !la.fold);
    assert_eq!(la.raise, Some((bb(2.0), bb(100.0))));
    g.apply(Action::Check).unwrap();
    assert_eq!(g.phase(), Phase::AwaitingBoard { street: Street::Flop });
    assert_eq!(g.pot_total(), bb(6.0));
}

#[test]
fn minimum_raise_tracks_last_full_raise() {
    let mut g = GameState::new(no_rake()).unwrap();
    g.apply(Action::Raise { to: bb(2.5) }).unwrap(); // raise of 1.5
    assert_eq!(g.legal_actions().unwrap().raise.unwrap().0, bb(4.0));
    assert!(g.apply(Action::Raise { to: bb(3.9) }).is_err());
    g.apply(Action::Raise { to: bb(8.0) }).unwrap(); // raise of 5.5
    assert_eq!(g.legal_actions().unwrap().raise.unwrap().0, bb(13.5));
}

#[test]
fn illegal_actions_are_rejected_not_corrected() {
    let mut g = GameState::new(no_rake()).unwrap();
    assert!(g.apply(Action::Check).is_err());
    assert!(g.apply(Action::Bet { to: bb(3.0) }).is_err());
    assert!(g.apply(Action::Raise { to: bb(101.0) }).is_err());
    assert!(g.deal_board(&cards("AsKdQc")).is_err());
    fold_to(&mut g, 4);
    g.apply(Action::Call).unwrap();
    g.apply(Action::Check).unwrap();
    assert!(g.apply(Action::Check).is_err(), "no one to act");
    g.deal_board(&cards("AsKdQc")).unwrap();
    assert_eq!(g.to_act(), Some(4), "SB acts first postflop");
    assert!(g.apply(Action::Fold).is_err(), "folding when check is free is not allowed");
    assert!(g.apply(Action::Bet { to: bb(0.5) }).is_err(), "below minimum bet");
}

#[test]
fn postflop_order_starts_with_small_blind_and_ends_with_button() {
    let mut g = GameState::new(no_rake()).unwrap();
    fold_to(&mut g, 3); // UTG, HJ, CO fold; BTN to act
    g.apply(Action::Raise { to: bb(2.5) }).unwrap();
    g.apply(Action::Call).unwrap(); // SB
    g.apply(Action::Call).unwrap(); // BB
    g.deal_board(&cards("2c7d9h")).unwrap();
    let mut order = Vec::new();
    for _ in 0..3 {
        order.push(g.players()[g.to_act().unwrap()].position);
        g.apply(Action::Check).unwrap();
    }
    assert_eq!(order, vec![Position::Sb, Position::Bb, Position::Btn]);
    assert_eq!(g.phase(), Phase::AwaitingBoard { street: Street::Turn });
}

#[test]
fn short_all_in_does_not_reopen_but_cumulative_full_raise_does() {
    // 4-handed: CO, BTN, SB, BB. BTN has only 16bb.
    let cfg = GameConfig { players: 4, ..no_rake() };
    let mut g = GameState::with_stacks(cfg, &[bb(100.0), bb(16.0), bb(100.0), bb(100.0)]).unwrap();
    g.apply(Action::Call).unwrap(); // CO limps
    g.apply(Action::Call).unwrap(); // BTN limps
    g.apply(Action::Call).unwrap(); // SB completes
    g.apply(Action::Check).unwrap(); // BB
    g.deal_board(&cards("2c7d9h")).unwrap();
    g.apply(Action::Bet { to: bb(10.0) }).unwrap(); // SB bets 10
    g.apply(Action::Call).unwrap(); // BB calls
    g.apply(Action::Call).unwrap(); // CO calls
    g.apply(Action::Raise { to: bb(15.0) }).unwrap(); // BTN all-in: +5, less than a full raise
    assert!(g.players()[1].all_in);
    // SB acted at 10 facing a raise of only 5: may call or fold, not raise.
    let la = g.legal_actions().unwrap();
    assert_eq!(la.seat, 2);
    assert_eq!(la.call, Some(bb(5.0)));
    assert_eq!(la.raise, None);
    assert!(g.apply(Action::Raise { to: bb(30.0) }).is_err());
    g.apply(Action::Call).unwrap();
    g.apply(Action::Call).unwrap(); // BB
    g.apply(Action::Call).unwrap(); // CO
    assert_eq!(g.phase(), Phase::AwaitingBoard { street: Street::Turn });
}

#[test]
fn two_short_all_ins_totalling_a_full_raise_reopen_action() {
    // 5-handed: HJ(100) CO(100) BTN(14) SB(19) BB(100)
    let cfg = GameConfig { players: 5, ..no_rake() };
    let mut g = GameState::with_stacks(cfg, &[bb(100.0), bb(100.0), bb(14.0), bb(19.0), bb(100.0)]).unwrap();
    for _ in 0..4 {
        g.apply(Action::Call).unwrap(); // HJ, CO, BTN limp, SB completes
    }
    g.apply(Action::Check).unwrap(); // BB
    g.deal_board(&cards("2c7d9h")).unwrap();
    // Postflop order: SB, BB, HJ, CO, BTN
    g.apply(Action::Check).unwrap(); // SB
    g.apply(Action::Bet { to: bb(10.0) }).unwrap(); // BB bets 10
    g.apply(Action::Call).unwrap(); // HJ calls 10
    g.apply(Action::Fold).unwrap(); // CO
    g.apply(Action::Raise { to: bb(13.0) }).unwrap(); // BTN all-in 13 (short)
    g.apply(Action::Raise { to: bb(18.0) }).unwrap(); // SB all-in 18 (short over 13, but 18 >= 10+10)
    // BB acted at 10 with threshold 20: 18 < 20 → still closed.
    assert_eq!(g.legal_actions().unwrap().raise, None);
    g.apply(Action::Call).unwrap();
    // HJ acted at 10 as well → closed.
    assert_eq!(g.legal_actions().unwrap().raise, None);

    // Variant where the cumulative raise reaches a full raise for the original bettor.
    let cfg = GameConfig { players: 5, ..no_rake() };
    let mut g = GameState::with_stacks(cfg, &[bb(100.0), bb(100.0), bb(14.0), bb(21.0), bb(100.0)]).unwrap();
    for _ in 0..4 {
        g.apply(Action::Call).unwrap();
    }
    g.apply(Action::Check).unwrap();
    g.deal_board(&cards("2c7d9h")).unwrap();
    g.apply(Action::Check).unwrap(); // SB
    g.apply(Action::Bet { to: bb(10.0) }).unwrap(); // BB
    g.apply(Action::Call).unwrap(); // HJ
    g.apply(Action::Fold).unwrap(); // CO
    g.apply(Action::Raise { to: bb(13.0) }).unwrap(); // BTN short all-in
    g.apply(Action::Raise { to: bb(20.0) }).unwrap(); // SB all-in to 20 = 10 + full 10
    let la = g.legal_actions().unwrap();
    assert_eq!(la.seat, 4);
    assert!(la.raise.is_some(), "action is reopened for the BB");
}

#[test]
fn uncalled_bet_is_returned_and_board_runs_out() {
    let cfg = no_rake();
    let mut g =
        GameState::with_stacks(cfg, &[bb(100.0), bb(100.0), bb(100.0), bb(100.0), bb(100.0), bb(50.0)]).unwrap();
    fold_to(&mut g, 4);
    g.apply(Action::Raise { to: bb(100.0) }).unwrap(); // SB shoves 100
    g.apply(Action::Call).unwrap(); // BB calls all-in for 50
    assert_eq!(g.players()[4].stack, bb(50.0), "50bb uncalled returned to SB");
    assert_eq!(g.pot_total(), bb(100.0));
    assert_eq!(g.phase(), Phase::AwaitingBoard { street: Street::Flop });
    g.deal_board(&cards("2c7d9h")).unwrap();
    assert_eq!(g.phase(), Phase::AwaitingBoard { street: Street::Turn });
    g.deal_board(&cards("Ts")).unwrap();
    g.deal_board(&cards("3s")).unwrap();
    assert_eq!(g.phase(), Phase::Showdown);
    let mut v = vec![None; 6];
    v[4] = Some(1);
    v[5] = Some(2);
    let s = g.settle_showdown(&v).unwrap();
    assert_eq!(s.final_stacks[5], bb(100.0));
    assert_eq!(s.final_stacks[4], bb(50.0));
}

#[test]
fn side_pots_are_awarded_to_eligible_best_hands() {
    // 3-handed BTN 10bb, SB 30bb, BB 60bb, all in preflop.
    let cfg = GameConfig { players: 3, ..no_rake() };
    let mut g = GameState::with_stacks(cfg, &[bb(10.0), bb(30.0), bb(60.0)]).unwrap();
    g.apply(Action::Raise { to: bb(10.0) }).unwrap(); // BTN all-in
    g.apply(Action::Raise { to: bb(30.0) }).unwrap(); // SB all-in
    g.apply(Action::Call).unwrap(); // BB calls 30
    assert_eq!(g.players()[2].stack, bb(30.0));
    for b in ["2c7d9h", "Ts", "3s"] {
        g.deal_board(&cards(b)).unwrap();
    }
    // BTN best, SB second, BB worst.
    let s = g.settle_showdown(&[Some(3), Some(2), Some(1)]).unwrap();
    assert_eq!(s.pots.len(), 2);
    assert_eq!(s.pots[0].amount, bb(30.0));
    assert_eq!(s.pots[0].winners, vec![0]);
    assert_eq!(s.pots[1].amount, bb(40.0));
    assert_eq!(s.pots[1].winners, vec![1]);
    assert_eq!(s.final_stacks, vec![bb(30.0), bb(40.0), bb(30.0)]);
}

#[test]
fn split_pot_odd_chip_goes_left_of_button() {
    let cfg = GameConfig { players: 3, ante: Chips(1), ..no_rake() };
    let mut g = GameState::new(cfg).unwrap();
    g.apply(Action::Call).unwrap(); // BTN
    g.apply(Action::Call).unwrap(); // SB
    g.apply(Action::Check).unwrap(); // BB
    for b in ["2c7d9h", "Ts", "3s"] {
        g.deal_board(&cards(b)).unwrap();
        while g.to_act().is_some() {
            g.apply(Action::Check).unwrap();
        }
    }
    // Pot = 3bb + 3 mbb antes = 3003. BTN and SB tie, BB loses → 1501.5 each.
    let s = g.settle_showdown(&[Some(9), Some(9), Some(1)]).unwrap();
    assert_eq!(s.pots[0].winners, vec![1, 0], "SB is first left of the button");
    assert_eq!(s.pots[0].shares, vec![Chips(1502), Chips(1501)]);
}

#[test]
fn rake_is_taken_once_flop_is_seen_and_capped() {
    let cfg = GameConfig {
        players: 3,
        rake: RakeConfig { cap: Some(RakeAmount::BigBlinds { mbb: 3000 }), ..RakeConfig::default() },
        ..GameConfig::default()
    };
    let mut g = GameState::new(cfg).unwrap();
    g.apply(Action::Raise { to: bb(100.0) }).unwrap();
    g.apply(Action::Call).unwrap();
    g.apply(Action::Fold).unwrap();
    for b in ["2c7d9h", "Ts", "3s"] {
        g.deal_board(&cards(b)).unwrap();
    }
    let s = g.settle_showdown(&[Some(2), Some(1), None]).unwrap();
    assert_eq!(s.rake.rake, bb(3.0));
    assert!(s.rake.capped);
    assert_eq!(s.final_stacks[0], bb(201.0 - 3.0));
    assert_eq!(s.final_stacks.iter().copied().sum::<Chips>() + s.rake.rake, bb(300.0));
}

#[test]
fn hole_and_board_cards_cannot_collide() {
    let mut g = GameState::new(no_rake()).unwrap();
    g.set_hole_cards(0, "AsKs".parse::<HoleCards>().unwrap()).unwrap();
    assert!(g.set_hole_cards(1, "AsQd".parse().unwrap()).is_err());
    fold_to(&mut g, 4);
    g.apply(Action::Call).unwrap();
    g.apply(Action::Check).unwrap();
    assert!(g.deal_board(&cards("Ks2c3d")).is_err(), "seat 0 folded but its known cards stay dead");
    g.deal_board(&cards("Qs2c3d")).unwrap();
    let mut g = GameState::new(no_rake()).unwrap();
    g.set_hole_cards(5, "AsKs".parse().unwrap()).unwrap();
    fold_to(&mut g, 4);
    g.apply(Action::Call).unwrap();
    g.apply(Action::Check).unwrap();
    assert!(g.deal_board(&cards("Ks2c3d")).is_err());
}

#[test]
fn per_seat_stacks_are_bounded() {
    let cfg = GameConfig { players: 3, ..no_rake() };
    assert!(GameState::with_stacks(cfg.clone(), &[bb(1000.0), bb(50.0), bb(50.0)]).is_ok());
    assert!(GameState::with_stacks(cfg.clone(), &[Chips(1_000_001), bb(50.0), bb(50.0)]).is_err());
    assert!(GameState::with_stacks(cfg, &[bb(50.0), bb(50.0)]).is_err());
}

#[test]
fn settlement_requires_values_for_live_players() {
    let cfg = GameConfig { players: 3, ..no_rake() };
    let mut g = GameState::new(cfg).unwrap();
    g.apply(Action::Raise { to: bb(100.0) }).unwrap();
    g.apply(Action::Call).unwrap();
    g.apply(Action::Call).unwrap();
    for b in ["2c7d9h", "Ts", "3s"] {
        g.deal_board(&cards(b)).unwrap();
    }
    assert!(g.settle_showdown(&[Some(1), None, Some(1)]).is_err());
    assert!(g.settle_uncontested().is_err());
}

// ---------- property test: random legal hands ----------

#[derive(Debug, Clone)]
struct Script {
    players: u8,
    stacks: Vec<u64>,
    choices: Vec<(u8, u16)>,
    values: Vec<u8>,
    rake_bps: u32,
}

fn script() -> impl Strategy<Value = Script> {
    (3u8..=6).prop_flat_map(|n| {
        (
            Just(n),
            proptest::collection::vec(1_100u64..300_000, n as usize),
            proptest::collection::vec((any::<u8>(), any::<u16>()), 200),
            proptest::collection::vec(0u8..4, n as usize),
            prop_oneof![Just(0u32), Just(500u32), Just(10_000u32)],
        )
            .prop_map(|(players, stacks, choices, values, rake_bps)| Script {
                players,
                stacks,
                choices,
                values,
                rake_bps,
            })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]
    #[test]
    fn random_hands_conserve_chips_and_terminate(sc in script()) {
        let cfg = GameConfig {
            players: sc.players,
            rake: RakeConfig { percentage_bps: sc.rake_bps, ..RakeConfig::default() },
            ..GameConfig::default()
        };
        let stacks: Vec<Chips> = sc.stacks.iter().copied().map(Chips).collect();
        let start: Chips = stacks.iter().copied().sum();
        let mut g = GameState::with_stacks(cfg, &stacks).unwrap();
        let boards = ["2c7d9h", "Ts", "3s"];
        let mut steps = sc.choices.iter();
        let mut actions = 0;
        loop {
            prop_assert_eq!(total_chips(&g), start);
            for p in g.players() {
                prop_assert!(p.street_commit <= g.current_bet() || g.to_act().is_none());
            }
            match g.phase() {
                Phase::Betting { .. } => {
                    actions += 1;
                    prop_assert!(actions < 200, "hand did not terminate");
                    let la = g.legal_actions().unwrap();
                    let &(kind, amt) = steps.next().unwrap();
                    let pick = |(lo, hi): (Chips, Chips)| {
                        let span = hi.mbb() - lo.mbb();
                        Chips(lo.mbb() + if span == 0 { 0 } else { amt as u64 % (span + 1) })
                    };
                    let mut options = Vec::new();
                    if la.fold { options.push(Action::Fold); }
                    if la.check { options.push(Action::Check); }
                    if la.call.is_some() { options.push(Action::Call); }
                    if let Some(r) = la.bet { options.push(Action::Bet { to: pick(r) }); }
                    if let Some(r) = la.raise { options.push(Action::Raise { to: pick(r) }); }
                    prop_assert!(!options.is_empty());
                    let a = options[kind as usize % options.len()];
                    g.apply(a).unwrap();
                }
                Phase::AwaitingBoard { street } => {
                    let idx = street as usize - 1;
                    g.deal_board(&cards(boards[idx])).unwrap();
                }
                Phase::Showdown => {
                    let v: Vec<Option<u8>> = g.players().iter().zip(&sc.values)
                        .map(|(p, v)| (!p.folded).then_some(*v)).collect();
                    let s = g.settle_showdown(&v).unwrap();
                    prop_assert_eq!(s.final_stacks.iter().copied().sum::<Chips>() + s.rake.rake, start);
                    prop_assert_eq!(s.net.iter().sum::<i64>(), -(s.rake.rake.mbb() as i64));
                    break;
                }
                Phase::Uncontested { winner } => {
                    let s = g.settle_uncontested().unwrap();
                    prop_assert_eq!(s.final_stacks.iter().copied().sum::<Chips>() + s.rake.rake, start);
                    for (i, &n) in s.net.iter().enumerate() {
                        if i != winner { prop_assert!(n <= 0); }
                    }
                    break;
                }
            }
        }
    }
}
