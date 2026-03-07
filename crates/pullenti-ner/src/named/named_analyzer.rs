/// NamedEntityAnalyzer — simplified port of NamedEntityAnalyzer.cs.
///
/// Recognizes:
///  1. Type keyword + proper name:   "планета Марс", "река Волга"
///  2. Type keyword + quoted name:   "фильм «Матрица»"
///  3. Well-known name standalone:   "Марс", "Волга", "Кремль"

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::named::named_referent as nr;
use crate::named::named_table;

pub struct NamedEntityAnalyzer;

impl NamedEntityAnalyzer {
    pub fn new() -> Self { NamedEntityAnalyzer }
}

impl Analyzer for NamedEntityAnalyzer {
    fn name(&self) -> &'static str { "NAMEDENTITY" }
    fn caption(&self) -> &'static str { "Мелкие именованные сущности" }

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

// ── Entry point ─────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();

    // Must be a text token
    match &tb.kind {
        TokenKind::Text(_) => {}
        _ => return None,
    }

    let surface = sofa.substring(tb.begin_char, tb.end_char);
    let starts_upper = surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

    let uppers = collect_upper_forms(&tb);
    drop(tb);

    // Pattern 1/2: type keyword + name (or quoted name).
    // Type keywords can appear lowercase mid-sentence ("памятник Пушкину").
    for upper in &uppers {
        if let Some(entry) = named_table::lookup_type(upper) {
            if let Some(r) = try_type_then_name(t, upper, entry.kind, sofa) {
                return Some(r);
            }
        }
    }

    // Pattern 3: well-known name standalone — requires uppercase start (proper noun).
    if !starts_upper {
        return None;
    }
    for upper in &uppers {
        if let Some(entry) = named_table::lookup_name(upper) {
            let mut r = nr::new_named_referent();
            nr::set_kind(&mut r, entry.kind.as_str());
            nr::add_name(&mut r, &entry.canonical);
            if let Some(lbl) = &entry.type_label {
                nr::set_type(&mut r, lbl);
            }
            return Some((r, t.clone()));
        }
    }

    None
}

// ── Pattern: type keyword + name ────────────────────────────────────────────

fn try_type_then_name(
    t: &TokenRef,
    type_kw: &str,
    kind: named_table::NamedKind,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let next = t.borrow().next.clone()?;
    let nb = next.borrow();

    if nb.whitespaces_before_count(sofa) > 3 {
        return None;
    }

    // Try quoted name first
    if let Some(close_ch) = is_open_quote_token(&next, sofa) {
        drop(nb);
        if let Some((name, end)) = collect_quoted_name(&next, close_ch, sofa) {
            let mut r = nr::new_named_referent();
            nr::set_kind(&mut r, kind.as_str());
            nr::set_type(&mut r, type_kw);
            nr::add_name(&mut r, &name);
            return Some((r, end));
        }
        return None;
    }

    // Try proper noun sequence: "планета Марс", "река Волга"
    if let TokenKind::Text(_) = &nb.kind {
        let surf = sofa.substring(nb.begin_char, nb.end_char);
        if surf.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let start_name = get_normal_or_surface(&next, sofa);
            drop(nb);
            let (full_name, end) = extend_name_from(start_name, next.clone(), 4, sofa);
            if !full_name.is_empty() {
                let mut r = nr::new_named_referent();
                nr::set_kind(&mut r, kind.as_str());
                nr::set_type(&mut r, type_kw);
                nr::add_name(&mut r, &full_name);
                return Some((r, end));
            }
        } else {
            drop(nb);
        }
    } else {
        drop(nb);
    }

    None
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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
        if cb.whitespaces_before_count(sofa) > 5 { break; }

        if cb.length_char() == 1 && sofa.char_at(cb.begin_char) == close_ch {
            end = cur.clone();
            drop(cb);
            break;
        }
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

/// Collect up to `max_extra` additional capitalized tokens for a name.
fn extend_name_from(
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
        if tb.whitespaces_before_count(sofa) > 2 { break; }

        match &tb.kind {
            TokenKind::Text(txt) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                if txt.term.chars().all(|c| !c.is_alphabetic()) { break; }
                let first_ch = surf.chars().next().unwrap_or(' ');
                if first_ch.is_lowercase() {
                    // Allow common connectors
                    let up = txt.term.to_uppercase();
                    if !matches!(up.as_str(), "И" | "OF" | "AND") {
                        break;
                    }
                }
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
        "В" | "НА" | "ПО" | "ДЛЯ" | "ЗА" | "ПРИ" | "С" | "ОТ" | "ДО" |
        "ЯВЛЯЕТСЯ" | "КАК" | "ТАК" | "НЕ" | "НИ"
    )
}
