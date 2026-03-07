/// DenominationAnalyzer — detects alphanumeric codes/designations like "C#", "A-320", "1С".
/// Mirrors `DenominationAnalyzer.cs`.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind, NumberSpellingType};
use crate::source_of_analysis::SourceOfAnalysis;
use super::denomination_referent::{
    OBJ_TYPENAME as _, new_denomination_referent, add_value_from_tokens,
};

pub struct DenominationAnalyzer;

impl DenominationAnalyzer {
    pub fn new() -> Self { DenominationAnalyzer }
}

impl Default for DenominationAnalyzer {
    fn default() -> Self { DenominationAnalyzer }
}

impl Analyzer for DenominationAnalyzer {
    fn name(&self)    -> &'static str { "DENOMINATION" }
    fn caption(&self) -> &'static str { "Деноминации" }
    fn is_specific(&self) -> bool { true }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = &kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur {
            let next_after;
            {
                // Check position condition: must have whitespace before OR prev is comma/bracket
                let tb = t.borrow();
                let is_ok_pos = tb.is_whitespace_before(sofa) || {
                    let prev = tb.prev.as_ref().and_then(|p| p.upgrade());
                    prev.map_or(false, |p| {
                        let pb = p.borrow();
                        pb.is_char(',', sofa) || pb.is_char('(', sofa) || pb.is_char('[', sofa)
                    })
                };
                next_after = tb.next.clone();
                drop(tb);

                if !is_ok_pos {
                    cur = next_after;
                    continue;
                }
            }

            if let Some(rt) = try_attach_spec(&t, sofa) {
                let end_next = rt.borrow().next.clone();
                // Extract referent and register it before embedding
                if let TokenKind::Referent(rd) = &rt.borrow().kind {
                    kit.add_entity(rd.referent.clone());
                }
                kit.embed_token(rt);
                cur = end_next;
                continue;
            }

            // Must be a letter token to start a denomination
            if !t.borrow().is_letters() {
                cur = next_after;
                continue;
            }

            if !can_be_start_of_denom(&t, sofa) {
                cur = next_after;
                continue;
            }

            if let Some(rt) = try_attach(&t, sofa) {
                let end_next = rt.borrow().next.clone();
                if let TokenKind::Referent(rd) = &rt.borrow().kind {
                    kit.add_entity(rd.referent.clone());
                }
                kit.embed_token(rt);
                cur = end_next;
                continue;
            }

            cur = next_after;
        }
    }
}

/// Quick prefilter: short letter token (≤4 chars) immediately followed by digit/separator/special.
fn can_be_start_of_denom(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    if tb.length_char() > 4 { return false; }
    if tb.is_newline_after(sofa) { return false; }
    let next = match tb.next.as_ref() { Some(n) => n.clone(), None => return false };
    drop(tb);
    let nb = next.borrow();
    if nb.is_whitespace_before(sofa) { return false; } // must be adjacent
    if matches!(&nb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit) {
        return true;
    }
    if nb.is_char_of("/\\-", sofa) {
        let nn = match nb.next.as_ref() { Some(n) => n.clone(), None => return false };
        return matches!(&nn.borrow().kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit);
    }
    if nb.is_char_of("+*&^#@!_", sofa) { return true; }
    false
}

/// Try to parse a denomination starting at `t`.  Returns a wrapped referent Token or None.
pub fn try_attach(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    if let Some(rt) = try_attach_spec(t, sofa) { return Some(rt); }

    // First token must not be all-lowercase unless immediately followed by a digit
    {
        let tb = t.borrow();
        if tb.chars.is_all_lower() {
            let next = tb.next.as_ref()?;
            let nb = next.borrow();
            if !nb.is_whitespace_before(sofa) {
                if !matches!(&nb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit) {
                    return None;
                }
                // must also be at start of sentence (prev is null / whitespace / colon)
                let prev = tb.prev.as_ref().and_then(|p| p.upgrade());
                let ok = prev.map_or(true, |p| {
                    let pb = p.borrow();
                    pb.is_whitespace_after(sofa) || pb.is_char_of(",:", sofa)
                });
                if !ok { return None; }
            } else {
                return None;
            }
        }
    }

    let mut t1 = t.clone();
    let mut tmp_len: usize = 0;
    let mut hiph = false;
    let mut ok = true;
    let mut nums: usize = 0;
    let mut specials: usize = 0;

    let mut w_opt = t.borrow().next.clone();
    while let Some(w) = w_opt {
        {
            let wb = w.borrow();
            // Stop at whitespace
            if wb.is_whitespace_before(sofa) { break; }

            if wb.is_char_of("/\\_", sofa) || wb.is_hiphen(sofa) {
                hiph = true;
                tmp_len += 1;
                w_opt = wb.next.clone();
                continue;
            }
            hiph = false;

            match &wb.kind {
                TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit => {
                    t1 = w.clone();
                    tmp_len += n.value.len();
                    nums += 1;
                    w_opt = wb.next.clone();
                    continue;
                }
                TokenKind::Number(_) => { break; } // non-digit number
                TokenKind::Text(txt) => {
                    if txt.term.len() > 3 {
                        ok = false; break;
                    }
                    let first = txt.term.chars().next().unwrap_or('\0');
                    if !first.is_alphabetic() {
                        if wb.is_char_of(",:", sofa) { break; }
                        // bracket end
                        if wb.is_char_of(")]}", sofa) { break; }
                        if wb.is_char_of("+*&^#@!", sofa) {
                            specials += 1;
                        } else {
                            ok = false; break;
                        }
                    }
                    t1 = w.clone();
                    tmp_len += txt.term.chars().count();
                    w_opt = wb.next.clone();
                    continue;
                }
                _ => { break; }
            }
        }
        break; // unreachable unless continue above
    }

    // Validation
    if tmp_len == 0 || !ok || hiph { return None; }
    if tmp_len > 12 { return None; }
    // Check last char isn't '!'
    {
        let last_term = match &t1.borrow().kind {
            TokenKind::Text(txt) => txt.term.chars().last().unwrap_or('\0'),
            _ => '\0',
        };
        if last_term == '!' { return None; }
    }
    if (nums + specials) == 0 { return None; }
    if !check_attach(t, &t1, sofa) { return None; }

    let mut referent = new_denomination_referent();
    add_value_from_tokens(&mut referent, t, &t1, sofa);
    let r_rc = Rc::new(RefCell::new(referent));
    let tok = Rc::new(RefCell::new(
        Token::new_referent(t.clone(), t1, r_rc)
    ));
    Some(tok)
}

