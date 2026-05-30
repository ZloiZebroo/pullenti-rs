use std::cell::RefCell;
/// AddressAnalyzer — simplified address/street recognition for Russian text.
///
/// Recognizes patterns like:
///   "ул. Ленина, д. 5"          → STREET + ADDRESS
///   "проспект Мира, 12, кв. 4"  → STREET + ADDRESS
///   "Ленинский проспект, 10"    → STREET (prefix-less form)
///   "ул. Ленина"                → STREET alone (without house number)
use std::rc::Rc;

use crate::address::address_referent as ar;
use crate::address::street_table;
use crate::analysis_kit::AnalysisKit;
use crate::analyzer::Analyzer;
use crate::referent::Referent;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::{Token, TokenKind, TokenRef};

pub struct AddressAnalyzer;

impl AddressAnalyzer {
    pub fn new() -> Self {
        AddressAnalyzer
    }
}

impl Analyzer for AddressAnalyzer {
    fn name(&self) -> &'static str {
        "ADDRESS"
    }
    fn caption(&self) -> &'static str {
        "Адреса"
    }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            {
                let tb = t.borrow();
                if tb.is_ignored(&sofa) || !matches!(tb.kind, TokenKind::Text(_)) {
                    cur = tb.next.clone();
                    continue;
                }
            }
            match try_parse_street(&t, &sofa) {
                None => {
                    cur = t.borrow().next.clone();
                }
                Some((street, street_end)) => {
                    // Register the STREET referent
                    let s_rc = Rc::new(RefCell::new(street));
                    let s_rc = kit.add_entity(s_rc);
                    let street_tok = Rc::new(RefCell::new(Token::new_referent(
                        t.clone(),
                        street_end.clone(),
                        s_rc.clone(),
                    )));
                    kit.embed_token(street_tok.clone());

                    // Try to build an ADDRESS from the street + following house/flat info
                    let next_after_street = street_tok.borrow().next.clone();
                    if let Some((address, addr_end)) =
                        try_parse_address_after_street(s_rc.clone(), &next_after_street, &sofa)
                    {
                        let a_rc = Rc::new(RefCell::new(address));
                        let a_rc = kit.add_entity(a_rc);
                        let addr_tok = Rc::new(RefCell::new(Token::new_referent(
                            street_tok.clone(),
                            addr_end,
                            a_rc,
                        )));
                        kit.embed_token(addr_tok.clone());
                        cur = addr_tok.borrow().next.clone();
                    } else {
                        cur = street_tok.borrow().next.clone();
                    }
                }
            }
        }
    }
}

// ── Street parser ─────────────────────────────────────────────────────────────

/// Try to parse a STREET referent starting at `t`.
/// Returns (StreetReferent, end_token).
fn try_parse_street(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();
    let txt = match &tb.kind {
        TokenKind::Text(txt) => txt,
        _ => return None,
    };
    // txt.term is already uppercase; morph normal_case/normal_full are too

    // Fast path: try term directly — no Vec allocation for the ~99% of tokens
    // that are not street-type keywords.
    if let Some(entry) = street_table::lookup_street_type(&txt.term) {
        let canonical = entry.canonical.clone();
        drop(tb);
        if let Some((name, end)) = collect_street_name_after(t, sofa) {
            let mut r = ar::new_street_referent();
            ar::add_slot_str(&mut r, ar::STREET_ATTR_TYPE, &canonical);
            ar::add_slot_str(&mut r, ar::STREET_ATTR_NAME, &name);
            return Some((r, end));
        }
        return None;
    }

    let first_char = sofa.char_at(tb.begin_char);
    drop(tb);

    // ── Suffix pattern: "Name Type" (e.g. "Невский проспект", "Московское шоссе") ──
    // Try it before morph-only street type fallback for capitalized starts, so
    // ordinary proper names do not pay extra morph-form scan costs.
    if first_char.is_uppercase() {
        if let Some(r) = try_suffix_street_type(t, sofa) {
            return Some(r);
        }
    }

    let tb = t.borrow();
    let canonical_from_morph = tb.morph.items().iter().find_map(|wf| {
        wf.normal_case
            .as_deref()
            .and_then(street_table::lookup_street_type)
            .or_else(|| {
                wf.normal_full
                    .as_deref()
                    .and_then(street_table::lookup_street_type)
            })
            .map(|entry| entry.canonical.clone())
    });
    drop(tb);

    if let Some(canonical) = canonical_from_morph {
        if let Some((name, end)) = collect_street_name_after(t, sofa) {
            let mut r = ar::new_street_referent();
            ar::add_slot_str(&mut r, ar::STREET_ATTR_TYPE, &canonical);
            ar::add_slot_str(&mut r, ar::STREET_ATTR_NAME, &name);
            return Some((r, end));
        }
        return None;
    }

    None
}

