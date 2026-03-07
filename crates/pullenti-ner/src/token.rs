use std::rc::{Rc, Weak};
use std::cell::{Cell, RefCell};
use std::any::Any;
use pullenti_morph::{CharsInfo, LanguageHelper};
use crate::morph_collection::MorphCollection;
use crate::referent::Referent;
use crate::source_of_analysis::SourceOfAnalysis;

pub type TokenRef = Rc<RefCell<Token>>;
pub type WeakTokenRef = Weak<RefCell<Token>>;

/// Number spelling type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberSpellingType {
    #[default]
    Digit,
    Word,
    Roman,
}

/// Data specific to a text (leaf) token
#[derive(Debug, Clone)]
pub struct TextTokenData {
    /// Normalized term (uppercase)
    pub term: String,
    /// Dictionary base form (lemma)
    pub lemma: Option<String>,
}

/// Data specific to a meta (composite) token
#[derive(Debug, Clone)]
pub struct MetaTokenData {
    pub begin_token: Option<TokenRef>,
    pub end_token: Option<TokenRef>,
}

/// Data specific to a number token
#[derive(Debug, Clone)]
pub struct NumberTokenData {
    /// Numeric value as string
    pub value: String,
    /// Spelling type (digit, word, Roman)
    pub spelling_type: NumberSpellingType,
    /// Is a real (floating-point) number
    pub is_real: bool,
    /// Meta span
    pub meta: MetaTokenData,
}

/// Data specific to a referent token (wraps a named entity)
#[derive(Debug)]
pub struct ReferentTokenData {
    /// The entity this token represents
    pub referent: Rc<RefCell<Referent>>,
    /// Meta span
    pub meta: MetaTokenData,
}

/// Discriminated union of all token kinds
#[derive(Debug)]
pub enum TokenKind {
    Text(TextTokenData),
    Meta(MetaTokenData),
    Number(NumberTokenData),
    Referent(ReferentTokenData),
}

/// Cached whitespace/newline attributes for a token (lazily computed)
/// Bit layout:
///   bit 0: initialized
///   bit 1: whitespace before
///   bit 2: whitespace after
///   bit 3: newline before
///   bit 4: newline after
///   bit 5: inner bool
///   bit 6: not noun phrase
const ATTR_INIT: i16 = 1;
const ATTR_WS_BEFORE: i16 = 1 << 1;
const ATTR_WS_AFTER: i16 = 1 << 2;
const ATTR_NL_BEFORE: i16 = 1 << 3;
const ATTR_NL_AFTER: i16 = 1 << 4;
const ATTR_INNER_BOOL: i16 = 1 << 5;
const ATTR_NOT_NOUN_PHRASE: i16 = 1 << 6;

/// Base token — a range of characters in the source text with morphological info
pub struct Token {
    pub begin_char: i32,
    pub end_char: i32,
    pub morph: MorphCollection,
    pub chars: CharsInfo,
    pub kind: TokenKind,
    pub tag: Option<Box<dyn Any>>,
    pub next: Option<TokenRef>,
    pub prev: Option<WeakTokenRef>,
    attrs: Cell<i16>,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token({}-{})", self.begin_char, self.end_char)
    }
}

impl Token {
    pub fn new_text(begin: i32, end: i32, term: String, morph: MorphCollection, chars: CharsInfo) -> Self {
        Token {
            begin_char: begin,
            end_char: end,
            morph,
            chars,
            kind: TokenKind::Text(TextTokenData { term, lemma: None }),
            tag: None,
            next: None,
            prev: None,
            attrs: Cell::new(0),
        }
    }

    pub fn new_meta(begin_token: TokenRef, end_token: TokenRef) -> Self {
        let begin_char = begin_token.borrow().begin_char;
        let end_char = end_token.borrow().end_char;
        Token {
            begin_char,
            end_char,
            morph: MorphCollection::new(),
            chars: CharsInfo::new(),
            kind: TokenKind::Meta(MetaTokenData {
                begin_token: Some(begin_token),
                end_token: Some(end_token),
            }),
            tag: None,
            next: None,
            prev: None,
            attrs: Cell::new(0),
        }
    }

