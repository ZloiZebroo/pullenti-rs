/// DenominationReferent — alphanumeric code/designation (e.g. "C#", "A-320", "1С").
/// Mirrors `DenominationReferent.cs`.

use crate::referent::{Referent, SlotValue};
use crate::token::{TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;

pub const OBJ_TYPENAME: &str = "DENOMINATION";
pub const ATTR_VALUE:   &str = "VALUE";

pub fn new_denomination_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_value(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_VALUE).map(|s| s.to_string())
}

/// Build the canonical value string by iterating tokens begin..=end,
/// normalising separators and stripping hyphens between letter↔digit.
pub fn add_value_from_tokens(r: &mut Referent, begin: &TokenRef, end: &TokenRef, sofa: &SourceOfAnalysis) {
    let mut tmp = String::new();
    let end_char = end.borrow().end_char;

    let mut cur = Some(begin.clone());
    while let Some(t) = cur {
        {
            let tb = t.borrow();
            if tb.begin_char > end_char { break; }
            match &tb.kind {
                TokenKind::Number(_) => {
                    tmp.push_str(tb.get_source_text(sofa));
                }
                TokenKind::Text(txt) => {
                    if tb.is_char_of("-\\/", sofa) {
                        tmp.push('-');
                    } else {
                        tmp.push_str(&txt.term);
                    }
                }
                _ => {}
            }
        }
        let next = t.borrow().next.clone();
        if t.borrow().end_char >= end_char { break; }
        cur = next;
    }

    // Remove hyphens between letter↔digit (C-3 → C3, 3-D → 3D)
    // but keep digit-digit (3-14) and letter-letter (A-B)
    let chars: Vec<char> = tmp.chars().collect();
    let mut out = String::with_capacity(tmp.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '-' && i > 0 && i + 1 < chars.len() {
            let c0 = chars[i - 1];
            let c1 = chars[i + 1];
            if c0.is_ascii_digit() && !c1.is_ascii_digit() {
                i += 1; continue; // digit-letter: remove
            } else if c1.is_ascii_digit() && !c0.is_ascii_digit() {
                i += 1; continue; // letter-digit: remove
            }
        }
        out.push(c);
        i += 1;
    }

    r.add_slot(ATTR_VALUE, SlotValue::Str(out), false);
}
