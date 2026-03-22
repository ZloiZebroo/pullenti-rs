/// DefinitionAnalyzer — simplified port of DefinitionAnalyzer.cs.
///
/// Recognizes "X is/are/—/: Y" patterns (thesis/definition/assertion) in Russian text.
///
/// Examples:
///   "Предприниматель — физическое лицо, осуществляющее..."
///       → THESIS(TERMIN="предприниматель", VALUE="физическое лицо, ...", KIND=Definition)
///   "Договор является соглашением двух или более лиц..."
///       → THESIS(TERMIN="договор", VALUE="соглашением двух или более лиц...", KIND=Assertation)
///   "Под термином X понимается Y"
///       → THESIS(TERMIN="X", VALUE="Y", KIND=Assertation)
///
/// is_specific() returns true — must be explicitly added to the processor.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::misc_helper::can_be_start_of_sentence;
use crate::core::noun_phrase::{try_parse as npt_try_parse, NounPhraseParseAttr};
use crate::definition::definition_referent as dr;
use crate::definition::definition_referent::DefinitionKind;

pub struct DefinitionAnalyzer;

impl DefinitionAnalyzer {
    pub fn new() -> Self { DefinitionAnalyzer }
}

impl Analyzer for DefinitionAnalyzer {
    fn name(&self)    -> &'static str { "THESIS" }
    fn caption(&self) -> &'static str { "Тезисы" }
    fn is_specific(&self) -> bool { true }
    fn progress_weight(&self) -> i32 { 1 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }
            let ok = can_be_start_of_sentence(&t, &sofa)
                || {
                    let is_newline = t.borrow().is_newline_before(&sofa);
                    is_newline && {
                        let prev_is_semi_colon = t.borrow().prev.as_ref()
                            .and_then(|w| w.upgrade())
                            .map(|p| p.borrow().is_char_of(";:", &sofa))
                            .unwrap_or(false);
                        prev_is_semi_colon
                    }
                };
            if !ok {
                cur = t.borrow().next.clone();
                continue;
            }
            match try_attach(&t, &sofa) {
                None => { cur = t.borrow().next.clone(); }
                Some((referent, begin, end)) => {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let tok = Rc::new(RefCell::new(
                        Token::new_referent(begin, end.clone(), r_rc)
                    ));
                    kit.embed_token(tok.clone());
                    cur = tok.borrow().next.clone();
                }
            }
        }
    }
}

// ── Main try_attach ───────────────────────────────────────────────────────────

/// Try to parse a thesis/definition starting at `t`.
/// Returns (referent, begin_token, end_token) or None.
fn try_attach(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef, TokenRef)> {
    // Step 1: skip list prefix (numbers like "1.", "(a)", "а)")
    let t0_orig = t.clone();
    let t0 = ignore_list_prefix(t, sofa)?;

    // Step 2: handle "ПОД X понимается" / "ИМЕННО" prefix
    let t_start = handle_special_prefix(&t0, sofa)?;

    // Skip "УТРАТИТЬ СИЛУ" (force-expire clause)
    {
        let ts_b = t_start.borrow();
        if ts_b.is_value("УТРАТИТЬ", None) {
            if let Some(nx) = ts_b.next.clone() {
                if nx.borrow().is_value("СИЛА", None) {
                    drop(ts_b);
                    return None; // just skip this
                }
            }
        }
        drop(ts_b);
    }

    // Step 3: collect left-side noun phrases (the TERM)
    let (l0, l1, coef_hint) = collect_left_side(&t_start, sofa)?;

    // Step 4: after left side, current token should be copula / dash
    let connector = l1.borrow().next.clone()?;

    // Step 5: detect connector and find right-side start
    let (kind, r0) = detect_connector(&connector, coef_hint, sofa)?;

    // Step 6: collect right-side text until end of sentence
    let (r0, r1) = collect_right_side(&r0, sofa)?;

    // Extract term text (raw surface text from l0 to l1)
    let term_text = {
        let b = l0.borrow().begin_char;
        let e = l1.borrow().end_char;
        sofa.substring(b, e).to_string()
    };

    if term_text.trim().is_empty() {
        return None;
    }

    // Extract value text
    let value_text = {
        let b = r0.borrow().begin_char;
        let e = r1.borrow().end_char;
        sofa.substring(b, e).to_string()
    };

    if value_text.trim().len() < 5 {
        return None;
    }

    // Build the referent
    let mut r = dr::new_thesis_referent();
    dr::add_slot_str(&mut r, dr::ATTR_TERMIN, term_text.trim());
    dr::add_slot_str(&mut r, dr::ATTR_VALUE, value_text.trim());
    dr::set_kind(&mut r, &kind);

    Some((r, t0_orig, r1))
}