    pub fn new_referent(begin_token: TokenRef, end_token: TokenRef, referent: Rc<RefCell<Referent>>) -> Self {
        let begin_char = begin_token.borrow().begin_char;
        let end_char = end_token.borrow().end_char;
        let morph = begin_token.borrow().morph.clone_collection();
        let chars = begin_token.borrow().chars;
        Token {
            begin_char,
            end_char,
            morph,
            chars,
            kind: TokenKind::Referent(ReferentTokenData {
                referent,
                meta: MetaTokenData {
                    begin_token: Some(begin_token),
                    end_token: Some(end_token),
                },
            }),
            tag: None,
            next: None,
            prev: None,
            attrs: Cell::new(0),
        }
    }

    pub fn new_number(begin: i32, end: i32, value: String, morph: MorphCollection, chars: CharsInfo) -> Self {
        Token {
            begin_char: begin,
            end_char: end,
            morph,
            chars,
            kind: TokenKind::Number(NumberTokenData {
                value,
                spelling_type: NumberSpellingType::Digit,
                is_real: false,
                meta: MetaTokenData { begin_token: None, end_token: None },
            }),
            tag: None,
            next: None,
            prev: None,
            attrs: Cell::new(0),
        }
    }

    pub fn length_char(&self) -> i32 {
        self.end_char - self.begin_char + 1
    }

