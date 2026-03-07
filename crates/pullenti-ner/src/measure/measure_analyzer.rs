/// Measure analyzer — simplified port of MeasureAnalyzer.cs.
///
/// Handles the most common pattern: NumberToken → unit word/abbreviation.
/// Also handles compound abbreviations like "кв.м.", "куб.м.", speed "м/с", "км/ч".

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::measure::measure_referent as mr;
use crate::measure::unit_table;

pub struct MeasureAnalyzer;

impl MeasureAnalyzer {
    pub fn new() -> Self { MeasureAnalyzer }
}

impl Analyzer for MeasureAnalyzer {
    fn name(&self) -> &'static str { "MEASURE" }
    fn caption(&self) -> &'static str { "Измеряемые величины" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }
            match try_parse(&t, &sofa) {
                None => { cur = t.borrow().next.clone(); }
                Some((referent, end)) => {
                    let r_rc = Rc::new(RefCell::new(referent));
                    kit.add_entity(r_rc.clone());
                    let tok = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), end, r_rc)
                    ));
                    kit.embed_token(tok.clone());
                    cur = tok.borrow().next.clone();
                }
            }
        }
    }
}

// ── TryParse ─────────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Must start with a number
    if !is_number_token(t) { return None; }

    let (value_str, end_num) = get_number_string(t, sofa);

    // Try to find a unit starting at the token right after the number
    let after = end_num.borrow().next.clone()?;

    // Try compound abbreviations first (e.g. "кв.м.", "куб.м.", "м/с", "км/ч")
    if let Some((unit_name, kind, unit_end)) = try_compound_unit(&after, sofa) {
        return build_measure(&value_str, &unit_name, kind, unit_end);
    }

    // Try single token unit
    if let Some((unit_name, kind, unit_end)) = try_single_unit(&after, sofa) {
        return build_measure(&value_str, &unit_name, kind, unit_end);
    }

    None
}

fn build_measure(value: &str, unit_name: &str, kind_str: &str, end: TokenRef) -> Option<(Referent, TokenRef)> {
    let mut r = mr::new_measure_referent();
    mr::add_value(&mut r, value);
    mr::set_unit(&mut r, unit_name);
    mr::set_kind(&mut r, kind_str);
    Some((r, end))
}

// ── Number extraction ─────────────────────────────────────────────────────────

fn is_number_token(t: &TokenRef) -> bool {
    matches!(t.borrow().kind, TokenKind::Number(_))
}

/// Get the number value string and the end token of the number (including optional decimal).
fn get_number_string(t: &TokenRef, sofa: &SourceOfAnalysis) -> (String, TokenRef) {
    let int_str = match &t.borrow().kind {
        TokenKind::Number(n) => n.value.clone(),
        _ => return ("0".to_string(), t.clone()),
    };

    // Check for decimal separator directly adjacent
    let next = t.borrow().next.clone();
    if let Some(ref sep) = next {
        let sep_b = sep.borrow();
        if sep_b.whitespaces_before_count(sofa) == 0 && sep_b.length_char() == 1 {
            let sep_ch = sofa.char_at(sep_b.begin_char);
            if sep_ch == ',' || sep_ch == '.' {
                let after_sep = sep_b.next.clone();
                drop(sep_b);
                if let Some(ref frac_tok) = after_sep {
                    let fb = frac_tok.borrow();
                    if fb.whitespaces_before_count(sofa) == 0 {
                        if let TokenKind::Number(n) = &fb.kind {
                            let frac_str = n.value.clone();
                            let full = format!("{}.{}", int_str, frac_str);
                            drop(fb);
                            return (full, frac_tok.clone());
                        }
                    }
                }
            }
        }
    }

    (int_str, t.clone())
}

// ── Unit detection ────────────────────────────────────────────────────────────

/// Try to match a multi-token compound unit abbreviation like "кв.м.", "куб.м.", "м/с", "км/ч".
fn try_compound_unit(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(String, &'static str, TokenRef)> {
    // Build a string from up to 5 adjacent tokens with no spaces (or single /)
    let mut s = String::new();
    let mut end = t.clone();
    let mut cur: Option<TokenRef> = Some(t.clone());
    let mut count = 0;

    while let Some(tok) = cur.take() {
        let tb = tok.borrow();
        // Stop if whitespace before this token (except for the first)
        if count > 0 && tb.whitespaces_before_count(sofa) > 0 { break; }
        let text = sofa.substring(tb.begin_char, tb.end_char);
        s.push_str(&text.to_uppercase());
        let next = tb.next.clone();
        drop(tb);
        end = tok;
        count += 1;
        if count >= 5 { break; }
        // Check if current accumulated string is already a known unit
        if let Some(info) = unit_table::lookup(&s) {
            return Some((info.canonical.to_string(), info.kind.as_str(), end));
        }
        cur = next;
    }

    // Final check on the accumulated string
    if let Some(info) = unit_table::lookup(&s) {
        return Some((info.canonical.to_string(), info.kind.as_str(), end));
    }

    None
}

/// Try to match a single token as a unit (with morph normalization).
fn try_single_unit(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, &'static str, TokenRef)> {
    let tb = t.borrow();
    match &tb.kind {
        TokenKind::Text(txt) => {
            let term = txt.term.to_uppercase();
            // Try morph normal forms first
            let mut candidates = Vec::new();
            for wf in t.borrow().morph.items() {
                if let Some(nc) = &wf.normal_case { candidates.push(nc.to_uppercase()); }
                if let Some(nf) = &wf.normal_full  { candidates.push(nf.to_uppercase()); }
            }
            candidates.push(term);
            let surface = sofa.substring(tb.begin_char, tb.end_char).to_uppercase();
            candidates.push(surface);
            drop(tb);
            for c in candidates {
                if let Some(info) = unit_table::lookup(&c) {
                    return Some((info.canonical.to_string(), info.kind.as_str(), t.clone()));
                }
            }
            // Try two-word unit (e.g. "лошадиная сила", "квадратный метр")
            let next = t.borrow().next.clone()?;
            let tb2 = next.borrow();
            if tb2.whitespaces_before_count(sofa) <= 1 {
                if let TokenKind::Text(txt2) = &tb2.kind {
                    let phrase = format!("{} {}", t.borrow().term().unwrap_or("").to_uppercase(), txt2.term.to_uppercase());
                    drop(tb2);
                    if let Some(info) = unit_table::lookup(&phrase) {
                        return Some((info.canonical.to_string(), info.kind.as_str(), next.clone()));
                    }
                }
            }
            None
        }
        _ => None,
    }
}
