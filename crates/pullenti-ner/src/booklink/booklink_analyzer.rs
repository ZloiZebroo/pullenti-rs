/// BookLinkAnalyzer — simplified port of BookLinkAnalyzer.cs.
///
/// Recognizes bibliographic references in two main forms:
///
/// Form 1 — Numbered list entries (literature section):
///   "[1] Иванов И.И. Название книги. М.: Наука, 2020. – 300 с."
///   "1. Иванов И.И. Название. Москва: Изд-во, 2020."
///
/// Form 2 — Inline citations in running text:
///   "[1]"         → BOOKLINKREF with number=1
///   "[1, с. 23]"  → BOOKLINKREF with number=1, pages=23
///
/// Produces:
///   BOOKLINK  — the bibliographic entry itself (ATTR_AUTHOR, ATTR_NAME, ATTR_YEAR, ...)
///   BOOKLINKREF — an in-text citation pointing to a BOOKLINK

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::{Referent, SlotValue};
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use super::booklink_referent as br;
use super::booklink_referent::BookLinkRefType;

pub struct BookLinkAnalyzer;

impl BookLinkAnalyzer {
    pub fn new() -> Self { BookLinkAnalyzer }
}

impl Default for BookLinkAnalyzer {
    fn default() -> Self { BookLinkAnalyzer }
}

impl Analyzer for BookLinkAnalyzer {
    fn name(&self) -> &'static str { "BOOKLINK" }
    fn caption(&self) -> &'static str { "Ссылки на литературу" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();

        // ── Pass 1: detect list entries (numbered bibliography items) ──────────
        // We track a "literature block" counter: when a numbered list is detected
        // the counter goes up and we accept un-numbered entries too.
        let mut lit_block: i32 = 0;
        // Map from list-number string → Rc of the BOOKLINKREF
        let mut refs_by_num: Vec<(String, Rc<RefCell<Referent>>)> = Vec::new();

        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            // Only look at the very start of a line for list entries
            if !t.borrow().is_newline_before(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            // Try to parse a bibliography list entry starting at this line
            match try_parse_list_entry(&t, lit_block > 0, &sofa) {
                None => {
                    lit_block = (lit_block - 1).max(0);
                    cur = t.borrow().next.clone();
                }
                Some(ParseResult { ref_ref, book_ref, ref_end }) => {
                    // Get number before we move ref_ref into Rc
                    let num_str = br::get_ref_number(&ref_ref);

                    // Register BOOKLINK entity (if we have one)
                    let book_rc_opt: Option<Rc<RefCell<Referent>>> = book_ref.map(|book| {
                        let rc = Rc::new(RefCell::new(book));
                        let rc = kit.add_entity(rc);
                        rc
                    });

                    // Build BOOKLINKREF, linking to BOOKLINK
                    let ref_rc = Rc::new(RefCell::new(ref_ref));
                    if let Some(ref bk_rc) = book_rc_opt {
                        br::set_ref_book(&mut ref_rc.borrow_mut(), bk_rc.clone());
                    }
                    let ref_rc = kit.add_entity(ref_rc);

                    // Embed BOOKLINK token (if exists), spanning the full range
                    if let Some(ref bk_rc) = book_rc_opt {
                        let bl_tok = Rc::new(RefCell::new(
                            Token::new_referent(t.clone(), ref_end.clone(), bk_rc.clone())
                        ));
                        kit.embed_token(bl_tok.clone());
                    }

                    // Embed BOOKLINKREF token spanning the full range
                    let bref_tok = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), ref_end.clone(), ref_rc.clone())
                    ));
                    kit.embed_token(bref_tok.clone());
                    cur = bref_tok.borrow().next.clone();

                    if lit_block < 5 { lit_block += 1; }

                    if let Some(num) = num_str {
                        refs_by_num.push((num, ref_rc.clone()));
                    }
                }
            }
        }

        // ── Pass 2: detect inline citations "[N]" and "[N, с. P]" ─────────────
        let mut cur2 = kit.first_token.clone();
        while let Some(t) = cur2.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur2 = t.borrow().next.clone();
                continue;
            }
            // Skip referent tokens created in pass 1
            if t.borrow().get_referent().is_some() {
                cur2 = t.borrow().next.clone();
                continue;
            }

            if let Some((inline_ref, end_tok)) = try_parse_inline_citation(&t, &sofa) {
                let num_str = br::get_ref_number(&inline_ref);

                // Try to find the BOOKLINKREF that has this number and link the book
                let book_to_link: Option<Rc<RefCell<Referent>>> = num_str.as_ref().and_then(|n| {
                    refs_by_num.iter()
                        .find(|(k, _)| k == n)
                        .and_then(|(_, bref_rc)| {
                            bref_rc.borrow().slots.iter()
                                .find(|s| s.type_name == br::REF_ATTR_BOOK)
                                .and_then(|s| s.value.as_ref())
                                .and_then(|v| if let SlotValue::Referent(r) = v { Some(r.clone()) } else { None })
                        })
                });

                let mut final_ref = inline_ref;
                if let Some(bk) = book_to_link {
                    br::set_ref_book(&mut final_ref, bk);
                }

                br::set_ref_type(&mut final_ref, BookLinkRefType::Inline);
                let r_rc = Rc::new(RefCell::new(final_ref));
                let r_rc = kit.add_entity(r_rc);
                let tok = Rc::new(RefCell::new(
                    Token::new_referent(t.clone(), end_tok, r_rc)
                ));
                kit.embed_token(tok.clone());
                cur2 = tok.borrow().next.clone();
            } else {
                cur2 = t.borrow().next.clone();
            }
        }
    }
}

