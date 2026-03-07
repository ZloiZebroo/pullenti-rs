/// OrgAnalyzer — simplified port of OrganizationAnalyzer.cs.
///
/// Recognizes Russian/English organizations using:
///  1. Legal form abbreviation + name in quotes: "ООО «Газпром»"
///  2. Legal form + proper nouns:                "ООО Ромашка"
///  3. Type keyword + proper name:               "Министерство финансов"
///  4. Well-known org name (from Orgs_ru.dat):   "ГИБДД", "ФСБ"
///  5. Quoted name after type keyword:            "банк «Открытие»"

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::org::org_referent as or_;
use crate::org::org_table;

pub struct OrgAnalyzer;

impl OrgAnalyzer {
    pub fn new() -> Self { OrgAnalyzer }
}

impl Analyzer for OrgAnalyzer {
    fn name(&self) -> &'static str { "ORGANIZATION" }
    fn caption(&self) -> &'static str { "Организации" }

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

// ── Entry point ───────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();

    match &tb.kind {
        TokenKind::Text(_) => {}
        _ => return None,
    }
    // Must start with uppercase
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    if !surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return None;
    }

    // Get all morph normal forms for this token
    let uppers = collect_upper_forms(&tb);
    drop(tb);

    // Pattern 1/2: Legal form abbreviation → try name that follows
    for upper in &uppers {
        if org_table::is_legal_abbr(upper) {
            if let Some(r) = try_legal_abbr_then_name(t, upper, sofa) {
                return Some(r);
            }
        }
    }

    // Pattern 3: Type keyword (minister, court, etc.) + proper name(s)
    for upper in &uppers {
        if let Some(entry) = org_table::lookup_type(upper) {
            if let Some(r) = try_type_then_name(t, upper, entry.profile.as_deref(), sofa) {
                return Some(r);
            }
        }
    }

    // Pattern 4: Known org name (ГИБДД, ФСБ, etc.)
    for upper in &uppers {
        if let Some(known) = org_table::lookup_known(upper) {
            let mut r = or_::new_org_referent();
            for name in &known.names {
                or_::add_name(&mut r, name);
            }
            if let Some(typ) = &known.typ {
                or_::set_type(&mut r, typ);
            }
            return Some((r, t.clone()));
        }
    }

    None
}

// ── Pattern 1/2: Legal form abbreviation + name ───────────────────────────────

fn try_legal_abbr_then_name(
    t: &TokenRef,
    abbr: &str,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    // Next non-whitespace token
    let next = t.borrow().next.clone()?;
    let nb = next.borrow();

    // Try name in quotes: ООО «Газпром» or ООО "Газпром"
    if nb.whitespaces_before_count(sofa) <= 2 {
        if let Some(open_ch) = is_open_quote_token(&next, sofa) {
            drop(nb);
            if let Some((name, end)) = collect_quoted_name(&next, open_ch, sofa) {
                let mut r = or_::new_org_referent();
                or_::set_type(&mut r, abbr);
                or_::add_name(&mut r, &name);
                return Some((r, end));
            }
            return None;
        }

        // Try proper noun(s): ООО Ромашка, ООО АЛЛО
        if let TokenKind::Text(_) = &nb.kind {
            let surf_n = sofa.substring(nb.begin_char, nb.end_char);
            if surf_n.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                let is_proper = nb.morph.items().iter().any(|wf| {
                    wf.base.class.is_proper_name() || wf.base.class.is_proper_surname()
                }) || nb.chars.is_all_upper();
                if is_proper {
                    let name = get_normal_or_surface(&next, sofa);
                    drop(nb);
                    // Collect up to 3 more proper words
                    let (full_name, end) = extend_proper_name(name, next.clone(), sofa);
                    let mut r = or_::new_org_referent();
                    or_::set_type(&mut r, abbr);
                    or_::add_name(&mut r, &full_name);
                    return Some((r, end));
                }
            }
        }
    }
    None
}

