/// DecreeAnalyzer — simplified port of DecreeAnalyzer.cs.
///
/// Recognizes patterns like:
///   "Федеральный закон № 123-ФЗ"   → DECREE, kind=Law, type=Федеральный закон, number=123-ФЗ
///   "Приказ Минфина № 45"           → DECREE, kind=Order, type=Приказ, number=45
///   "Постановление № 567 от 01.01.2024" → DECREE, kind=Order, type=Постановление, number=567
///   "ГОСТ 12345-2020"               → DECREE, kind=Standard, type=ГОСТ, number=12345-2020
///   "ISO 9001:2015"                 → DECREE, kind=Standard, type=ISO, number=9001:2015
///   "Уголовный кодекс"              → DECREE, kind=Kodex, type=Уголовный кодекс
///   "Конституция Российской Федерации" → DECREE, kind=Ustav

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::decree::decree_referent as dr;
use crate::decree::decree_referent::DecreeKind;
use crate::decree::decree_table;

pub struct DecreeAnalyzer;

impl DecreeAnalyzer {
    pub fn new() -> Self { DecreeAnalyzer }
}

impl Analyzer for DecreeAnalyzer {
    fn name(&self) -> &'static str { "DECREE" }
    fn caption(&self) -> &'static str { "Нормативные акты" }

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

// ── Main parse entry ──────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();
    let TokenKind::Text(_) = &tb.kind else { return None; };
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    let starts_upper = surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
    let uppers = collect_upper_forms(&tb);
    drop(tb);

    // Pattern A: standard abbreviation (ГОСТ, ISO, ТУ) + number
    // Must be ALL-CAPS and start uppercase — these are typically spelled as abbr
    if starts_upper {
        for up in &uppers {
            if let Some(result) = try_standard_pattern(t, up, sofa) {
                return Some(result);
            }
        }
    }

    // Pattern B: decree type keyword + optional number
    // Can appear lowercase mid-sentence (закон, приказ, кодекс, etc.)
    for up in &uppers {
        if let Some(result) = try_type_keyword_pattern(t, up, sofa) {
            return Some(result);
        }
    }

    None
}

// ── Pattern A: standard abbreviation (ГОСТ 12345-2020, ISO 9001:2015) ─────────

fn try_standard_pattern(t: &TokenRef, upper: &str, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Check for "ГОСТ Р" two-token pattern first
    let entry = decree_table::lookup_type(upper)?;
    if entry.kind != DecreeKind::Standard { return None; }
    let mut end = t.clone();
    let canonical_type = entry.canonical;

    // For "ГОСТ", try "ГОСТ Р" two-token variant
    if upper == "ГОСТ" {
        if let Some(next) = t.borrow().next.clone() {
            let nb = next.borrow();
            let surf2 = sofa.substring(nb.begin_char, nb.end_char);
            if nb.whitespaces_before_count(sofa) <= 1 && surf2 == "Р" {
                // Check if "Р" is followed by number
                if let Some(after_r) = nb.next.clone() {
                    drop(nb);
                    let (num, num_end) = try_standard_number(&after_r, sofa)?;
                    let mut r = dr::new_decree_referent();
                    dr::set_kind(&mut r, &DecreeKind::Standard);
                    dr::add_slot_str(&mut r, dr::ATTR_TYPE, "ГОСТ Р");
                    dr::add_slot_str(&mut r, dr::ATTR_NUMBER, &num);
                    return Some((r, num_end));
                }
            }
        }
    }

    // Standard number must follow immediately (0-1 space)
    let next = t.borrow().next.clone()?;
    let (num, num_end) = try_standard_number(&next, sofa)?;
    end = num_end;

    let mut r = dr::new_decree_referent();
    dr::set_kind(&mut r, &DecreeKind::Standard);
    dr::add_slot_str(&mut r, dr::ATTR_TYPE, canonical_type);
    dr::add_slot_str(&mut r, dr::ATTR_NUMBER, &num);
    Some((r, end))
}

