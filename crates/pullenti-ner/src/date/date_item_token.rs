use std::sync::OnceLock;
use pullenti_morph::MorphLang;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::{Token, TokenRef, TokenKind};
use crate::date::date_pointer_type::DatePointerType;

// ── DateItemType ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateItemType {
    Number,
    Year,
    Month,
    Day,
    Delim,
    Hour,
    Minute,
    Second,
    Halfyear,
    Quartal,
    Pointer,
    Century,
    Tenyears,
}

// ── Russian ordinal day-number words (genitive & nominative) ─────────────────
//
// "девятнадцатого января" → day=19, month=9.
// We list all declined forms (genitive singular, masculine — the most common in
// date phrases like "N-го месяца") plus nominative for completeness.
//
// Each entry: (surface_uppercase, day_value)
static ORDINAL_DAYS_RU: &[(&str, i32)] = &[
    // 1
    ("ПЕРВОГО", 1), ("ПЕРВЫЙ", 1), ("ПЕРВОЕ", 1), ("ПЕРВАЯ", 1),
    // 2
    ("ВТОРОГО", 2), ("ВТОРОЙ", 2), ("ВТОРОЕ", 2), ("ВТОРАЯ", 2),
    // 3
    ("ТРЕТЬЕГО", 3), ("ТРЕТИЙ", 3), ("ТРЕТЬЕ", 3), ("ТРЕТЬЯ", 3),
    // 4
    ("ЧЕТВЁРТОГО", 4), ("ЧЕТВЕРТОГО", 4), ("ЧЕТВЁРТЫЙ", 4), ("ЧЕТВЕРТЫЙ", 4),
    // 5
    ("ПЯТОГО", 5), ("ПЯТЫЙ", 5), ("ПЯТОЕ", 5), ("ПЯТАЯ", 5),
    // 6
    ("ШЕСТОГО", 6), ("ШЕСТОЙ", 6), ("ШЕСТОЕ", 6), ("ШЕСТАЯ", 6),
    // 7
    ("СЕДЬМОГО", 7), ("СЕДЬМОЙ", 7), ("СЕДЬМОЕ", 7), ("СЕДЬМАЯ", 7),
    // 8
    ("ВОСЬМОГО", 8), ("ВОСЬМОЙ", 8), ("ВОСЬМОЕ", 8), ("ВОСЬМАЯ", 8),
    // 9
    ("ДЕВЯТОГО", 9), ("ДЕВЯТЫЙ", 9), ("ДЕВЯТОЕ", 9), ("ДЕВЯТАЯ", 9),
    // 10
    ("ДЕСЯТОГО", 10), ("ДЕСЯТЫЙ", 10), ("ДЕСЯТОЕ", 10), ("ДЕСЯТАЯ", 10),
    // 11
    ("ОДИННАДЦАТОГО", 11), ("ОДИННАДЦАТЫЙ", 11), ("ОДИННАДЦАТОЕ", 11),
    // 12
    ("ДВЕНАДЦАТОГО", 12), ("ДВЕНАДЦАТЫЙ", 12), ("ДВЕНАДЦАТОЕ", 12),
    // 13
    ("ТРИНАДЦАТОГО", 13), ("ТРИНАДЦАТЫЙ", 13), ("ТРИНАДЦАТОЕ", 13),
    // 14
    ("ЧЕТЫРНАДЦАТОГО", 14), ("ЧЕТЫРНАДЦАТЫЙ", 14), ("ЧЕТЫРНАДЦАТОЕ", 14),
    // 15
    ("ПЯТНАДЦАТОГО", 15), ("ПЯТНАДЦАТЫЙ", 15), ("ПЯТНАДЦАТОЕ", 15),
    // 16
    ("ШЕСТНАДЦАТОГО", 16), ("ШЕСТНАДЦАТЫЙ", 16), ("ШЕСТНАДЦАТОЕ", 16),
    // 17
    ("СЕМНАДЦАТОГО", 17), ("СЕМНАДЦАТЫЙ", 17), ("СЕМНАДЦАТОЕ", 17),
    // 18
    ("ВОСЕМНАДЦАТОГО", 18), ("ВОСЕМНАДЦАТЫЙ", 18), ("ВОСЕМНАДЦАТОЕ", 18),
    // 19
    ("ДЕВЯТНАДЦАТОГО", 19), ("ДЕВЯТНАДЦАТЫЙ", 19), ("ДЕВЯТНАДЦАТОЕ", 19),
    // 20
    ("ДВАДЦАТОГО", 20), ("ДВАДЦАТЫЙ", 20), ("ДВАДЦАТОЕ", 20), ("ДВАДЦАТАЯ", 20),
    // 21
    ("ДВАДЦАТЬ", 21), // handled as compound "ДВАДЦАТЬ ПЕРВОГО" via two-token lookup
    // 22-29 (compounds handled by two-token lookup)
    // 30
    ("ТРИДЦАТОГО", 30), ("ТРИДЦАТЫЙ", 30), ("ТРИДЦАТОЕ", 30), ("ТРИДЦАТАЯ", 30),
    // 31 (compound "ТРИДЦАТЬ ПЕРВОГО" handled by two-token lookup)
];