    /// Get the source text for this token (uses char-based positions)
    pub fn get_source_text<'a>(&self, sofa: &'a SourceOfAnalysis) -> &'a str {
        sofa.substring(self.begin_char, self.end_char)
    }

    /// Get the normalized term for TextToken, or source text for others
    pub fn term(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Text(t) => Some(&t.term),
            _ => None,
        }
    }

    /// Get the referent if this is a ReferentToken
    pub fn get_referent(&self) -> Option<Rc<RefCell<Referent>>> {
        match &self.kind {
            TokenKind::Referent(r) => Some(r.referent.clone()),
            _ => None,
        }
    }

    /// Check if this token is a letter-type TextToken
    pub fn is_letters(&self) -> bool {
        matches!(&self.kind, TokenKind::Text(_)) && self.chars.is_letter()
    }

    /// Check if token equals a given word value (case-insensitive, checks all morph variants)
    pub fn is_value(&self, term: &str, term_ua: Option<&str>) -> bool {
        match &self.kind {
            TokenKind::Text(t) => {
                if t.term == term { return true; }
                if let Some(ua) = term_ua {
                    if t.term == ua { return true; }
                }
                // Check morph word forms
                self.morph.items().iter().any(|wf| {
                    wf.normal_case.as_deref() == Some(term)
                        || wf.normal_full.as_deref() == Some(term)
                        || term_ua.map_or(false, |ua| {
                            wf.normal_case.as_deref() == Some(ua)
                                || wf.normal_full.as_deref() == Some(ua)
                        })
                })
            }
            _ => false,
        }
    }

    /// Check two consecutive tokens
    pub fn is_value2(&self, term: &str, next_term: &str) -> bool {
        if !self.is_value(term, None) { return false; }
        self.next.as_ref().map_or(false, |n| n.borrow().is_value(next_term, None))
    }

    /// Comma or coordinating conjunction ("и", "and", ",")
    pub fn is_comma_and(&self, sofa: &SourceOfAnalysis) -> bool {
        if self.is_char(',', sofa) { return true; }
        self.is_value("И", Some("І")) || self.is_value("AND", None)
    }

    pub fn is_and(&self, sofa: &SourceOfAnalysis) -> bool {
        self.is_value("И", Some("І")) || self.is_value("AND", None) || {
            if self.begin_char == self.end_char {
                let ch = sofa.char_at(self.begin_char);
                ch == '&'
            } else { false }
        }
    }

    pub fn is_or(&self, _sofa: &SourceOfAnalysis) -> bool {
        self.is_value("ИЛИ", Some("АБО")) || self.is_value("OR", None)
    }

    pub fn is_comma(&self, sofa: &SourceOfAnalysis) -> bool {
        self.is_char(',', sofa)
    }

    pub fn is_char(&self, ch: char, sofa: &SourceOfAnalysis) -> bool {
        if self.begin_char != self.end_char { return false; }
        let c = sofa.char_at(self.begin_char);
        if ch == '-' && LanguageHelper::is_hiphen(c) { return true; }
        c == ch
    }

    pub fn is_char_of(&self, chars: &str, sofa: &SourceOfAnalysis) -> bool {
        if self.begin_char != self.end_char { return false; }
        let c = sofa.char_at(self.begin_char);
        if chars.contains(c) { return true; }
        if chars.contains('-') && LanguageHelper::is_hiphen(c) { return true; }
        false
    }

    pub fn is_hiphen(&self, sofa: &SourceOfAnalysis) -> bool {
        let ch = sofa.char_at(self.begin_char);
        ch == '―' || LanguageHelper::is_hiphen(ch)
    }

    pub fn is_table_control_char(&self, sofa: &SourceOfAnalysis) -> bool {
        if self.begin_char != self.end_char { return false; }
        let ch = sofa.char_at(self.begin_char);
        ch == '\x07' || ch == '\x0C' || ch == '\u{001F}'
    }

    pub fn is_number_token(&self) -> bool {
        matches!(&self.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit)
    }

    pub fn number_value(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Number(n) => Some(&n.value),
            _ => None,
        }
    }

    pub fn is_ignored(&self, sofa: &SourceOfAnalysis) -> bool {
        if sofa.ignored_end_char > 0 {
            self.begin_char >= sofa.ignored_begin_char && self.end_char <= sofa.ignored_end_char
        } else {
            false
        }
    }

    fn compute_attrs(&self, sofa: &SourceOfAnalysis) {
        let mut a: i16 = ATTR_INIT;

        // Whitespace/newline BEFORE
        let prev_end = self.prev.as_ref()
            .and_then(|w| w.upgrade())
            .map(|p| p.borrow().end_char)
            .unwrap_or(-1);

        if prev_end < 0 {
            // First token
            a |= ATTR_WS_BEFORE | ATTR_NL_BEFORE;
        } else {
            for j in (prev_end + 1) as usize..self.begin_char as usize {
                let ch = sofa.char_at(j as i32);
                if ch.is_whitespace() || ch == '\x1F' {
                    a |= ATTR_WS_BEFORE;
                    if ch == '\r' || ch == '\n' || ch == '\x0C' || ch == '\u{2028}' || ch == '\x1F' {
                        a |= ATTR_NL_BEFORE;
                    }
                }
            }
        }

        // Whitespace/newline AFTER
        let next_begin = self.next.as_ref()
            .map(|n| n.borrow().begin_char)
            .unwrap_or(i32::MAX);

        if next_begin == i32::MAX {
            a |= ATTR_WS_AFTER | ATTR_NL_AFTER;
        } else {
            for j in (self.end_char + 1) as usize..next_begin as usize {
                let ch = sofa.char_at(j as i32);
                if ch.is_whitespace() {
                    a |= ATTR_WS_AFTER;
                    if ch == '\r' || ch == '\n' || ch == '\x0C' || ch == '\u{2028}' {
                        a |= ATTR_NL_AFTER;
                    }
                }
            }
        }
        self.attrs.set(a);
    }

    fn get_attr(&self, bit: i16, sofa: &SourceOfAnalysis) -> bool {
        if (self.attrs.get() & ATTR_INIT) == 0 {
            self.compute_attrs(sofa);
        }
        (self.attrs.get() & bit) != 0
    }

    pub fn is_whitespace_before(&self, sofa: &SourceOfAnalysis) -> bool {
        self.get_attr(ATTR_WS_BEFORE, sofa)
    }

    pub fn is_whitespace_after(&self, sofa: &SourceOfAnalysis) -> bool {
        self.get_attr(ATTR_WS_AFTER, sofa)
    }

    pub fn is_newline_before(&self, sofa: &SourceOfAnalysis) -> bool {
        self.get_attr(ATTR_NL_BEFORE, sofa)
    }

    pub fn is_newline_after(&self, sofa: &SourceOfAnalysis) -> bool {
        self.get_attr(ATTR_NL_AFTER, sofa)
    }

    pub fn inner_bool(&self) -> bool {
        (self.attrs.get() & ATTR_INNER_BOOL) != 0
    }
    pub fn set_inner_bool(&self, val: bool) {
        let a = self.attrs.get();
        self.attrs.set(if val { a | ATTR_INNER_BOOL } else { a & !ATTR_INNER_BOOL });
    }

    pub fn not_noun_phrase(&self) -> bool {
        (self.attrs.get() & ATTR_NOT_NOUN_PHRASE) != 0
    }
    pub fn set_not_noun_phrase(&self, val: bool) {
        let a = self.attrs.get();
        self.attrs.set(if val { a | ATTR_NOT_NOUN_PHRASE } else { a & !ATTR_NOT_NOUN_PHRASE });
    }

    /// Union of MorphClass for all in-dictionary word forms
    pub fn get_morph_class_in_dictionary(&self) -> pullenti_morph::MorphClass {
        let mut res = pullenti_morph::MorphClass::new();
        for wf in self.morph.items() {
            if wf.is_in_dictionary() {
                res.value |= wf.base.class.value;
            }
        }
        res
    }

    /// True if token is purely a verb form (not also noun/adjective)
    pub fn is_pure_verb(&self) -> bool {
        if self.is_value("МОЖНО", None) || self.is_value("МОЖЕТ", None)
            || self.is_value("ДОЛЖНЫЙ", None) || self.is_value("НУЖНО", None)
        {
            return true;
        }
        if let TokenKind::Text(t) = &self.kind {
            if t.term == "ВПРАВЕ" || t.term == "ДОПУСТИМО" || t.term == "НЕДОПУСТИМО" {
                return true;
            }
        }
        let mut ret = false;
        let mut short_form = false;
        for wf in self.morph.items() {
            if wf.is_in_dictionary() {
                if wf.base.class.is_verb() && wf.base.case.is_undefined() {
                    ret = true;
                } else if !wf.base.class.is_verb() {
                    if wf.base.class.is_adjective() && wf.contains_attr("к.ф.") {
                        short_form = true;
                    } else {
                        return false;
                    }
                }
            }
        }
        if short_form { return true; }
        ret
    }

    /// True if token is a form of "быть" (to be)
    pub fn is_verb_be(&self) -> bool {
        if self.is_value("БЫТЬ", None) || self.is_value("ЕСТЬ", None)
            || self.is_value("ЯВЛЯТЬ", None) || self.is_value("BE", None)
        {
            return true;
        }
        if let TokenKind::Text(t) = &self.kind {
            matches!(t.term.as_str(), "IS" | "WAS" | "BECAME" | "Є")
        } else {
            false
        }
    }

    /// Invalidate cached attrs (call when prev/next changes)
    pub fn invalidate_attrs(&self) {
        self.attrs.set(0);
    }

    /// Get normalized text in nominative case (or source text if not a TextToken)
    pub fn get_normal_case_text(&self, sofa: &SourceOfAnalysis) -> String {
        match &self.kind {
            TokenKind::Text(t) => {
                // Try to find nominative form in word forms
                let wf = self.morph.find_item(
                    pullenti_morph::MorphCase::NOMINATIVE,
                    pullenti_morph::MorphNumber::UNDEFINED,
                    pullenti_morph::MorphGenderFlags::UNDEFINED,
                );
                if let Some(wf) = wf {
                    if let Some(ref nc) = wf.normal_case {
                        return nc.clone();
                    }
                }
                t.term.clone()
            }
            _ => self.get_source_text(sofa).to_string(),
        }
    }

    /// Count newline characters before this token
    pub fn newlines_before_count(&self, sofa: &SourceOfAnalysis) -> i32 {
        match &self.prev {
            None => 100,
            Some(prev_weak) => {
                let prev_end = match prev_weak.upgrade() {
                    None => return 100,
                    Some(p) => p.borrow().end_char,
                };
                if prev_end + 1 > self.begin_char { return 0; }
                let mut count = 0i32;
                for pos in (prev_end + 1)..self.begin_char {
                    if pos as usize >= sofa.text.len() { break; }
                    let ch = sofa.char_at(pos);
                    if ch == '\r' || ch == '\n' || ch == '\u{2028}' {
                        count += 1;
                    }
                }
                count
            }
        }
    }

    /// Count whitespace units before this token
    pub fn whitespaces_before_count(&self, sofa: &SourceOfAnalysis) -> i32 {
        match &self.prev {
            None => 100,
            Some(prev_weak) => {
                let prev_end = match prev_weak.upgrade() {
                    None => return 100,
                    Some(p) => p.borrow().end_char,
                };
                if prev_end + 1 == self.begin_char { return 0; }
                self.calc_whitespaces(prev_end + 1, self.begin_char - 1, sofa)
            }
        }
    }

    fn calc_whitespaces(&self, p0: i32, p1: i32, sofa: &SourceOfAnalysis) -> i32 {
        if p0 < 0 || p0 > p1 || p1 as usize >= sofa.char_len() { return -1; }
        let mut res = 0i32;
        let mut i = p0;
        while i <= p1 {
            let ch = sofa.char_at(i);
            if ch == '\r' || ch == '\n' || ch == '\u{2028}' {
                res += 10;
                if i + 1 <= p1 {
                    let next = sofa.char_at(i + 1);
                    if ch != next && (next == '\r' || next == '\n') { i += 1; }
                }
            } else if ch == '\t' {
                res += 5;
            } else if ch == '\u{0007}' || ch == '\x0C' {
                res += 100;
            } else {
                res += 1;
            }
            i += 1;
        }
        res
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TokenKind::Text(t) => write!(f, "{}", t.term),
            TokenKind::Meta(_) => write!(f, "[meta {}-{}]", self.begin_char, self.end_char),
            TokenKind::Number(n) => write!(f, "{}", n.value),
            TokenKind::Referent(r) => write!(f, "{}", r.referent.borrow()),
        }
    }
}

