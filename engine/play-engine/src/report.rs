//! End-of-session report: your statistics, and weaknesses measured against stated references
//! (the preflop model's frequencies, and pot odds against the bots' tracked ranges). Nothing
//! here compares you with invented "typical player" numbers.

use crate::judge::judge;
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
    /// Index of the hand in the records the report was built from.
    pub record: usize,
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
    /// Your river play against bots given a leak (docs/play.md, "Practice leaks").
    pub leak_practice: Vec<LeakPractice>,
    /// Spot-the-leak practice: every bot and the leak it really had (the interface asks for
    /// your guess before showing these and `leak_practice`).
    pub hidden_leaks: Vec<HiddenLeak>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HiddenLeak {
    pub name: String,
    pub leak: crate::config::Leak,
}

/// How you played the river against bots with one documented leak.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeakPractice {
    pub leak: crate::config::Leak,
    pub label: String,
    /// What the best response to this leak does, in words.
    pub best_response: String,
    /// River calls and folds facing this bot's bet heads-up (raises left out)…
    pub decisions: u32,
    /// …and how many matched the best response for that decision: call exactly when your
    /// equity against the bot's actual range is at least the price.
    pub matched: u32,
    pub examples: Vec<Example>,
    /// Each graded decision in order: (hand number, matched the best response), to see
    /// whether you adjusted as the session went on.
    pub timeline: Vec<(u64, bool)>,
    /// Your river bets and raises this bot faced, and how often its range (as you could see it)
    /// would have folded on average, against what a pure bluff of each bet needed.
    pub bets: u32,
    pub mean_fold_share: Option<f64>,
    pub mean_break_even: Option<f64>,
    /// Bets where the bot's range folded at least as often as a pure bluff needed.
    pub bets_profitable_as_bluffs: u32,
}

fn best_response(leak: crate::config::Leak) -> &'static str {
    use crate::config::Leak;
    match leak {
        Leak::None => "",
        Leak::OverbluffsRiver => {
            "It bluffs more than the price allows, so calling with any hand that beats its bluffs gains: \
             the best response calls far more than the minimum defence frequency."
        }
        Leak::NeverBluffsRiver => {
            "Its river bets are all value, so the best response folds every hand that only beats bluffs."
        }
        Leak::OverfoldsRiver => {
            "It folds more than a bet's break-even share, so the best response bets the river with \
             more hands, bluffs included."
        }
        Leak::CallsTooMuchRiver => {
            "It calls more than the minimum defence frequency, so the best response bets thinner for \
             value and stops bluffing."
        }
    }
}