// ── ignore_list_prefix ────────────────────────────────────────────────────────

/// Skip over a leading list prefix like "1.", "(a)", "а)", etc.
/// Returns the first "real" token or None if we run off the end.
fn ignore_list_prefix(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let mut cur = t.clone();
    loop {
        // Check newline_after — don't scan into next line
        if cur.borrow().is_newline_after(sofa) {
            return Some(cur);
        }
        let (kind_tag, is_letter, len, next_opt) = {
            let tb = cur.borrow();
            let kt = match &tb.kind {
                TokenKind::Number(_) => 0u8,
                TokenKind::Text(_)   => 1u8,
                _                    => 2u8,
            };
            let il = tb.chars.is_letter();
            let ln = tb.length_char();
            let nx = tb.next.clone();
            (kt, il, ln, nx)
        };

        match kind_tag {
            0 => {
                // Number token — could be list prefix "1", "2"
                let next = match next_opt { Some(n) => n, None => return Some(cur) };
                if next.borrow().is_char_of(".)", sofa) {
                    let after = next.borrow().next.clone()?;
                    cur = after;
                    continue;
                }
                return Some(cur);
            }
            1 => {
                if is_letter {
                    if let Some(next) = next_opt {
                        // Single lowercase letter followed by "." or ")" → list prefix
                        if len == 1 && next.borrow().is_char_of(".)", sofa) {
                            let after = next.borrow().next.clone()?;
                            cur = after;
                            continue;
                        }
                    }
                    return Some(cur);
                } else {
                    // Non-letter TextToken (punctuation)
                    // Try to handle "(X)" bracket skip
                    let is_open_paren = cur.borrow().is_char('(', sofa);
                    if is_open_paren {
                        if let Some(inner) = next_opt {
                            let next2 = inner.borrow().next.clone();
                            if let Some(close) = next2 {
                                if close.borrow().is_char(')', sofa) {
                                    if let Some(after) = close.borrow().next.clone() {
                                        cur = after;
                                        continue;
                                    }
                                }
                            }
                        }
                        return Some(cur);
                    }
                    // Skip misc punctuation
                    match next_opt {
                        Some(nx) => { cur = nx; continue; }
                        None     => return Some(cur),
                    }
                }
            }
            _ => return Some(cur),
        }
    }
}

// ── handle_special_prefix ─────────────────────────────────────────────────────

/// Handle "ПОД X понимается", "ИМЕННО X" type opening phrases.
/// Returns new start token or None.
fn handle_special_prefix(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    if t.borrow().is_value("ПОД", None) {
        return t.borrow().next.clone();
    }
    if t.borrow().is_value("ИМЕННО", None) {
        return t.borrow().next.clone();
    }
    Some(t.clone())
}

// ── collect_left_side ─────────────────────────────────────────────────────────