// Two-token ordinal: "ДВАДЦАТЬ ПЕРВОГО", "ТРИДЦАТЬ ПЕРВОГО", etc.
// (prefix, suffix, value)
static ORDINAL_DAYS_RU_COMPOUND: &[(&str, &str, i32)] = &[
    ("ДВАДЦАТЬ", "ПЕРВОГО", 21), ("ДВАДЦАТЬ", "ПЕРВЫЙ", 21), ("ДВАДЦАТЬ", "ПЕРВОЕ", 21),
    ("ДВАДЦАТЬ", "ВТОРОГО", 22), ("ДВАДЦАТЬ", "ВТОРОЙ", 22), ("ДВАДЦАТЬ", "ВТОРОЕ", 22),
    ("ДВАДЦАТЬ", "ТРЕТЬЕГО", 23), ("ДВАДЦАТЬ", "ТРЕТИЙ", 23), ("ДВАДЦАТЬ", "ТРЕТЬЕ", 23),
    ("ДВАДЦАТЬ", "ЧЕТВЁРТОГО", 24), ("ДВАДЦАТЬ", "ЧЕТВЕРТОГО", 24), ("ДВАДЦАТЬ", "ЧЕТВЁРТЫЙ", 24), ("ДВАДЦАТЬ", "ЧЕТВЕРТЫЙ", 24),
    ("ДВАДЦАТЬ", "ПЯТОГО", 25), ("ДВАДЦАТЬ", "ПЯТЫЙ", 25), ("ДВАДЦАТЬ", "ПЯТОЕ", 25),
    ("ДВАДЦАТЬ", "ШЕСТОГО", 26), ("ДВАДЦАТЬ", "ШЕСТОЙ", 26), ("ДВАДЦАТЬ", "ШЕСТОЕ", 26),
    ("ДВАДЦАТЬ", "СЕДЬМОГО", 27), ("ДВАДЦАТЬ", "СЕДЬМОЙ", 27), ("ДВАДЦАТЬ", "СЕДЬМОЕ", 27),
    ("ДВАДЦАТЬ", "ВОСЬМОГО", 28), ("ДВАДЦАТЬ", "ВОСЬМОЙ", 28), ("ДВАДЦАТЬ", "ВОСЬМОЕ", 28),
    ("ДВАДЦАТЬ", "ДЕВЯТОГО", 29), ("ДВАДЦАТЬ", "ДЕВЯТЫЙ", 29), ("ДВАДЦАТЬ", "ДЕВЯТОЕ", 29),
    ("ТРИДЦАТЬ", "ПЕРВОГО", 31), ("ТРИДЦАТЬ", "ПЕРВЫЙ", 31), ("ТРИДЦАТЬ", "ПЕРВОЕ", 31),
];