// ── Pattern 3: Type keyword + proper name(s) ─────────────────────────────────

fn try_type_then_name(
    t: &TokenRef,
    type_keyword: &str,
    profile: Option<&str>,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    // Find the end of the multi-word type keyword (e.g. "Министерство финансов" could
    // be just one token if it's in the map, or we might already have matched a multi-word key)
    // For simplicity: the type keyword matched at a single token t.
    // Now look for subsequent capitalized words as the org name.
    let type_end = t.clone();

    let next = t.borrow().next.clone()?;
    let nb = next.borrow();

    // Skip articles/prepositions of one letter
    if nb.whitespaces_before_count(sofa) > 3 {
        return None;
    }

    // Try name in quotes first
    if let Some(open_ch) = is_open_quote_token(&next, sofa) {
        drop(nb);
        if let Some((name, end)) = collect_quoted_name(&next, open_ch, sofa) {
            let mut r = or_::new_org_referent();
            or_::set_type(&mut r, type_keyword);
            if let Some(p) = profile { or_::set_profile(&mut r, p); }
            or_::add_name(&mut r, &name);
            return Some((r, end));
        }
        return None;
    }

    // Try proper noun sequence: "Министерство финансов", "Федеральная служба безопасности"
    if let TokenKind::Text(txt) = &nb.kind {
        let surf = sofa.substring(nb.begin_char, nb.end_char);
        if surf.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let start_name = get_normal_or_surface(&next, sofa);
            drop(nb);
            let (full_name, end) = extend_org_name_from(start_name, next.clone(), 5, sofa);
            if !full_name.is_empty() {
                let mut r = or_::new_org_referent();
                or_::set_type(&mut r, type_keyword);
                if let Some(p) = profile { or_::set_profile(&mut r, p); }
                or_::add_name(&mut r, &full_name);
                return Some((r, end));
            }
        } else {
            drop(nb);
        }
    } else {
        drop(nb);
    }

    // Last resort: type-only entity (e.g. standalone "Министерство финансов" where
    // the type keyword itself IS the full name — like "Парламент")
    // Only if the type keyword is high-confidence (top=true entries)
    if let Some(entry) = org_table::lookup_type(type_keyword) {
        if entry.is_prefix {
            let mut r = or_::new_org_referent();
            or_::set_type(&mut r, type_keyword);
            if let Some(p) = profile { or_::set_profile(&mut r, p); }
            return Some((r, type_end));
        }
    }

    None
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect all uppercase morph normal forms + term for a token.
fn collect_upper_forms(tb: &crate::token::Token) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let TokenKind::Text(txt) = &tb.kind {
        v.push(txt.term.to_uppercase());
        for wf in tb.morph.items() {
            if let Some(nc) = &wf.normal_case { v.push(nc.to_uppercase()); }
            if let Some(nf) = &wf.normal_full { v.push(nf.to_uppercase()); }
        }
        v.dedup();
    }
    v
}