/// Try suffix street type pattern: "Невский проспект", "Московское шоссе".
/// The current token is the name; check if a subsequent token (1-3 ahead) is a street type.
fn try_suffix_street_type(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Current token must be uppercase text
    {
        let tb = t.borrow();
        if !matches!(tb.kind, TokenKind::Text(_)) {
            return None;
        }
        let surf = sofa.substring(tb.begin_char, tb.end_char);
        if !surf
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return None;
        }
    }

    // Scan ahead up to 3 tokens for a street type keyword
    let mut name_parts: Vec<String> = Vec::new();
    {
        let tb = t.borrow();
        name_parts.push(get_nominative_form(&tb, sofa));
    }
    let mut cur = t.borrow().next.clone();
    let mut type_end: Option<TokenRef> = None;
    let mut canonical: Option<String> = None;

    for _i in 0..3 {
        let tok = match cur {
            Some(ref c) => c.clone(),
            None => break,
        };
        let tb = tok.borrow();
        if tb.whitespaces_before_count(sofa) > 1 {
            break;
        }

        if let TokenKind::Text(ref txt) = tb.kind {
            // Check if this token is a street type keyword
            let entry = street_table::lookup_street_type(&txt.term).or_else(|| {
                tb.morph.items().iter().find_map(|wf| {
                    wf.normal_case
                        .as_deref()
                        .and_then(street_table::lookup_street_type)
                        .or_else(|| {
                            wf.normal_full
                                .as_deref()
                                .and_then(street_table::lookup_street_type)
                        })
                })
            });

            if let Some(e) = entry {
                canonical = Some(e.canonical.clone());
                type_end = Some(tok.clone());
                drop(tb);
                break;
            }

            // Not a street type — accumulate as name part if uppercase
            let surf = sofa.substring(tb.begin_char, tb.end_char);
            if surf
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                name_parts.push(get_nominative_form(&tb, sofa));
            } else {
                break;
            }
        } else {
            break;
        }
        let next = tb.next.clone();
        drop(tb);
        cur = next;
    }

    let canonical = canonical?;
    let _type_end = type_end?;

    let name = name_parts.join(" ");
    let mut r = ar::new_street_referent();
    ar::add_slot_str(&mut r, ar::STREET_ATTR_TYPE, &canonical);
    ar::add_slot_str(&mut r, ar::STREET_ATTR_NAME, &name);
    Some((r, _type_end))
}

