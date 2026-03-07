/// AddressAnalyzer — simplified address/street recognition for Russian text.
///
/// Recognizes patterns like:
///   "ул. Ленина, д. 5"          → STREET + ADDRESS
///   "проспект Мира, 12, кв. 4"  → STREET + ADDRESS
///   "Ленинский проспект, 10"    → STREET (prefix-less form)
///   "ул. Ленина"                → STREET alone (without house number)

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::address::address_referent as ar;
use crate::address::street_table;

pub struct AddressAnalyzer;

impl AddressAnalyzer {
    pub fn new() -> Self { AddressAnalyzer }
}

impl Analyzer for AddressAnalyzer {
    fn name(&self) -> &'static str { "ADDRESS" }
    fn caption(&self) -> &'static str { "Адреса" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }
            match try_parse_street(&t, &sofa) {
                None => { cur = t.borrow().next.clone(); }
                Some((street, street_end)) => {
                    // Register the STREET referent
                    let s_rc = Rc::new(RefCell::new(street));
                    kit.add_entity(s_rc.clone());
                    let street_tok = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), street_end.clone(), s_rc.clone())
                    ));
                    kit.embed_token(street_tok.clone());

                    // Try to build an ADDRESS from the street + following house/flat info
                    let next_after_street = street_tok.borrow().next.clone();
                    if let Some((address, addr_end)) = try_parse_address_after_street(
                        s_rc.clone(), &next_after_street, &sofa
                    ) {
                        let a_rc = Rc::new(RefCell::new(address));
                        kit.add_entity(a_rc.clone());
                        let addr_tok = Rc::new(RefCell::new(
                            Token::new_referent(street_tok.clone(), addr_end, a_rc)
                        ));
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
    if let TokenKind::Text(txt) = &tb.kind {
        let surf_upper = txt.term.to_uppercase();
        let morph_uppers = collect_upper_forms_morph(&tb);
        drop(tb);

        // Pattern A: street-type abbreviation/keyword FIRST, then name(s)
        // e.g. "ул. Ленина", "пр. Мира", "проспект Революции"
        for key in std::iter::once(surf_upper.as_str()).chain(morph_uppers.iter().map(String::as_str)) {
            if let Some(entry) = street_table::lookup_street_type(key) {
                if let Some((name, end)) = collect_street_name_after(t, sofa) {
                    let mut r = ar::new_street_referent();
                    ar::add_slot_str(&mut r, ar::STREET_ATTR_TYPE, &entry.canonical);
                    ar::add_slot_str(&mut r, ar::STREET_ATTR_NAME, &name);
                    return Some((r, end));
                }
            }
        }
        return None;
    }
    drop(tb);
    None
}

/// Collect uppercase morph normal forms for a Token.
fn collect_upper_forms_morph(tb: &crate::token::Token) -> Vec<String> {
    let mut v = Vec::new();
    for wf in tb.morph.items() {
        if let Some(nc) = &wf.normal_case { v.push(nc.to_uppercase()); }
        if let Some(nf) = &wf.normal_full { v.push(nf.to_uppercase()); }
    }
    v.dedup();
    v
}

/// Starting after the street-type token (at `type_tok`), collect the street name.
/// The name is one or more capitalized or all-caps tokens, possibly separated by '-'.
fn collect_street_name_after(type_tok: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
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
    if sb.whitespaces_before_count(sofa) > 3 { return None; }

    let (first_upper, first_part) = match &sb.kind {
        TokenKind::Text(_) => {
            let surf = sofa.substring(sb.begin_char, sb.end_char);
            let upper = surf.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
            let part = get_nominative_form(&sb, sofa);
            (upper, part)
        }
        _ => { drop(sb); return None; }
    };
    if !first_upper { drop(sb); return None; }
    drop(sb);

    // Extend with more tokens
    let (name, end) = extend_street_name(first_part, start.clone(), sofa);
    Some((name, end))
}

/// Get nominative form of a token for the street name.
fn get_nominative_form(tb: &crate::token::Token, sofa: &SourceOfAnalysis) -> String {
    if let TokenKind::Text(_) = &tb.kind {
        // Use morph normal form if available
        for wf in tb.morph.items() {
            if let Some(nc) = &wf.normal_case { return nc.to_uppercase(); }
        }
        return sofa.substring(tb.begin_char, tb.end_char).to_uppercase();
    }
    String::new()
}

