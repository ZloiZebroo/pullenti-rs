/// PersonIdToken — identity document pattern parsing.
/// Mirrors `PersonIdToken.cs`.
///
/// Recognizes patterns like:
///   "паспорт 1234 567890"
///   "паспорт серия 12 34 номер 567890"
///   "водительское удостоверение 77ВВ 123456, выдан 01.01.2020"

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

use crate::token::{TokenRef, TokenKind};
use crate::referent::Referent;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::termin::{Termin, TerminCollection};
use super::person_identity_referent as pir;

// ── Token type ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdTokenTyp {
    Keyword,  // document type keyword
    Seria,    // series
    Number,   // document number
    Date,     // issue date
    Org,      // issuing org
    Vidan,    // "issued by" keyword (скип)
    Code,     // division code (К/П)
    Address,  // registration address
}

// ── Parsed element ────────────────────────────────────────────────────────────

struct IdItem {
    typ:         IdTokenTyp,
    value:       String,
    end_tok:     TokenRef,
    referent:    Option<Rc<RefCell<Referent>>>,
    has_prefix:  bool,
}

// ── Termin collection ─────────────────────────────────────────────────────────

static ONTOLOGY: OnceLock<TerminCollection> = OnceLock::new();

fn tag_typ(t: IdTokenTyp) -> Arc<dyn std::any::Any + Send + Sync> {
    Arc::new(t)
}