/// Starting after the street-type token (at `type_tok`), collect the street name.
/// The name is one or more capitalized or all-caps tokens, possibly separated by '-'.
fn collect_street_name_after(
    type_tok: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(String, TokenRef)> {
    let next = type_tok.borrow().next.clone()?;

    // Skip a dot after an abbreviation (e.g. "ул." — the '.' is separate token)
    let start = {
        let nb = next.borrow();
        if nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '.' {
            drop(nb);
            next.borrow().next.clone()?
        } else {
            drop(nb);
            next.clone()
        }
    };

    let sb = start.borrow();
    if sb.whitespaces_before_count(sofa) > 3 {
        return None;
    }

    let (first_upper, first_part) = match &sb.kind {
        TokenKind::Text(_) | TokenKind::Referent(_) => {
            let surf = sofa.substring(sb.begin_char, sb.end_char);
            let upper = surf
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let part = get_street_name_part(&sb, sofa)?;
            (upper, part)
        }
        TokenKind::Number(n) => (true, n.value.clone()),
        _ => {
            drop(sb);
            return None;
        }
    };
    if !first_upper {
        drop(sb);
        return None;
    }
    drop(sb);

    // Extend with more tokens
    let (name, end) = extend_street_name(first_part, start.clone(), sofa);
    Some((name, end))
}

/// Get nominative form of a token for the street name.
/// Morph normal_case is already uppercase; fallback to surface form uppercased.
fn get_nominative_form(tb: &crate::token::Token, sofa: &SourceOfAnalysis) -> String {
    if let TokenKind::Text(_) = &tb.kind {
        // Use morph normal form if available (already uppercase)
        for wf in tb.morph.items() {
            if let Some(nc) = &wf.normal_case {
                return nc.clone();
            }
        }
        return sofa.substring(tb.begin_char, tb.end_char).to_uppercase();
    }
    String::new()
}

fn get_street_name_part(tb: &crate::token::Token, sofa: &SourceOfAnalysis) -> Option<String> {
    match &tb.kind {
        TokenKind::Text(_) => Some(get_nominative_form(tb, sofa)),
        TokenKind::Number(n) => Some(n.value.clone()),
        TokenKind::Referent(_) => Some(sofa.substring(tb.begin_char, tb.end_char).to_uppercase()),
        _ => None,
    }
}

fn is_month_name(up: &str) -> bool {
    matches!(
        up,
        "ЯНВАРЯ"
            | "ФЕВРАЛЯ"
            | "МАРТА"
            | "АПРЕЛЯ"
            | "МАЯ"
            | "ИЮНЯ"
            | "ИЮЛЯ"
            | "АВГУСТА"
            | "СЕНТЯБРЯ"
            | "ОКТЯБРЯ"
            | "НОЯБРЯ"
            | "ДЕКАБРЯ"
    )
}

/// Extend street name with subsequent capitalized tokens.
fn extend_street_name(
    start: String,
    start_tok: TokenRef,
    sofa: &SourceOfAnalysis,
) -> (String, TokenRef) {
    let mut parts = vec![start];
    let mut end = start_tok.clone();
    let mut cur = start_tok.borrow().next.clone();
    let mut count = 0;
    const MAX: usize = 5;

    while let Some(t) = cur {
        if count >= MAX {
            break;
        }
        let tb = t.borrow();
        if tb.whitespaces_before_count(sofa) > 1 {
            break;
        }

        if tb.length_char() == 1
            && sofa.char_at(tb.begin_char) == '-'
            && parts
                .last()
                .map(|p| p.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        {
            let after_hyphen = tb.next.clone();
            drop(tb);
            let Some(next_word) = after_hyphen else { break };
            let nb = next_word.borrow();
            if nb.whitespaces_before_count(sofa) != 0 {
                break;
            }
            if let TokenKind::Text(_) = &nb.kind {
                let suffix = sofa.substring(nb.begin_char, nb.end_char).to_uppercase();
                if suffix.chars().count() <= 2 {
                    if let Some(last) = parts.last_mut() {
                        last.push('-');
                        last.push_str(&suffix);
                    }
                    end = next_word.clone();
                    count += 1;
                    cur = nb.next.clone();
                    drop(nb);
                    continue;
                }
            }
            break;
        }

        if tb.length_char() == 1
            && sofa.char_at(tb.begin_char) == '.'
            && parts
                .last()
                .map(|p| p.chars().count() == 1)
                .unwrap_or(false)
        {
            let after_dot = tb.next.clone();
            drop(tb);
            let Some(next_word) = after_dot else { break };
            let nb = next_word.borrow();
            if nb.whitespaces_before_count(sofa) != 0 {
                break;
            }
            if matches!(&nb.kind, TokenKind::Text(_) | TokenKind::Referent(_)) {
                let surf = sofa.substring(nb.begin_char, nb.end_char);
                if !surf
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    break;
                }
                let Some(part) = get_street_name_part(&nb, sofa) else {
                    break;
                };
                parts.push(part);
                end = next_word.clone();
                count += 1;
                cur = nb.next.clone();
                drop(nb);
                continue;
            }
            break;
        }

        match &tb.kind {
            TokenKind::Number(n) => {
                parts.push(n.value.clone());
                end = t.clone();
                count += 1;
            }
            TokenKind::Text(txt) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                // Stop on punctuation
                if txt.term.chars().all(|c| !c.is_alphanumeric()) {
                    break;
                }
                // Stop if next word is clearly an address keyword (house, flat)
                // txt.term is already uppercase
                if is_address_stop_word(&txt.term) {
                    break;
                }
                // Allow all-caps abbreviations (ВОЙСК, etc.) and capitalized words and hyphens
                let first_ch = surf.chars().next().unwrap_or(' ');
                if first_ch.is_lowercase() {
                    // Allow Russian genitive particles
                    if !matches!(
                        txt.term.as_str(),
                        "И" | "ИМ" | "ИМ." | "ИМЕНИ" | "ЛЕТ" | "OF" | "AND"
                    ) && !is_month_name(&txt.term)
                    {
                        break;
                    }
                    if txt.term == "ЛЕТ" || is_month_name(&txt.term) {
                        parts.push(
                            get_street_name_part(&tb, sofa).unwrap_or_else(|| txt.term.clone()),
                        );
                        end = t.clone();
                        count += 1;
                    }
                    let next = tb.next.clone();
                    drop(tb);
                    cur = next;
                    continue;
                }
                // If this token is a street type keyword, stop (e.g. "Ленина ул." — suffix form).
                // Check term first (no alloc), then morph forms inline (no Vec).
                if street_table::lookup_street_type(&txt.term).is_some() {
                    break;
                }
                if tb.morph.items().iter().any(|wf| {
                    wf.normal_case
                        .as_deref()
                        .map_or(false, |s| street_table::lookup_street_type(s).is_some())
                        || wf
                            .normal_full
                            .as_deref()
                            .map_or(false, |s| street_table::lookup_street_type(s).is_some())
                }) {
                    break;
                }

                // Stop on clearly-stop words
                if is_name_stop_word(&txt.term) {
                    break;
                }

                let form = get_nominative_form(&tb, sofa);
                parts.push(form);
                end = t.clone();
                count += 1;
            }
            TokenKind::Referent(_) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                if surf.chars().all(|c| !c.is_alphanumeric()) {
                    break;
                }
                if surf
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                {
                    break;
                }
                parts.push(surf.to_uppercase());
                end = t.clone();
                count += 1;
            }
            _ => break,
        }
        let next = tb.next.clone();
        drop(tb);
        cur = next;
    }

    (parts.join(" "), end)
}