/// Try to match a Russian ordinal day word at token `t`.
/// Returns `(day_value, end_token)` on success.
fn try_match_ordinal_day_ru(t: &TokenRef) -> Option<(i32, TokenRef)> {
    let term = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(td) => td.term.to_uppercase(),
            _ => return None,
        }
    };
    // Single-token ordinals (1-20, 30)
    // Skip "ДВАДЦАТЬ" alone — always compound
    if term != "ДВАДЦАТЬ" && term != "ТРИДЦАТЬ" {
        for &(word, day) in ORDINAL_DAYS_RU {
            if word == term {
                return Some((day, t.clone()));
            }
        }
    }
    // Check morph normal forms too (handles inflected forms not in our table)
    {
        let tb = t.borrow();
        if let TokenKind::Text(_) = &tb.kind {
            for wf in tb.morph.items() {
                for nc in [wf.normal_case.as_deref(), wf.normal_full.as_deref()].iter().flatten() {
                    let nc_up = nc.to_uppercase();
                    for &(word, day) in ORDINAL_DAYS_RU {
                        if word == nc_up {
                            return Some((day, t.clone()));
                        }
                    }
                }
            }
        }
    }
    // Two-token compound ordinals: "ДВАДЦАТЬ ПЕРВОГО" etc.
    if term == "ДВАДЦАТЬ" || term == "ТРИДЦАТЬ" {
        let n1 = t.borrow().next.clone()?;
        let n1_term = {
            let nb = n1.borrow();
            match &nb.kind {
                TokenKind::Text(td) => td.term.to_uppercase(),
                _ => return None,
            }
        };
        for &(prefix, suffix, day) in ORDINAL_DAYS_RU_COMPOUND {
            if term == prefix && n1_term == suffix {
                return Some((day, n1.clone()));
            }
        }
    }
    None
}

// ── Static month lookup tables ────────────────────────────────────────────────

static MONTHS_RU: &[&str] = &[
    "ЯНВАРЬ",  "ФЕВРАЛЬ", "МАРТ",     "АПРЕЛЬ",   "МАЙ",     "ИЮНЬ",
    "ИЮЛЬ",    "АВГУСТ",  "СЕНТЯБРЬ", "ОКТЯБРЬ",  "НОЯБРЬ",  "ДЕКАБРЬ",
];
static MONTHS_UA: &[&str] = &[
    "СІЧЕНЬ",  "ЛЮТИЙ",  "БЕРЕЗЕНЬ", "КВІТЕНЬ",  "ТРАВЕНЬ", "ЧЕРВЕНЬ",
    "ЛИПЕНЬ",  "СЕРПЕНЬ","ВЕРЕСЕНЬ", "ЖОВТЕНЬ",  "ЛИСТОПАД","ГРУДЕНЬ",
];
static MONTHS_EN: &[&str] = &[
    "JANUARY", "FEBRUARY","MARCH",   "APRIL",    "MAY",     "JUNE",
    "JULY",    "AUGUST",  "SEPTEMBER","OCTOBER", "NOVEMBER","DECEMBER",
];
// Common abbreviations (index into MONTHS_* arrays, 0-based)
static MONTH_ABRIDGES_RU: &[(&str, usize)] = &[
    ("ЯНВ",0),("ФЕВ",1),("ФЕВР",1),("МАР",2),("АПР",3),
    ("ИЮН",5),("ИЮЛ",6),("АВГ",7),("СЕН",8),("СЕНТ",8),
    ("ОКТ",9),("НОЯ",10),("НОЯБ",10),("ДЕК",11),
];
static MONTH_ABRIDGES_EN: &[(&str, usize)] = &[
    ("JAN",0),("FEB",1),("MAR",2),("APR",3),
    ("JUN",5),("JUL",6),("AUG",7),("SEP",8),("SEPT",8),
    ("OCT",9),("NOV",10),("DEC",11),
];

/// Current year approximation (for 2-digit year disambiguation)
const APPROX_CUR_YEAR: i32 = 2026;

// ── Try to identify a month from a token ─────────────────────────────────────