fn ontology() -> &'static TerminCollection {
    ONTOLOGY.get_or_init(|| {
        let mut tc = TerminCollection::new();

        // ── Document type keywords ──────────────────────────────────────────
        let mut t = Termin::new("ПАСПОРТ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        t.add_variant("ПАССПОРТ");
        t.add_variant("ПАСПОРТНЫЕ ДАННЫЕ");
        t.add_variant("ВНУТРЕННИЙ ПАСПОРТ");
        tc.add(t);

        let mut t = Termin::new("ЗАГРАНИЧНЫЙ ПАСПОРТ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        t.add_variant("ЗАГРАНПАСПОРТ");
        tc.add(t);

        let mut t = Termin::new("СВИДЕТЕЛЬСТВО О РОЖДЕНИИ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        tc.add(t);

        let mut t = Termin::new("СВИДЕТЕЛЬСТВО О СМЕРТИ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        tc.add(t);

        let mut t = Termin::new("СПРАВКА О СМЕРТИ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        tc.add(t);

        let mut t = Termin::new("УДОСТОВЕРЕНИЕ ЛИЧНОСТИ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        t.add_variant("УДОСТОВЕРЕНИЕ ЛИЧНОСТИ ОФИЦЕРА");
        tc.add(t);

        let mut t = Termin::new("ВОДИТЕЛЬСКОЕ УДОСТОВЕРЕНИЕ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        tc.add(t);

        let mut t = Termin::new("СВИДЕТЕЛЬСТВО О ГОСУДАРСТВЕННОЙ РЕГИСТРАЦИИ ФИЗИЧЕСКОГО ЛИЦА В КАЧЕСТВЕ ИНДИВИДУАЛЬНОГО ПРЕДПРИНИМАТЕЛЯ");
        t.tag = Some(tag_typ(IdTokenTyp::Keyword));
        t.add_variant("СВИДЕТЕЛЬСТВО О ГОСУДАРСТВЕННОЙ РЕГИСТРАЦИИ ФИЗИЧЕСКОГО ЛИЦА В КАЧЕСТВЕ ИП");
        t.add_variant("СВИДЕТЕЛЬСТВО ГОСУДАРСТВЕННОЙ РЕГИСТРАЦИИ");
        tc.add(t);

        // ── Serial number ───────────────────────────────────────────────────
        let mut t = Termin::new("СЕРИЯ");
        t.tag = Some(tag_typ(IdTokenTyp::Seria));
        t.add_variant("СЕРИ");
        tc.add(t);

        // ── Document number ─────────────────────────────────────────────────
        let mut t = Termin::new("НОМЕР");
        t.tag = Some(tag_typ(IdTokenTyp::Number));
        t.add_variant("№");
        t.add_variant("N");
        tc.add(t);

        // ── "Issued" marker ─────────────────────────────────────────────────
        let mut t = Termin::new("ВЫДАТЬ");
        t.tag = Some(tag_typ(IdTokenTyp::Vidan));
        t.add_variant("ВЫДАВАТЬ");
        t.add_variant("ДАТА ВЫДАЧИ");
        t.add_variant("ДАТА РЕГИСТРАЦИИ");
        tc.add(t);

        // ── Division code ───────────────────────────────────────────────────
        let mut t = Termin::new("КОД ПОДРАЗДЕЛЕНИЯ");
        t.tag = Some(tag_typ(IdTokenTyp::Code));
        t.add_variant("КОД");
        tc.add(t);

        // ── Registration address ────────────────────────────────────────────
        let mut t = Termin::new("РЕГИСТРАЦИЯ");
        t.tag = Some(tag_typ(IdTokenTyp::Address));
        t.add_variant("ЗАРЕГИСТРИРОВАН");
        t.add_variant("АДРЕС РЕГИСТРАЦИИ");
        t.add_variant("ЗАРЕГИСТРИРОВАННЫЙ");
        t.add_variant("АДРЕС ПРОПИСКИ");
        t.add_variant("АДРЕС ПО ПРОПИСКЕ");
        tc.add(t);

        tc
    })
}

fn typ_from_tag(tag: &Option<Arc<dyn std::any::Any + Send + Sync>>) -> Option<IdTokenTyp> {
    tag.as_ref()?.downcast_ref::<IdTokenTyp>().copied()
}

// ── Internal parse ────────────────────────────────────────────────────────────

/// Try to parse one ID token element starting at `t`.
/// `is_first` = true means we're looking for the document type keyword (prev=null in C#).
fn try_parse_one(t: &TokenRef, is_first: bool, sofa: &SourceOfAnalysis) -> Option<IdItem> {
    let tb = t.borrow();
    // Skip table control chars and punct-only tokens
    if !tb.chars.is_letter() && !matches!(tb.kind, TokenKind::Referent(_)) && !matches!(tb.kind, TokenKind::Number(_)) {
        return None;
    }
    drop(tb);

    // ── Try termin collection lookup ────────────────────────────────────────
    if let Some(tok) = ontology().try_parse(t) {
        let typ = typ_from_tag(&tok.termin.tag)?;
        let mut res = IdItem {
            typ,
            value: tok.termin.canonic_text.clone(),
            end_tok: tok.end_token.clone(),
            referent: None,
            has_prefix: false,
        };

        if is_first {
            // Only Keyword type allowed as first item
            if typ != IdTokenTyp::Keyword {
                return None;
            }
            // Look for a trailing GeoReferent (state/country) after the keyword
            let mut tt = tok.end_token.borrow().next.clone();
            while let Some(cur) = tt {
                let cb = cur.borrow();
                if cb.is_newline_before(sofa) { break; }
                if let TokenKind::Referent(ref rd) = cb.kind {
                    let rtype = rd.referent.borrow().type_name.clone();
                    if rtype == "GEO" {
                        // check if it's a state
                        let is_state = rd.referent.borrow().slots.iter()
                            .any(|s| s.type_name == "ALPHA2" || s.type_name == "ALPHA3");
                        if is_state {
                            res.referent = Some(rd.referent.clone());
                            res.end_tok = cur.clone();
                        }
                        let next = cb.next.clone();
                        drop(cb);
                        tt = next;
                        continue;
                    }
                    break;
                } else if let TokenKind::Text(ref td) = cb.kind {
                    // "ГРАЖДАНИН" + GEO pattern
                    let term = td.term.clone();
                    if term == "ГРАЖДАНИН" || term == "ГРАЖДАНКА" {
                        let next = cb.next.clone();
                        drop(cb);
                        if let Some(nn) = next {
                            if let TokenKind::Referent(ref rd) = nn.borrow().kind {
                                if rd.referent.borrow().type_name == "GEO" {
                                    res.referent = Some(rd.referent.clone());
                                    res.end_tok = nn.clone();
                                }
                            }
                        }
                        break;
                    }
                    break;
                } else {
                    break;
                }
            }
            return Some(res);
        }

        // Non-first: handle Number and Seria which need additional digit scanning
        match typ {
            IdTokenTyp::Number => {
                let mut value = String::new();
                let mut cur = tok.end_token.borrow().next.clone();
                // skip optional ':'
                if let Some(c) = cur.clone() {
                    if c.borrow().is_char(':', sofa) {
                        cur = c.borrow().next.clone();
                    }
                }
                while let Some(c) = cur.clone() {
                    let cb = c.borrow();
                    if cb.is_newline_before(sofa) { break; }
                    if let TokenKind::Number(ref nd) = cb.kind {
                        value.push_str(&nd.value.to_string());
                        res.end_tok = c.clone();
                        cur = cb.next.clone();
                    } else if let TokenKind::Text(ref td) = cb.kind {
                        // check for slash separator
                        if td.term == "/" { cur = cb.next.clone(); continue; }
                        break;
                    } else {
                        break;
                    }
                }
                if value.is_empty() { return None; }
                res.value = value;
                res.has_prefix = true;
                return Some(res);
            }
            IdTokenTyp::Seria => {
                let mut value = String::new();
                let mut cur = tok.end_token.borrow().next.clone();
                if let Some(c) = cur.clone() {
                    if c.borrow().is_char(':', sofa) {
                        cur = c.borrow().next.clone();
                    }
                }
                while let Some(c) = cur.clone() {
                    let cb = c.borrow();
                    if cb.is_newline_before(sofa) { break; }
                    if let TokenKind::Number(ref nd) = cb.kind {
                        if value.len() >= 4 { break; }
                        value.push_str(&nd.value.to_string());
                        res.end_tok = c.clone();
                        cur = cb.next.clone();
                    } else if let TokenKind::Text(ref td) = cb.kind {
                        if td.term.chars().count() == 2 && cb.chars.is_all_upper() {
                            value.push_str(&td.term);
                            res.end_tok = c.clone();
                            cur = cb.next.clone();
                            // skip hyphen
                            if let Some(nx) = cur.clone() {
                                if nx.borrow().is_hiphen(sofa) {
                                    cur = nx.borrow().next.clone();
                                }
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if value.len() < 2 { return None; }
                res.value = value;
                res.has_prefix = true;
                return Some(res);
            }
            IdTokenTyp::Code => {
                // scan digits after Code keyword
                let mut cur = tok.end_token.borrow().next.clone();
                while let Some(c) = cur.clone() {
                    let cb = c.borrow();
                    if cb.is_newline_before(sofa) { break; }
                    if let TokenKind::Number(_) = cb.kind {
                        res.end_tok = c.clone();
                        cur = cb.next.clone();
                    } else if cb.is_char('-', sofa) || cb.is_char(':', sofa) {
                        cur = cb.next.clone();
                    } else {
                        break;
                    }
                }
                return Some(res);
            }
            IdTokenTyp::Address => {
                // Look for an AddressReferent token
                let mut cur = tok.end_token.borrow().next.clone();
                while let Some(c) = cur.clone() {
                    let cb = c.borrow();
                    if cb.is_newline_before(sofa) { break; }
                    if let TokenKind::Referent(ref rd) = cb.kind {
                        if rd.referent.borrow().type_name == "ADDRESS" {
                            res.referent = Some(rd.referent.clone());
                            res.end_tok = c.clone();
                        }
                        break;
                    }
                    cur = cb.next.clone();
                }
                if res.referent.is_none() { return None; }
                return Some(res);
            }
            _ => { return Some(res); }
        }
    }

    // ── Non-first token: try referent tokens ────────────────────────────────
    if !is_first {
        let tb = t.borrow();
        if let TokenKind::Referent(ref rd) = tb.kind {
            let rtype = rd.referent.borrow().type_name.clone();
            let typ = match rtype.as_str() {
                "DATE"         => IdTokenTyp::Date,
                "ORGANIZATION" => IdTokenTyp::Org,
                "ADDRESS"      => IdTokenTyp::Address,
                _ => return None,
            };
            return Some(IdItem {
                typ,
                value: String::new(),
                end_tok: t.clone(),
                referent: Some(rd.referent.clone()),
                has_prefix: false,
            });
        }
        drop(tb);

        // ── "ОТ" + DATE referent ────────────────────────────────────────────
        {
            let tb = t.borrow();
            if let TokenKind::Text(ref td) = tb.kind {
                if td.term == "ОТ" || td.term == "ВІД" {
                    let next = tb.next.clone();
                    drop(tb);
                    if let Some(nn) = next {
                        let nb = nn.borrow();
                        if let TokenKind::Referent(ref rd) = nb.kind {
                            if rd.referent.borrow().type_name == "DATE" {
                                return Some(IdItem {
                                    typ: IdTokenTyp::Date,
                                    value: String::new(),
                                    end_tok: nn.clone(),
                                    referent: Some(rd.referent.clone()),
                                    has_prefix: false,
                                });
                            }
                        }
                    }
                    return None;
                }
            } else {
                drop(tb);
            }
        }

        // ── Bare number token ────────────────────────────────────────────────
        {
            let tb = t.borrow();
            if let TokenKind::Number(ref nd) = tb.kind {
                let mut value = nd.value.to_string();
                let mut end = t.clone();
                let mut cur = tb.next.clone();
                drop(tb);
                // Collect consecutive number tokens on same line
                while let Some(c) = cur {
                    let cb = c.borrow();
                    if cb.is_newline_before(sofa) { break; }
                    if let TokenKind::Number(ref nd2) = cb.kind {
                        value.push_str(&nd2.value.to_string());
                        end = c.clone();
                        cur = cb.next.clone();
                    } else {
                        break;
                    }
                }
                if value.len() < 4 { return None; }
                return Some(IdItem {
                    typ: IdTokenTyp::Number,
                    value,
                    end_tok: end,
                    referent: None,
                    has_prefix: false,
                });
            }
        }

        // ── Bare uppercase 2-char text (seria without prefix) ───────────────
        {
            let (term_opt, next_opt) = {
                let tb = t.borrow();
                if let TokenKind::Text(ref td) = tb.kind {
                    if td.term.chars().count() == 2 && tb.chars.is_all_upper() {
                        (Some(td.term.clone()), tb.next.clone())
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(term), Some(next)) = (term_opt, next_opt) {
                if let TokenKind::Number(_) = next.borrow().kind {
                    return Some(IdItem {
                        typ: IdTokenTyp::Seria,
                        value: term,
                        end_tok: t.clone(),
                        referent: None,
                        has_prefix: false,
                    });
                }
            }
        }
    }

    None
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Try to attach an identity document referent starting at token `t`.
/// Returns `Some((referent, begin_tok, end_tok))` on success.
pub fn try_attach(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef, TokenRef)> {
    let tb = t.borrow();
    if !tb.chars.is_letter() { return None; }
    drop(tb);

    // First token must be a Keyword
    let noun = try_parse_one(t, true, sofa)?;
    if noun.typ != IdTokenTyp::Keyword { return None; }

    let begin_tok = t.clone();
    let doc_type = noun.value.clone();
    let mut doc_state = noun.referent.clone();
    let mut end_tok = noun.end_tok.clone();

    // Collect subsequent elements
    let mut elements: Vec<IdItem> = Vec::new();
    let mut cur = end_tok.borrow().next.clone();
    while let Some(c) = cur.clone() {
        let cb = c.borrow();
        // Stop at table control / long newline gap
        if cb.is_newline_before(sofa) { break; }
        let c_str = matches!(cb.kind, TokenKind::Text(_)) && {
            if let TokenKind::Text(ref td) = cb.kind {
                td.term == "," || td.term == ":" || td.term == ";"
            } else { false }
        };
        drop(cb);
        if c_str {
            cur = c.borrow().next.clone();
            continue;
        }
        if c.borrow().is_char('-', sofa) {
            cur = c.borrow().next.clone();
            continue;
        }

        match try_parse_one(&c, false, sofa) {
            None => break,
            Some(item) => {
                if item.typ == IdTokenTyp::Keyword { break; }
                end_tok = item.end_tok.clone();
                cur = end_tok.borrow().next.clone();
                elements.push(item);
            }
        }
    }

    if elements.is_empty() { return None; }

    // ── Determine number from series of elements ─────────────────────────────
    // Patterns: [Seria, Number], [Number], [Number, Number(prefix)]
    let mut i = 0usize;
    let num_str: Option<String>;

    if i < elements.len() && elements[i].typ == IdTokenTyp::Number {
        if elements.len() > i + 1
            && elements[i + 1].typ == IdTokenTyp::Number
            && elements[i + 1].has_prefix
        {
            // Seria in first slot + Number in second
            num_str = Some(format!("{}{}", elements[i].value, elements[i + 1].value));
            i += 2;
        } else {
            num_str = Some(elements[i].value.clone());
            i += 1;
        }
    } else if i < elements.len() && elements[i].typ == IdTokenTyp::Seria {
        if elements.len() > i + 1 && elements[i + 1].typ == IdTokenTyp::Number {
            num_str = Some(format!("{}{}", elements[i].value, elements[i + 1].value));
            i += 2;
        } else if elements[i].value.len() > 5 {
            num_str = Some(elements[i].value.clone());
            i += 1;
        } else {
            return None;
        }
    } else if i < elements.len() && elements[i].typ == IdTokenTyp::Org {
        // Org first, then Number
        if elements.len() > i + 1 && elements[i + 1].typ == IdTokenTyp::Number {
            num_str = Some(elements[i + 1].value.clone());
            // don't advance i — org at [0] will be processed below
        } else {
            return None;
        }
    } else {
        return None;
    }

    let number = num_str?;

    // ── Build the referent ───────────────────────────────────────────────────
    let mut pid = pir::new_person_identity_referent();
    pir::set_type(&mut pid, &doc_type.to_lowercase());
    pir::set_number(&mut pid, &number);
    if let Some(state_ref) = doc_state {
        pid.slots.push(crate::referent::Slot::new(
            pir::ATTR_STATE,
            Some(crate::referent::SlotValue::Referent(state_ref)),
        ));
    }

    // Process remaining elements
    while i < elements.len() {
        let el = &elements[i];
        i += 1;
        match el.typ {
            IdTokenTyp::Vidan | IdTokenTyp::Code => {} // skip
            IdTokenTyp::Date => {
                if let Some(ref r) = el.referent {
                    if pid.find_slot(pir::ATTR_DATE, None).is_none() {
                        pid.slots.push(crate::referent::Slot::new(
                            pir::ATTR_DATE,
                            Some(crate::referent::SlotValue::Referent(r.clone())),
                        ));
                    }
                }
            }
            IdTokenTyp::Org => {
                if let Some(ref r) = el.referent {
                    if pid.find_slot(pir::ATTR_ORG, None).is_none() {
                        pid.slots.push(crate::referent::Slot::new(
                            pir::ATTR_ORG,
                            Some(crate::referent::SlotValue::Referent(r.clone())),
                        ));
                    }
                }
            }
            IdTokenTyp::Address => {
                if let Some(ref r) = el.referent {
                    if pid.find_slot(pir::ATTR_ADDRESS, None).is_none() {
                        pid.slots.push(crate::referent::Slot::new(
                            pir::ATTR_ADDRESS,
                            Some(crate::referent::SlotValue::Referent(r.clone())),
                        ));
                    }
                }
            }
            _ => { break; }
        }
    }

    Some((pid, begin_tok, end_tok))
}