fn is_address_stop_word(up: &str) -> bool {
    // House/flat/corpus/building/office/floor abbreviations — O(1) matches! instead of 6 linear searches
    matches!(
        up,
        // HOUSE_ABBRS
        "Д" | "Д." | "ДОМ" | "ДОМОВЛ" | "ДОМОВЛАДЕНИЕ" |
        // FLAT_ABBRS
        "КВ" | "КВ." | "КВАРТИРА" | "ПОМЕЩЕНИЕ" | "ПОМ" | "ПОМ." |
        // CORPUS_ABBRS
        "КОРП" | "КОРП." | "КОРПУС" | "К." |
        // BUILDING_ABBRS
        "СТР" | "СТР." | "СТРОЕНИЕ" |
        // OFFICE_ABBRS
        "ОФ" | "ОФ." | "ОФИС" |
        // FLOOR_ABBRS
        "ЭТ" | "ЭТ." | "ЭТАЖ"
    )
}

fn is_name_stop_word(up: &str) -> bool {
    matches!(
        up,
        "В" | "НА"
            | "ПО"
            | "ОТ"
            | "ДО"
            | "ЗА"
            | "ПРИ"
            | "С"
            | "ЯВЛЯЕТСЯ"
            | "КАК"
            | "ТАК"
            | "НЕ"
            | "НИ"
            | "УЖЕ"
            | "ЕЩЁ"
    )
}

// ── Address parser (house + flat) ─────────────────────────────────────────────

/// After recognizing a STREET, look for comma + house number (+ optional flat).
fn try_parse_address_after_street(
    street_ref: Rc<RefCell<Referent>>,
    start: &Option<TokenRef>,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let mut cur = start.clone()?;

    // Skip optional comma
    {
        let cb = cur.borrow();
        if cb.length_char() == 1 && sofa.char_at(cb.begin_char) == ',' {
            let next = cb.next.clone()?;
            drop(cb);
            cur = next;
        } else {
            drop(cb);
        }
    }

    // Try to match: [Д. / Д / ДОМ] NUMBER
    let house = parse_house_component(&mut cur, sofa)?;

    let mut r = ar::new_address_referent();
    // Add street reference slot
    r.slots.push(crate::referent::Slot {
        type_name: ar::ADDRESS_ATTR_STREET.to_string(),
        value: Some(crate::referent::SlotValue::Referent(street_ref)),
        count: 1,
        occurrence: Vec::new(),
    });
    ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_HOUSE, &house.0);
    let mut end = house.1;

    // Optionally parse more components: корп., стр., кв., оф., эт.
    loop {
        let Some(next_tok) = end.borrow().next.clone() else {
            break;
        };
        let (is_comma, next_after_comma) = {
            let nb = next_tok.borrow();
            (
                nb.length_char() == 1 && sofa.char_at(nb.begin_char) == ',',
                nb.next.clone(),
            )
        };
        let mut probe = if is_comma {
            let Some(after_comma) = next_after_comma else {
                break;
            };
            after_comma
        } else {
            let ws = next_tok.borrow().whitespaces_before_count(sofa);
            if ws == 0 || ws > 2 {
                break;
            }
            next_tok
        };

        if let Some((kind, val, val_end)) = parse_address_component(&mut probe, sofa) {
            match kind {
                "flat" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_FLAT, &val),
                "corpus" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_CORPUS, &val),
                "floor" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_FLOOR, &val),
                "office" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_OFFICE, &val),
                _ => {}
            }
            end = val_end;
        } else {
            break;
        }
    }

    Some((r, end))
}

