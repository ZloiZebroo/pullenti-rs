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

    // Pattern 5: Multi-word known org ("Государственная дума", "Российская академия наук")
    if let Some(r) = try_multiword_known_org(t, sofa) {
        return Some(r);
    }

    // Pattern 6: Adjective prefix + type keyword
    // ("Центральный банк России", "Московский государственный университет")
    if let Some(r) = try_adj_prefix_then_type(t, sofa) {
        return Some(r);
    }

    None
}

// ── Pattern 5: Multi-word known org ──────────────────────────────────────────

/// Try scanning forward ≤6 consecutive text tokens, building an accumulated phrase,
/// and check `lookup_known` on each 2+-token prefix.
/// Handles "Государственная дума", "Российская академия наук", etc.
fn try_multiword_known_org(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Collect up to 6 consecutive text tokens
    let mut tokens: Vec<TokenRef> = Vec::new();
    {
        let mut cur = Some(t.clone());
        while tokens.len() < 6 {
            let tok = match cur.take() {
                None => break,
                Some(tok) => tok,
            };
            let tb = tok.borrow();
            if !tokens.is_empty() && tb.whitespaces_before_count(sofa) > 2 { break; }
            if let TokenKind::Text(_) = &tb.kind {
                let next = tb.next.clone();
                drop(tb);
                tokens.push(tok);
                cur = next;
            } else {
                break;
            }
        }
    }

    if tokens.len() < 2 { return None; } // single-token handled by pattern 4

    let mut phrase = String::new();
    let mut best: Option<(&'static org_table::KnownOrg, usize)> = None;

    for (i, tok) in tokens.iter().enumerate() {
        let term = {
            let tb = tok.borrow();
            if let TokenKind::Text(txt) = &tb.kind {
                txt.term.to_uppercase()
            } else {
                break;
            }
        };
        if i > 0 { phrase.push(' '); }
        phrase.push_str(&term);
        if i == 0 { continue; } // single-token is handled by pattern 4
        if let Some(known) = org_table::lookup_known(&phrase) {
            best = Some((known, i));
        }
    }

    let (known, end_idx) = best?;
    let end = tokens[end_idx].clone();

    let mut r = or_::new_org_referent();
    for name in &known.names {
        or_::add_name(&mut r, name);
    }
    if let Some(typ) = &known.typ {
        or_::set_type(&mut r, typ);
    }
    Some((r, end))
}

// ── Pattern 6: Adjective prefix + type keyword ────────────────────────────────

/// Try scanning forward from an uppercase token, collecting word tokens, and look for
/// a type keyword (possibly lowercase) at positions 1+.  Everything before the type
/// keyword becomes the org name prefix; words after extend the name.
///
/// Handles:
///   "Центральный банк России"              → type=БАНК,        name=ЦЕНТРАЛЬНЫЙ РОССИЯ
///   "Московский государственный университет" → type=УНИВЕРСИТЕТ, name=МОСКОВСКИЙ ГОСУДАРСТВЕННЫЙ
///   "Государственная дума"                 → type=ДУМА,        name=ГОСУДАРСТВЕННАЯ
///   "Высший арбитражный суд"               → type=СУД,         name=ВЫСШИЙ АРБИТРАЖНЫЙ
fn try_adj_prefix_then_type(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    struct WordInfo {
        tok: TokenRef,
        surface_upper: String,
        morph_forms: Vec<String>,
    }

    // Collect up to 7 consecutive alphabetic text tokens (no long gaps)
    let mut words: Vec<WordInfo> = Vec::new();
    {
        let mut cur = Some(t.clone());
        while words.len() < 7 {
            let tok = match cur.take() {
                None => break,
                Some(tok) => tok,
            };
            let tb = tok.borrow();
            if !words.is_empty() && tb.whitespaces_before_count(sofa) > 2 { break; }
            if let TokenKind::Text(txt) = &tb.kind {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                let first = surf.chars().next().unwrap_or(' ');
                if !first.is_alphabetic() { break; }
                let surface_upper = txt.term.to_uppercase();
                if is_stop_word(&surface_upper) { break; }
                let mut morph_forms = vec![surface_upper.clone()];
                for wf in tb.morph.items() {
                    if let Some(nc) = &wf.normal_case { morph_forms.push(nc.to_uppercase()); }
                    if let Some(nf) = &wf.normal_full { morph_forms.push(nf.to_uppercase()); }
                }
                morph_forms.dedup();
                let next = tb.next.clone();
                drop(tb);
                words.push(WordInfo { tok, surface_upper, morph_forms });
                cur = next;
            } else {
                break;
            }
        }
    }

    // Try type keyword at each position from 1 onward (position 0 handled by pattern 3)
    for type_pos in 1..words.len() {
        let type_word = &words[type_pos];
        let found_entry = type_word.morph_forms.iter()
            .find_map(|f| org_table::lookup_type(f));
        let Some(entry) = found_entry else { continue };

        let name_parts: Vec<&str> = words[..type_pos]
            .iter()
            .map(|w| w.surface_upper.as_str())
            .collect();
        let base_name = name_parts.join(" ");
        let type_canonical = entry.canonical.clone();
        let profile = entry.profile.clone();
        let type_tok = type_word.tok.clone();

        // Optionally extend name with words after the type keyword
        let after_next = type_tok.borrow().next.clone();
        let (full_name, end) = if let Some(after) = after_next {
            let (ok, is_text, first_a, first_a_lower_cyr, is_verb) = {
                let ab = after.borrow();
                let ok = ab.whitespaces_before_count(sofa) <= 2;
                let is_text = matches!(&ab.kind, TokenKind::Text(_));
                let first_a = if is_text {
                    let s = sofa.substring(ab.begin_char, ab.end_char);
                    s.chars().next().unwrap_or(' ')
                } else { ' ' };
                let first_a_lower_cyr = !first_a.is_uppercase()
                    && first_a.is_alphabetic()
                    && (first_a as u32) >= 0x0400;
                let is_verb = first_a_lower_cyr && ab.get_morph_class_in_dictionary().is_verb();
                (ok, is_text, first_a, first_a_lower_cyr, is_verb)
            };
            if ok && is_text && (first_a.is_uppercase() || first_a_lower_cyr) && !is_verb {
                let start = get_normal_or_surface(&after, sofa);
                let combined = format!("{} {}", base_name, start);
                extend_org_name_from_with_lower(combined, after, 5, first_a_lower_cyr, sofa)
            } else {
                (base_name.clone(), type_tok.clone())
            }
        } else {
            (base_name.clone(), type_tok.clone())
        };

        if !full_name.is_empty() {
            let mut r = or_::new_org_referent();
            or_::set_type(&mut r, &type_canonical);
            if let Some(p) = profile { or_::set_profile(&mut r, p.as_str()); }
            or_::add_name(&mut r, &full_name);
            return Some((r, end));
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

    // Try proper noun sequence: "Федеральная служба безопасности", "Министерство финансов"
    // Allow lowercase initial word for state org names (genitive: "финансов", "образования").
    if let TokenKind::Text(_txt) = &nb.kind {
        let surf = sofa.substring(nb.begin_char, nb.end_char);
        let first_upper = surf.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let first_lower_cyrillic = !first_upper
            && surf.chars().next().map(|c| c.is_alphabetic() && (c as u32) >= 0x0400).unwrap_or(false);
        if first_upper || first_lower_cyrillic {
            // Don't start an org name from a verb word ("производит", "является"…)
            let is_verb_start = first_lower_cyrillic && nb.get_morph_class_in_dictionary().is_verb();
            let start_name = get_normal_or_surface(&next, sofa);
            drop(nb);
            if !is_verb_start {
                let (full_name, end) = extend_org_name_from_with_lower(start_name, next.clone(), 6, first_lower_cyrillic, sofa);
                if !full_name.is_empty() {
                    let mut r = or_::new_org_referent();
                    or_::set_type(&mut r, type_keyword);
                    if let Some(p) = profile { or_::set_profile(&mut r, p); }
                    or_::add_name(&mut r, &full_name);
                    return Some((r, end));
                }
            }
        } else {
            drop(nb);
        }
    } else {
        drop(nb);
    }

    // Last resort: type-only entity (e.g. standalone "Парламент")
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

/// Like `extend_org_name_from` but also allows lowercase Cyrillic continuation words
/// (genitive nouns like "финансов", "образования", "арбитражного").
/// `allow_lower_start`: if true, the start word was already lowercase — be permissive about
/// continuing with more lowercase Cyrillic words until a clear stop.
fn extend_org_name_from_with_lower(
    start: String,
    start_tok: TokenRef,
    max_extra: usize,
    allow_lower_start: bool,
    sofa: &SourceOfAnalysis,
) -> (String, TokenRef) {
    let mut parts = vec![start];
    let mut end = start_tok.clone();
    let mut cur = start_tok.borrow().next.clone();
    let mut count = 0;
    // Track whether we are still in "lowercase-allowed" territory.
    // Once we hit an uppercase word, subsequent lowercase words are normal connectors.
    let mut lower_mode = allow_lower_start;

    while let Some(t) = cur {
        if count >= max_extra { break; }
        let tb = t.borrow();
        if tb.whitespaces_before_count(sofa) > 2 { break; }

        match &tb.kind {
            TokenKind::Text(txt) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                // Stop on punctuation-only token
                if txt.term.chars().all(|c| !c.is_alphabetic()) { break; }
                let first_ch = surf.chars().next().unwrap_or(' ');
                let up = txt.term.to_uppercase();
                // Stop on known stop words
                if is_stop_word(&up) { break; }

                if first_ch.is_lowercase() {
                    let is_cyrillic = (first_ch as u32) >= 0x0400;
                    if is_cyrillic && lower_mode {
                        // Skip verbs — "производит", "является", etc. are not org name parts
                        if tb.get_morph_class_in_dictionary().is_verb() { break; }
                        // Collect lowercase Cyrillic genitive/adjective word
                        parts.push(up);
                        end = t.clone();
                        count += 1;
                    } else {
                        // Allow small connectors (и, of, and) inside org names
                        if matches!(up.as_str(), "И" | "ОФ" | "OF" | "AND" | "ДЛЯ") {
                            // Don't count connectors
                        } else {
                            break;
                        }
                    }
                } else {
                    // Uppercase word — accepted, and turn off lower_mode going forward
                    // (subsequent lowercase words need to be connectors only)
                    lower_mode = false;
                    if is_stop_word(&up) { break; }
                    parts.push(up);
                    end = t.clone();
                    count += 1;
                }
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
