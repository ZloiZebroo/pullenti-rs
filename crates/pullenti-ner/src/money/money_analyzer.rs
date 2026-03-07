/// Money analyzer — ports MoneyAnalyzer.cs.
///
/// Supported patterns:
///   [CUR_SYMBOL] NUM [.NUM] [CUR_WORD]
///   NUM [.NUM] [CUR_WORD]

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::money::money_referent as mr;
use crate::money::currency_table;

pub struct MoneyAnalyzer;

impl MoneyAnalyzer {
    pub fn new() -> Self { MoneyAnalyzer }
}

impl Analyzer for MoneyAnalyzer {
    fn name(&self) -> &'static str { "MONEY" }
    fn caption(&self) -> &'static str { "Деньги" }

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
    // Pattern A: currency symbol before number  (e.g. "$100", "€500")
    if let Some(cur_iso) = leading_currency_symbol(t, sofa) {
        let num_tok = t.borrow().next.clone()?;
        if !is_number_token(&num_tok) { return None; }
        let (int_val, frac_val, end_tok) = parse_number_with_fraction(&num_tok, sofa);
        let mut r = mr::new_money_referent();
        mr::set_currency(&mut r, cur_iso);
        mr::set_value(&mut r, &int_val);
        mr::set_rest(&mut r, frac_val);
        return Some((r, end_tok));
    }

    // Pattern B: number [fraction] currency_word
    if !is_number_token(t) { return None; }
    let (int_val, frac_val, after_num) = parse_number_with_fraction(t, sofa);

    // Skip optional hyphen between number and currency
    let currency_start = skip_connector(after_num.borrow().next.clone(), sofa);

    // Try to find a currency word
    let (iso, cur_end) = find_currency_word(currency_start, sofa)?;
    if currency_table::is_subunit(&iso) { return None; }

    let mut r = mr::new_money_referent();
    mr::set_currency(&mut r, &iso);
    mr::set_value(&mut r, &int_val);
    mr::set_rest(&mut r, frac_val);
    Some((r, cur_end))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_number_token(t: &TokenRef) -> bool {
    matches!(t.borrow().kind, TokenKind::Number(_))
}

/// If `t` is a single currency-symbol character (e.g. '$', '€'), return its ISO code.
fn leading_currency_symbol<'a>(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<&'static str> {
    let tb = t.borrow();
    if tb.length_char() != 1 { return None; }
    let ch = sofa.char_at(tb.begin_char);
    if ch == '\0' { return None; }
    let s = ch.to_string();
    drop(tb);
    currency_table::lookup(&s)
}

/// Parse a (possibly multi-token) number with optional thousands separators and decimal fraction.
///
/// Handles Russian/European notation: `1.120.000.001,99`
///   - periods between groups of exactly 3 digits are thousands separators
///   - comma (or final period) + 1-2 digit suffix is the fractional kopeck part
///
/// Returns `(integer_string, kopeck_int, end_token)`.
fn parse_number_with_fraction(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> (String, i32, TokenRef) {
    let first_val = match &t.borrow().kind {
        TokenKind::Number(n) => n.value.clone(),
        _ => return ("0".to_string(), 0, t.clone()),
    };

    // Collect integer segments separated by thousands-separator dots.
    // A segment separator "." is a thousands-separator iff the NEXT token is a
    // Number of exactly 3 digits and there is no whitespace around the dot.
    let mut segments: Vec<String> = vec![first_val];
    let mut end_tok = t.clone();

    loop {
        let sep_opt = end_tok.borrow().next.clone();
        let sep = match sep_opt {
            Some(s) => s,
            None => break,
        };
        {
            let sb = sep.borrow();
            if sb.whitespaces_before_count(sofa) != 0 || sb.length_char() != 1 { break; }
            if sofa.char_at(sb.begin_char) != '.' { break; }
        }
        // Peek at the token after the dot
        let after_sep = sep.borrow().next.clone();
        let num_after = match after_sep {
            Some(n) => n,
            None => break,
        };
        {
            let nb = num_after.borrow();
            if nb.whitespaces_before_count(sofa) != 0 { break; }
            match &nb.kind {
                TokenKind::Number(n) => {
                    // Only consume as thousands-separator if exactly 3 digits
                    if n.value.len() != 3 { break; }
                    segments.push(n.value.clone());
                }
                _ => break,
            }
        }
        end_tok = num_after;
    }

    let int_str = segments.join("");

    // Now check for decimal fraction (comma or period + 1-2 digits, no whitespace)
    let after_int = end_tok.borrow().next.clone();
    if let Some(ref sep) = after_int {
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
                            drop(fb);
                            if frac_str.len() <= 2 {
                                if let Ok(frac_int) = frac_str.parse::<i32>() {
                                    let kopecks = match frac_str.len() {
                                        1 => frac_int * 10,
                                        2 => frac_int,
                                        _ => 0,
                                    };
                                    return (int_str, kopecks, frac_tok.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (int_str, 0, end_tok)
}

/// Skip an optional hyphen connector.
fn skip_connector(t: Option<TokenRef>, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let tok = t?;
    if tok.borrow().is_hiphen(sofa) {
        tok.borrow().next.clone()
    } else {
        Some(tok)
    }
}

/// Try to match a currency word at `t`.  Returns `(ISO_code, end_token)`.
fn find_currency_word(
    t: Option<TokenRef>,
    sofa: &SourceOfAnalysis,
) -> Option<(String, TokenRef)> {
    let tok = t?;

    // Try two-token phrase first (e.g. "австралийский доллар")
    {
        let next = tok.borrow().next.clone();
        if let Some(ref n) = next {
            if let Some(ph) = two_word_phrase(&tok, n, sofa) {
                if let Some(iso) = currency_table::lookup(&ph) {
                    return Some((iso.to_string(), n.clone()));
                }
            }
        }
    }

    // Single-token
    let term = token_currency_term(&tok, sofa)?;
    let iso = currency_table::lookup(&term)?;
    Some((iso.to_string(), tok.clone()))
}

/// Return a lookup key for `tok` (tries normal forms then raw term).
fn token_currency_term(tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<String> {
    let tb = tok.borrow();
    match &tb.kind {
        TokenKind::Text(t_data) => {
            let raw_term = t_data.term.to_uppercase();
            // Try morph normal forms (handles inflected "рублей"→"РУБЛЬ")
            let mut candidates: Vec<String> = Vec::new();
            for wf in tok.borrow().morph.items() {
                if let Some(nc) = &wf.normal_case { candidates.push(nc.to_uppercase()); }
                if let Some(nf) = &wf.normal_full  { candidates.push(nf.to_uppercase()); }
            }
            candidates.push(raw_term);
            let surface = sofa.substring(tb.begin_char, tb.end_char).to_uppercase();
            candidates.push(surface);
            drop(tb);
            for c in candidates {
                if currency_table::lookup(&c).is_some() {
                    return Some(c);
                }
            }
            None
        }
        _ => None,
    }
}

/// Try to form a two-word currency phrase from two adjacent text tokens.
fn two_word_phrase(t1: &TokenRef, t2: &TokenRef, sofa: &SourceOfAnalysis) -> Option<String> {
    if t2.borrow().whitespaces_before_count(sofa) > 1 { return None; }
    let t1b = t1.borrow();
    let t2b = t2.borrow();
    let term1 = match &t1b.kind { TokenKind::Text(t) => t.term.to_uppercase(), _ => return None };
    let term2 = match &t2b.kind { TokenKind::Text(t) => t.term.to_uppercase(), _ => return None };
    Some(format!("{} {}", term1, term2))
}