/// Returns `(1-based month, lang)` or None
fn match_month(tok: &Token) -> Option<(i32, MorphLang)> {
    // Only text tokens can be month names
    let term = match &tok.kind {
        TokenKind::Text(t) => &t.term,
        _ => return None,
    };

    // Direct match against primary forms
    for (i, &m) in MONTHS_RU.iter().enumerate() {
        if term.as_str() == m { return Some(((i + 1) as i32, MorphLang::RU)); }
    }
    for (i, &m) in MONTHS_UA.iter().enumerate() {
        if term.as_str() == m { return Some(((i + 1) as i32, MorphLang::UA)); }
    }
    for (i, &m) in MONTHS_EN.iter().enumerate() {
        if term.as_str() == m { return Some(((i + 1) as i32, MorphLang::EN)); }
    }
    // Abbreviations
    for &(abbr, idx) in MONTH_ABRIDGES_RU {
        if term.as_str() == abbr { return Some(((idx + 1) as i32, MorphLang::RU)); }
    }
    for &(abbr, idx) in MONTH_ABRIDGES_EN {
        if term.as_str() == abbr { return Some(((idx + 1) as i32, MorphLang::EN)); }
    }

    // Check morph word-form nominative/base forms (handles inflected "января"→"ЯНВАРЬ")
    for wf in tok.morph.items() {
        let nc = wf.normal_case.as_deref().unwrap_or("");
        let nf = wf.normal_full.as_deref().unwrap_or("");
        for &candidate in &[nc, nf] {
            if candidate.is_empty() { continue; }
            for (i, &m) in MONTHS_RU.iter().enumerate() {
                if candidate == m { return Some(((i + 1) as i32, MorphLang::RU)); }
            }
            for (i, &m) in MONTHS_UA.iter().enumerate() {
                if candidate == m { return Some(((i + 1) as i32, MorphLang::UA)); }
            }
            for (i, &m) in MONTHS_EN.iter().enumerate() {
                if candidate == m { return Some(((i + 1) as i32, MorphLang::EN)); }
            }
        }
    }
    None
}

// ── Season / pointer detection ────────────────────────────────────────────────

fn match_season(tok: &Token) -> Option<DatePointerType> {
    let term = tok.term()?;
    // Check morph base forms too
    let candidates: Vec<&str> = {
        let mut v = vec![term];
        for wf in tok.morph.items() {
            if let Some(s) = wf.normal_case.as_deref() { v.push(s); }
            if let Some(s) = wf.normal_full.as_deref() { v.push(s); }
        }
        v
    };
    for c in candidates {
        match c {
            "ЗИМА" | "ЗИМОЙ" | "ЗИМОЮ" | "ЗИМЕ" | "ЗИМУ" => return Some(DatePointerType::Winter),
            "ВЕСНА" | "ВЕСНОЙ" | "ВЕСНОЮ" | "ВЕСНЕ" | "ВЕСНУ" => return Some(DatePointerType::Spring),
            "ЛЕТО" | "ЛЕТОМ" | "ЛЕТА" | "ЛЕТУ" => return Some(DatePointerType::Summer),
            "ОСЕНЬ" | "ОСЕНЬЮ" | "ОСЕНИ" | "AUTUMN" | "FALL" => return Some(DatePointerType::Autumn),
            "WINTER" => return Some(DatePointerType::Winter),
            "SPRING" => return Some(DatePointerType::Spring),
            "SUMMER" => return Some(DatePointerType::Summer),
            _ => {}
        }
    }
    None
}

// ── Year-word detection ───────────────────────────────────────────────────────

/// C# `TestYearRusWord` — checks if token is a "год" / "г." / "г" type word.
/// Returns the last token consumed (may be "г." → returns the "." token).
pub fn test_year_rus_word(t0: &TokenRef) -> Option<TokenRef> {
    let term = t0.borrow().term().map(|s| s.to_string())?;
    match term.as_str() {
        "ГОД" | "ГОДОВ" | "ГОДУ" | "ГОДА" | "РІК" | "РОКУ" | "РОКИ" | "РОКІВ" => {
            return Some(t0.clone());
        }
        "ГГ" | "Г" | "Р" | "РР" => {
            // Check for trailing "."
            let next = t0.borrow().next.clone();
            if let Some(n) = next {
                let is_dot = n.borrow().term().map_or(false, |t| t == ".");
                if is_dot { return Some(n); }
            }
            return Some(t0.clone());
        }
        _ => {}
    }
    None
}

// ── DateItemToken ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DateItemToken {
    pub typ:          DateItemType,
    pub string_value: String,
    pub int_value:    i32,
    pub ptr:          DatePointerType,
    pub lang:         MorphLang,
    pub new_age:      i32,   // -1 = BC, 0 = normal, 1 = AD explicit
    pub relate:       bool,
    pub begin_token:  TokenRef,
    pub end_token:    TokenRef,
    /// Overridden year value (set when we explicitly resolved the year from a 2-digit number)
    year_override:    i32,   // -1 = not set
}

