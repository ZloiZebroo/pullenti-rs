/// Weapon item token — ports WeaponItemToken.cs (simplified).

use crate::token::{TokenRef, TokenKind, NumberSpellingType};
use crate::source_of_analysis::SourceOfAnalysis;
use super::weapon_table::{WeaponItemTyp, get_ontology, get_typ, is_noun_doubt, get_model_inner};

// ── WeaponItemToken ───────────────────────────────────────────────────────────

pub struct WeaponItemToken {
    pub begin: TokenRef,
    pub end: TokenRef,
    pub typ: WeaponItemTyp,
    pub value: String,
    pub alt_value: Option<String>,
    pub is_doubt: bool,
    pub is_internal: bool,
    pub inner_tokens: Vec<WeaponItemToken>,
}

impl WeaponItemToken {
    fn new(begin: TokenRef, end: TokenRef, typ: WeaponItemTyp, value: String) -> Self {
        WeaponItemToken {
            begin, end, typ, value,
            alt_value: None, is_doubt: false, is_internal: false,
            inner_tokens: vec![],
        }
    }
}

// ── try_parse_list ────────────────────────────────────────────────────────────

pub fn try_parse_list(t_start: &TokenRef, sofa: &SourceOfAnalysis) -> Option<Vec<WeaponItemToken>> {
    let mut tr = _try_parse(t_start, None, sofa)?;

    // Skip Class/Date as first items
    if tr.typ == WeaponItemTyp::Class || tr.typ == WeaponItemTyp::Date {
        return None;
    }

    let mut res: Vec<WeaponItemToken> = Vec::new();

    // Prepend inner tokens first
    let inner_begin = tr.begin.clone();
    let inners = std::mem::take(&mut tr.inner_tokens);
    for mut inner in inners {
        inner.begin = inner_begin.clone();
        res.push(inner);
    }
    let tr0_typ = tr.typ;
    let tr0_begin = tr.begin.clone();
    let tr0_end = tr.end.clone();
    res.push(tr);

    let mut cur = tr0_end.borrow().next.clone();

    // Skip colons/hyphens after noun
    if tr0_typ == WeaponItemTyp::Noun {
        while let Some(ref t) = cur.clone() {
            if t.borrow().is_char(':', sofa) || t.borrow().is_hiphen(sofa) {
                cur = t.borrow().next.clone();
            } else {
                break;
            }
        }
    }

    let max_count = 10usize;
    let last_typ = res.last().map(|x| x.typ).unwrap_or(tr0_typ);
    let _ = tr0_begin; // suppress unused warning

    let mut prev_typ = last_typ;
    let mut prev_end = tr0_end;

    loop {
        if res.len() >= max_count { break; }
        let t = match cur.clone() {
            None => break,
            Some(t) => t,
        };

        // Skip colons
        if t.borrow().is_char(':', sofa) {
            cur = t.borrow().next.clone();
            continue;
        }

        // Hyphen: skip for noun/brand/model contexts
        if t.borrow().is_hiphen(sofa) {
            if prev_typ == WeaponItemTyp::Noun || prev_typ == WeaponItemTyp::Brand || prev_typ == WeaponItemTyp::Model {
                cur = t.borrow().next.clone();
                continue;
            }
            break;
        }

        // Comma: only if looking for Number after brand/model/class
        if t.borrow().is_char(',', sofa) {
            if prev_typ == WeaponItemTyp::Name || prev_typ == WeaponItemTyp::Brand
                || prev_typ == WeaponItemTyp::Model || prev_typ == WeaponItemTyp::Class
                || prev_typ == WeaponItemTyp::Date
            {
                // Try to parse a Number after the comma
                let after_comma = t.borrow().next.clone();
                if let Some(ref ac) = after_comma {
                    if let Some(new_tr) = _try_parse(ac, Some(prev_typ), sofa) {
                        if new_tr.typ == WeaponItemTyp::Number {
                            let new_end = new_tr.end.clone();
                            prev_typ = new_tr.typ;
                            prev_end = new_end.clone();
                            res.push(new_tr);
                            cur = new_end.borrow().next.clone();
                            continue;
                        }
                    }
                }
            }
            break;
        }

        // Newline break: only Numbers are allowed after newline
        if t.borrow().is_newline_before(sofa) {
            if let Some(new_tr) = _try_parse(&t, Some(prev_typ), sofa) {
                if new_tr.typ == WeaponItemTyp::Number {
                    let new_end = new_tr.end.clone();
                    prev_typ = new_tr.typ;
                    prev_end = new_end.clone();
                    res.push(new_tr);
                    cur = new_end.borrow().next.clone();
                    continue;
                }
            }
            break;
        }

        // Normal token
        if let Some(new_tr) = _try_parse(&t, Some(prev_typ), sofa) {
            let new_end = new_tr.end.clone();
            let new_typ = new_tr.typ;
            let inner_begin = new_tr.begin.clone();
            let inners: Vec<WeaponItemToken> = Vec::new();
            // Re-parse to get inner tokens (since we didn't mutate new_tr yet)
            let mut new_tr2 = _try_parse(&t, Some(prev_typ), sofa)?;
            let inner_begin2 = new_tr2.begin.clone();
            let inners2 = std::mem::take(&mut new_tr2.inner_tokens);
            for mut inner in inners2 {
                inner.begin = inner_begin2.clone();
                res.push(inner);
            }
            let _ = (new_tr, inners, inner_begin);
            prev_end = new_tr2.end.clone();
            prev_typ = new_tr2.typ;
            res.push(new_tr2);
            cur = prev_end.borrow().next.clone();
        } else {
            break;
        }
    }

    // Merge consecutive Model items
    let mut i = 0;
    while i + 1 < res.len() {
        if res[i].typ == WeaponItemTyp::Model && res[i + 1].typ == WeaponItemTyp::Model {
            let next_val = res[i + 1].value.clone();
            let next_end = res[i + 1].end.clone();
            res[i].value = format!("{} {}", res[i].value, next_val);
            res[i].end = next_end;
            res.remove(i + 1);
        } else {
            i += 1;
        }
    }

    Some(res)
}