/// Check if token is an open-quote character (« " ' ‹ 「).
/// Returns Some(matching_close_quote) if it is.
fn is_open_quote_token(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<char> {
    let tb = t.borrow();
    if tb.length_char() != 1 { return None; }
    match sofa.char_at(tb.begin_char) {
        '«' => Some('»'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '‹' => Some('›'),
        '„' => Some('"'),
        '"' => Some('"'),
        _ => None,
    }
}

/// Collect text inside quote marks starting at open-quote token.
/// Returns (collected_name_uppercase, end_token=close_quote).
fn collect_quoted_name(
    open: &TokenRef,
    close_ch: char,
    sofa: &SourceOfAnalysis,
) -> Option<(String, TokenRef)> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = open.borrow().next.clone()?;
    let mut end = open.clone();

    loop {
        let cb = cur.borrow();
        if cb.whitespaces_before_count(sofa) > 5 { break; } // newline inside quote = stop

        if cb.length_char() == 1 && sofa.char_at(cb.begin_char) == close_ch {
            end = cur.clone();
            drop(cb);
            break;
        }
        // Collect text
        if let TokenKind::Text(txt) = &cb.kind {
            parts.push(txt.term.to_uppercase());
        } else if let TokenKind::Number(n) = &cb.kind {
            parts.push(n.value.clone());
        }
        let next = cb.next.clone();
        drop(cb);
        match next {
            None => break,
            Some(n) => cur = n,
        }
    }

    if parts.is_empty() { return None; }
    Some((parts.join(" "), end))
}

/// Get nominative form of a proper noun token, or its uppercase surface.
fn get_normal_or_surface(t: &TokenRef, sofa: &SourceOfAnalysis) -> String {
    let tb = t.borrow();
    if let TokenKind::Text(txt) = &tb.kind {
        for wf in tb.morph.items() {
            if wf.base.class.is_proper_name() || wf.base.class.is_proper_surname()
                || wf.base.class.is_proper_secname()
            {
                if let Some(nc) = &wf.normal_case { return nc.to_uppercase(); }
            }
        }
        if let Some(wf) = tb.morph.items().first() {
            if let Some(nc) = &wf.normal_case { return nc.to_uppercase(); }
        }
        return txt.term.to_uppercase();
    }
    String::new()
}

/// Extend a proper name with following capitalized tokens (max `limit` tokens).
fn extend_proper_name(
    start: String,
    start_tok: TokenRef,
    sofa: &SourceOfAnalysis,
) -> (String, TokenRef) {
    extend_org_name_from(start, start_tok, 4, sofa)
}

/// Collect up to `max_extra` additional tokens for an org name.
/// Stops on: lowercase token, punctuation, long whitespace gap, or known stop word.
fn extend_org_name_from(
    start: String,
    start_tok: TokenRef,
    max_extra: usize,
    sofa: &SourceOfAnalysis,
) -> (String, TokenRef) {
    let mut parts = vec![start];
    let mut end = start_tok.clone();
    let mut cur = start_tok.borrow().next.clone();
    let mut count = 0;

    while let Some(t) = cur {
        if count >= max_extra { break; }
        let tb = t.borrow();
        if tb.whitespaces_before_count(sofa) > 2 { break; } // newline or large gap = stop

        match &tb.kind {
            TokenKind::Text(txt) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                // Stop on punctuation-only token
                if txt.term.chars().all(|c| !c.is_alphabetic()) { break; }
                // Stop on lowercase token (unless it's a preposition/conjunction allowed inside org names)
                let first_ch = surf.chars().next().unwrap_or(' ');
                if first_ch.is_lowercase() {
                    // Allow common Russian genitive prepositions inside org names
                    let up = txt.term.to_uppercase();
                    if !matches!(up.as_str(), "ПО" | "И" | "ОФ" | "OF" | "AND" | "И" | "ДЛЯ") {
                        break;
                    }
                    // Don't count connector words as name parts
                }
                // Stop on a token that is clearly a common non-name word
                if is_stop_word(&txt.term.to_uppercase()) { break; }

                parts.push(txt.term.to_uppercase());
                end = t.clone();
                count += 1;
            }
            _ => break,
        }
        let next = tb.next.clone();
        drop(tb);
        cur = next;
    }

    (parts.join(" "), end)
}

fn is_stop_word(up: &str) -> bool {
    matches!(up,
        "ОТ" | "В" | "С" | "НА" | "ПО" | "ДЛЯ" | "ЗА" | "ПРИ" |
        "ЯВЛЯЕТСЯ" | "ОСУЩЕСТВЛЯЕТ" | "ИМЕЕТ" | "БЫЛО" | "БЫЛА" | "БЫЛИ" |
        "КАК" | "ТАК" | "УЖЕ" | "ЕЩЁ" | "ЕЩЕ" | "НЕ" | "НИ" | "БЫЛ"
    )
}