impl DateItemToken {
    pub fn new(begin: TokenRef, end: TokenRef, typ: DateItemType) -> Self {
        DateItemToken {
            typ,
            string_value: String::new(),
            int_value: 0,
            ptr: DatePointerType::No,
            lang: MorphLang::RU,
            new_age: 0,
            relate: false,
            begin_token: begin,
            end_token: end,
            year_override: -1,
        }
    }

    pub fn set_year_override(&mut self, v: i32) {
        self.year_override = v;
    }

    /// Resolved year value (handles 2-digit disambiguation)
    pub fn year(&self) -> i32 {
        if self.year_override >= 0 { return self.year_override; }
        let v = self.int_value;
        if v == 0 { return 0; }
        if self.new_age == 0 {
            if v < 16 { return 2000 + v; }
            if v <= (APPROX_CUR_YEAR - 2000 + 5) { return 2000 + v; }
            if v < 100 { return 1900 + v; }
        }
        v
    }

    pub fn can_be_year(&self) -> bool {
        if matches!(self.typ, DateItemType::Year) { return true; }
        if matches!(self.typ,
            DateItemType::Month | DateItemType::Quartal |
            DateItemType::Halfyear | DateItemType::Pointer |
            DateItemType::Delim | DateItemType::Hour |
            DateItemType::Minute | DateItemType::Second |
            DateItemType::Century | DateItemType::Tenyears) { return false; }
        let v = self.int_value;
        if v >= 50 && v < 100 { return self.length_char() == 2; }
        if v < 1000 || v > 2100 { return false; }
        true
    }

    pub fn can_by_month(&self) -> bool {
        if matches!(self.typ, DateItemType::Month) { return true; }
        if matches!(self.typ,
            DateItemType::Quartal | DateItemType::Halfyear |
            DateItemType::Pointer | DateItemType::Delim |
            DateItemType::Hour | DateItemType::Minute | DateItemType::Second) { return false; }
        self.int_value > 0 && self.int_value <= 12
    }

    pub fn can_be_day(&self) -> bool {
        if matches!(self.typ,
            DateItemType::Month | DateItemType::Quartal |
            DateItemType::Halfyear | DateItemType::Pointer |
            DateItemType::Delim | DateItemType::Hour |
            DateItemType::Minute | DateItemType::Second) { return false; }
        self.int_value > 0 && self.int_value <= 31
    }

    pub fn is_zero_headed(&self, sofa: &SourceOfAnalysis) -> bool {
        let ch = sofa.char_at(self.begin_token.borrow().begin_char);
        ch == '0'
    }

    pub fn length_char(&self) -> i32 {
        let b = self.begin_token.borrow().begin_char;
        let e = self.end_token.borrow().end_char;
        e - b + 1
    }

    pub fn is_whitespace_before(&self, sofa: &SourceOfAnalysis) -> bool {
        self.begin_token.borrow().is_whitespace_before(sofa)
    }

    pub fn is_whitespace_after(&self, sofa: &SourceOfAnalysis) -> bool {
        self.end_token.borrow().is_whitespace_after(sofa)
    }

    pub fn is_newline_before(&self, sofa: &SourceOfAnalysis) -> bool {
        self.begin_token.borrow().is_newline_before(sofa)
    }

    pub fn is_newline_after(&self, sofa: &SourceOfAnalysis) -> bool {
        self.end_token.borrow().is_newline_after(sofa)
    }
}

// ── TryParse ──────────────────────────────────────────────────────────────────