// ── _try_parse (internal) ─────────────────────────────────────────────────────

fn _try_parse(t: &TokenRef, prev_typ: Option<WeaponItemTyp>, sofa: &SourceOfAnalysis) -> Option<WeaponItemToken> {
    // ── 1. Ontology lookup ─────────────────────────────────────────────────
    let tok_opt = get_ontology().try_parse(t);

    if let Some(tok) = tok_opt {
        let canon = tok.termin.canonic_text.clone();
        let typ = get_typ(&tok.termin)?;

        match typ {
            WeaponItemTyp::Noun => {
                let is_doubt = is_noun_doubt(&tok.termin);
                let mut res = WeaponItemToken::new(t.clone(), tok.end_token.clone(), WeaponItemTyp::Noun, canon.clone());
                res.is_doubt = is_doubt;
                // Look ahead for trailing brands/adjectives
                let mut end_cur = tok.end_token.clone();
                loop {
                    let next = end_cur.borrow().next.clone();
                    let next = match next { None => break, Some(n) => n };
                    if next.borrow().whitespaces_before_count(sofa) > 2 { break; }
                    // Check for brand
                    if let Some(inner_tok) = _try_parse(&next, None, sofa) {
                        if inner_tok.typ == WeaponItemTyp::Brand {
                            let inner_end = inner_tok.end.clone();
                            res.inner_tokens.push(inner_tok);
                            end_cur = inner_end.clone();
                            res.end = inner_end;
                            continue;
                        }
                        break;
                    }
                    // Check for adjective modifying the noun
                    if matches!(&next.borrow().kind, TokenKind::Text(_)) {
                        let is_adj = next.borrow().morph.items().iter()
                            .any(|wf| wf.base.class.is_adjective());
                        if is_adj {
                            let term_str = next.borrow().term().unwrap_or("").to_string();
                            if res.alt_value.is_none() {
                                res.alt_value = Some(canon.clone());
                            }
                            if let Some(ref alt) = res.alt_value.clone() {
                                if alt.ends_with(&canon) {
                                    let prefix = &alt[..alt.len() - canon.len()];
                                    res.alt_value = Some(format!("{}{} {}", prefix, term_str, canon));
                                } else {
                                    res.alt_value = Some(format!("{} {}", term_str, canon));
                                }
                            }
                            res.end = next.clone();
                            end_cur = next;
                            continue;
                        }
                    }
                    break;
                }
                return Some(res);
            }

            WeaponItemTyp::Brand | WeaponItemTyp::Name => {
                let res = WeaponItemToken::new(t.clone(), tok.end_token.clone(), typ, canon);
                return Some(res);
            }

            WeaponItemTyp::Model => {
                let mut res = WeaponItemToken::new(t.clone(), tok.end_token.clone(), WeaponItemTyp::Model, canon);
                // Load inner tokens from tag2
                if let Some(inner) = get_model_inner(&tok.termin) {
                    let is_internal = {
                        let begin_char = t.borrow().begin_char;
                        let end_char = tok.end_token.borrow().end_char;
                        begin_char == end_char
                    };
                    for (itype, ivalue, ialt) in &inner.items {
                        let mut it = WeaponItemToken::new(
                            t.clone(), tok.end_token.clone(),
                            *itype, ivalue.to_string()
                        );
                        it.is_internal = is_internal;
                        it.alt_value = ialt.map(|s| s.to_string());
                        res.inner_tokens.push(it);
                    }
                }
                correct_model(&mut res, sofa);
                return Some(res);
            }

            _ => {}
        }
    }

    // ── 2. Short uppercase letter(s) + hyphen/dot + number → Model ─────────
    if matches!(&t.borrow().kind, TokenKind::Text(_)) {
        let is_letter = t.borrow().chars.is_letter();
        let is_all_upper = t.borrow().chars.is_all_upper();
        let len_char = t.borrow().length_char();

        if is_letter && is_all_upper && len_char < 4 {
            let term_str = t.borrow().term().unwrap_or("").to_string();
            let next = t.borrow().next.clone();

            // Pattern: LETTER-hyphen-NUMBER or LETTER-dot-NUMBER
            if let Some(ref sep) = next {
                let is_hyp_or_dot = sep.borrow().is_hiphen(sofa)
                    || sep.borrow().is_char('.', sofa);
                let no_space_after = sep.borrow().whitespaces_before_count(sofa) < 2;

                if is_hyp_or_dot && no_space_after {
                    let after_sep = sep.borrow().next.clone();
                    if let Some(ref num) = after_sep {
                        if matches!(&num.borrow().kind, TokenKind::Number(_)) {
                            let mut res = WeaponItemToken::new(
                                t.clone(), sep.clone(),
                                WeaponItemTyp::Model, term_str
                            );
                            res.is_doubt = true;
                            correct_model(&mut res, sofa);
                            return Some(res);
                        }
                    }
                }
            }

            // Pattern: LETTER(S) immediately followed by NUMBER (no space)
            if let Some(ref num) = next {
                let no_space = !t.borrow().is_whitespace_after(sofa);
                if no_space && matches!(&num.borrow().kind, TokenKind::Number(_)) {
                    let mut res = WeaponItemToken::new(
                        t.clone(), t.clone(),
                        WeaponItemTyp::Model, term_str
                    );
                    res.is_doubt = true;
                    correct_model(&mut res, sofa);
                    return Some(res);
                }
            }
        }
    }

    // ── 3. Contextual Name/Brand detection (when prev is Noun/Brand/Model) ──
    if let Some(prev) = prev_typ {
        if prev == WeaponItemTyp::Noun || prev == WeaponItemTyp::Brand || prev == WeaponItemTyp::Model {
            if matches!(&t.borrow().kind, TokenKind::Text(_)) {
                let is_letter = t.borrow().chars.is_letter();
                let is_not_all_lower = !t.borrow().chars.is_all_lower();
                let len_char = t.borrow().length_char();

                if is_letter && is_not_all_lower && len_char > 2 {
                    let term_str = t.borrow().term().unwrap_or("").to_string();
                    let mut typ = WeaponItemTyp::Name;
                    let mut value = term_str.clone();

                    // Extend with hyphen continuation
                    let next = t.borrow().next.clone();
                    let mut end = t.clone();
                    if let Some(ref n) = next {
                        if n.borrow().is_hiphen(sofa) {
                            let after_hyp = n.borrow().next.clone();
                            if let Some(ref at) = after_hyp {
                                if matches!(&at.borrow().kind, TokenKind::Text(_)) {
                                    let at_chars_eq = at.borrow().chars.is_all_upper() == t.borrow().chars.is_all_upper();
                                    if at_chars_eq {
                                        let at_term = at.borrow().term().unwrap_or("").to_string();
                                        value = format!("{}-{}", value, at_term);
                                        end = at.clone();
                                    }
                                }
                            }
                        }
                    }

                    if prev == WeaponItemTyp::Noun {
                        typ = WeaponItemTyp::Brand;
                    }

                    // Upgrade to Model if followed by hyphen+number
                    let end_next = end.borrow().next.clone();
                    if let Some(ref en) = end_next {
                        if en.borrow().is_hiphen(sofa) {
                            let after_hyp = en.borrow().next.clone();
                            if let Some(ref num) = after_hyp {
                                if matches!(&num.borrow().kind, TokenKind::Number(_)) {
                                    typ = WeaponItemTyp::Model;
                                }
                            }
                        } else if !end.borrow().is_whitespace_after(sofa) {
                            if matches!(&en.borrow().kind, TokenKind::Number(_)) {
                                typ = WeaponItemTyp::Model;
                            }
                        }
                    }

                    let mut res = WeaponItemToken::new(t.clone(), end, typ, value);
                    res.is_doubt = true;
                    if typ == WeaponItemTyp::Model {
                        correct_model(&mut res, sofa);
                    }
                    return Some(res);
                }
            }
        }
    }

    None
}

