/// MiscHelper — a subset of MiscHelper.cs needed by the semantic layer.
///
/// Specifically: `can_be_start_of_sentence()` and `is_eng_article()`.

use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;

// ── is_eng_article ─────────────────────────────────────────────────────────

pub fn is_eng_article(t: &TokenRef) -> bool {
    let tb = t.borrow();
    if !tb.chars.is_latin_letter() { return false; }
    let TokenKind::Text(ref txt) = tb.kind else { return false; };
    matches!(txt.term.as_str(), "THE" | "A" | "AN" | "DER" | "DIE" | "DAS")
}

// ── can_be_start_of_sentence ───────────────────────────────────────────────

/// Determine whether token `t` can be the start of a new sentence.
///
/// This is a port of `MiscHelper.CanBeStartOfSentence()`.
pub fn can_be_start_of_sentence(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();

    // First token in chain
    let prev_ref = match &tb.prev {
        None => return true,
        Some(pw) => match pw.upgrade() {
            None => return true,
            Some(p) => p,
        },
    };
    drop(tb);

    let prev = prev_ref.clone();

    // Table control char transitions
    if !t.borrow().is_table_control_char(sofa) && prev.borrow().is_table_control_char(sofa) {
        return true;
    }
    if !t.borrow().is_whitespace_before(sofa) {
        if !prev.borrow().is_table_control_char(sofa) {
            return false;
        }
    }

    let tb = t.borrow();
    let pb = prev.borrow();

    // All-lowercase letter check
    if tb.chars.is_letter() && tb.chars.is_all_lower() {
        if pb.chars.is_letter() && pb.chars.is_all_lower() {
            return false;
        }
        if (pb.is_hiphen(sofa) || pb.is_comma(sofa)) && !pb.is_whitespace_before(sofa) {
            if let Some(pp_weak) = &pb.prev {
                if let Some(pp) = pp_weak.upgrade() {
                    let ppb = pp.borrow();
                    if ppb.chars.is_letter() && ppb.chars.is_all_lower() {
                        return false;
                    }
                }
            }
        }
        if pb.is_hiphen(sofa) && pb.is_newline_before(sofa) {
            return false;
        }
    }

    let ws_count   = tb.whitespaces_before_count(sofa);
    let nl_count   = tb.newlines_before_count(sofa);
    drop(tb);
    drop(pb);

    if ws_count > 25 || nl_count > 2 {
        return true;
    }

    let pb = prev.borrow();
    if pb.is_comma_and(sofa) || pb.get_morph_class_in_dictionary().is_conjunction() {
        return false;
    }
    drop(pb);

    if is_eng_article(&prev) {
        return false;
    }

    if prev.borrow().is_char(':', sofa) {
        return false;
    }

    if prev.borrow().is_char_of(";", sofa) && t.borrow().is_newline_before(sofa) {
        return true;
    }

    if prev.borrow().is_hiphen(sofa) {
        if prev.borrow().is_newline_before(sofa) {
            return true;
        }
        // check prev.prev == '.'
        let pb = prev.borrow();
        if let Some(pp_weak) = &pb.prev {
            if let Some(pp) = pp_weak.upgrade() {
                if pp.borrow().is_char('.', sofa) {
                    return true;
                }
            }
        }
        drop(pb);
        // fall through to all-lower check
    }

    let tb = t.borrow();
    if tb.chars.is_letter() && tb.chars.is_all_lower() {
        return false;
    }
    drop(tb);

    if t.borrow().is_newline_before(sofa) {
        return true;
    }

    if prev.borrow().is_char_of("!?", sofa) || prev.borrow().is_table_control_char(sofa) {
        return true;
    }

    // Previous is '.'
    if prev.borrow().is_char('.', sofa) || matches!(prev.borrow().kind, TokenKind::Referent(_)) {
        // Check if referent's end token is '.'
        let is_period = {
            let pb = prev.borrow();
            match &pb.kind {
                TokenKind::Referent(r) => {
                    // Can't easily check EndToken here without the referent chain;
                    // just treat ReferentToken ending in '.' as sentence-ending
                    true
                }
                _ => pb.is_char('.', sofa),
            }
        };
        if !is_period { return false; }

        if ws_count > 1 {
            return true;
        }

        // Check next.isChar('.')
        let next_is_period = t.borrow().next.as_ref()
            .and_then(|n| Some(n.borrow().is_char('.', sofa)))
            .unwrap_or(false);
        if next_is_period {
            // Check prev.prev
            let ppb_all_lower = {
                let pb = prev.borrow();
                match &pb.prev {
                    None => false,
                    Some(ppw) => match ppw.upgrade() {
                        None => false,
                        Some(pp) => {
                            let ppb = pp.borrow();
                            ppb.chars.is_all_lower() && ppb.chars.is_letter()
                        }
                    }
                }
            };
            if ppb_all_lower {
                // allow fall-through
            } else {
                return false;
            }
        }

        // Check: prev.prev == NumberToken(not Words) && prev.is_whitespace_before
        {
            let pb = prev.borrow();
            if pb.is_whitespace_before(sofa) {
                if let Some(ppw) = &pb.prev {
                    if let Some(pp) = ppw.upgrade() {
                        let ppb = pp.borrow();
                        if ppb.is_number_token() {
                            // NumberSpellingType::Words check omitted (hard to access here)
                            // conservatively return false
                            return false;
                        }
                    }
                }
            }
            // Check prev.prev == "Г" (год)
            if let Some(ppw) = &pb.prev {
                if let Some(pp) = ppw.upgrade() {
                    if pp.borrow().is_value("Г", None) && ws_count < 2 {
                        return false;
                    }
                }
            }
        }

        return true;
    }

    if is_eng_article(t) {
        return true;
    }

    false
}

/// Get text value of a token span (simplified version of GetTextValue)
pub fn get_text_value(t: &TokenRef, sofa: &SourceOfAnalysis) -> String {
    let tb = t.borrow();
    sofa.substring(tb.begin_char, tb.end_char).to_string()
}
