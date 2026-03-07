use std::sync::{Arc, OnceLock};
use crate::core::{Termin, TerminCollection};
use crate::token::TokenRef;
use crate::source_of_analysis::SourceOfAnalysis;
use super::phone_kind::PhoneKind;
use super::phone_helper;

/// Type of a single phone item token component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneItemType {
    Number,
    CityCode,
    Delim,
    Prefix,
    AddNumber,
    CountryCode,
    Alt,
}

/// A primitive component of a phone number (PhoneItemToken)
#[derive(Debug, Clone)]
pub struct PhoneItemToken {
    pub begin: TokenRef,
    pub end: TokenRef,
    pub item_type: PhoneItemType,
    pub value: String,
    pub kind: PhoneKind,
    pub kind2: PhoneKind,
    pub is_in_brackets: bool,
}

impl PhoneItemToken {
    pub fn new(begin: TokenRef, end: TokenRef, item_type: PhoneItemType, value: String) -> Self {
        PhoneItemToken {
            begin,
            end,
            item_type,
            value,
            kind: PhoneKind::Undefined,
            kind2: PhoneKind::Undefined,
            is_in_brackets: false,
        }
    }

    pub fn begin_char(&self) -> i32 { self.begin.borrow().begin_char }
    pub fn end_char(&self) -> i32 { self.end.borrow().end_char }
    pub fn length_char(&self) -> i32 { self.end_char() - self.begin_char() + 1 }

    /// Whether this item's value is a valid country code prefix
    pub fn can_be_country_prefix(&self, sofa: &SourceOfAnalysis) -> bool {
        if let Some(prefix) = phone_helper::get_country_prefix(&self.value) {
            if prefix == self.value {
                if self.value.len() != 3 { return true; }
                // 3-digit value: return false if token begins with '('
                if self.begin.borrow().is_char('(', sofa) { return false; }
                return true;
            }
        }
        false
    }

    pub fn is_newline_before(&self, sofa: &SourceOfAnalysis) -> bool {
        self.begin.borrow().is_newline_before(sofa)
    }
}

static PHONE_TERMINS: OnceLock<TerminCollection> = OnceLock::new();