/// Spot-the-leak sessions: each bot's name and real leak, by name.
fn hidden_leaks(records: &[HandRecord]) -> Vec<HiddenLeak> {
    let mut out: Vec<HiddenLeak> = vec![];
    for seat in records.iter().flat_map(|r| &r.seats).filter(|s| s.leak_hidden) {
        if !out.iter().any(|h| h.name == seat.name) {
            out.push(HiddenLeak { name: seat.name.clone(), leak: seat.leak });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// River play against leaky bots: graded calls and folds, and the fold shares your bets met.
fn leak_practice(records: &[HandRecord]) -> Vec<LeakPractice> {
    use crate::config::Leak;
    let mut out: BTreeMap<String, (Leak, LeakPractice, f64, f64)> = BTreeMap::new();
    let entry = |out: &mut BTreeMap<String, (Leak, LeakPractice, f64, f64)>, leak: Leak| {
        out.entry(leak.label().to_string())
            .or_insert_with(|| {
                (
                    leak,
                    LeakPractice {
                        leak,
                        label: leak.label().into(),
                        best_response: best_response(leak).into(),
                        decisions: 0,
                        matched: 0,
                        examples: vec![],
                        timeline: vec![],
                        bets: 0,
                        mean_fold_share: None,
                        mean_break_even: None,
                        bets_profitable_as_bluffs: 0,
                    },
                    0.0,
                    0.0,
                )
            })
            .clone()
    };
    for (ri, r) in records.iter().enumerate() {
        let hero = r.hero_seat();
        let mut next = 0usize;
        for (ai, a) in r.actions.iter().enumerate() {
            if a.seat != hero {
                continue;
            }
            let d = r.hero_decisions.get(next);
            next += 1;
            if a.street != "river" {
                continue;
            }
            // Facing a bet: who made it?
            if let Some(d) = d.filter(|d| d.to_call > 0 && d.opponents == 1) {
                let bettor = r.actions[..ai]
                    .iter()
                    .rev()
                    .take_while(|b| b.street == "river")
                    .find(|b| b.seat != hero && is_aggressive(&b.action))
                    .map(|b| b.seat);
                let leak = bettor.map_or(Leak::None, |b| r.seats[b].leak);
                if let (true, Some(eq), Some(req)) = (leak != Leak::None, d.equity_vs_ranges, d.required_equity) {
                    if matches!(a.action, Action::Call | Action::Fold) {
                        let mut e = entry(&mut out, leak);
                        let good = (a.action == Action::Call) == (eq >= req);
                        e.1.decisions += 1;
                        e.1.matched += u32::from(good);
                        e.1.timeline.push((r.hand_no, good));
                        if !good && e.1.examples.len() < MAX_EXAMPLES {
                            e.1.examples.push(Example {
                                record: ri,
                                hand_no: r.hand_no,
                                street: "river".into(),
                                text: format!(
                                    "{} facing {}: needed {:.0}% equity, had {:.0}% vs its range",
                                    if a.action == Action::Call { "called" } else { "folded" },
                                    fmt_amount(d.to_call, r),
                                    req * 100.0,
                                    eq * 100.0
                                ),
                            });
                        }
                        out.insert(leak.label().into(), e);
                    }
                }
            }
            // Your bet: how did each leaky bot's range respond?
            if is_aggressive(&a.action) {
                for b in r.actions[ai + 1..].iter().take_while(|b| b.street == "river" && b.seat != hero) {
                    let leak = r.seats[b.seat].leak;
                    if let (true, Some(f), Some(be)) = (leak != Leak::None, b.fold_share, b.bluff_break_even) {
                        let mut e = entry(&mut out, leak);
                        e.1.bets += 1;
                        e.1.bets_profitable_as_bluffs += u32::from(f >= be);
                        e.2 += f;
                        e.3 += be;
                        out.insert(leak.label().into(), e);
                    }
                }
            }
        }
    }
    out.into_values()
        .map(|(_, mut p, f, be)| {
            if p.bets > 0 {
                p.mean_fold_share = Some(f / f64::from(p.bets));
                p.mean_break_even = Some(be / f64::from(p.bets));
            }
            p
        })
        .collect()
}

fn is_aggressive(a: &Action) -> bool {
    matches!(a, Action::Bet { .. } | Action::Raise { .. })
}

/// Hands kept per finding, to browse in the app.
pub const MAX_EXAMPLES: usize = 200;

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
                if f.examples.len() < MAX_EXAMPLES {
                    f.examples.push(e);
                }
            }
        }
    };

    for (ri, r) in records.iter().enumerate() {
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
            let j = judge(d, &r.seats[h].position, |x| fmt_amount(x, r));
            if let Some(m) = j.model {
                cmp.0 += 1;
                cmp.1 += f64::from(u8::from(m.you_voluntary));
                cmp.2 += m.model_voluntary;
                cmp.3 += f64::from(u8::from(m.you_raise));
                cmp.4 += m.model_raise;
            }
            for c in j.checks {
                let example =
                    c.hit.then(|| Example { record: ri, hand_no: r.hand_no, street: d.street.clone(), text: c.text });
                add(c.key, c.title, c.detail, c.reference, c.hit, example);
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
    let revealed = records.iter().filter(|r| r.revealed).count();
    if revealed > 0 {
        caveats.push(format!(
            "{revealed} of {hands} hands were played with the bots' cards revealed; decisions in them were easier than normal."
        ));
    }
    if tournament {
        caveats.push(
            "Sit & go results are in chips. Heads-up all-in calls and folds are graded with ICM (the Malmuth–Harville model) for the payouts; every other finding uses chip equity.".into(),
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
        leak_practice: leak_practice(records),
        hidden_leaks: hidden_leaks(records),
        caveats,
    }
}

fn fmt_amount(x: u64, r: &HandRecord) -> String {
    if r.tournament { x.to_string() } else { poker_core::Chips(x).to_string() }
}