/// Extend street name with subsequent capitalized tokens.
fn extend_street_name(start: String, start_tok: TokenRef, sofa: &SourceOfAnalysis) -> (String, TokenRef) {
    let mut parts = vec![start];
    let mut end = start_tok.clone();
    let mut cur = start_tok.borrow().next.clone();
    let mut count = 0;
    const MAX: usize = 5;

    while let Some(t) = cur {
        if count >= MAX { break; }
        let tb = t.borrow();
        if tb.whitespaces_before_count(sofa) > 1 { break; }

        match &tb.kind {
            TokenKind::Text(txt) => {
                let surf = sofa.substring(tb.begin_char, tb.end_char);
                // Stop on punctuation
                if txt.term.chars().all(|c| !c.is_alphanumeric()) { break; }
                // Stop if next word is clearly an address keyword (house, flat)
                let up = txt.term.to_uppercase();
                if is_address_stop_word(&up) { break; }
                // Allow all-caps abbreviations (ВОЙСК, etc.) and capitalized words and hyphens
                let first_ch = surf.chars().next().unwrap_or(' ');
                if first_ch.is_lowercase() {
                    // Allow Russian genitive particles
                    if !matches!(up.as_str(), "И" | "ИМ" | "ИМ." | "ИМЕНИ" | "OF" | "AND") {
                        break;
                    }
                    // connector — don't count as name word
                    let next = tb.next.clone();
                    drop(tb);
                    cur = next;
                    continue;
                }
                // If this token is a street type keyword, stop (e.g. "Ленина ул." — suffix form)
                let morph_ups = collect_upper_forms_morph(&tb);
                if morph_ups.iter().any(|u| street_table::lookup_street_type(u).is_some()) { break; }

                // Stop on clearly-stop words
                if is_name_stop_word(&up) { break; }

                let form = get_nominative_form(&tb, sofa);
                parts.push(form);
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
    // House/flat abbreviations signal end of street name
    street_table::HOUSE_ABBRS.contains(&up) ||
    street_table::FLAT_ABBRS.contains(&up) ||
    street_table::CORPUS_ABBRS.contains(&up) ||
    street_table::BUILDING_ABBRS.contains(&up) ||
    street_table::OFFICE_ABBRS.contains(&up) ||
    street_table::FLOOR_ABBRS.contains(&up)
}

fn is_name_stop_word(up: &str) -> bool {
    matches!(up,
        "В" | "НА" | "ПО" | "ОТ" | "ДО" | "ЗА" | "ПРИ" | "С" |
        "ЯВЛЯЕТСЯ" | "КАК" | "ТАК" | "НЕ" | "НИ" | "УЖЕ" | "ЕЩЁ"
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

    // Optionally parse more components: корп., кв., оф., эт.
    loop {
        let comma_tok = end.borrow().next.clone();
        if comma_tok.is_none() { break; }
        let comma_tok = comma_tok.unwrap();
        let is_comma = {
            let cb = comma_tok.borrow();
            cb.length_char() == 1 && sofa.char_at(cb.begin_char) == ','
        };
        if !is_comma { break; }

        let after_comma = comma_tok.borrow().next.clone();
        if after_comma.is_none() { break; }
        let mut probe = after_comma.unwrap();

        if let Some((kind, val, val_end)) = parse_address_component(&mut probe, sofa) {
            match kind.as_str() {
                "flat"   => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_FLAT, &val),
                "corpus" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_CORPUS, &val),
                "floor"  => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_FLOOR, &val),
                "office" => ar::add_slot_str(&mut r, ar::ADDRESS_ATTR_OFFICE, &val),
                _        => {}
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
fn parse_house_component(cur: &mut TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let cb = cur.borrow();
    let upper = match &cb.kind {
        TokenKind::Text(txt) => txt.term.to_uppercase(),
        TokenKind::Number(n) => {
            // Standalone number after comma — treat as house
            let val = n.value.clone();
            let end = cur.clone();
            drop(cb);
            return Some((val, end));
        }
        _ => { drop(cb); return None; }
    };
    drop(cb);

    if street_table::HOUSE_ABBRS.contains(&upper.as_str()) {
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
        // Now expect a number
        let nb2 = next2.borrow();
        match &nb2.kind {
            TokenKind::Number(n) => {
                let val = n.value.clone();
                let end = next2.clone();
                drop(nb2);
                *cur = end.clone();
                return Some((val, end));
            }
            TokenKind::Text(txt) => {
                // Could be something like "5А", "10/2"
                let surf = sofa.substring(nb2.begin_char, nb2.end_char);
                if surf.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    let val = surf.to_uppercase();
                    let end = next2.clone();
                    drop(nb2);
                    *cur = end.clone();
                    return Some((val, end));
                }
                drop(nb2);
                return None;
            }
            _ => { drop(nb2); return None; }
        }
    }

    None
}

/// Try to parse кв./корп./оф./эт. followed by a number.
/// Returns (kind_str, value, end_token).
fn parse_address_component(cur: &mut TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, String, TokenRef)> {
    let cb = cur.borrow();
    let upper = match &cb.kind {
        TokenKind::Text(txt) => txt.term.to_uppercase(),
        _ => { drop(cb); return None; }
    };
    drop(cb);

    let kind = if street_table::FLAT_ABBRS.contains(&upper.as_str()) { "flat" }
    else if street_table::CORPUS_ABBRS.contains(&upper.as_str()) { "corpus" }
    else if street_table::FLOOR_ABBRS.contains(&upper.as_str()) { "floor" }
    else if street_table::OFFICE_ABBRS.contains(&upper.as_str()) { "office" }
    else { return None; };

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

    let nb2 = next2.borrow();
    let val = match &nb2.kind {
        TokenKind::Number(n) => n.value.clone(),
        TokenKind::Text(txt) => {
            let surf = sofa.substring(nb2.begin_char, nb2.end_char);
            if surf.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                surf.to_uppercase()
            } else {
                drop(nb2);
                return None;
            }
        }
        _ => { drop(nb2); return None; }
    };
    let end = next2.clone();
    drop(nb2);
    *cur = end.clone();

    Some((kind.to_string(), val, end))
}