/// Scan tokens to collect the left-side (TERM) noun phrases.
/// Stops when a copula/dash is found or a new sentence starts.
/// Returns (first_term_tok, last_term_tok, coef_hint).
fn collect_left_side(
    t_start: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(TokenRef, TokenRef, Option<DefinitionKind>)> {
    let mut l0: Option<TokenRef> = None;
    let mut l1: Option<TokenRef> = None;
    let mut cur = t_start.clone();
    let coef_hint: Option<DefinitionKind> = None;

    loop {
        // Sentence boundary check
        if l0.is_some() && can_be_start_of_sentence(&cur, sofa) {
            break;
        }

        // Collect attributes before borrowing
        let (is_letter, is_hiphen, is_colon, is_char_open, is_comma, next_opt) = {
            let tb = cur.borrow();
            (
                tb.chars.is_letter(),
                tb.is_hiphen(sofa),
                tb.is_char(':', sofa),
                tb.is_char('(', sofa),
                tb.is_comma(sofa),
                tb.next.clone(),
            )
        };

        if !is_letter {
            if is_hiphen || is_colon {
                break;
            }
            if is_char_open {
                if l1.is_some() {
                    // Skip bracket group after l1
                    let after_paren = skip_bracket_group(&cur, sofa);
                    match after_paren {
                        Some(after) => { cur = after; continue; }
                        None        => break,
                    }
                } else if l0.is_none() {
                    // No l0 yet — try quoted term
                    if let Some(inner_start) = next_opt {
                        let npt = npt_try_parse(&inner_start, NounPhraseParseAttr::No, 0, sofa);
                        if npt.is_some() {
                            let close = find_matching_close_paren(&inner_start, sofa);
                            l0 = Some(inner_start.clone());
                            l1 = Some(close.clone());
                            let after = close.borrow().next.clone();
                            match after {
                                Some(a) => { cur = a; continue; }
                                None    => break,
                            }
                        }
                    }
                    break;
                } else {
                    break;
                }
            }
            // Comma — try to skip to continue left side
            if is_comma && l0.is_some() {
                match next_opt {
                    Some(nx) => { cur = nx; continue; }
                    None     => break,
                }
            }
            // Other non-letter
            if l0.is_some() {
                break;
            }
            match next_opt {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        }

        // Letter token — check various stopping conditions
        // "ЭТО" is always a connector
        if cur.borrow().is_value("ЭТО", None) {
            break;
        }

        let (mc_is_pronoun, mc_is_verb, mc_is_conj, term_str) = {
            let tb = cur.borrow();
            let mc = tb.get_morph_class_in_dictionary();
            let ts = match &tb.kind {
                TokenKind::Text(t) => t.term.clone(),
                _ => String::new(),
            };
            (mc.is_pronoun(), mc.is_verb(), mc.is_conjunction(), ts)
        };

        if mc_is_pronoun && !cur.borrow().is_value("ИНОЙ", None) {
            if l0.is_some() { break; }
            match next_opt {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        }

        if mc_is_verb {
            if is_copula_verb(&term_str) {
                break;
            }
            if l0.is_some() { break; }
            let next = cur.borrow().next.clone();
            match next {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        }

        if mc_is_conj {
            if l0.is_some() { break; }
            match next_opt {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        }

        // Try NounPhrase
        let npt = npt_try_parse(&cur, NounPhraseParseAttr::ParsePreposition, 0, sofa);
        if let Some(ref np) = npt {
            // Preposition-headed that's a "forbidden last word" → stop
            if np.preposition.is_some() && l0.is_some() {
                if is_forbidden_last_phrase(&np.begin_token) {
                    break;
                }
            }
            // Internal noun (complex NP) → stop
            if np.internal_noun.is_some() {
                break;
            }
            // Pronouns inside NP → stop
            if npt_has_pronoun(np) { break; }

            // Forbidden first word
            if l0.is_none() && is_forbidden_first_word(&np.begin_token) {
                return None;
            }

            if l0.is_none() {
                l0 = Some(np.begin_token.clone());
            }
            l1 = Some(np.end_token.clone());
            let end_tok_next = np.end_token.borrow().next.clone();
            match end_tok_next {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        } else {
            // No NounPhrase
            if l0.is_some() { break; }

            // Check if this might be a multi-capital proper-noun term
            let (not_lower, len, is_undef) = {
                let tb = cur.borrow();
                let mc = tb.get_morph_class_in_dictionary();
                (!tb.chars.is_all_lower(), tb.length_char(), mc.is_undefined())
            };
            if not_lower && len > 2 && is_undef {
                l0 = Some(cur.clone());
                l1 = Some(cur.clone());
                match next_opt {
                    Some(nx) => { cur = nx; continue; }
                    None     => break,
                }
            }

            // Skip short/common words when l0 not set
            match next_opt {
                Some(nx) => { cur = nx; continue; }
                None     => break,
            }
        }
    }

    match (l0, l1) {
        (Some(l0), Some(l1)) => Some((l0, l1, coef_hint)),
        _ => None,
    }
}

// ── detect_connector ─────────────────────────────────────────────────────────

/// Given the token immediately after the left-side (potential connector),
/// determine what kind of thesis this is and return the first right-side token.
fn detect_connector(
    connector: &TokenRef,
    _coef_hint: Option<DefinitionKind>,
    sofa: &SourceOfAnalysis,
) -> Option<(DefinitionKind, TokenRef)> {

    // Collect key properties from connector
    let (is_hiphen, ws_before, ws_after, is_colon, term_str, next_opt) = {
        let tb = connector.borrow();
        (
            tb.is_hiphen(sofa),
            tb.is_whitespace_before(sofa),
            tb.is_whitespace_after(sofa),
            tb.is_char(':', sofa),
            match &tb.kind { TokenKind::Text(t) => t.term.clone(), _ => String::new() },
            tb.next.clone(),
        )
    };

    // Em dash "—" as definition separator
    if is_hiphen && ws_before && ws_after {
        let next = next_opt?;
        // Check if right side starts with "ЭТО" or "НЕ"
        let next_is_eto = next.borrow().is_value("ЭТО", None);
        let next_is_ne  = next.borrow().is_value("НЕ", None);
        if next_is_eto {
            let after = next.borrow().next.clone()?;
            let npt = npt_try_parse(&after, NounPhraseParseAttr::No, 0, sofa);
            let _ = npt; // don't need, just checking
            return Some((DefinitionKind::Definition, after));
        }
        if next_is_ne {
            let after = next.borrow().next.clone()?;
            return Some((DefinitionKind::Negation, after));
        }
        // Check that right side has NP in nominative
        let mut npt = npt_try_parse(&next, NounPhraseParseAttr::No, 0, sofa);
        if let Some(ref mut np) = npt {
            if np.morph.case().is_nominative() {
                return Some((DefinitionKind::Definition, next));
            }
        }
        // Even without nominative NP — accept as definition
        return Some((DefinitionKind::Definition, next));
    }

    // Em dash WITHOUT whitespace — not a sentence-level separator
    if is_hiphen {
        return None;
    }

    // Colon ":" as separator
    if is_colon {
        if let Some(next) = next_opt {
            // Don't cross newline
            if !next.borrow().is_newline_before(sofa) {
                return Some((DefinitionKind::Definition, next));
            }
        }
        return None;
    }

    // Handle TextToken connectors
    if term_str.is_empty() {
        return None;
    }

    // "ЭТО" connector (without preceding dash)
    if term_str == "ЭТО" {
        if let Some(next) = next_opt {
            let npt = npt_try_parse(&next, NounPhraseParseAttr::No, 0, sofa);
            if npt.is_some() {
                return Some((DefinitionKind::Assertation, next));
            }
        }
        return None;
    }

    // "ЯВЛЯЕТСЯ" / "ЕСТЬ" / "ПРИЗНАЁТСЯ"
    if matches!(term_str.as_str(), "ЯВЛЯЕТСЯ" | "ЯВЛЯТЬСЯ" | "ЕСТЬ" | "ПРИЗНАЁТСЯ" | "ПРИЗНАВАТЬСЯ") {
        return parse_cop_right(next_opt, false, sofa);
    }

    // "ОЗНАЧАТЬ", "НЕСТИ"
    if matches!(term_str.as_str(), "ОЗНАЧАТЬ" | "НЕСТИ") {
        let mut r0 = next_opt?;
        if r0.borrow().is_char(':', sofa) {
            let nx = r0.borrow().next.clone()?;
            r0 = nx;
        }
        if r0.borrow().is_value("НЕ", None) {
            let nx = r0.borrow().next.clone()?;
            let npt = npt_try_parse(&nx, NounPhraseParseAttr::No, 0, sofa);
            if npt.is_some() {
                return Some((DefinitionKind::Negation, nx));
            }
            return None;
        }
        let npt = npt_try_parse(&r0, NounPhraseParseAttr::No, 0, sofa);
        if npt.is_some() {
            return Some((DefinitionKind::Assertation, r0));
        }
        return None;
    }

    // "ПРЕДСТАВЛЯТЬ СОБОЙ"
    if term_str == "ПРЕДСТАВЛЯТЬ" {
        if let Some(next) = next_opt {
            if next.borrow().is_value("СОБОЙ", None) {
                let r0 = next.borrow().next.clone()?;
                let npt = npt_try_parse(&r0, NounPhraseParseAttr::No, 0, sofa);
                let is_adj = r0.borrow().get_morph_class_in_dictionary().is_adjective();
                if npt.is_some() || is_adj {
                    return Some((DefinitionKind::Assertation, r0));
                }
            }
        }
        return None;
    }

    // "СЛЕДУЕТ/СЛЕДОВАТЬ/МОЖНО ПОНИМАТЬ/СЧИТАТЬ [КАК] ..."
    if matches!(term_str.as_str(), "СЛЕДУЕТ" | "СЛЕДОВАТЬ" | "МОЖНО") {
        if let Some(next) = next_opt {
            let (is_ponimay, inner_next) = {
                let nb = next.borrow();
                let ok = nb.is_value("ПОНИМАТЬ", None)
                    || nb.is_value("СЧИТАТЬ", None)
                    || nb.is_value("ОПРЕДЕЛИТЬ", None);
                let nn = nb.next.clone();
                (ok, nn)
            };
            if is_ponimay {
                let mut r0 = inner_next?;
                if r0.borrow().is_value("КАК", None) {
                    let nx = r0.borrow().next.clone()?;
                    r0 = nx;
                }
                return Some((DefinitionKind::Assertation, r0));
            }
        }
        return None;
    }

    // "ВЫРАЖАТЬ [...]"
    if term_str == "ВЫРАЖАТЬ" {
        let mut r0 = next_opt?;
        loop {
            let (mc_is_pronoun, mc_is_prep, mc_is_conj, is_comma, nx) = {
                let rb = r0.borrow();
                let mc = rb.get_morph_class_in_dictionary();
                (mc.is_pronoun(), mc.is_preposition(), mc.is_conjunction(),
                 rb.is_comma(sofa), rb.next.clone())
            };
            if mc_is_pronoun || mc_is_prep || mc_is_conj || is_comma {
                match nx { Some(n) => { r0 = n; continue; } None => return None }
            }
            break;
        }
        let npt = npt_try_parse(&r0, NounPhraseParseAttr::No, 0, sofa);
        if npt.is_some() {
            return Some((DefinitionKind::Assertation, r0));
        }
        return None;
    }

    // Various assertion verbs: может, должен, подлежит, принимает, имеет, etc.
    if is_assertion_verb(&term_str) {
        return Some((DefinitionKind::Assertation, connector.clone()));
    }

    None
}

/// Parse right side for copula verbs (является/есть/признаётся).
fn parse_cop_right(
    next_opt: Option<TokenRef>,
    _is_negation: bool,
    sofa: &SourceOfAnalysis,
) -> Option<(DefinitionKind, TokenRef)> {
    let mut r0 = next_opt?;
    loop {
        let (mc_is_prep, mc_is_conj, is_comma, is_ne, nx) = {
            let rb = r0.borrow();
            let mc = rb.get_morph_class_in_dictionary();
            (mc.is_preposition(), mc.is_conjunction(),
             rb.is_comma(sofa), rb.is_value("НЕ", None), rb.next.clone())
        };
        if is_ne {
            let nx2 = nx?;
            let npt = npt_try_parse(&nx2, NounPhraseParseAttr::No, 0, sofa);
            if npt.is_some() {
                return Some((DefinitionKind::Negation, nx2));
            }
            return None;
        }
        if mc_is_prep || mc_is_conj || is_comma {
            match nx { Some(n) => { r0 = n; continue; } None => return None }
        }
        break;
    }
    let npt = npt_try_parse(&r0, NounPhraseParseAttr::No, 0, sofa);
    if npt.is_some() {
        return Some((DefinitionKind::Assertation, r0));
    }
    let is_adj  = r0.borrow().get_morph_class_in_dictionary().is_adjective();
    if is_adj { return Some((DefinitionKind::Assertation, r0)); }
    // "один из" pattern
    let is_odin = r0.borrow().is_value("ОДИН", None);
    if is_odin {
        let next_of_r0 = r0.borrow().next.clone();
        let has_iz = next_of_r0.map(|nx| nx.borrow().is_value("ИЗ", None)).unwrap_or(false);
        if has_iz {
            return Some((DefinitionKind::Assertation, r0));
        }
    }
    None
}

// ── collect_right_side ────────────────────────────────────────────────────────

/// Scan right-side tokens until end of sentence.
/// Returns (first, last) tokens, or None if right side is empty.
fn collect_right_side(
    r0: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(TokenRef, TokenRef)> {
    // Skip leading non-letter punctuation
    let mut r0 = r0.clone();
    {
        let is_letter = r0.borrow().chars.is_letter();
        let has_next  = r0.borrow().next.is_some();
        if !is_letter && has_next {
            let next = r0.borrow().next.clone().unwrap();
            r0 = next;
        }
    }

    let mut r1: Option<TokenRef> = None;
    let mut cur = r0.clone();

    loop {
        // Check for sentence boundary (only once we have something)
        if r1.is_some() && can_be_start_of_sentence(&cur, sofa) {
            break;
        }

        let (is_dot_semi, is_newline_after) = {
            let tb = cur.borrow();
            (tb.is_char_of(".;", sofa), tb.is_newline_after(sofa))
        };

        if is_dot_semi && is_newline_after {
            break;
        }

        // Accumulate (not trailing "." or ";")
        if !is_dot_semi {
            r1 = Some(cur.clone());
        }

        let next = cur.borrow().next.clone();
        match next {
            None => break,
            Some(nx) => {
                let nl = nx.borrow().is_newline_before(sofa);
                if nl && can_be_start_of_sentence(&nx, sofa) {
                    break;
                }
                cur = nx;
            }
        }
    }

    match r1 {
        Some(r1) => Some((r0, r1)),
        None => None,
    }
}

// ── bracket helpers ───────────────────────────────────────────────────────────

/// Skip a bracket group starting at open_tok "(…)".
/// Returns the token AFTER the closing ")" or None.
fn skip_bracket_group(open_tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let mut depth = 1i32;
    let mut scan = open_tok.borrow().next.clone();
    loop {
        let tt = scan?;
        if tt.borrow().is_char('(', sofa) { depth += 1; }
        if tt.borrow().is_char(')', sofa) {
            depth -= 1;
            if depth == 0 {
                return tt.borrow().next.clone();
            }
        }
        scan = tt.borrow().next.clone();
    }
}

/// Find the closing ")" matching an opening "(".
/// `inner_start` is the first token INSIDE the open paren.
/// Returns the ")" token itself.
fn find_matching_close_paren(inner_start: &TokenRef, sofa: &SourceOfAnalysis) -> TokenRef {
    let mut depth = 1i32;
    let mut cur = inner_start.clone();
    loop {
        if cur.borrow().is_char('(', sofa) { depth += 1; }
        if cur.borrow().is_char(')', sofa) {
            depth -= 1;
            if depth == 0 { return cur; }
        }
        let next = cur.borrow().next.clone();
        match next {
            Some(nx) => cur = nx,
            None     => return cur, // best we can do
        }
    }
}

// ── NounPhrase helpers ────────────────────────────────────────────────────────

use crate::core::noun_phrase::NounPhraseToken;

fn npt_has_pronoun(np: &NounPhraseToken) -> bool {
    let mut scan = np.begin_token.clone();
    let end_c = np.end_token.borrow().end_char;
    loop {
        if scan.borrow().begin_char > end_c { break; }
        if scan.borrow().get_morph_class_in_dictionary().is_pronoun() {
            if !scan.borrow().is_value("ИНОЙ", None) {
                return true;
            }
        }
        let next = scan.borrow().next.clone();
        match next { Some(nx) => scan = nx, None => break }
    }
    false
}

// ── helper predicates ─────────────────────────────────────────────────────────

fn is_copula_verb(term: &str) -> bool {
    matches!(term,
        "ЯВЛЯЕТСЯ" | "ЯВЛЯТЬСЯ" | "ЕСТЬ" | "ПРИЗНАЁТСЯ" | "ПРИЗНАВАТЬСЯ" |
        "ОЗНАЧАТЬ" | "НЕСТИ" | "ПРЕДСТАВЛЯТЬ" | "ВЫРАЖАТЬ" |
        "СЛЕДУЕТ" | "СЛЕДОВАТЬ" | "МОЖНО"
    )
}

fn is_assertion_verb(term: &str) -> bool {
    matches!(term,
        "МОЖЕТ" | "МОЧЬ" | "ВПРАВЕ" | "ЗАПРЕЩЕНО" | "РАЗРЕШЕНО" |
        "ОТВЕЧАТЬ" | "ПРИЗНАВАТЬ" | "ОСВОБОЖДАТЬ" | "ОСУЩЕСТВЛЯТЬ" |
        "ПРОИЗВОДИТЬ" | "ПОДЛЕЖАТЬ" | "ПРИНИМАТЬ" | "СЧИТАТЬ" |
        "ИМЕТЬ" | "ОБЯЗАН" | "ОБЯЗАТЬ" | "ДОЛЖЕН" | "ДОЛЖНЫЙ"
    )
}

fn is_forbidden_first_word(t: &TokenRef) -> bool {
    let forbidden = [
        "ЦЕЛЬ", "БОЛЬШИНСТВО", "ЧАСТЬ", "ЗАДАЧА", "ИСКЛЮЧЕНИЕ",
        "ПРИМЕР", "ЭТАП", "ШАГ", "СЛЕДУЮЩИЙ", "ПОДОБНЫЙ",
        "АНАЛОГИЧНЫЙ", "ПРЕДЫДУЩИЙ", "ПОХОЖИЙ", "СХОЖИЙ",
        "НАЙДЕННЫЙ", "НАИБОЛЕЕ", "НАИМЕНЕЕ", "ВАЖНЫЙ",
        "РАСПРОСТРАНЁННЫЙ", "РАСПРОСТРАНЕННЫЙ",
    ];
    for f in &forbidden {
        if t.borrow().is_value(f, None) { return true; }
    }
    false
}

fn is_forbidden_last_phrase(t: &TokenRef) -> bool {
    let forbidden = [
        "СТАТЬЯ", "ГЛАВА", "РАЗДЕЛ", "КОДЕКС", "ЗАКОН",
        "ФОРМУЛИРОВКА", "НАСТОЯЩИЙ", "ВЫШЕУКАЗАННЫЙ", "ДАННЫЙ",
    ];
    for f in &forbidden {
        if t.borrow().is_value(f, None) { return true; }
    }
    false
}
