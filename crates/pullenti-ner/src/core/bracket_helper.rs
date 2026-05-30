use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::TokenRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketKind {
    Round,
    Square,
    Curly,
    AngleQuote,
    DoubleQuote,
    SingleQuote,
}

impl BracketKind {
    fn open_char(self) -> char {
        match self {
            BracketKind::Round => '(',
            BracketKind::Square => '[',
            BracketKind::Curly => '{',
            BracketKind::AngleQuote => '«',
            BracketKind::DoubleQuote => '"',
            BracketKind::SingleQuote => '\'',
        }
    }

    fn close_char(self) -> char {
        match self {
            BracketKind::Round => ')',
            BracketKind::Square => ']',
            BracketKind::Curly => '}',
            BracketKind::AngleQuote => '»',
            BracketKind::DoubleQuote => '"',
            BracketKind::SingleQuote => '\'',
        }
    }

    fn is_symmetric(self) -> bool {
        matches!(self, BracketKind::DoubleQuote | BracketKind::SingleQuote)
    }
}

pub fn get_open_bracket_kind(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<BracketKind> {
    let tb = t.borrow();
    if tb.length_char() != 1 {
        return None;
    }
    match sofa.char_at(tb.begin_char) {
        '(' => Some(BracketKind::Round),
        '[' => Some(BracketKind::Square),
        '{' => Some(BracketKind::Curly),
        '«' => Some(BracketKind::AngleQuote),
        '"' => Some(BracketKind::DoubleQuote),
        '\'' => Some(BracketKind::SingleQuote),
        _ => None,
    }
}

pub fn is_any_close_bracket(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    if tb.length_char() != 1 {
        return false;
    }
    matches!(sofa.char_at(tb.begin_char), ')' | ']' | '}' | '»' | '"' | '\'')
}

pub fn is_close_bracket(t: &TokenRef, kind: BracketKind, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    tb.length_char() == 1 && sofa.char_at(tb.begin_char) == kind.close_char()
}

pub fn find_matching_bracket(
    open_tok: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<TokenRef> {
    let kind = get_open_bracket_kind(open_tok, sofa)?;
    let mut depth = 1i32;
    let mut scan = open_tok.borrow().next.clone();
    while let Some(tt) = scan {
        if kind.is_symmetric() {
            if is_close_bracket(&tt, kind, sofa) {
                return Some(tt);
            }
        } else {
            if tt.borrow().length_char() == 1 && sofa.char_at(tt.borrow().begin_char) == kind.open_char() {
                depth += 1;
            }
            if is_close_bracket(&tt, kind, sofa) {
                depth -= 1;
                if depth == 0 {
                    return Some(tt);
                }
            }
        }
        scan = tt.borrow().next.clone();
    }
    None
}

pub fn skip_bracket_group(open_tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    find_matching_bracket(open_tok, sofa)?.borrow().next.clone()
}

pub fn inner_bounds(open_tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(TokenRef, TokenRef)> {
    let first = open_tok.borrow().next.clone()?;
    let close = find_matching_bracket(open_tok, sofa)?;
    let last = close.borrow().prev.as_ref().and_then(|w| w.upgrade())?;
    if last.borrow().end_char < first.borrow().begin_char {
        return None;
    }
    Some((first, last))
}