/// Try to consume a standard number (e.g. "12345-2020", "9001:2015", "R 52537").
/// Returns (number_string, end_token) or None.
fn try_standard_number(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let tb = t.borrow();
    if tb.whitespaces_before_count(sofa) > 1 { return None; }

    match &tb.kind {
        TokenKind::Number(n) => {
            let val = n.value.clone();
            let mut end = t.clone();
            let next = tb.next.clone();
            drop(tb);
            // Allow dash/colon continuation: "12345-2020", "9001:2015"
            if let Some(sep) = next {
                let sb = sep.borrow();
                if sb.whitespaces_before_count(sofa) == 0 && sb.length_char() == 1 {
                    let ch = sofa.char_at(sb.begin_char);
                    if ch == '-' || ch == ':' || ch == '.' {
                        if let Some(after_sep) = sb.next.clone() {
                            drop(sb);
                            let ab = after_sep.borrow();
                            let (is_num, num_str) = match &ab.kind {
                                TokenKind::Number(n2) => (true, n2.value.clone()),
                                TokenKind::Text(txt2) => {
                                    let s = txt2.term.clone();
                                    let ok = s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
                                    (ok, s)
                                }
                                _ => (false, String::new()),
                            };
                            if is_num {
                                let combined = format!("{}{}{}", val, ch, num_str);
                                end = after_sep.clone();
                                drop(ab);
                                return Some((combined, end));
                            }
                            drop(ab);
                        }
                    }
                }
            }
            Some((val, end))
        }
        TokenKind::Text(txt) => {
            let surf = sofa.substring(tb.begin_char, tb.end_char);
            // Must start with digit for standard numbers
            if surf.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                let val = surf.to_string();
                let end = t.clone();
                drop(tb);
                return Some((val, end));
            }
            drop(tb);
            None
        }
        _ => { drop(tb); None }
    }
}

// ── Pattern B: type keyword [adjective qualifier] [number] ────────────────────

fn try_type_keyword_pattern(t: &TokenRef, upper: &str, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let entry = decree_table::lookup_type(upper)?;
    // Skip standard entries here (handled by Pattern A)
    if entry.kind == DecreeKind::Standard { return None; }

    let canonical_type = entry.canonical;
    let kind = entry.kind.clone();
    let mut end = t.clone();

    // Try to consume a two-word type phrase (e.g. "Федеральный закон", "Гражданский кодекс")
    // by checking if the NEXT token extends the type phrase
    let next_opt = t.borrow().next.clone();
    if let Some(next_t) = next_opt {
        let nb = next_t.borrow();
        if nb.whitespaces_before_count(sofa) <= 1 {
            if let TokenKind::Text(_) = &nb.kind {
                let next_uppers = collect_upper_forms(&nb);
                drop(nb);
                for nu in &next_uppers {
                    let two_word = format!("{} {}", upper, nu);
                    if let Some(e2) = decree_table::lookup_type(&two_word) {
                        if e2.kind != DecreeKind::Standard {
                            // Use the two-word entry instead
                            end = next_t.clone();
                            let canonical2 = e2.canonical;
                            let kind2 = e2.kind.clone();
                            return build_decree_result(&end, canonical2, kind2, sofa);
                        }
                    }
                }
            } else {
                drop(nb);
            }
        } else {
            drop(nb);
        }
    }

    build_decree_result(&end, canonical_type, kind, sofa)
}