// ── Result type ───────────────────────────────────────────────────────────────

struct ParseResult {
    ref_ref: Referent,          // the BOOKLINKREF
    book_ref: Option<Referent>, // the BOOKLINK (or None if no title found)
    ref_end: TokenRef,          // inclusive end token
}

// ── Pass-1 parser: detect numbered bibliography list entries ──────────────────

/// Try to parse a bibliographic list entry starting at token `t`.
///
/// Patterns:
///   "[1] Author. Title. City: Press, 2020."
///   "1. Author. Title. City: Press, 2020."
///   "Author, A.A. Title. – Москва: Наука, 2019. – 250 с."  (when in lit block)
fn try_parse_list_entry(
    t: &TokenRef,
    in_lit_block: bool,
    sofa: &SourceOfAnalysis,
) -> Option<ParseResult> {
    let mut cur = t.clone();
    let mut list_num: Option<String> = None;

    // ── Try to consume "[N]" or "N." number prefix ────────────────────────────
    {
        let first_ch = {
            let tb = cur.borrow();
            sofa.char_at(tb.begin_char)
        };

        if first_ch == '[' {
            // "[N]" pattern: "[" number "]"
            let inner_opt = cur.borrow().next.clone();
            if let Some(inner) = inner_opt {
                let (num_str, close_opt) = {
                    let ib = inner.borrow();
                    match &ib.kind {
                        TokenKind::Number(n) => (Some(n.value.clone()), ib.next.clone()),
                        _ => (None, None),
                    }
                };
                if let (Some(num_str), Some(close)) = (num_str, close_opt) {
                    let is_close = sofa.char_at(close.borrow().begin_char) == ']';
                    if is_close {
                        let next_after = close.borrow().next.clone();
                        list_num = Some(num_str);
                        cur = next_after?;
                    }
                }
            }
        } else {
            // "N." pattern
            let (num_str_opt, next_opt) = {
                let tb = cur.borrow();
                match &tb.kind {
                    TokenKind::Number(n) => (Some(n.value.clone()), tb.next.clone()),
                    _ => (None, None),
                }
            };
            if let (Some(num_str), Some(dot_tok)) = (num_str_opt, next_opt) {
                let is_dot = sofa.char_at(dot_tok.borrow().begin_char) == '.';
                if is_dot {
                    let after_dot = dot_tok.borrow().next.clone();
                    if let Some(after) = after_dot {
                        // Sanity check: the token after the dot should NOT be a newline
                        if after.borrow().is_newline_before(sofa) {
                            return None;
                        }
                        list_num = Some(num_str);
                        cur = after;
                    }
                }
            }
        }
    }

    // If no number prefix and we're not in a lit block, bail out
    if list_num.is_none() && !in_lit_block {
        return None;
    }

    // ── Try to consume author(s) ───────────────────────────────────────────────
    let (authors, cur_after_authors) = collect_authors(&cur, sofa);

    // If no authors and no list number, we need at least something
    if authors.is_empty() && list_num.is_none() {
        return None;
    }

    let title_start = cur_after_authors.clone();

    // ── Collect title text up to a "." or terminator ─────────────────────────
    let (title_str, cur_after_title) = collect_title(&title_start, sofa);

    // ── Collect year, city, publisher, pages ──────────────────────────────────
    let (year, pages, _city, cur_end) = collect_biblio_fields(&cur_after_title, sofa);

    // ── Determine final end token ─────────────────────────────────────────────
    let end_tok = cur_end.clone()
        .or_else(|| cur_after_title.clone())
        .unwrap_or_else(|| title_start.clone());

    // Need SOME content — author or title
    if authors.is_empty() && title_str.is_none() {
        return None;
    }

    // Build BOOKLINK referent
    let mut book = br::new_booklink_referent();
    for (author_str, author_ref_opt) in &authors {
        if let Some(aref) = author_ref_opt {
            br::add_author_ref(&mut book, aref.clone());
        } else {
            br::add_author_str(&mut book, author_str);
        }
    }
    if let Some(ref title) = title_str {
        br::set_name(&mut book, title);
    }
    if let Some(y) = year {
        br::set_year(&mut book, y);
    }

    // Build BOOKLINKREF referent
    let mut bref = br::new_booklinkref_referent();
    if let Some(ref num) = list_num {
        br::set_ref_number(&mut bref, num);
    }
    if let Some(p) = pages {
        br::set_ref_pages(&mut bref, &p);
    }

    Some(ParseResult {
        ref_ref: bref,
        book_ref: Some(book),
        ref_end: end_tok,
    })
}

