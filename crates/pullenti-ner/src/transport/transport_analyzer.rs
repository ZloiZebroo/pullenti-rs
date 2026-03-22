/// TransportAnalyzer — simplified port of TransportAnalyzer.cs.
///
/// Recognizes patterns like:
///   "автомобиль Toyota Camry"  → TRANSPORT kind=Auto, type=автомобиль, brand=Toyota, model=Camry
///   "самолет Boeing 737"       → TRANSPORT kind=Fly,  type=самолет,  brand=Boeing, model=737
///   "теплоход «Победа»"        → TRANSPORT kind=Ship, type=теплоход, name=ПОБЕДА
///   "Toyota Camry"             → TRANSPORT kind=Auto, brand=Toyota  (brand-only pattern)
///   "Ford"                     → TRANSPORT kind=Auto, brand=Ford    (brand alone)

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::transport::transport_referent as tr_ref;
use crate::transport::transport_referent::TransportKind;
use crate::transport::transport_table;

pub struct TransportAnalyzer;

impl TransportAnalyzer {
    pub fn new() -> Self { TransportAnalyzer }
}

impl Analyzer for TransportAnalyzer {
    fn name(&self) -> &'static str { "TRANSPORT" }
    fn caption(&self) -> &'static str { "Транспорт" }

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
                    let r_rc = kit.add_entity(r_rc);
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

    // Pattern 1: type keyword [brand] [model/name]
    // Type keywords can appear lowercase mid-sentence (e.g. "на теплоходе «Победа»")
    for up in &uppers {
        if let Some(entry) = transport_table::lookup_type(up) {
            if let Some(result) = try_type_then_details(t, up, entry.canonical, entry.kind.clone(), sofa) {
                return Some(result);
            }
        }
    }

    // Pattern 2: brand alone or brand + model
    // Brands must start uppercase to avoid false positives
    if !starts_upper { return None; }
    for up in &uppers {
        if let Some(entry) = transport_table::lookup_brand(up) {
            if let Some(result) = try_brand_pattern(t, entry.canonical, entry.kind.clone(), sofa) {
                return Some(result);
            }
        }
    }

    None
}

// ── Pattern 1: type keyword + optional brand/model/name ──────────────────────

fn try_type_then_details(
    t: &TokenRef,
    _matched_key: &str,
    canonical_type: &str,
    kind: TransportKind,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let mut r = tr_ref::new_transport_referent();
    tr_ref::add_slot_str(&mut r, tr_ref::ATTR_TYPE, canonical_type);
    tr_ref::set_kind(&mut r, &kind);
    let mut end = t.clone();

    // For ships/spacecraft, look for a quoted name immediately after type keyword
    if kind == TransportKind::Ship || kind == TransportKind::Space {
        if let Some((name, name_end)) = try_quoted_name(&t.borrow().next.clone(), sofa) {
            tr_ref::add_slot_str(&mut r, tr_ref::ATTR_NAME, &name);
            return Some((r, name_end));
        }
    }

    // Try to consume brand + optional model after the type keyword
    let next = t.borrow().next.clone();
    if let Some(n) = next {
        // Skip punctuation like colon, hyphen
        let probe = skip_punct(&n, sofa);
        if let Some((brand, brand_tok)) = try_brand_token(&probe, sofa) {
            // Verify brand matches the right kind (or kind is auto which is generic)
            let brand_kind = transport_table::lookup_brand(&brand.to_uppercase())
                .map(|e| e.kind.clone())
                .unwrap_or(TransportKind::Undefined);
            if brand_kind == TransportKind::Undefined || brand_kind == kind {
                tr_ref::add_slot_str(&mut r, tr_ref::ATTR_BRAND, &brand);
                end = brand_tok.clone();

                // After brand, try model
                if let Some((model, model_end)) = try_model_after_brand(&brand_tok, sofa) {
                    tr_ref::add_slot_str(&mut r, tr_ref::ATTR_MODEL, &model);
                    end = model_end;
                }
                return Some((r, end));
            }
        }
    }

    // Return type alone (without brand/model)
    Some((r, end))
}

// ── Pattern 2: brand [model] ──────────────────────────────────────────────────