/// After matching a type keyword (ending at `type_end`), try to consume
/// an optional number (№ NNN) and return the complete Referent.
fn build_decree_result(
    type_end: &TokenRef,
    canonical_type: &str,
    kind: DecreeKind,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let mut r = dr::new_decree_referent();
    dr::set_kind(&mut r, &kind);
    dr::add_slot_str(&mut r, dr::ATTR_TYPE, canonical_type);

    let mut end = type_end.clone();

    // Look for "№ NUMBER" or "N NUMBER" or "от" (date) after the type keyword
    // We allow skipping up to 3 tokens of "filler" like org names before finding the number
    let mut probe = type_end.borrow().next.clone();
    let mut skip = 0;
    while let Some(pt) = probe.clone() {
        if skip > 5 { break; }
        let pb = pt.borrow();
        if pb.whitespaces_before_count(sofa) > 2 && skip == 0 {
            break; // big gap right after type keyword
        }
        match &pb.kind {
            TokenKind::Text(txt) => {
                let surf = sofa.substring(pb.begin_char, pb.end_char);
                let up = txt.term.to_uppercase();
                // "№" or "N" or "NO" signals a number
                if up == "№" || surf == "№" || up == "N" || up == "NO" || up == "НОМ" || up == "НОМЕР" {
                    let after = pb.next.clone();
                    drop(pb);
                    if let Some(num_tok) = after {
                        if let Some((num_str, num_end)) = try_decree_number(&num_tok, sofa) {
                            dr::add_slot_str(&mut r, dr::ATTR_NUMBER, &num_str);
                            end = num_end;
                        }
                    }
                    break;
                }
                // Stop on sentence-boundary markers
                if up == "ОТ" || up == "О" || up == "ОБ" || up == "ОБО" {
                    // "от" can precede a date, "о"/"об" precedes document title
                    break;
                }
                let next = pb.next.clone();
                drop(pb);
                probe = next;
                skip += 1;
            }
            TokenKind::Number(_) => {
                // A bare number right after type keyword — could be the reference number
                // Only take it if skip==0 or we just passed the type keyword
                if skip == 0 {
                    if let Some((num_str, num_end)) = try_decree_number(&pt, sofa) {
                        dr::add_slot_str(&mut r, dr::ATTR_NUMBER, &num_str);
                        end = num_end;
                    }
                }
                break;
            }
            _ => break,
        }
    }

    Some((r, end))
}

/// Try to parse a decree registration number.
/// Formats: "123", "123-ФЗ", "123/456", "123-А"
fn try_decree_number(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let tb = t.borrow();
    let (base, tok_end) = match &tb.kind {
        TokenKind::Number(n) => (n.value.clone(), t.clone()),
        TokenKind::Text(txt) => {
            let surf = sofa.substring(tb.begin_char, tb.end_char);
            // Allow mixed like "123-А" starting with digit
            if surf.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                (surf.to_string(), t.clone())
            } else {
                drop(tb);
                return None;
            }
        }
        _ => { drop(tb); return None; }
    };
    let next = tb.next.clone();
    drop(tb);

    // Try to consume suffix: "-ФЗ", "-П", etc.
    if let Some(sep) = next {
        let sb = sep.borrow();
        if sb.whitespaces_before_count(sofa) == 0 && sb.length_char() == 1 {
            let ch = sofa.char_at(sb.begin_char);
            if ch == '-' || ch == '/' {
                if let Some(after) = sb.next.clone() {
                    drop(sb);
                    let ab = after.borrow();
                    let after_surf = sofa.substring(ab.begin_char, ab.end_char);
                    if ab.whitespaces_before_count(sofa) == 0 {
                        let combined = format!("{}{}{}", base, ch, after_surf);
                        let end = after.clone();
                        drop(ab);
                        return Some((combined, end));
                    }
                    drop(ab);
                }
            }
        }
    }

    Some((base, tok_end))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn collect_upper_forms(tb: &Token) -> Vec<String> {
    let mut v = Vec::new();
    if let TokenKind::Text(txt) = &tb.kind {
        v.push(txt.term.to_uppercase());
    }
    for wf in tb.morph.items() {
        if let Some(nc) = &wf.normal_case { v.push(nc.to_uppercase()); }
        if let Some(nf) = &wf.normal_full { v.push(nf.to_uppercase()); }
    }
    v.dedup();
    v
}