/// Build a linked chain of tokens from morph results
pub fn build_token_chain(
    morph_tokens: Vec<pullenti_morph::MorphToken>,
    _sofa: &SourceOfAnalysis,
) -> Option<TokenRef> {
    if morph_tokens.is_empty() { return None; }

    let mut chain: Vec<TokenRef> = Vec::with_capacity(morph_tokens.len());

    for mt in morph_tokens {
        let term = mt.term.unwrap_or_default();
        let morph = MorphCollection::from_word_forms(mt.word_forms.unwrap_or_default());
        let chars = mt.char_info;
        // Detect pure-digit tokens and create NumberToken
        let is_digit = !term.is_empty() && term.chars().all(|c| c.is_ascii_digit());
        let tok = if is_digit {
            Token::new_number(mt.begin_char, mt.end_char, term, morph, chars)
        } else {
            Token::new_text(mt.begin_char, mt.end_char, term, morph, chars)
        };
        chain.push(Rc::new(RefCell::new(tok)));
    }

    // Link prev/next
    for i in 0..chain.len() {
        if i > 0 {
            let prev_weak = Rc::downgrade(&chain[i - 1]);
            chain[i].borrow_mut().prev = Some(prev_weak);
        }
        if i + 1 < chain.len() {
            chain[i].borrow_mut().next = Some(chain[i + 1].clone());
        }
    }

    Some(chain[0].clone())
}

/// Iterate over a token chain from a starting token
pub struct TokenChainIter {
    current: Option<TokenRef>,
}

impl TokenChainIter {
    pub fn new(start: Option<TokenRef>) -> Self {
        TokenChainIter { current: start }
    }
}

impl Iterator for TokenChainIter {
    type Item = TokenRef;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.current.take()?;
        self.current = cur.borrow().next.clone();
        Some(cur)
    }
}
