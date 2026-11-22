//! Range text notation.
//!
//! Grammar (comma or whitespace separated, case-insensitive ranks/suits):
//! - `AA`, `AKs`, `AKo`, `AK` (= suited + offsuit), `AsKh` (single combo)
//! - `TT+` (TT through AA), `ATs+` / `ATo+` / `AT+` (kicker up to one below the top card)
//! - `QQ-99`, `A5s-A2s`, `KTo-K7o` (inclusive; both ends share the top card for non-pairs)
//! - any item may carry `:w` with `w` in `[0, 1]`; later items override earlier ones.

use crate::class::{HandClass, HandKind};
use crate::combo::Combo;
use crate::range::{Range, RangeError, check_weight};
use poker_core::{Card, Rank};
use std::str::FromStr;

impl Range {
    pub fn parse(s: &str) -> Result<Range, RangeError> {
        let mut r = Range::empty();
        for token in s.split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()) {
            let (item, w) = match token.split_once(':') {
                Some((i, w)) => {
                    let wv: f64 = w.parse().map_err(|_| RangeError::InvalidWeight(w.to_string()))?;
                    (i, check_weight(wv)?)
                }
                None => (token, 1.0),
            };
            for combo in expand(item)? {
                r.set_weight(combo, w)?;
            }
        }
        Ok(r)
    }

    /// Canonical, compact notation. `Range::parse(&r.to_notation()) == r` for every range.
    pub fn to_notation(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        // Pairs: AA down to 22.
        let pair_items: Vec<(Rank, Option<f64>)> = (0..13)
            .rev()
            .map(|i| {
                let r = Rank::from_index(i).expect("<13");
                (r, uniform(self, HandClass::new(r, r, HandKind::Pair).expect("pair")))
            })
            .collect();
        emit_runs(&mut parts, &pair_items, |hi, lo, top| match (hi == lo, top) {
            (true, _) => format!("{}{}", hi.to_char(), hi.to_char()),
            (false, true) => format!("{}{}+", lo.to_char(), lo.to_char()),
            (false, false) => format!("{0}{0}-{1}{1}", hi.to_char(), lo.to_char()),
        });
        for hi_i in (1..13).rev() {
            let hi = Rank::from_index(hi_i).expect("<13");
            for (kind, sfx) in [(HandKind::Suited, 's'), (HandKind::Offsuit, 'o')] {
                let items: Vec<(Rank, Option<f64>)> = (0..hi_i)
                    .rev()
                    .map(|lo_i| {
                        let lo = Rank::from_index(lo_i).expect("<13");
                        (lo, uniform(self, HandClass::new(hi, lo, kind).expect("non-pair")))
                    })
                    .collect();
                let h = hi.to_char();
                emit_runs(&mut parts, &items, |a, b, top| match (a == b, top) {
                    (true, _) => format!("{h}{}{sfx}", a.to_char()),
                    (false, true) => format!("{h}{}{sfx}+", b.to_char()),
                    (false, false) => format!("{h}{}{sfx}-{h}{}{sfx}", a.to_char(), b.to_char()),
                });
            }
        }
        // Classes with mixed per-combo weights are written combo by combo.
        for class in HandClass::all() {
            if uniform(self, class).is_none() {
                for c in class.combos() {
                    let w = self.weight(c);
                    if w > 0.0 {
                        parts.push(with_weight(c.to_string(), w));
                    }
                }
            }
        }
        parts.join(",")
    }
}

/// `Some(w)` if every combo of the class has the same weight `w`, else `None`.
fn uniform(r: &Range, class: HandClass) -> Option<f64> {
    let combos = class.combos();
    let w = r.weight(combos[0]);
    combos.iter().all(|c| r.weight(*c) == w).then_some(w)
}

fn with_weight(s: String, w: f64) -> String {
    if w == 1.0 { s } else { format!("{s}:{w}") }
}

/// Emits maximal runs of consecutive items (ordered from the top rank down) that share the
/// same non-zero uniform weight. `fmt(start, end, starts_at_top)`.
fn emit_runs(out: &mut Vec<String>, items: &[(Rank, Option<f64>)], fmt: impl Fn(Rank, Rank, bool) -> String) {
    let mut i = 0;
    while i < items.len() {
        let Some(w) = items[i].1.filter(|w| *w > 0.0) else {
            i += 1;
            continue;
        };
        let mut j = i;
        while j + 1 < items.len() && items[j + 1].1 == Some(w) {
            j += 1;
        }
        out.push(with_weight(fmt(items[i].0, items[j].0, i == 0 && j > i), w));
        i = j + 1;
    }
}

fn bad(s: &str) -> RangeError {
    RangeError::InvalidToken(s.to_string())
}

fn rank(c: char, item: &str) -> Result<Rank, RangeError> {
    Rank::from_char(c).map_err(|_| bad(item))
}

/// Parses a hand-class-like token: returns (high, low, kinds).
fn class_token(t: &str, item: &str) -> Result<(Rank, Rank, Vec<HandKind>), RangeError> {
    let c: Vec<char> = t.chars().collect();
    let (a, b) = match c.as_slice() {
        [a, b] | [a, b, _] => (rank(*a, item)?, rank(*b, item)?),
        _ => return Err(bad(item)),
    };
    let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
    let kinds = match (c.get(2).map(|x| x.to_ascii_lowercase()), hi == lo) {
        (None, true) => vec![HandKind::Pair],
        (None, false) => vec![HandKind::Suited, HandKind::Offsuit],
        (Some('s'), false) => vec![HandKind::Suited],
        (Some('o'), false) => vec![HandKind::Offsuit],
        _ => return Err(bad(item)),
    };
    Ok((hi, lo, kinds))
}