/// Special-case patterns:
///  • NumberToken("1") [optional hyphen] + "С"/"C" → "1С"/"1C"
///  • digit NumberToken + adjacent letter TextToken  → "digitLETTER"
fn try_attach_spec(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let tb = t.borrow();
    match &tb.kind {
        TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit => {
            let val = n.value.clone();
            let t_next = tb.next.as_ref()?.clone();
            drop(tb);

            // Case 1: "1" + optional hyphen + "С"/"C"
            if val == "1" {
                let (letter_tok, prefix_end) = {
                    let nb = t_next.borrow();
                    if nb.is_hiphen(sofa) {
                        let nn = nb.next.as_ref()?.clone();
                        drop(nb);
                        (nn, true)
                    } else {
                        (t_next.clone(), false)
                    }
                };
                {
                    let lb = letter_tok.borrow();
                    if !lb.is_whitespace_before(sofa) {
                        if lb.is_char('С', sofa) || lb.is_char('C', sofa) {
                            let _ = prefix_end;
                            let mut r = new_denomination_referent();
                            r.add_slot("VALUE", crate::referent::SlotValue::Str("1С".into()), false);
                            r.add_slot("VALUE", crate::referent::SlotValue::Str("1C".into()), false);
                            let r_rc = Rc::new(RefCell::new(r));
                            let tok = Rc::new(RefCell::new(
                                Token::new_referent(t.clone(), letter_tok.clone(), r_rc)
                            ));
                            return Some(tok);
                        }
                    }
                }
            }

            // Case 2: any digit number immediately followed by letter
            {
                let nb = t_next.borrow();
                if !nb.is_whitespace_before(sofa) && nb.is_letters() && !nb.chars.is_all_lower() {
                    if let TokenKind::Text(txt) = &nb.kind {
                        let combined = format!("{}{}", val, txt.term);
                        drop(nb);
                        let mut r = new_denomination_referent();
                        r.add_slot("VALUE", crate::referent::SlotValue::Str(combined), false);
                        let r_rc = Rc::new(RefCell::new(r));
                        let tok = Rc::new(RefCell::new(
                            Token::new_referent(t.clone(), t_next, r_rc)
                        ));
                        return Some(tok);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Validate that the token span has no double-whitespace and ends properly.
fn check_attach(begin: &TokenRef, end: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let end_char = end.borrow().end_char;
    let mut cur = Some(begin.clone());
    while let Some(t) = cur {
        {
            let tb = t.borrow();
            if tb.begin_char > end_char { break; }
            if !Rc::ptr_eq(&t, begin) {
                let spaces = tb.whitespaces_before_count(sofa);
                if spaces > 1 { return false; }
                if spaces > 0 {
                    if tb.chars.is_all_lower() { return false; }
                    let prev = tb.prev.as_ref().and_then(|p| p.upgrade());
                    if prev.map_or(false, |p| p.borrow().chars.is_all_lower()) {
                        return false;
                    }
                }
            }
            cur = tb.next.clone();
        }
    }
    // Check what follows end
    let after = end.borrow().next.clone();
    if !end.borrow().is_whitespace_after(sofa) {
        if let Some(aft) = after {
            let ab = aft.borrow();
            if !ab.is_char_of(",;", sofa) && !ab.is_char_of(")]}.", sofa) {
                return false;
            }
        }
    }
    true
}