fn phone_termins() -> &'static TerminCollection {
    PHONE_TERMINS.get_or_init(|| {
        let mut tc = TerminCollection::new();
        macro_rules! term {
            ($text:expr) => {{
                let mut t = Termin::new($text);
                t
            }};
            ($text:expr, kind2: $k:expr) => {{
                let mut t = Termin::new($text);
                t.tag2 = Some(Arc::new($k) as Arc<dyn std::any::Any + Send + Sync>);
                t
            }};
            ($text:expr, tag: $tg:expr) => {{
                let mut t = Termin::new($text);
                t.tag = Some(Arc::new($tg) as Arc<dyn std::any::Any + Send + Sync>);
                t
            }};
        }

        // ТЕЛЕФОН and abbreviations
        let mut t = Termin::new("ТЕЛЕФОН");
        t.add_abridge("ТЕЛ.");
        t.add_abridge("TEL.");
        t.add_abridge("Т-Н");
        t.add_abridge("Т.");
        t.add_abridge("T.");
        t.add_abridge("TEL.EXT.");
        t.add_variant("ТЛФ");
        t.add_variant("ТЛФН");
        t.add_abridge("Т/Ф");
        tc.add(t);

        // МОБИЛЬНЫЙ
        let mut t = Termin::new("МОБИЛЬНЫЙ");
        t.tag2 = Some(Arc::new(PhoneKind::Mobile) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("МОБ.");
        t.add_abridge("Т.М.");
        t.add_abridge("М.Т.");
        t.add_abridge("М.");
        tc.add(t);

        // СОТОВЫЙ
        let mut t = Termin::new("СОТОВЫЙ");
        t.tag2 = Some(Arc::new(PhoneKind::Mobile) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("СОТ.");
        t.add_abridge("CELL.");
        tc.add(t);

        // РАБОЧИЙ
        let mut t = Termin::new("РАБОЧИЙ");
        t.tag2 = Some(Arc::new(PhoneKind::Work) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("РАБ.");
        t.add_abridge("Т.Р.");
        t.add_abridge("Р.Т.");
        tc.add(t);

        // ГОРОДСКОЙ
        let mut t = Termin::new("ГОРОДСКОЙ");
        t.add_abridge("ГОР.");
        t.add_abridge("Г.Т.");
        tc.add(t);

        // ДОМАШНИЙ
        let mut t = Termin::new("ДОМАШНИЙ");
        t.tag2 = Some(Arc::new(PhoneKind::Home) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("ДОМ.");
        tc.add(t);

        // КОНТАКТНЫЙ
        let mut t = Termin::new("КОНТАКТНЫЙ");
        t.add_variant("КОНТАКТНЫЕ ДАННЫЕ");
        tc.add(t);

        // МНОГОКАНАЛЬНЫЙ
        tc.add(Termin::new("МНОГОКАНАЛЬНЫЙ"));

        // ФАКС
        let mut t = Termin::new("ФАКС");
        t.tag2 = Some(Arc::new(PhoneKind::Fax) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("Ф.");
        t.add_variant("ТЕЛЕФАКС");
        tc.add(t);

        // ЗВОНИТЬ
        tc.add(Termin::new("ЗВОНИТЬ"));

        // ПРИЕМНАЯ
        let mut t = Termin::new("ПРИЕМНАЯ");
        t.tag2 = Some(Arc::new(PhoneKind::Work) as Arc<dyn std::any::Any + Send + Sync>);
        tc.add(t);

        // PHONE (EN)
        let mut t = Termin::new("PHONE");
        t.add_abridge("PH.");
        t.add_variant("TELEFON");
        tc.add(t);

        // DIRECT LINE (EN)
        let mut t = Termin::new("DIRECT LINE");
        t.tag2 = Some(Arc::new(PhoneKind::Work) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_variant("DIRECT LINES");
        tc.add(t);

        // MOBILE (EN)
        let mut t = Termin::new("MOBILE");
        t.tag2 = Some(Arc::new(PhoneKind::Mobile) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("MOB.");
        t.add_variant("MOBIL");
        t.add_abridge("M.");
        tc.add(t);

        // FAX (EN)
        let mut t = Termin::new("FAX");
        t.tag2 = Some(Arc::new(PhoneKind::Fax) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("F.");
        tc.add(t);

        // HOME (EN)
        let mut t = Termin::new("HOME");
        t.tag2 = Some(Arc::new(PhoneKind::Home) as Arc<dyn std::any::Any + Send + Sync>);
        tc.add(t);

        // CALL (EN)
        let mut t = Termin::new("CALL");
        t.add_variant("SEDIU");
        t.add_variant("CALL AT");
        tc.add(t);

        // ДОПОЛНИТЕЛЬНЫЙ (additional number marker - tag != None)
        let mut t = Termin::new("ДОПОЛНИТЕЛЬНЫЙ");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("ДОП.");
        t.add_abridge("EXT.");
        tc.add(t);

        // ДОБАВОЧНЫЙ
        let mut t = Termin::new("ДОБАВОЧНЫЙ");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("ДОБ.");
        t.add_abridge("Д.");
        tc.add(t);

        // ВНУТРЕННИЙ
        let mut t = Termin::new("ВНУТРЕННИЙ");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("ВНУТР.");
        t.add_abridge("ВН.");
        t.add_abridge("ВНТ.");
        t.add_abridge("Т.ВН.");
        tc.add(t);

        // TONE MODE (EN)
        let mut t = Termin::new("TONE MODE");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        tc.add(t);

        // TONE (EN)
        let mut t = Termin::new("TONE");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        tc.add(t);

        // ADDITIONAL (EN)
        let mut t = Termin::new("ADDITIONAL");
        t.tag = Some(Arc::new(true) as Arc<dyn std::any::Any + Send + Sync>);
        t.add_abridge("ADD.");
        t.add_variant("INTERNAL");
        t.add_abridge("INT.");
        tc.add(t);

        tc
    })
}

/// Core attach logic for a single phone component token
fn _try_attach(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<PhoneItemToken> {
    let t0b = t0.borrow();

    // NumberToken → Number item
    if t0b.is_number_token() {
        let val = t0b.number_value().unwrap_or("").to_string();
        drop(t0b);
        return Some(PhoneItemToken::new(t0.clone(), t0.clone(), PhoneItemType::Number, val));
    }

    // Single punctuation checks
    if t0b.is_char('.', sofa) {
        drop(t0b);
        return Some(PhoneItemToken::new(t0.clone(), t0.clone(), PhoneItemType::Delim, ".".to_string()));
    }
    if t0b.is_hiphen(sofa) {
        drop(t0b);
        return Some(PhoneItemToken::new(t0.clone(), t0.clone(), PhoneItemType::Delim, "-".to_string()));
    }

    // '+' followed by a digit token → country code
    if t0b.is_char('+', sofa) {
        let next_opt = t0b.next.clone();
        drop(t0b);
        if let Some(next) = next_opt {
            let nb = next.borrow();
            if nb.is_number_token() {
                let val = nb.number_value().unwrap_or("").to_string();
                drop(nb);
                // Strip leading zeros
                let val = val.trim_start_matches('0').to_string();
                if val.is_empty() { return None; }
                return Some(PhoneItemToken::new(t0.clone(), next.clone(), PhoneItemType::CountryCode, val));
            }
        }
        return None;
    }

    // Non-breaking hyphen (U+2011) followed by 2-digit number
    if t0b.begin_char == t0b.end_char {
        let ch = sofa.char_at(t0b.begin_char);
        if ch == '\u{2011}' {
            let next_opt = t0b.next.clone();
            drop(t0b);
            if let Some(next) = next_opt {
                let nb = next.borrow();
                if nb.is_number_token() && nb.length_char() == 2 {
                    drop(nb);
                    return Some(PhoneItemToken::new(t0.clone(), t0.clone(), PhoneItemType::Delim, "-".to_string()));
                }
            }
            return None;
        }
    }

    // '(' → city code in brackets or prefix in brackets
    if t0b.is_char_of("(", sofa) {
        let next_opt = t0b.next.clone();
        drop(t0b);

        if let Some(next) = next_opt {
            let nb = next.borrow();
            if nb.is_number_token() {
                // Accumulate digits between ( and )
                let mut val = String::new();
                let mut et = next.clone();
                loop {
                    let etb = et.borrow();
                    if etb.is_char(')', sofa) { break; }
                    if etb.is_number_token() {
                        val.push_str(etb.number_value().unwrap_or(""));
                    } else if !etb.is_hiphen(sofa) && !etb.is_char('.', sofa) {
                        drop(etb);
                        return None;
                    }
                    let next2 = etb.next.clone();
                    drop(etb);
                    match next2 {
                        Some(n) => et = n,
                        None => return None,
                    }
                }
                if val.is_empty() { return None; }
                let mut item = PhoneItemToken::new(t0.clone(), et.clone(), PhoneItemType::CityCode, val);
                item.is_in_brackets = true;
                return Some(item);
            } else {
                drop(nb);
                // Try to parse a phone termin in brackets
                let tt = phone_termins().try_parse(&next);
                if let Some(tt) = tt {
                    if tt.termin.tag.is_none() {
                        // Check that TerminToken.end.next is ')'
                        let end_next = tt.end_token.borrow().next.clone();
                        if let Some(en) = end_next {
                            if en.borrow().is_char(')', sofa) {
                                let item = PhoneItemToken {
                                    begin: t0.clone(),
                                    end: en.clone(),
                                    item_type: PhoneItemType::Prefix,
                                    value: String::new(),
                                    kind: PhoneKind::Undefined,
                                    kind2: PhoneKind::Undefined,
                                    is_in_brackets: true,
                                };
                                return Some(item);
                            }
                        }
                    }
                }
                return None;
            }
        }
        return None;
    }

    // '/NNN/' style city code
    if t0b.is_char('/', sofa) {
        let n1_opt = t0b.next.clone();
        drop(t0b);
        if let Some(n1) = n1_opt {
            let n1b = n1.borrow();
            if n1b.is_number_token() && n1b.length_char() == 3 {
                let val = n1b.number_value().unwrap_or("").to_string();
                let n2_opt = n1b.next.clone();
                drop(n1b);
                if let Some(n2) = n2_opt {
                    if n2.borrow().is_char('/', sofa) {
                        let mut item = PhoneItemToken::new(t0.clone(), n2.clone(), PhoneItemType::CityCode, val);
                        item.is_in_brackets = true;
                        return Some(item);
                    }
                }
            }
        }
        return None;
    }

    // Т/Р or Т/М pattern
    if t0b.is_value("Т", None) {
        let n1_opt = t0b.next.clone();
        drop(t0b);
        if let Some(n1) = n1_opt {
            let n1b = n1.borrow();
            if n1b.is_char_of("\\/", sofa) {
                let n2_opt = n1b.next.clone();
                drop(n1b);
                if let Some(n2) = n2_opt {
                    let n2b = n2.borrow();
                    if n2b.is_value("Р", None) || n2b.is_value("М", None) {
                        let ki = if n2b.is_value("Р", None) { PhoneKind::Work } else { PhoneKind::Mobile };
                        drop(n2b);
                        let mut item = PhoneItemToken::new(t0.clone(), n2.clone(), PhoneItemType::Prefix, String::new());
                        item.kind = ki;
                        return Some(item);
                    }
                }
            }
        }
        // Fall through to termin matching
    } else {
        drop(t0b);
    }

    // Try phone termins
    let t0b = t0.borrow();
    let t0_next_opt = t0b.next.clone();
    let t0_len = t0b.length_char();
    let t0_is_all_upper = t0b.chars.is_all_upper();
    let t0_ws_after = t0b.is_whitespace_after(sofa);
    drop(t0b);

    // Check for НОМЕР
    {
        let t0b = t0.borrow();
        if t0b.is_value("НОМЕР", None) {
            drop(t0b);
            if let Some(next) = t0_next_opt.clone() {
                if let Some(mut rr) = _try_attach(&next, sofa) {
                    if rr.item_type == PhoneItemType::Prefix {
                        rr.begin = t0.clone();
                        return Some(rr);
                    }
                }
            }
            return None;
        }
    }

    let tt = phone_termins().try_parse(t0)?;
    if tt.termin.tag.is_some() {
        return None; // It's an "additional" termin, not a prefix
    }

    // Single uppercase letter followed by dot → check context
    if t0_len == 1 && t0_is_all_upper {
        if let Some(next) = t0_next_opt.clone() {
            if next.borrow().is_char('.', sofa) {
                // Check if preceded by another single-letter.dot pattern
                let t0b = t0.borrow();
                if let Some(prev_weak) = &t0b.prev {
                    if let Some(prev) = prev_weak.upgrade() {
                        let prevb = prev.borrow();
                        if prevb.is_char('.', sofa) {
                            if let Some(prev2_weak) = &prevb.prev {
                                if let Some(prev2) = prev2_weak.upgrade() {
                                    let prev2b = prev2.borrow();
                                    if prev2b.length_char() == 1 {
                                        drop(prev2b);
                                        drop(prevb);
                                        drop(t0b);
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Get the phone kind from tag2
    let ki = tt.termin.tag2.as_ref()
        .and_then(|t2| t2.downcast_ref::<PhoneKind>())
        .copied()
        .unwrap_or(PhoneKind::Undefined);

    let mut t1 = tt.end_token.clone();
    let mut res = PhoneItemToken::new(t0.clone(), t1.clone(), PhoneItemType::Prefix, String::new());
    res.kind = ki;

    // Extend with trailing '.' or '/' or '/' + ФАКС
    loop {
        let t1b = t1.borrow();
        let t1_next_opt = t1b.next.clone();
        drop(t1b);

        if let Some(t1n) = t1_next_opt.clone() {
            let t1nb = t1n.borrow();
            if t1nb.is_char_of(".:", sofa) {
                let new_end = t1n.clone();
                drop(t1nb);
                res.end = new_end.clone();
                t1 = new_end;
                continue;
            } else if t1nb.is_table_control_char(sofa) {
                drop(t1nb);
                t1 = t1n;
                continue;
            } else if t1nb.is_char_of("/\\", sofa) {
                let t1nn_opt = t1nb.next.clone();
                drop(t1nb);
                if let Some(t1nn) = t1nn_opt {
                    if t1nn.borrow().is_value("ФАКС", None) {
                        res.kind2 = PhoneKind::Fax;
                        res.end = t1nn.clone();
                        t1 = t1nn;
                        break;
                    }
                }
                break;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Single-char uppercase (abbreviation) must have whitespace after
    if t0_len == 1 && t0_is_all_upper && !t0_ws_after {
        return None;
    }

    Some(res)
}

/// Attach a single phone component, with prefix extension
pub fn try_attach(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<PhoneItemToken> {
    let mut res = _try_attach(t0, sofa)?;

    if res.item_type != PhoneItemType::Prefix {
        return Some(res);
    }

    // Try to extend the prefix with following text (noun phrase etc.) — simplified
    let mut t = res.end.borrow().next.clone();
    while let Some(tok) = t {
        let tb = tok.borrow();
        if tb.is_table_control_char(sofa) { break; }
        if tb.is_newline_before(sofa) { break; }

        if let Some(res2) = _try_attach(&tok, sofa) {
            if res2.item_type == PhoneItemType::Prefix {
                if res.kind == PhoneKind::Undefined {
                    res.kind = res2.kind;
                }
                let new_end = res2.end.clone();
                drop(tb);
                res.end = new_end.clone();
                t = new_end.borrow().next.clone();
                continue;
            }
            drop(tb);
            break;
        }

        if tb.is_char(':', sofa) {
            res.end = tok.clone();
            drop(tb);
            break;
        }

        // Stop on non-text tokens (simplified — skip noun phrase parsing)
        if !matches!(tb.kind, crate::token::TokenKind::Text(_)) {
            drop(tb);
            break;
        }
        let t0_len = res.begin.borrow().length_char();
        drop(tb);
        if t0_len == 1 { break; }

        // Advance through text tokens (simplified)
        res.end = tok.clone();
        t = res.end.borrow().next.clone();
    }

    Some(res)
}

/// Try to attach an additional (extension) number after the main phone number
pub fn try_attach_additional(t0_opt: Option<TokenRef>, sofa: &SourceOfAnalysis) -> Option<PhoneItemToken> {
    let t0 = t0_opt?;
    let mut t = t0.clone();

    {
        let tb = t.borrow();
        if tb.is_char(',', sofa) {
            let next = tb.next.clone();
            drop(tb);
            t = next?;
        } else if tb.is_char_of("*#", sofa) {
            let next = tb.next.clone();
            if let Some(n) = next {
                let nb = n.borrow();
                if nb.is_number_token() {
                    let mut val = nb.number_value().unwrap_or("").to_string();
                    let mut t1 = n.clone();
                    let n_next_opt = nb.next.clone();
                    let n_ws_after = nb.is_whitespace_after(sofa);
                    drop(nb);
                    // Check for hyphen-concatenated addition
                    if let Some(n2) = n_next_opt {
                        if n2.borrow().is_hiphen(sofa) && !n_ws_after {
                            if let Some(n3_opt) = n2.borrow().next.clone() {
                                let n3b = n3_opt.borrow();
                                if n3b.is_number_token() {
                                    val.push_str(n3b.number_value().unwrap_or(""));
                                    t1 = n3_opt.clone();
                                }
                            }
                        }
                    }
                    if val.len() >= 3 && val.len() < 7 {
                        return Some(PhoneItemToken::new(t0.clone(), t1.clone(), PhoneItemType::AddNumber, val));
                    }
                }
            }
            return None;
        } else {
            drop(tb);
        }
    }

    let mut br = false;
    {
        let tb = t.borrow();
        if tb.is_char('(', sofa) {
            // If preceded by comma → return None
            let prev_opt = tb.prev.as_ref().and_then(|w| w.upgrade());
            if let Some(prev) = prev_opt {
                if prev.borrow().is_comma(sofa) { return None; }
            }
            br = true;
            let next = tb.next.clone();
            drop(tb);
            t = next?;
        } else {
            drop(tb);
        }
    }

    let to = phone_termins().try_parse(&t);
    if let Some(ref to_tok) = to {
        if to_tok.termin.tag.is_none() { return None; } // not an additional termin
        let end_next = to_tok.end_token.borrow().next.clone();
        t = end_next?;
    } else {
        if !br { return None; }
        let tb = t.borrow();
        if tb.whitespaces_before_count(sofa) > 1 { return None; }
    }

    // Skip НОМЕР / N / # / № / NUMBER / +
    {
        let tb = t.borrow();
        if tb.is_value("НОМЕР", None)
            || tb.is_value("N", None)
            || tb.is_char('#', sofa)
            || tb.is_value("№", None)
            || tb.is_value("NUMBER", None)
            || (tb.is_char('+', sofa) && br)
        {
            let next = tb.next.clone();
            drop(tb);
            t = next?;
        } else if to.is_none() && !br {
            return None;
        } else if tb.is_value("НОМ", None) || tb.is_value("ТЕЛ", None) {
            let next = tb.next.clone();
            drop(tb);
            let mut t2 = next?;
            if t2.borrow().is_char('.', sofa) {
                let n = t2.borrow().next.clone();
                t2 = n?;
            }
            t = t2;
        } else {
            drop(tb);
        }
    }

    // Skip ':' or ','
    {
        let tb = t.borrow();
        if tb.is_char_of(":,", sofa) && !tb.is_newline_after(sofa) {
            let next = tb.next.clone();
            drop(tb);
            t = next?;
        } else {
            drop(tb);
        }
    }

    // Must be a digit token now
    {
        let tb = t.borrow();
        if !tb.is_number_token() { return None; }
        let mut val = tb.number_value().unwrap_or("").to_string();
        let t_ws_after = tb.is_whitespace_after(sofa);
        let t_next_opt = tb.next.clone();
        let mut t1 = t.clone();
        drop(tb);

        // Maybe concatenate hyphen-number
        if let Some(tn) = t_next_opt {
            let tnb = tn.borrow();
            if tnb.is_hiphen(sofa) && !t_ws_after {
                let tn2_opt = tnb.next.clone();
                drop(tnb);
                if let Some(tn2) = tn2_opt {
                    if tn2.borrow().is_number_token() {
                        val.push_str(tn2.borrow().number_value().unwrap_or(""));
                        t1 = tn2;
                    }
                }
            }
        }

        if val.len() < 2 || val.len() > 7 { return None; }

        if br {
            let t1b = t1.borrow();
            let t1_next_opt = t1b.next.clone();
            drop(t1b);
            let closing = t1_next_opt?;
            if !closing.borrow().is_char(')', sofa) { return None; }
            t1 = closing;
        }

        Some(PhoneItemToken::new(t0.clone(), t1, PhoneItemType::AddNumber, val))
    }
}

/// Attach all phone item tokens starting from t0, up to maxCount items
pub fn try_attach_all(t0: &TokenRef, sofa: &SourceOfAnalysis, max_count: usize) -> Option<Vec<PhoneItemToken>> {
    let mut p_opt = try_attach(t0, sofa);
    let mut br = false;

    if p_opt.is_none() {
        // Try starting with '('
        if t0.borrow().is_char('(', sofa) {
            if let Some(next) = t0.borrow().next.clone() {
                p_opt = try_attach(&next, sofa);
                if let Some(ref mut p) = p_opt {
                    p.begin = t0.clone();
                    p.is_in_brackets = true;
                    if p.item_type == PhoneItemType::Prefix { br = false; } else { br = true; }
                }
            }
        }
    }

    let mut p = p_opt?;
    if p.item_type == PhoneItemType::Delim { return None; }

    let mut res: Vec<PhoneItemToken> = Vec::new();
    res.push(p.clone());

    let mut t_cur: Option<TokenRef> = p.end.borrow().next.clone();
    loop {
        let tok = match t_cur.clone() { None => break, Some(x) => x };
        if res.len() > max_count { break; }

        let is_table_ctrl = tok.borrow().is_table_control_char(sofa);
        let is_close_paren = tok.borrow().is_char(')', sofa);

        if is_table_ctrl {
            if res.len() == 1 && res[0].item_type == PhoneItemType::Prefix {
                t_cur = tok.borrow().next.clone();
                continue;
            } else {
                break;
            }
        }

        if br && is_close_paren {
            br = false;
            t_cur = tok.borrow().next.clone();
            continue;
        }

        let p0_opt = try_attach(&tok, sofa);
        let is_newline;
        let is_ws2plus;
        let is_slash;
        let is_hiphen;
        let tok_ws_before;
        {
            let tb = tok.borrow();
            is_newline = tb.is_newline_before(sofa);
            is_ws2plus = tb.whitespaces_before_count(sofa) > 1;
            is_slash = tb.is_char_of("/\\", sofa);
            is_hiphen = tb.is_hiphen(sofa);
            tok_ws_before = tb.is_whitespace_before(sofa);
        }

        if p0_opt.is_none() {
            if is_newline { break; }

            // Try prefix handling
            if p.item_type == PhoneItemType::Prefix && (is_slash || is_hiphen) {
                if let Some(next) = tok.borrow().next.clone() {
                    if let Some(p0) = try_attach(&next, sofa) {
                        if p0.item_type == PhoneItemType::Prefix {
                            let new_end = p0.end.clone();
                            res.last_mut().unwrap().end = new_end.clone();
                            t_cur = new_end.borrow().next.clone();
                            continue;
                        }
                    }
                }
            }

            // Try slash-number continuations for prefix items
            if !res.is_empty() && res[0].item_type == PhoneItemType::Prefix
                && is_slash && !tok_ws_before && !tok.borrow().is_whitespace_after(sofa)
            {
                if let Some(next) = tok.borrow().next.clone() {
                    if next.borrow().is_number_token() {
                        let sum_num: i32 = res.iter()
                            .filter(|it| matches!(it.item_type, PhoneItemType::CityCode | PhoneItemType::CountryCode | PhoneItemType::Number))
                            .map(|it| it.value.len() as i32)
                            .sum();
                        if sum_num < 7 {
                            let mut sum2 = sum_num;
                            let mut nt = next.clone();
                            loop {
                                let ntb = nt.borrow();
                                if ntb.is_whitespace_before(sofa) { break; }
                                if ntb.is_number_token() {
                                    sum2 += ntb.length_char();
                                } else if matches!(ntb.kind, crate::token::TokenKind::Text(_)) && !ntb.chars.is_letter() {
                                    // ok
                                } else {
                                    break;
                                }
                                let nn = ntb.next.clone();
                                drop(ntb);
                                match nn { Some(n) => nt = n, None => break }
                            }
                            if sum2 == 10 || sum2 == 11 {
                                t_cur = tok.borrow().next.clone();
                                continue;
                            }
                        }
                    }
                }
            }
            break;
        }

        let mut p0 = p0_opt.unwrap();

        if is_newline && p.item_type != PhoneItemType::Prefix {
            break;
        }

        if is_ws2plus {
            let ok = res.iter().any(|pp| matches!(pp.item_type, PhoneItemType::Prefix | PhoneItemType::CountryCode));
            if !ok { break; }
        }

        if br && p.item_type == PhoneItemType::Number {
            p.item_type = PhoneItemType::CityCode;
        }

        // Insert implicit delimiter if two consecutive number items
        if p0.item_type == PhoneItemType::Number && res.last().map_or(false, |l| l.item_type == PhoneItemType::Number) {
            res.push(PhoneItemToken::new(tok.clone(), tok.clone(), PhoneItemType::Delim, " ".to_string()));
        }

        if br { p0.is_in_brackets = true; }
        let p0_end = p0.end.clone();
        p = p0.clone();
        res.push(p0);
        t_cur = p0_end.borrow().next.clone();
    }

    // Try to append additional number
    if let Some(add) = try_attach_additional(t_cur, sofa) {
        res.push(add);
    }

    // Clean up: remove trailing delims
    while res.last().map_or(false, |r| r.item_type == PhoneItemType::Delim) {
        res.pop();
    }

    // Remove delim before in-brackets item, or double delims
    let mut i = 1usize;
    while i + 1 < res.len() {
        if res[i].item_type == PhoneItemType::Delim && res[i + 1].is_in_brackets {
            res.remove(i);
        } else if res[i].item_type == PhoneItemType::Delim && res[i + 1].item_type == PhoneItemType::Delim {
            let new_end = res[i + 1].end.clone();
            res[i].end = new_end;
            res.remove(i + 1);
        } else {
            i += 1;
        }
    }

    // If last item has ')' after and first is '(', include closing paren
    if res.len() > 1 && res[0].is_in_brackets && res[0].item_type == PhoneItemType::Prefix {
        let last_next = res.last().and_then(|last| last.end.borrow().next.clone());
        if let Some(ln) = last_next {
            if ln.borrow().is_char(')', sofa) {
                res.last_mut().unwrap().end = ln;
            }
        }
    }

    // If first is Prefix, trim extra inner Prefix sequences
    if res.first().map_or(false, |r| r.item_type == PhoneItemType::Prefix) {
        let mut i = 2usize;
        while i + 1 < res.len() {
            if res[i].item_type == PhoneItemType::Prefix && res[i + 1].item_type != PhoneItemType::Prefix {
                res.drain(i..);
                break;
            }
            i += 1;
        }
    }

    // Normalize triple-CityCode: treat extra CityCodes as Numbers
    let mut i = 0usize;
    while i + 2 < res.len() {
        if res[i].item_type == PhoneItemType::CityCode
            && res[i + 1].item_type == PhoneItemType::CityCode
            && res[i + 2].item_type == PhoneItemType::CityCode
        {
            for j in (i + 1)..res.len() {
                if res[j].item_type == PhoneItemType::CityCode {
                    res[j].item_type = PhoneItemType::Number;
                }
            }
            break;
        }
        i += 1;
    }

    Some(res)
}

/// Try to attach alternate digit variants after a phone number
pub fn try_attach_alternate(
    t0_opt: Option<TokenRef>,
    ph_template: Option<&str>,
    pli: &[PhoneItemToken],
    sofa: &SourceOfAnalysis,
) -> Option<PhoneItemToken> {
    let t0 = t0_opt?;
    let t0b = t0.borrow();

    // /N or \N where N is 1-2 digit number
    if t0b.is_char_of("/\\", sofa) {
        let next_opt = t0b.next.clone();
        drop(t0b);
        if let Some(next) = next_opt {
            let nb = next.borrow();
            if nb.is_number_token() && nb.length_char() <= 2 {
                let val = nb.number_value().unwrap_or("").to_string();
                let next_end = next.clone();
                drop(nb);

                // Try to match full sequence
                if let Some(pli1) = try_attach_all(&next, sofa, 15) {
                    let mut pli1 = pli1;
                    if pli1.last().map_or(false, |r| r.item_type == PhoneItemType::Delim) {
                        pli1.pop();
                    }
                    if pli1.len() > 1 && pli1.len() <= pli.len() {
                        let offset = pli.len() - pli1.len();
                        let mut ii = 0usize;
                        let mut num = String::new();
                        while ii < pli1.len() {
                            let p1 = &pli1[ii];
                            let p0 = &pli[offset + ii];
                            if p1.item_type != p0.item_type { break; }
                            if p1.item_type != PhoneItemType::Number && p1.item_type != PhoneItemType::Delim { break; }
                            if p1.item_type == PhoneItemType::Number {
                                if p1.length_char() != p0.length_char() { break; }
                                num.push_str(&p1.value);
                            }
                            ii += 1;
                        }
                        if ii >= pli1.len() {
                            let last_end = pli1.last().unwrap().end.clone();
                            return Some(PhoneItemToken::new(t0.clone(), last_end, PhoneItemType::Alt, num));
                        }
                    }
                }

                return Some(PhoneItemToken::new(t0.clone(), next_end, PhoneItemType::Alt, val));
            }
        }
        return None;
    }

    // -N where N is single digit at end
    if t0b.is_hiphen(sofa) {
        let next_opt = t0b.next.clone();
        drop(t0b);
        if let Some(next) = next_opt {
            let nb = next.borrow();
            if nb.is_number_token() && nb.length_char() <= 2 {
                let val = nb.number_value().unwrap_or("").to_string();
                let t1 = next.clone();
                let t1_next_opt = nb.next.clone();
                drop(nb);
                let ok = t1_next_opt.as_ref()
                    .map_or(true, |n| {
                        n.borrow().is_newline_before(sofa) || n.borrow().is_char_of(",.", sofa)
                    });
                if ok {
                    return Some(PhoneItemToken::new(t0.clone(), t1, PhoneItemType::Alt, val));
                }
            }
        }
        return None;
    }

    // (N) where N is exactly 2 chars
    if t0b.is_char('(', sofa) {
        let next_opt = t0b.next.clone();
        drop(t0b);
        if let Some(next) = next_opt {
            let nb = next.borrow();
            if nb.is_number_token() && nb.length_char() == 2 {
                let val = nb.number_value().unwrap_or("").to_string();
                let t1_opt = nb.next.clone();
                drop(nb);
                if let Some(t1) = t1_opt {
                    if t1.borrow().is_char(')', sofa) {
                        return Some(PhoneItemToken::new(t0.clone(), t1.clone(), PhoneItemType::Alt, val));
                    }
                }
            }
        }
        return None;
    }

    // /N or -N where template ends with that number length
    {
        let t0b = t0.borrow();
        if t0b.is_char_of("/-", sofa) {
            let next_opt = t0b.next.clone();
            drop(t0b);
            if let Some(next) = next_opt {
                let nb = next.borrow();
                if nb.is_number_token() {
                    if let Some(tmpl) = ph_template {
                        let len_str = nb.length_char().to_string();
                        if tmpl.ends_with(len_str.as_str()) {
                            let val = nb.number_value().unwrap_or("").to_string();
                            let t1 = next.clone();
                            drop(nb);
                            return Some(PhoneItemToken::new(t0.clone(), t1, PhoneItemType::Alt, val));
                        }
                    }
                }
            }
        } else {
            drop(t0b);
        }
    }

    None
}