/// Try to consume "Д." / "Д" / "ДОМ" / standalone number for house.
/// Returns (house_number_string, end_token).
fn parse_house_component(
    cur: &mut TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(String, TokenRef)> {
    let cb = cur.borrow();
    let is_house_abbr = match &cb.kind {
        TokenKind::Text(txt) => matches!(
            txt.term.as_str(),
            "Д" | "Д." | "ДОМ" | "ДОМОВЛ" | "ДОМОВЛАДЕНИЕ"
        ),
        TokenKind::Number(n) => {
            // Standalone number after comma — treat as house
            let val = n.value.clone();
            let end = cur.clone();
            drop(cb);
            return Some((val, end));
        }
        _ => {
            drop(cb);
            return None;
        }
    };
    drop(cb);

    if is_house_abbr {
        // Consume the abbreviation token
        let next = cur.borrow().next.clone()?;
        // Skip optional dot
        let next2 = {
            let nb = next.borrow();
            if nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '.' {
                drop(nb);
                next.borrow().next.clone()?
            } else {
                drop(nb);
                next.clone()
            }
        };
        if let Some((val, end)) = parse_number_like(&next2, sofa) {
            *cur = end.clone();
            return Some((val, end));
        }
    }

    None
}

/// Try to parse кв./корп./оф./эт. followed by a number.
/// Returns (kind_str, value, end_token).
fn parse_address_component(
    cur: &mut TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(&'static str, String, TokenRef)> {
    let cb = cur.borrow();
    let term = match &cb.kind {
        TokenKind::Text(txt) => txt.term.as_str(),
        _ => {
            drop(cb);
            return None;
        }
    };
    // Classify component type using matches! (O(1)) instead of 4 linear .contains() calls
    let kind: &'static str = match term {
        "КВ" | "КВ." | "КВАРТИРА" | "ПОМЕЩЕНИЕ" | "ПОМ" | "ПОМ." => {
            "flat"
        }
        "КОРП" | "КОРП." | "КОРПУС" | "К." | "К" => "corpus",
        "СТР" | "СТР." | "СТРОЕНИЕ" => "corpus",
        "ЭТ" | "ЭТ." | "ЭТАЖ" => "floor",
        "ОФ" | "ОФ." | "ОФИС" => "office",
        _ => {
            drop(cb);
            return None;
        }
    };
    drop(cb);

    // Skip optional dot
    let next = cur.borrow().next.clone()?;
    let next2 = {
        let nb = next.borrow();
        if nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '.' {
            drop(nb);
            next.borrow().next.clone()?
        } else {
            drop(nb);
            next.clone()
        }
    };

    let (val, end) = parse_number_like(&next2, sofa)?;
    *cur = end.clone();

    Some((kind, val, end))
}

fn parse_number_like(start: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let mut value = String::new();
    let mut end = start.clone();
    let mut cur = Some(start.clone());
    let mut saw_digit = false;
    let mut allow_separator = false;

    while let Some(tok) = cur.clone() {
        let tb = tok.borrow();
        if !Rc::ptr_eq(&tok, start) && tb.whitespaces_before_count(sofa) != 0 {
            break;
        }
        match &tb.kind {
            TokenKind::Number(n) => {
                value.push_str(&n.value);
                saw_digit = true;
                allow_separator = true;
                end = tok.clone();
                cur = tb.next.clone();
            }
            TokenKind::Text(_) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                if allow_separator && surf.len() == 1 && (surf == "/" || surf == "-") {
                    let next = tb.next.clone();
                    if next.is_some() {
                        value.push_str(&surf);
                        end = tok.clone();
                        cur = next;
                        allow_separator = false;
                        continue;
                    }
                }
                if saw_digit
                    && surf.chars().count() == 1
                    && surf.chars().all(|c| c.is_alphabetic())
                {
                    value.push_str(&surf.to_uppercase());
                    allow_separator = true;
                    end = tok.clone();
                    cur = tb.next.clone();
                    continue;
                }
                if surf
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    value.push_str(&surf.to_uppercase());
                    saw_digit = true;
                    allow_separator = true;
                    end = tok.clone();
                    cur = tb.next.clone();
                } else {
                    break;
                }
            }
            _ => {
                if allow_separator && tb.length_char() == 1 {
                    let ch = sofa.char_at(tb.begin_char);
                    if ch == '/' || ch == '-' {
                        let next = tb.next.clone();
                        if next.is_some() {
                            value.push(ch);
                            end = tok.clone();
                            cur = next;
                            allow_separator = false;
                            continue;
                        }
                    }
                }
                break;
            }
        }
    }

    if saw_digit {
        Some((value, end))
    } else {
        None
    }
}