// ── Author collection ─────────────────────────────────────────────────────────

/// Collect a sequence of author name tokens from position `t`.
/// Returns a vector of (display_string, optional_PersonReferent_rc) and the
/// token after the last consumed author token.
fn collect_authors(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> (Vec<(String, Option<Rc<RefCell<Referent>>>)>, TokenRef) {
    let mut authors: Vec<(String, Option<Rc<RefCell<Referent>>>)> = Vec::new();
    let mut cur = t.clone();

    loop {
        // Skip separators between authors: ", " / "; "
        let probe = {
            let tb = cur.borrow();
            let ch = sofa.char_at(tb.begin_char);
            if ch == ',' || ch == ';' {
                tb.next.clone().unwrap_or_else(|| cur.clone())
            } else {
                cur.clone()
            }
        };

        // Check for PERSON referent token
        let person_rc = probe.borrow().get_referent().filter(|r| {
            r.borrow().type_name == "PERSON"
        });
        if let Some(rc) = person_rc {
            let bc = probe.borrow().begin_char;
            let ec = probe.borrow().end_char;
            let name = sofa.substring(bc, ec).to_string();
            authors.push((name, Some(rc)));
            let next = probe.borrow().next.clone();
            cur = next.unwrap_or(probe);
            continue;
        }

        // "и др." / "et al." — stop
        let is_and = probe.borrow().is_value("И", Some("І"))
            || probe.borrow().is_value("ET", None);
        if is_and {
            let next = probe.borrow().next.clone();
            if let Some(ref n) = next {
                let nb = n.borrow();
                let is_others = nb.is_value("ДР", None)
                    || nb.is_value("ДРУГИЕ", None)
                    || nb.is_value("AL", None);
                if is_others {
                    let after = nb.next.clone();
                    drop(nb);
                    cur = if let Some(after_t) = after {
                        let ch = sofa.char_at(after_t.borrow().begin_char);
                        if ch == '.' {
                            let next_after_dot = after_t.borrow().next.clone();
                            next_after_dot.unwrap_or(after_t)
                        } else {
                            after_t
                        }
                    } else {
                        n.clone()
                    };
                    break;
                }
            }
        }

        // Try initials+surname or surname+initials pattern
        if let Some((author_str, end_tok)) = try_parse_author_pattern(&probe, sofa) {
            authors.push((author_str, None));
            let next = end_tok.borrow().next.clone();
            cur = next.unwrap_or(end_tok);
            continue;
        }

        break;
    }

    (authors, cur)
}

/// Try to parse a "Surname I.I." or "I.I. Surname" author pattern.
///
/// Heuristic: a capitalized word followed by 1-2 uppercase+dot abbreviations,
/// or vice versa.  Returns (display_text, end_token).
fn try_parse_author_pattern(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(String, TokenRef)> {
    // Must start with a capital letter text token
    {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(_) => {
                if !tb.chars.is_capital_upper() { return None; }
            }
            _ => return None,
        }
    }

    let start_bc = t.borrow().begin_char;
    let next1 = t.borrow().next.clone();

    // Look for abbreviation(s) like "И." "И.И." following the first word
    let (after_initials, has_initials) = consume_initials(&next1, sofa);

    if has_initials {
        // Pattern: Surname I.I. [possible surname at end]
        // Check if after initials there's another capital word (unlikely — usually done)
        let end_tok = if let Some(ref ai) = after_initials {
            let aib = ai.borrow();
            if let TokenKind::Text(_) = &aib.kind {
                if aib.chars.is_capital_upper() {
                    drop(aib);
                    // It's "I.I. Surname" — surname is after initials
                    ai.clone()
                } else {
                    drop(aib);
                    // Initials came after surname — end is last initials dot
                    find_last_initials_tok(&next1, sofa).unwrap_or_else(|| t.clone())
                }
            } else {
                drop(aib);
                find_last_initials_tok(&next1, sofa).unwrap_or_else(|| t.clone())
            }
        } else {
            find_last_initials_tok(&next1, sofa).unwrap_or_else(|| t.clone())
        };

        let end_ec = end_tok.borrow().end_char;
        let text = sofa.substring(start_bc, end_ec).to_string();
        return Some((text, end_tok));
    }

    // No initials after first word. Try: first token is initials, next is surname.
    // Check next token for initials
    if let Some(ref next) = next1 {
        let (after_init2, has_init2) = consume_initials(&Some(next.clone()), sofa);
        if has_init2 {
            // Check if what follows initials is a capital word (the surname)
            if let Some(ref surname_candidate) = after_init2 {
                let sb = surname_candidate.borrow();
                if let TokenKind::Text(_) = &sb.kind {
                    if sb.chars.is_capital_upper() {
                        let end_ec = sb.end_char;
                        let end_tok = surname_candidate.clone();
                        drop(sb);
                        let text = sofa.substring(start_bc, end_ec).to_string();
                        return Some((text, end_tok));
                    }
                }
            }
            // Still have initials but no trailing surname — "I.I." pattern
            let end_tok = find_last_initials_tok(&Some(next.clone()), sofa)
                .unwrap_or_else(|| t.clone());
            let end_ec = end_tok.borrow().end_char;
            let text = sofa.substring(start_bc, end_ec).to_string();
            return Some((text, end_tok));
        }
    }

    None
}

/// Consume a sequence of uppercase-letter+dot abbreviations (initials like "И.").
/// Returns (token_after_initials, found_any).
fn consume_initials(
    start: &Option<TokenRef>,
    sofa: &SourceOfAnalysis,
) -> (Option<TokenRef>, bool) {
    let first = match start {
        Some(t) => t.clone(),
        None => return (None, false),
    };
    let mut cur = first;
    let mut found = false;

    loop {
        let (ch, length, next_opt) = {
            let tb = cur.borrow();
            (sofa.char_at(tb.begin_char), tb.length_char(), tb.next.clone())
        };

        // Single uppercase letter — expect a dot after
        if length == 1 && ch.is_uppercase() {
            match next_opt {
                Some(dot_tok) => {
                    let (dot_ch, dot_next) = {
                        let db = dot_tok.borrow();
                        (sofa.char_at(db.begin_char), db.next.clone())
                    };
                    if dot_ch == '.' {
                        found = true;
                        match dot_next {
                            Some(after) => { cur = after; }
                            None => return (None, found),
                        }
                    } else {
                        return (Some(dot_tok), found);
                    }
                }
                None => return (None, found),
            }
        } else {
            return (Some(cur), found);
        }
    }
}

/// Find the last token of an initials sequence starting from `start`.
fn find_last_initials_tok(
    start: &Option<TokenRef>,
    sofa: &SourceOfAnalysis,
) -> Option<TokenRef> {
    let first = start.as_ref()?.clone();
    let (after_opt, found) = consume_initials(start, sofa);
    if !found { return None; }

    // Walk from first forward stopping just before `after_opt`
    let mut last = first.clone();
    let mut cur = first;
    loop {
        let is_stop = match &after_opt {
            Some(a) => Rc::ptr_eq(a, &cur),
            None => false,
        };
        if is_stop { break; }
        let next = cur.borrow().next.clone();
        match next {
            Some(n) => {
                last = cur;
                cur = n;
            }
            None => break,
        }
    }
    Some(last)
}

// ── Title collection ──────────────────────────────────────────────────────────

/// Collect title text from current position until a bibliographic terminator.
/// Returns (title_string, token_after_title).
fn collect_title(
    start: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> (Option<String>, Option<TokenRef>) {
    let mut cur = start.clone();

    // Skip leading punctuation
    {
        let ch = sofa.char_at(cur.borrow().begin_char);
        if ch == '.' || ch == ':' || ch == ',' || ch == ';' || ch == '–' || ch == '-' {
            let next = cur.borrow().next.clone();
            match next {
                Some(n) => cur = n,
                None => return (None, None),
            }
        }
    }

    let mut title_start_char: Option<i32> = None;
    let mut title_end_char: i32 = -1;
    let mut result_next: Option<TokenRef> = None;

    loop {
        let (begin_char, end_char, is_nl, ch) = {
            let tb = cur.borrow();
            (tb.begin_char, tb.end_char, tb.is_newline_before(sofa), sofa.char_at(tb.begin_char))
        };

        // Stop at newline if we have content
        if is_nl && title_start_char.is_some() {
            result_next = Some(cur.clone());
            break;
        }

        // Stop at "–" dash (em-dash usually precedes city in Russian refs)
        if ch == '–' {
            result_next = Some(cur.clone());
            break;
        }

        // Stop at "/" preceded by whitespace (delimeter between title and source)
        if ch == '/' {
            let is_ws = cur.borrow().is_whitespace_before(sofa);
            if is_ws {
                result_next = Some(cur.clone());
                break;
            }
        }

        // Check if this looks like a bibliographic field that ends the title
        if is_biblio_terminator(&cur, sofa) {
            result_next = Some(cur.clone());
            break;
        }

        // Accumulate this token
        if title_start_char.is_none() {
            title_start_char = Some(begin_char);
        }
        title_end_char = end_char;

        let next = cur.borrow().next.clone();

        // At a "." check if next token is capital (new sentence) or newline
        if sofa.char_at(end_char) == '.' {
            if let Some(ref next_t) = next {
                let nb = next_t.borrow();
                let next_nl = nb.is_newline_before(sofa);
                let next_cap = nb.chars.is_capital_upper();
                drop(nb);
                if next_nl || next_cap {
                    result_next = Some(next_t.clone());
                    break;
                }
                cur = next_t.clone();
                continue;
            } else {
                break;
            }
        }

        match next {
            Some(n) => cur = n,
            None => break,
        }
    }

    if let Some(ts) = title_start_char {
        if title_end_char >= ts {
            let s = sofa.substring(ts, title_end_char).trim().to_string();
            if !s.is_empty() {
                return (Some(s), result_next);
            }
        }
    }
    (None, result_next)
}

/// Returns true if this token looks like a bibliographic field that ends the title.
fn is_biblio_terminator(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    let ch = sofa.char_at(tb.begin_char);

    // Year in parentheses: "(" number ")" where number is 1900-2100
    if ch == '(' {
        if let Some(ref next) = tb.next {
            let nb = next.borrow();
            if let TokenKind::Number(n) = &nb.kind {
                let v: i32 = n.value.parse().unwrap_or(0);
                if (1900..=2100).contains(&v) {
                    return true;
                }
            }
        }
    }

    // City abbreviations: "М." "СПб." followed by ":"
    if let TokenKind::Text(txt) = &tb.kind {
        let s = txt.term.as_str();
        if matches!(s, "М" | "СПБ" | "Л" | "К") {
            if let Some(ref next) = tb.next {
                let nc = sofa.char_at(next.borrow().begin_char);
                if nc == ':' || nc == '.' || nc == ',' {
                    return true;
                }
            }
        }
    }

    // GEO referent followed by ":"
    drop(tb);
    if let Some(geo_rc) = t.borrow().get_referent() {
        if geo_rc.borrow().type_name == "GEO" {
            if let Some(ref next) = t.borrow().next {
                let nc = sofa.char_at(next.borrow().begin_char);
                if nc == ':' {
                    return true;
                }
            }
        }
    }

    false
}

// ── Bibliographic fields collection ───────────────────────────────────────────

/// Scan forward from `start` collecting year, pages, city.
/// Returns (year, pages_string, city, last_consumed_token).
fn collect_biblio_fields(
    start: &Option<TokenRef>,
    sofa: &SourceOfAnalysis,
) -> (Option<i32>, Option<String>, Option<String>, Option<TokenRef>) {
    let start_t = match start {
        Some(t) => t.clone(),
        None => return (None, None, None, None),
    };
    let mut cur = start_t;
    let mut year: Option<i32> = None;
    let mut pages: Option<String> = None;
    let mut city: Option<String> = None;
    let mut last_end: Option<TokenRef> = None;
    let mut iterations = 0;
    let mut seen_newline = false;

    loop {
        iterations += 1;
        if iterations > 200 { break; }

        let (begin_char, is_nl) = {
            let tb = cur.borrow();
            (tb.begin_char, tb.is_newline_before(sofa))
        };

        // Stop after second newline
        if is_nl {
            if seen_newline {
                break;
            }
            seen_newline = true;
        }

        let ch = sofa.char_at(begin_char);

        // Skip common separators
        if ch == '.' || ch == ',' || ch == ';' || ch == ':' || ch == '–'
            || ch == '-' || ch == '(' || ch == ')' || ch == ' ' {
            let next = cur.borrow().next.clone();
            match next { Some(n) => { cur = n; continue; } None => break }
        }

        // GEO referent → city
        let geo_opt = cur.borrow().get_referent().filter(|r| {
            r.borrow().type_name == "GEO"
        });
        if let Some(geo_rc) = geo_opt {
            if city.is_none() {
                let name = {
                    let gb = geo_rc.borrow();
                    gb.get_string_value("NAME")
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                };
                if !name.is_empty() {
                    city = Some(name);
                } else {
                    let bc = cur.borrow().begin_char;
                    let ec = cur.borrow().end_char;
                    city = Some(sofa.substring(bc, ec).to_string());
                }
                last_end = Some(cur.clone());
            }
            let next = cur.borrow().next.clone();
            match next { Some(n) => { cur = n; continue; } None => break }
        }

        // Inspect token kind
        let (kind_num, kind_term, next_tok) = {
            let tb = cur.borrow();
            let next = tb.next.clone();
            match &tb.kind {
                TokenKind::Number(n) => (Some(n.value.clone()), None, next),
                TokenKind::Text(txt) => (None, Some(txt.term.clone()), next),
                _ => (None, None, next),
            }
        };

        if let Some(num_str) = kind_num {
            let v: i32 = num_str.parse().unwrap_or(0);
            if (1900..=2100).contains(&v) && year.is_none() {
                year = Some(v);
                last_end = Some(cur.clone());
            }
            match next_tok { Some(n) => { cur = n; continue; } None => break }
        }

        if let Some(term) = kind_term {
            // Page keywords
            if matches!(term.as_str(), "С" | "СТР" | "P" | "PP" | "PAGES" | "СТОР") {
                if let Some(ref nt) = next_tok {
                    // skip optional "."
                    let num_start = {
                        let ch2 = sofa.char_at(nt.borrow().begin_char);
                        if ch2 == '.' {
                            nt.borrow().next.clone()
                        } else {
                            Some(nt.clone())
                        }
                    };
                    if let Some(ref ns) = num_start {
                        if let Some((page_str, page_end)) = try_read_page_range(ns, sofa) {
                            pages = Some(page_str);
                            last_end = Some(page_end.clone());
                            let after = page_end.borrow().next.clone();
                            match after { Some(n) => { cur = n; continue; } None => break }
                        }
                    }
                }
            }

            // City abbreviations: "М", "СПБ", "Л", "К"
            if matches!(term.as_str(), "М" | "СПБ" | "Л" | "К") && city.is_none() {
                city = Some(term.to_string());
                last_end = Some(cur.clone());
            }

            match next_tok { Some(n) => { cur = n; continue; } None => break }
        }

        // Anything else — just advance
        let next = cur.borrow().next.clone();
        match next { Some(n) => { cur = n; } None => break }
    }

    (year, pages, city, last_end)
}

/// Try to read a page number or range starting at `t`.
/// Returns (pages_string, end_token).
fn try_read_page_range(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let (val, next_opt) = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Number(n) => (n.value.clone(), tb.next.clone()),
            _ => return None,
        }
    };

    // Check for "-" or "–" range
    if let Some(ref sep) = next_opt {
        let (sep_ch, sep_next) = {
            let sb = sep.borrow();
            (sofa.char_at(sb.begin_char), sb.next.clone())
        };
        if sep_ch == '-' || sep_ch == '–' {
            if let Some(ref num2) = sep_next {
                let v2 = {
                    let n2b = num2.borrow();
                    match &n2b.kind {
                        TokenKind::Number(n2) => Some(n2.value.clone()),
                        _ => None,
                    }
                };
                if let Some(v2) = v2 {
                    let range = format!("{}-{}", val, v2);
                    return Some((range, num2.clone()));
                }
            }
        }
    }

    Some((val, t.clone()))
}