// ── correct_model ─────────────────────────────────────────────────────────────

/// Extend the model token to include a trailing number (e.g., АК → АК-47).
fn correct_model(res: &mut WeaponItemToken, sofa: &SourceOfAnalysis) {
    let tt = res.end.borrow().next.clone();
    let tt = match tt { None => return, Some(x) => x };

    if tt.borrow().whitespaces_before_count(sofa) > 2 { return; }

    // Skip separator (hyphen, slash, backslash, dot)
    let (skip_sep, num_start) = {
        let is_sep = tt.borrow().is_hiphen(sofa)
            || tt.borrow().is_char('/', sofa)
            || tt.borrow().is_char('\\', sofa)
            || tt.borrow().is_char('.', sofa);
        if is_sep {
            (true, tt.borrow().next.clone())
        } else {
            (false, Some(tt.clone()))
        }
    };

    let num_tok = match num_start { None => return, Some(x) => x };

    if !matches!(&num_tok.borrow().kind, TokenKind::Number(_)) { return; }

    // Get the number value
    let num_val = if let TokenKind::Number(nd) = &num_tok.borrow().kind {
        nd.value.clone()
    } else {
        return;
    };

    let sep_str = if skip_sep { "-" } else { "-" };
    res.value = format!("{}{}{}", res.value, sep_str, num_val);
    if let Some(ref alt) = res.alt_value.clone() {
        res.alt_value = Some(format!("{}{}{}", alt, sep_str, num_val));
    }
    res.end = num_tok.clone();

    // Continue: look for trailing single-letter suffix (e.g., АК-47М)
    let mut cur = num_tok;
    loop {
        let next = cur.borrow().next.clone();
        let next = match next { None => break, Some(n) => n };
        let no_space = !cur.borrow().is_whitespace_after(sofa);
        let is_single_letter = matches!(&next.borrow().kind, TokenKind::Text(_))
            && next.borrow().length_char() == 1
            && next.borrow().chars.is_letter();
        if no_space && is_single_letter {
            let letter = next.borrow().term().unwrap_or("").to_string();
            res.value.push_str(&letter);
            if let Some(ref mut alt) = res.alt_value {
                alt.push_str(&letter);
            }
            res.end = next.clone();
            cur = next;
        } else {
            break;
        }
    }

    // Also handle: end is not followed by space and next is hyphen+number
    let end_next = res.end.borrow().next.clone();
    if let Some(ref en) = end_next {
        let no_space = !res.end.borrow().is_whitespace_after(sofa);
        let en_no_space = en.borrow().whitespaces_before_count(sofa) == 0;
        if no_space && en_no_space && (en.borrow().is_hiphen(sofa) || en.borrow().is_char('/', sofa)) {
            let after = en.borrow().next.clone();
            if let Some(ref num2) = after {
                if matches!(&num2.borrow().kind, TokenKind::Number(_)) {
                    let num2_val = if let TokenKind::Number(nd) = &num2.borrow().kind {
                        nd.value.clone()
                    } else { return; };
                    res.value = format!("{}-{}", res.value, num2_val);
                    if let Some(ref alt) = res.alt_value.clone() {
                        res.alt_value = Some(format!("{}-{}", alt, num2_val));
                    }
                    res.end = num2.clone();
                }
            }
        }
    }
}