/// Parse a single date element starting at token `t`.
/// Returns `None` if token doesn't look like a date element.
pub fn try_parse(
    t: &TokenRef,
    prev: &[DateItemToken],
    sofa: &SourceOfAnalysis,
) -> Option<DateItemToken> {
    let tok = t.borrow();

    // ── NumberToken ──
    if let TokenKind::Number(n) = &tok.kind {
        if n.is_real { return None; }
        let v: i32 = n.value.parse().ok()?;
        drop(tok);

        let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Number);
        item.int_value = v;

        // Check next token for year/quarter/halfyear/hour/minute/second words
        let next = t.borrow().next.clone();
        if let Some(ref nt) = next {
            let nt_term = nt.borrow().term().map(|s| s.to_string());
            if let Some(ref term) = nt_term {
                match term.as_str() {
                    "ЧАС" | "ГОДИНА" | "HOUR" | "Ч" if v < 24 => {
                        item.typ = DateItemType::Hour;
                        item.end_token = nt.clone();
                        // consume trailing "."
                        if let Some(nn) = nt.borrow().next.clone() {
                            if nn.borrow().term() == Some(".") {
                                item.end_token = nn;
                            }
                        }
                        return Some(item);
                    }
                    "МИНУТА" | "ХВИЛИНА" | "МИН" | "MINUTE" if v < 60 => {
                        item.typ = DateItemType::Minute;
                        item.end_token = nt.clone();
                        if let Some(nn) = nt.borrow().next.clone() {
                            if nn.borrow().term() == Some(".") {
                                item.end_token = nn;
                            }
                        }
                        return Some(item);
                    }
                    "СЕКУНДА" | "СЕК" | "SECOND" if v < 60 => {
                        item.typ = DateItemType::Second;
                        item.end_token = nt.clone();
                        if let Some(nn) = nt.borrow().next.clone() {
                            if nn.borrow().term() == Some(".") {
                                item.end_token = nn;
                            }
                        }
                        return Some(item);
                    }
                    "ВЕК" | "ВІК" | "СТОЛЕТИЕ" | "СТОЛІТТЯ" if v < 30 => {
                        item.typ = DateItemType::Century;
                        item.end_token = nt.clone();
                        return Some(item);
                    }
                    "ДЕСЯТИЛЕТИЕ" | "ДЕСЯТИЛІТТЯ" | "ДЕКАДА" if v < 10 => {
                        item.typ = DateItemType::Tenyears;
                        item.end_token = nt.clone();
                        return Some(item);
                    }
                    "КВАРТАЛ" if v <= 4 => {
                        item.typ = DateItemType::Quartal;
                        item.end_token = nt.clone();
                        return Some(item);
                    }
                    "ПОЛУГОДИЕ" | "ПІВРІЧЧЯ" if v <= 2 => {
                        item.typ = DateItemType::Halfyear;
                        item.end_token = nt.clone();
                        return Some(item);
                    }
                    _ => {}
                }
            }

            // Check for year word ("год", "г.", "г", "р.")
            let year_end = test_year_rus_word(nt);
            if let Some(ye) = year_end {
                item.typ = DateItemType::Year;
                item.end_token = ye;
                return Some(item);
            }
        }

        // Check if preceding token is "В" / "IN" / "У" / "З" and value > 1900
        if let Some(prev_tok) = t.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
            let prev_term = prev_tok.borrow().term().map(|s| s.to_string());
            if let Some(ref pt) = prev_term {
                if matches!(pt.as_str(), "В" | "IN" | "У" | "З" | "SINCE") {
                    if v >= 1900 && v <= 2100 {
                        item.typ = DateItemType::Year;
                        return Some(item);
                    }
                }
            }
        }

        return Some(item);
    }

    // ── TextToken ──
    if let TokenKind::Text(td) = &tok.kind {
        let term = td.term.clone();
        drop(tok);

        // Russian ordinal day word? ("девятнадцатого", "первого", "двадцать первого", ...)
        // Only accepted when followed (eventually) by a month name OR when in a date list context.
        if let Some((day, end_tok)) = try_match_ordinal_day_ru(t) {
            let mut item = DateItemToken::new(t.clone(), end_tok, DateItemType::Day);
            item.int_value = day;
            return Some(item);
        }

        // Month name?
        if let Some((month_num, lang)) = match_month(&t.borrow()) {
            let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Month);
            item.int_value = month_num;
            item.lang = lang;
            return Some(item);
        }

        // Season?
        if let Some(ptr) = match_season(&t.borrow()) {
            let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Pointer);
            item.ptr = ptr;
            return Some(item);
        }

        // Pointer words (начало, середина, конец)
        if prev.is_empty() {
            match term.as_str() {
                "ОКОЛО" | "ПРИБЛИЗНО" | "ПРИМЕРНО" | "ABOUT" => {
                    let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Pointer);
                    item.ptr = DatePointerType::About;
                    return Some(item);
                }
                _ => {}
            }
        }

        // Delimiter characters
        let ch = term.chars().next()?;
        if !ch.is_alphanumeric() {
            match ch {
                '.' | '/' | '\\' | ':' => {
                    let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Delim);
                    item.string_value = term.clone();
                    return Some(item);
                }
                ',' => {
                    let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Delim);
                    item.string_value = term.clone();
                    return Some(item);
                }
                '-' | '\u{2013}' | '\u{2014}' | '\u{2012}' => {
                    let mut item = DateItemToken::new(t.clone(), t.clone(), DateItemType::Delim);
                    item.string_value = "-".to_string();
                    return Some(item);
                }
                _ => {}
            }
        }

        // "В" preposition before year: already handled in NumberToken branch
        // "О" / "О№": skip

        return None;
    }

    None
}