fn classes_to_combos(classes: impl IntoIterator<Item = HandClass>) -> Vec<Combo> {
    classes.into_iter().flat_map(|c| c.combos()).collect()
}

fn expand(item: &str) -> Result<Vec<Combo>, RangeError> {
    let chars: Vec<char> = item.chars().collect();
    // Exact combo, e.g. AsKh.
    if chars.len() == 4 && chars[1].is_ascii_alphabetic() && "cdhsCDHS".contains(chars[1]) {
        let cards = Card::parse_many(item).map_err(|_| bad(item))?;
        return Combo::new(cards[0], cards[1]).map(|c| vec![c]).ok_or_else(|| bad(item));
    }
    if let Some(base) = item.strip_suffix('+') {
        let (hi, lo, kinds) = class_token(base, item)?;
        if hi == lo {
            return Ok(classes_to_combos((lo.index()..13).map(|i| {
                let r = Rank::from_index(i).expect("<13");
                HandClass::new(r, r, HandKind::Pair).expect("pair")
            })));
        }
        let mut v = Vec::new();
        for k in kinds {
            for i in lo.index()..hi.index() {
                v.push(HandClass::new(hi, Rank::from_index(i).expect("<13"), k).expect("non-pair"));
            }
        }
        return Ok(classes_to_combos(v));
    }
    if let Some((a, b)) = item.split_once('-') {
        let (h1, l1, k1) = class_token(a, item)?;
        let (h2, l2, k2) = class_token(b, item)?;
        if k1 != k2 {
            return Err(bad(item));
        }
        if h1 == l1 && h2 == l2 {
            let (top, bot) = if h1 >= h2 { (h1, h2) } else { (h2, h1) };
            return Ok(classes_to_combos((bot.index()..=top.index()).map(|i| {
                let r = Rank::from_index(i).expect("<13");
                HandClass::new(r, r, HandKind::Pair).expect("pair")
            })));
        }
        if h1 != h2 || h1 == l1 || h2 == l2 {
            return Err(bad(item));
        }
        let (top, bot) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
        let mut v = Vec::new();
        for k in k1 {
            for i in bot.index()..=top.index() {
                v.push(HandClass::new(h1, Rank::from_index(i).expect("<13"), k).expect("non-pair"));
            }
        }
        return Ok(classes_to_combos(v));
    }
    let (hi, lo, kinds) = class_token(item, item)?;
    Ok(classes_to_combos(kinds.into_iter().map(|k| HandClass::new(hi, lo, k).expect("valid"))))
}

impl FromStr for Range {
    type Err = RangeError;
    fn from_str(s: &str) -> Result<Range, RangeError> {
        Range::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combo::COMBO_COUNT;
    use proptest::prelude::*;

    fn count(s: &str) -> f64 {
        Range::parse(s).unwrap().combo_count()
    }

    #[test]
    fn combo_counts_of_common_tokens() {
        assert_eq!(count("AA"), 6.0);
        assert_eq!(count("AKs"), 4.0);
        assert_eq!(count("AKo"), 12.0);
        assert_eq!(count("AK"), 16.0);
        assert_eq!(count("AsKh"), 1.0);
        assert_eq!(count("TT+"), 30.0);
        assert_eq!(count("22+"), 78.0);
        assert_eq!(count("ATs+"), 16.0);
        assert_eq!(count("A2s+"), 48.0);
        assert_eq!(count("KTo+"), 36.0);
        assert_eq!(count("QQ-99"), 24.0);
        assert_eq!(count("99-QQ"), 24.0);
        assert_eq!(count("A5s-A2s"), 16.0);
        assert_eq!(count("AKs:0.5"), 2.0);
        assert_eq!(count("AA, KK,QQ"), 18.0);
        assert_eq!(count("AA,AA"), 6.0, "duplicates do not double count");
        assert_eq!(count("AA,AA:0.5"), 3.0, "later items override");
    }

    #[test]
    fn rejects_invalid_tokens() {
        for bad in ["AAs", "AKx", "A", "AsAs", "AKs-QJs", "AA-AKs", "AKs:2", "AKs:x", "1A", "ZZ+"] {
            assert!(Range::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn canonical_notation() {
        assert_eq!(Range::parse("KK,AA,QQ").unwrap().to_notation(), "QQ+");
        assert_eq!(Range::parse("JJ,TT,99").unwrap().to_notation(), "JJ-99");
        assert_eq!(Range::parse("AKs,AQs,AJs").unwrap().to_notation(), "AJs+");
        assert_eq!(Range::parse("A5s-A2s,KQo:0.5").unwrap().to_notation(), "A5s-A2s,KQo:0.5");
        assert_eq!(Range::parse("AsKs").unwrap().to_notation(), "AsKs");
        assert_eq!(Range::full().to_notation(), {
            let mut p = vec!["22+".to_string()];
            for h in ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4'] {
                p.push(format!("{h}2s+"));
                p.push(format!("{h}2o+"));
            }
            p.push("32s".into());
            p.push("32o".into());
            p.join(",")
        });
    }

    proptest! {
        #[test]
        fn notation_round_trips(ws in proptest::collection::vec(
            prop_oneof![4 => Just(0.0), 3 => Just(1.0), 1 => Just(0.5), 1 => 0.0f64..=1.0], COMBO_COUNT)) {
            // Make classes mostly uniform so compression paths are exercised too.
            let mut r = Range::empty();
            for class in HandClass::all() {
                let combos = class.combos();
                let first = ws[combos[0].index()];
                for c in combos {
                    let w = if ws[c.index()] == 0.5 { ws[c.index()] } else { first };
                    r.set_weight(c, w).unwrap();
                }
            }
            let text = r.to_notation();
            prop_assert_eq!(Range::parse(&text).unwrap(), r);
        }
    }
}