// ── Pass-2 parser: inline citations ──────────────────────────────────────────

/// Try to parse an inline citation like "[1]", "[1, с. 23]".
/// Returns (BOOKLINKREF_referent, end_token).
fn try_parse_inline_citation(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let (first_ch, next_opt) = {
        let tb = t.borrow();
        (sofa.char_at(tb.begin_char), tb.next.clone())
    };

    if first_ch != '[' { return None; }

    let inner = next_opt?;
    let (num_str, after_num_opt) = {
        let ib = inner.borrow();
        match &ib.kind {
            TokenKind::Number(n) => (n.value.clone(), ib.next.clone()),
            _ => return None,
        }
    };

    let close_tok = after_num_opt?;
    let (close_ch, after_close_opt) = {
        let cb = close_tok.borrow();
        (sofa.char_at(cb.begin_char), cb.next.clone())
    };

    if close_ch == ']' {
        // Simple "[N]"
        let mut bref = br::new_booklinkref_referent();
        br::set_ref_number(&mut bref, &num_str);
        return Some((bref, close_tok));
    }

    // "[N, с. P]" pattern — close_ch is ","
    if close_ch == ',' {
        if let Some(page_kw_tok) = after_close_opt {
            let (is_page_kw, kw_next) = {
                let pkb = page_kw_tok.borrow();
                let ok = match &pkb.kind {
                    TokenKind::Text(txt) => matches!(
                        txt.term.as_str(),
                        "С" | "СТР" | "P" | "PP" | "СТОР" | "PAGES"
                    ),
                    _ => false,
                };
                (ok, pkb.next.clone())
            };
            if is_page_kw {
                // skip optional "."
                let num_start = if let Some(ref nt) = kw_next {
                    let dot_ch = sofa.char_at(nt.borrow().begin_char);
                    if dot_ch == '.' {
                        nt.borrow().next.clone()
                    } else {
                        Some(nt.clone())
                    }
                } else {
                    kw_next
                };

                if let Some(ref ns) = num_start {
                    if let Some((page_str, page_end)) = try_read_page_range(ns, sofa) {
                        if let Some(ref close2) = page_end.borrow().next.clone() {
                            let c2_ch = sofa.char_at(close2.borrow().begin_char);
                            if c2_ch == ']' {
                                let mut bref = br::new_booklinkref_referent();
                                br::set_ref_number(&mut bref, &num_str);
                                br::set_ref_pages(&mut bref, &page_str);
                                return Some((bref, close2.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