// ── TryParseList ──────────────────────────────────────────────────────────────

/// Parse a list of DateItemTokens starting at `t0`.
/// Returns None if the first token doesn't yield a date element.
pub fn try_parse_list(
    t0: &TokenRef,
    max_count: usize,
    sofa: &SourceOfAnalysis,
) -> Option<Vec<DateItemToken>> {
    let first = try_parse(t0, &[], sofa)?;
    if matches!(first.typ, DateItemType::Delim) {
        return None;
    }

    let mut res = vec![first];
    let mut tt = {
        let last_end = res.last().unwrap().end_token.clone();
        let next = last_end.borrow().next.clone();
        next
    };

    while let Some(cur) = tt.clone() {
        if max_count > 0 && res.len() >= max_count { break; }

        // Stop at newlines unless it's a known continuation
        let is_nl = cur.borrow().is_newline_before(sofa);
        if is_nl {
            // Allow: month followed by year on next line
            let last_typ = res.last().unwrap().typ;
            let can_continue = matches!(last_typ, DateItemType::Month | DateItemType::Number | DateItemType::Year);
            if !can_continue { break; }
        }

        // Skip certain empty words that appear inside date expressions ("В", "OF", "THE", "IN")
        if let Some(term) = cur.borrow().term().map(|s| s.to_string()) {
            if matches!(term.as_str(), "OF" | "THE") {
                let nxt = cur.borrow().next.clone();
                tt = nxt;
                continue;
            }
            // "В" before a number that looks like a year
            if term == "В" {
                if let Some(nxt) = cur.borrow().next.clone() {
                    if let Some(item) = try_parse(&nxt, &res, sofa) {
                        if item.can_be_year() {
                            let mut item2 = item;
                            item2.begin_token = cur.clone();
                            tt = item2.end_token.borrow().next.clone();
                            res.push(item2);
                            continue;
                        }
                    }
                }
                break;
            }
        }

        let p0 = match try_parse(&cur, &res, sofa) {
            None => {
                // Can't parse; stop
                if cur.borrow().is_newline_before(sofa) { break; }
                // Skip latin letters (breaks date context)
                if cur.borrow().chars.is_latin_letter() { break; }
                break;
            }
            Some(p) => p,
        };

        // Mark if previously last token was Month and now we have a year candidate
        if p0.can_be_year() && matches!(p0.typ, DateItemType::Number) {
            let last_typ = res.last().map(|r| r.typ);
            if matches!(last_typ, Some(DateItemType::Halfyear) | Some(DateItemType::Quartal)) {
                let mut p2 = p0;
                p2.typ = DateItemType::Year;
                tt = p2.end_token.borrow().next.clone();
                res.push(p2);
                continue;
            }
        }

        tt = p0.end_token.borrow().next.clone();
        res.push(p0);
    }

    // Trim trailing delimiters
    while res.last().map_or(false, |r| matches!(r.typ, DateItemType::Delim)) {
        res.pop();
    }

    if res.is_empty() { return None; }
    if res.len() == 1 {
        // Single item — only useful if it's a Year, Month, Pointer, Century, Tenyears, Halfyear, Quartal
        let ok = matches!(res[0].typ,
            DateItemType::Year | DateItemType::Month | DateItemType::Pointer |
            DateItemType::Century | DateItemType::Tenyears |
            DateItemType::Halfyear | DateItemType::Quartal |
            DateItemType::Hour | DateItemType::Number
        );
        if !ok { return None; }
    }

    Some(res)
}