fn try_brand_pattern(
    t: &TokenRef,
    canonical_brand: &str,
    kind: TransportKind,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    // Only extract brand-only if after a person-title or after punctuation context.
    // To avoid false positives (e.g. "Иж" as a surname), require either:
    // - Brand is followed by a model designation (number or short ALL-CAPS token)
    // - Or brand appears right after "автомобиль", "машина" etc. (handled by Pattern 1)
    // For this simplified version: extract brand if it has a model token following it.
    let model_probe = t.borrow().next.clone();
    if let Some((model, model_end)) = try_model_after_brand(t, sofa) {
        if model_probe.is_some() {
            let mut r = tr_ref::new_transport_referent();
            tr_ref::set_kind(&mut r, &kind);
            tr_ref::add_slot_str(&mut r, tr_ref::ATTR_BRAND, canonical_brand);
            tr_ref::add_slot_str(&mut r, tr_ref::ATTR_MODEL, &model);
            return Some((r, model_end));
        }
    }
    None
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Skip optional punctuation (colon, hyphen) before next token.
fn skip_punct(t: &TokenRef, sofa: &SourceOfAnalysis) -> TokenRef {
    let tb = t.borrow();
    if tb.length_char() == 1 {
        let ch = sofa.char_at(tb.begin_char);
        if ch == ':' || ch == '-' || ch == '–' || ch == '—' {
            if let Some(next) = tb.next.clone() {
                drop(tb);
                return next;
            }
        }
    }
    drop(tb);
    t.clone()
}

/// Try to recognize a brand token at position `t`.
/// Returns (canonical_brand, end_token) or None.
fn try_brand_token(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let tb = t.borrow();
    let TokenKind::Text(_) = &tb.kind else { return None; };
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    if !surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return None;
    }
    let uppers = collect_upper_forms(&tb);
    drop(tb);

    // Single token brand
    for up in &uppers {
        if let Some(entry) = transport_table::lookup_brand(up) {
            return Some((entry.canonical.to_string(), t.clone()));
        }
    }

    // Two-token brand (e.g. "Land Rover", "Rolls-Royce")
    let next = t.borrow().next.clone()?;
    let nb = next.borrow();
    let TokenKind::Text(_) = &nb.kind else { return None; };
    // Must be immediately adjacent or single space
    if nb.whitespaces_before_count(sofa) > 1 { return None; }
    let surf2 = sofa.substring(nb.begin_char, nb.end_char);
    drop(nb);

    let combined = format!("{} {}", surface.to_uppercase(), surf2.to_uppercase());
    if let Some(entry) = transport_table::lookup_brand(&combined) {
        return Some((entry.canonical.to_string(), next));
    }

    None
}

/// Try to read a model designation (number or short mixed token) after a brand.
/// Models look like: "737", "Camry", "A320", "X5", "2103"
fn try_model_after_brand(brand_tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let next = brand_tok.borrow().next.clone()?;
    let nb = next.borrow();
    if nb.whitespaces_before_count(sofa) > 1 { return None; }

    match &nb.kind {
        TokenKind::Number(n) => {
            let val = n.value.clone();
            let end = next.clone();
            drop(nb);
            Some((val, end))
        }
        TokenKind::Text(_) => {
            let surf = sofa.substring(nb.begin_char, nb.end_char);
            // Model: starts with digit or uppercase, short (≤ 10 chars), mixed alnum
            let first = surf.chars().next().unwrap_or(' ');
            if (first.is_ascii_digit() || first.is_uppercase())
                && surf.chars().count() <= 10
                && surf.chars().all(|c| c.is_alphanumeric() || c == '-')
                && surf.chars().any(|c| c.is_alphanumeric())
            {
                // Exclude transport type keywords
                let up = surf.to_uppercase();
                if transport_table::lookup_type(&up).is_some() { drop(nb); return None; }
                let val = surf.to_uppercase();
                let end = next.clone();
                drop(nb);
                Some((val, end))
            } else {
                drop(nb);
                None
            }
        }
        _ => { drop(nb); None }
    }
}

/// Try to consume a quoted name («NAME» or "NAME") right after a type keyword.
/// Returns (uppercase_name, end_token) or None.
fn try_quoted_name(start: &Option<TokenRef>, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let t = start.as_ref()?;
    let tb = t.borrow();
    if tb.length_char() != 1 { return None; }
    let open_ch = sofa.char_at(tb.begin_char);
    let close_ch = matching_close(open_ch)?;
    let name_start = tb.next.clone()?;
    drop(tb);

    // Collect name tokens until close quote
    let mut parts: Vec<String> = Vec::new();
    let mut cur: TokenRef = name_start;
    loop {
        let cb = cur.borrow();
        if cb.length_char() == 1 && sofa.char_at(cb.begin_char) == close_ch {
            let end = cur.clone();
            drop(cb);
            if parts.is_empty() { return None; }
            return Some((parts.join(" "), end));
        }
        match &cb.kind {
            TokenKind::Text(_) => {
                let surf = sofa.substring(cb.begin_char, cb.end_char);
                parts.push(surf.to_uppercase());
            }
            TokenKind::Number(n) => {
                parts.push(n.value.clone());
            }
            _ => { drop(cb); return None; }
        }
        let next = cb.next.clone();
        drop(cb);
        cur = next?;
        if parts.len() > 5 { return None; }
    }
}

fn matching_close(open: char) -> Option<char> {
    match open {
        '«' => Some('»'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '"' => Some('"'),
        _ => None,
    }
}

/// Collect uppercase normal forms from a token's morph data.
fn collect_upper_forms(tb: &Token) -> Vec<String> {
    let mut v = Vec::new();
    // Include the surface term itself
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
