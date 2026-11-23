//! End-of-session report: your statistics, and weaknesses measured against stated references
//! (the preflop model's frequencies, and pot odds against the bots' tracked ranges). Nothing
//! here compares you with invented "typical player" numbers.

use crate::record::HandRecord;
use poker_core::game::Action;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Ratio {
    pub made: u32,
    pub opportunities: u32,
}

impl Ratio {
    fn add(&mut self, opportunity: bool, made: bool) {
        if opportunity {
            self.opportunities += 1;
            self.made += u32::from(made);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Stats {
    /// Voluntarily put money in preflop.
    pub vpip: Ratio,
    /// Raised preflop.
    pub pfr: Ratio,
    /// Re-raised when facing a single raise.
    pub three_bet: Ratio,
    /// Folded to a re-raise after opening.
    pub fold_to_three_bet: Ratio,
    /// Bet the flop as the preflop raiser when checked to (or first to act).
    pub cbet: Ratio,
    /// Folded the flop to a bot's continuation bet.
    pub fold_to_cbet: Ratio,
    /// Went to showdown, of hands where you saw the flop.
    pub went_to_showdown: Ratio,
    /// Won money at showdown, of showdowns.
    pub won_at_showdown: Ratio,
    /// Postflop bets and raises divided by calls.
    pub aggression_factor: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PositionStats {
    pub position: String,
    pub hands: u32,
    pub net_bb: f64,
    pub vpip: Ratio,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelComparison {
    /// Preflop decisions that could be compared with the model.
    pub decisions: u32,
    /// Share of those decisions where you put money in voluntarily / the model's expected share
    /// for the same hands in the same spots.
    pub voluntary_you: f64,
    pub voluntary_model: f64,
    pub raise_you: f64,
    pub raise_model: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AllInLuck {
    pub hands: u32,
    pub actual_bb: f64,
    pub expected_bb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Example {
    pub hand_no: u64,
    pub street: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    pub key: String,
    pub title: String,
    pub detail: String,
    /// The reference the finding is measured against.
    pub measured_against: String,
    pub count: u32,
    pub out_of: u32,
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub hands: u32,
    /// "bb" or "chips" for `net`.
    pub units: String,
    pub net: i64,
    /// Net in big blinds (each hand converted at its own big blind).
    pub net_bb: f64,
    pub bb_per_100: Option<f64>,
    pub stats: Stats,
    pub by_position: Vec<PositionStats>,
    pub model_comparison: Option<ModelComparison>,
    pub all_in: Option<AllInLuck>,
    pub findings: Vec<Finding>,
    pub caveats: Vec<String>,
}

fn is_aggressive(a: &Action) -> bool {
    matches!(a, Action::Bet { .. } | Action::Raise { .. })
}

/// Thresholds for findings. They are choices about what to point out, not poker claims:
/// a model frequency at or below `RARE` means "the model almost never does this".
const RARE: f64 = 0.10;
/// Equity margin beyond the break-even point before a call or fold is flagged.
const MARGIN_STRICT: f64 = 0.05;
const MARGIN_LOOSE: f64 = 0.15;

pub fn build(records: &[HandRecord], tournament: bool) -> Report {
    let mut stats = Stats::default();
    let mut by_pos: BTreeMap<String, PositionStats> = BTreeMap::new();
    let (mut net, mut net_bb) = (0i64, 0.0f64);
    let (mut post_aggr, mut post_calls) = (0u32, 0u32);
    let mut cmp = (0u32, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut luck = AllInLuck { hands: 0, actual_bb: 0.0, expected_bb: 0.0 };
    let mut findings: BTreeMap<&'static str, Finding> = BTreeMap::new();
    let mut add = |key: &'static str, title: &str, detail: &str, against: &str, hit: bool, ex: Option<Example>| {
        let f = findings.entry(key).or_insert_with(|| Finding {
            key: key.into(),
            title: title.into(),
            detail: detail.into(),
            measured_against: against.into(),
            count: 0,
            out_of: 0,
            examples: vec![],
        });
        f.out_of += 1;
        if hit {
            f.count += 1;
            if let Some(e) = ex {
                if f.examples.len() < 5 {
                    f.examples.push(e);
                }
            }
        }
    };

    for r in records {
        let h = r.hero_seat();
        let bb = r.big_blind.max(1) as f64;
        net += r.net[h];
        let hand_bb = r.net[h] as f64 / bb;
        net_bb += hand_bb;
        let pre: Vec<_> = r.actions.iter().filter(|a| a.street == "preflop").collect();
        let hero_pre: Vec<_> = pre.iter().filter(|a| a.seat == h).collect();
        let vpip = hero_pre.iter().any(|a| matches!(a.action, Action::Call) || is_aggressive(&a.action));
        let pfr = hero_pre.iter().any(|a| is_aggressive(&a.action));
        stats.vpip.add(true, vpip);
        stats.pfr.add(true, pfr);
        let pos = by_pos.entry(r.seats[h].position.clone()).or_insert_with(|| PositionStats {
            position: r.seats[h].position.clone(),
            hands: 0,
            net_bb: 0.0,
            vpip: Ratio::default(),
        });
        pos.hands += 1;
        pos.net_bb += hand_bb;
        pos.vpip.add(true, vpip);

        // 3-bet and fold-to-3-bet, from the order of preflop raises.
        let mut raises_before = 0;
        let mut hero_opened = false;
        let mut faced_3bet = false;
        for a in &pre {
            if a.seat == h {
                if raises_before == 1 && !hero_opened {
                    stats.three_bet.add(true, is_aggressive(&a.action));
                }
                if faced_3bet && hero_opened {
                    stats.fold_to_three_bet.add(true, a.action == Action::Fold);
                    faced_3bet = false;
                }
                if is_aggressive(&a.action) && raises_before == 0 {
                    hero_opened = true;
                }
            }
            if is_aggressive(&a.action) {
                raises_before += 1;
                if a.seat != h && hero_opened && raises_before == 2 {
                    faced_3bet = true;
                }
            }
        }
        let last_raiser = pre.iter().rev().find(|a| is_aggressive(&a.action)).map(|a| a.seat);
        let flop: Vec<_> = r.actions.iter().filter(|a| a.street == "flop").collect();
        let hero_saw_flop = r.board.len() >= 3 && !pre.iter().any(|a| a.seat == h && a.action == Action::Fold);
        if hero_saw_flop && !flop.is_empty() {
            if last_raiser == Some(h) {
                // C-bet opportunity: nobody bet before your first flop action.
                if let Some(i) = flop.iter().position(|a| a.seat == h) {
                    if !flop[..i].iter().any(|a| is_aggressive(&a.action)) {
                        stats.cbet.add(true, is_aggressive(&flop[i].action));
                    }
                }
            } else if let Some(raiser) = last_raiser {
                // Facing the raiser's c-bet with no other bet before it.
                if let Some(bi) = flop.iter().position(|a| is_aggressive(&a.action)) {
                    if flop[bi].seat == raiser {
                        if let Some(resp) = flop[bi + 1..].iter().find(|a| a.seat == h) {
                            stats.fold_to_cbet.add(true, resp.action == Action::Fold);
                        }
                    }
                }
            }
        }
        if hero_saw_flop {
            let folded = r.actions.iter().any(|a| a.seat == h && a.action == Action::Fold);
            stats.went_to_showdown.add(true, r.showdown && !folded);
            if r.showdown && !folded {
                stats.won_at_showdown.add(true, r.net[h] > 0);
            }
        }
        for a in r.actions.iter().filter(|a| a.seat == h && a.street != "preflop") {
            if is_aggressive(&a.action) {
                post_aggr += 1;
            } else if a.action == Action::Call {
                post_calls += 1;
            }
        }
        if let Some(ev) = r.hero_all_in_ev {
            luck.hands += 1;
            luck.actual_bb += hand_bb;
            luck.expected_bb += ev / bb;
        }

        for d in &r.hero_decisions {
            let ex = |text: String| Some(Example { hand_no: r.hand_no, street: d.street.clone(), text });
            if let Some(m) = &d.model {
                let fold = m.actions.iter().position(|a| a.eq_ignore_ascii_case("fold"));
                let fold_f = fold.map_or(0.0, |i| m.frequencies[i]);
                let raise_f: f64 = m
                    .actions
                    .iter()
                    .zip(&m.frequencies)
                    .filter(|(a, _)| {
                        let a = a.to_ascii_lowercase();
                        a.starts_with("raise") || a.starts_with("all-in") || a.starts_with("bet")
                    })
                    .map(|(_, f)| f)
                    .sum();
                let you_vol = !matches!(d.action, Action::Fold | Action::Check);
                cmp.0 += 1;
                cmp.1 += f64::from(u8::from(you_vol));
                cmp.2 += 1.0
                    - fold_f
                    - m.actions.iter().position(|a| a.eq_ignore_ascii_case("check")).map_or(0.0, |i| m.frequencies[i]);
                cmp.3 += f64::from(u8::from(is_aggressive(&d.action)));
                cmp.4 += raise_f;
                if let Some(t) = m.taken {
                    let f = m.frequencies[t];
                    let pos = &r.seats[h].position;
                    let spot = m.spot.replace('_', " ");
                    let text = format!(
                        "{pos} {spot} with {}: you chose {} (model {:.0}%; model fold {:.0}%)",
                        m.class,
                        m.actions[t],
                        f * 100.0,
                        fold_f * 100.0
                    );
                    let against = "GTOpen's preflop model for this table (an approximate model)";
                    if you_vol && fold_f >= 1.0 - RARE {
                        add(
                            "preflop_too_loose",
                            "Played hands the model folds",
                            "You put money in with hands the model folds at least 90% of the time in that spot.",
                            against,
                            true,
                            ex(text.clone()),
                        );
                    } else if you_vol {
                        add(
                            "preflop_too_loose",
                            "Played hands the model folds",
                            "You put money in with hands the model folds at least 90% of the time in that spot.",
                            against,
                            false,
                            None,
                        );
                    }
                    if d.action == Action::Fold {
                        add(
                            "preflop_too_tight",
                            "Folded hands the model plays",
                            "You folded hands the model plays at least 90% of the time in that spot.",
                            against,
                            fold_f <= RARE,
                            if fold_f <= RARE { ex(text.clone()) } else { None },
                        );
                    }
                    if d.action == Action::Call && raise_f >= 1.0 - RARE {
                        add(
                            "preflop_passive",
                            "Called where the model raises",
                            "You called with hands the model raises at least 90% of the time.",
                            against,
                            true,
                            ex(text),
                        );
                    }
                }
            }
            // Preflop calls are judged against the model above; pot odds alone ignore position and
            // the play after the flop, so they only grade preflop calls that are all-in.
            let graded = d.street != "preflop" || d.all_in_call;
            if let (true, Some(req), Some(eq)) = (graded, d.required_equity, d.equity_vs_ranges) {
                let strict = d.all_in_call || d.street == "river";
                let margin = if strict { MARGIN_STRICT } else { MARGIN_LOOSE };
                let text = format!(
                    "{} facing {} into {}: needed {:.0}% equity, had {:.0}% vs the range{}",
                    d.street,
                    fmt_amount(d.to_call, r),
                    fmt_amount(d.pot, r),
                    req * 100.0,
                    eq * 100.0,
                    if d.opponents > 1 { "s" } else { "" }
                );
                let against = if strict {
                    "Pot odds vs your equity against the bots' actual ranges (river or all-in: no later betting)"
                } else {
                    "Pot odds vs your equity against the bots' actual ranges (flop/turn: ignores later betting, so only large gaps are flagged)"
                };
                match d.action {
                    Action::Fold => add(
                        if strict { "folded_with_odds" } else { "folded_with_odds_early" },
                        if strict {
                            "Folded with enough equity (river/all-in)"
                        } else {
                            "Folded with much more equity than needed (flop/turn)"
                        },
                        "Your equity against the range you faced was above what the pot odds required.",
                        against,
                        eq >= req + margin,
                        ex(text.clone()),
                    ),
                    Action::Call => add(
                        if strict { "called_without_odds" } else { "called_without_odds_early" },
                        if strict {
                            "Called without enough equity (river/all-in)"
                        } else {
                            "Called with much less equity than needed (flop/turn)"
                        },
                        "Your equity against the range you faced was below what the pot odds required.",
                        against,
                        eq <= req - margin,
                        ex(text),
                    ),
                    _ => {}
                }
            }
        }
    }
    stats.aggression_factor = (post_calls > 0).then(|| f64::from(post_aggr) / f64::from(post_calls));
    let hands = records.len() as u32;
    let mut findings: Vec<Finding> = findings.into_values().filter(|f| f.count > 0).collect();
    findings.sort_by(|a, b| (b.count as f64 / b.out_of as f64).total_cmp(&(a.count as f64 / a.out_of as f64)));
    let mut caveats = vec![
        "The bots are not perfect players: preflop they follow an approximate model, postflop simple rules. \
         Findings show where you differed from those references, not proof of mistakes against every opponent."
            .to_string(),
        "Bet and raise decisions after the flop are not graded: that needs a solver for the exact spot.".to_string(),
    ];
    if hands < 200 {
        caveats.push(format!(
            "{hands} hands is a small sample: results (and bb/100) swing a lot from luck. Use the all-in line to see some of it."
        ));
    }
    if tournament {
        caveats.push(
            "Sit & go results are in chips; findings use chip equity, not ICM (tournament payout equity).".into(),
        );
    }
    Report {
        hands,
        units: if tournament { "chips" } else { "bb" }.into(),
        net,
        net_bb,
        bb_per_100: (hands > 0).then(|| net_bb / f64::from(hands) * 100.0),
        stats,
        by_position: by_pos.into_values().collect(),
        model_comparison: (cmp.0 > 0).then(|| ModelComparison {
            decisions: cmp.0,
            voluntary_you: cmp.1 / f64::from(cmp.0),
            voluntary_model: cmp.2 / f64::from(cmp.0),
            raise_you: cmp.3 / f64::from(cmp.0),
            raise_model: cmp.4 / f64::from(cmp.0),
        }),
        all_in: (luck.hands > 0).then_some(luck),
        findings,
        caveats,
    }
}

fn fmt_amount(x: u64, r: &HandRecord) -> String {
    if r.tournament { x.to_string() } else { poker_core::Chips(x).to_string() }
}
