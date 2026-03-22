/// LinkAnalyzer — detects relationships (links) between already-recognized entities.
/// Mirrors `LinkAnalyzer.cs` (simplified port, skipping PersonPropertyReferent /
/// PersonIdentityReferent which are not yet ported).
///
/// This is a *specific* analyzer (is_specific = true) — it runs after all general
/// analyzers and post-processes their output.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::referent::{Referent, SlotValue};
use crate::source_of_analysis::SourceOfAnalysis;

use super::link_referent::{self as lr, LinkType};
use crate::resume::resume_analyzer::parse_org;
use crate::resume::resume_referent::{self as rr, ResumeItemType, ATTR_REF, ATTR_DATERANGE};
use crate::geo::geo_referent;
use crate::person::person_referent;

// ── Sentinel slot ──────────────────────────────────────────────────────────

const SLOT_OBJECT: &str = "OBJECT";
const VAL_TRUE: &str = "true";

fn is_object(r: &Referent) -> bool {
    r.get_string_value(SLOT_OBJECT) == Some(VAL_TRUE)
}

fn set_object(r: &mut Referent) {
    r.add_slot(SLOT_OBJECT, SlotValue::Str(VAL_TRUE.to_string()), true);
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Create a LINK referent, register it and embed a thin wrapper token.
fn emit_link(
    kit: &mut AnalysisKit,
    tok: TokenRef,
    typ: LinkType,
    obj1: Option<Rc<RefCell<Referent>>>,
    obj2: Option<Rc<RefCell<Referent>>>,
    param: Option<&str>,
    date_from: Option<Rc<RefCell<Referent>>>,
    date_to:   Option<Rc<RefCell<Referent>>>,
) {
    let mut link = lr::new_link_referent();
    lr::set_link_type(&mut link, &typ);
    if let Some(p) = param { lr::set_param(&mut link, p); }
    if let Some(o1) = obj1  { lr::set_object1(&mut link, o1); }
    if let Some(o2) = obj2  { lr::set_object2(&mut link, o2); }
    if let Some(df) = date_from { lr::set_datefrom(&mut link, df); }
    if let Some(dt) = date_to   { lr::set_dateto(&mut link, dt); }

    let r_rc = Rc::new(RefCell::new(link));
    let r_rc = kit.add_entity(r_rc);
    let wrap = Rc::new(RefCell::new(Token::new_referent(tok.clone(), tok, r_rc)));
    kit.embed_token(wrap);
}

/// Return the referent inside a ReferentToken, or None.
fn get_ref(t: &TokenRef) -> Option<Rc<RefCell<Referent>>> {
    t.borrow().get_referent()
}

/// Look backward for a word that starts with one of the given prefixes.
/// Stops at newlines or long text tokens (within `max` steps).
fn prev_term_starts_with(t: &TokenRef, prefixes: &[&str], max: usize, sofa: &SourceOfAnalysis) -> bool {
    let mut cur = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    let mut steps = 0;
    while let Some(tt) = cur {
        if steps >= max { break; }
        steps += 1;
        let is_nl_after = tt.borrow().is_newline_after(sofa);
        if is_nl_after { break; }
        let found = {
            let b = tt.borrow();
            if let TokenKind::Text(td) = &b.kind {
                let term = td.term.as_str();
                prefixes.iter().any(|p| term.starts_with(p))
            } else if let TokenKind::Referent(rd) = &b.kind {
                let rtn = rd.referent.borrow().type_name.clone();
                if rtn != "DATE" && rtn != "GEO" { break; }
                false
            } else {
                false
            }
        };
        if found { return true; }
        // If it's a long text token, stop searching
        {
            let b = tt.borrow();
            if let TokenKind::Text(_) = &b.kind {
                if b.length_char() > 1 { break; }
            }
        }
        cur = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
    }
    false
}

/// Look forward for a word that starts with one of the given prefixes.
fn next_term_starts_with(t: &TokenRef, prefixes: &[&str], sofa: &SourceOfAnalysis) -> bool {
    if let Some(nx) = t.borrow().next.clone() {
        let b = nx.borrow();
        if b.is_newline_before(sofa) { return false; }
        if let TokenKind::Text(td) = &b.kind {
            let term = td.term.as_str();
            return prefixes.iter().any(|p| term.starts_with(p));
        }
    }
    false
}

// ── Main analyzer ──────────────────────────────────────────────────────────

pub struct LinkAnalyzer;

impl LinkAnalyzer {
    pub fn new() -> Self { LinkAnalyzer }
}
impl Default for LinkAnalyzer { fn default() -> Self { LinkAnalyzer } }

impl Analyzer for LinkAnalyzer {
    fn name(&self)        -> &'static str { "LINK" }
    fn caption(&self)     -> &'static str { "Связи" }
    fn is_specific(&self) -> bool         { true }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();

        // ── Scan and build links ───────────────────────────────────────────
        let mut main_pers: Option<Rc<RefCell<Referent>>> = None;
        let mut cur_pers:  Option<Rc<RefCell<Referent>>> = None;
        let mut cur_org:   Option<Rc<RefCell<Referent>>> = None;
        let mut cur_typ = ResumeItemType::Undefined;

        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            let next = t.borrow().next.clone();
            let nl = t.borrow().is_newline_before(&sofa);

            // ── keyword tracking ──────────────────────────────────────────
            let term_opt = if let TokenKind::Text(td) = &t.borrow().kind { Some(td.term.clone()) } else { None };
            if let Some(term) = term_opt {
                let term = term.as_str();
                if term == "ОБРАЗОВАНИЕ" {
                    cur_typ = ResumeItemType::Study;
                } else if term == "РАБОТАТЬ" {
                    cur_typ = ResumeItemType::Organization;
                } else if term == "ОПЫТ" || term == "МЕСТО" || term == "ПЕРИОД" {
                    let is_raboty = next.as_ref().map_or(false, |n| {
                        if let TokenKind::Text(td2) = &n.borrow().kind { td2.term == "РАБОТЫ" } else { false }
                    });
                    if is_raboty { cur_typ = ResumeItemType::Organization; }
                }
            }

            // ── at newline: try parse_org ─────────────────────────────────
            if nl && main_pers.is_some() {
                if let Some(rt) = parse_org(&t, cur_typ, &sofa) {
                    // Extract the resume referent and parse its fields
                    let resume_ref = {
                        let rb = rt.borrow();
                        if let TokenKind::Referent(rd) = &rb.kind {
                            Some(rd.referent.clone())
                        } else { None }
                    };
                    if let Some(rref) = resume_ref {
                        let new_typ = rr::get_typ(&rref.borrow());
                        cur_typ = new_typ;

                        let org_ref: Option<Rc<RefCell<Referent>>> = rref.borrow().slots.iter()
                            .find(|s| s.type_name == ATTR_REF)
                            .and_then(|s| s.value.as_ref())
                            .and_then(|v| v.as_referent());
                        let date_range: Option<Rc<RefCell<Referent>>> = rref.borrow().slots.iter()
                            .find(|s| s.type_name == ATTR_DATERANGE)
                            .and_then(|s| s.value.as_ref())
                            .and_then(|v| v.as_referent());
                        let value_str = rr::get_value(&rref.borrow()).map(|s| s.to_string());

                        if let Some(org) = org_ref {
                            set_object(&mut org.borrow_mut());
                            let (df, dt) = extract_date_range(date_range.as_ref());
                            let p_ref = main_pers.clone();
                            let link_typ = if cur_typ == ResumeItemType::Study {
                                LinkType::Study
                            } else {
                                LinkType::Work
                            };
                            cur_org = Some(org.clone());

                            let span = t.clone();
                            let mut link = lr::new_link_referent();
                            lr::set_link_type(&mut link, &link_typ);
                            if let Some(ref p) = p_ref { lr::set_object1(&mut link, p.clone()); }
                            lr::set_object2(&mut link, org.clone());
                            if let Some(ref s) = value_str { lr::set_param(&mut link, s); }
                            if let Some(df2) = df { lr::set_datefrom(&mut link, df2); }
                            if let Some(dt2) = dt { lr::set_dateto(&mut link, dt2); }
                            let r_rc = Rc::new(RefCell::new(link));
                            let r_rc = kit.add_entity(r_rc);
                            let wrap = Rc::new(RefCell::new(Token::new_referent(span.clone(), span, r_rc)));
                            kit.embed_token(wrap);
                        }
                        kit.embed_token(rt.clone());
                        cur = rt.borrow().next.clone();
                        continue;
                    }
                }
            }

            // ── referent token processing ─────────────────────────────────
            let rref = match get_ref(&t) {
                Some(r) => r,
                None => { cur = next; continue; }
            };

            let rtype = rref.borrow().type_name.clone();

            match rtype.as_str() {
                "PERSON" => {
                    cur_org = None;
                    let p = rref.clone();
                    if is_object(&p.borrow()) {
                        cur_pers = Some(p);
                        cur = next; continue;
                    }
                    if main_pers.is_none() || nl {
                        // Become main person
                        set_object(&mut p.borrow_mut());
                        main_pers = Some(p.clone());
                        cur_pers = None;
                    } else {
                        let is_same_main = main_pers.as_ref().map_or(false, |mp| Rc::ptr_eq(mp, &p));
                        let is_same_cur  = cur_pers.as_ref().map_or(false, |cp| Rc::ptr_eq(cp, &p));
                        if is_same_main || is_same_cur {
                            cur = next; continue;
                        }
                        let has_first = person_referent::get_firstname(&p.borrow())
                            .map_or(false, |s| s.chars().count() >= 2);
                        if has_first {
                            set_object(&mut p.borrow_mut());
                            let mp = main_pers.clone();
                            emit_link(kit, t.clone(), LinkType::Undefined, mp, Some(p.clone()), None, None, None);
                        }
                        cur_pers = Some(p);
                    }
                }

                "GEO" => {
                    let is_city = geo_referent::is_city(&rref.borrow());
                    let p = cur_pers.clone().or_else(|| main_pers.clone());
                    if is_city && p.is_some() {
                        let birth_prefixes: &[&str] = &["УР", "РОДИ", "РОЖД"];
                        if prev_term_starts_with(&t, birth_prefixes, 4, &sofa) {
                            set_object(&mut rref.borrow_mut());
                            emit_link(kit, t.clone(), LinkType::Born, p, Some(rref.clone()), None, None, None);
                        }
                    }
                }

                "DATE" => {
                    let p = cur_pers.clone().or_else(|| main_pers.clone());
                    if let Some(p_ref) = p.clone() {
                        let has_born = p_ref.borrow().slots.iter().any(|s| s.type_name == "BORN");
                        if !has_born {
                            let birth_prefixes: &[&str] = &["УРОЖ", "РОДИ", "РОЖД"];
                            if prev_term_starts_with(&t, birth_prefixes, 4, &sofa)
                                || next_term_starts_with(&t, birth_prefixes, &sofa)
                            {
                                p_ref.borrow_mut().add_slot(
                                    "BORN",
                                    SlotValue::Referent(rref.clone()),
                                    false,
                                );
                            }
                        }
                    }
                }

                "PHONE" => {
                    let obj1 = cur_org.clone().or_else(|| cur_pers.clone()).or_else(|| main_pers.clone());
                    if obj1.is_some() {
                        set_object(&mut rref.borrow_mut());
                        emit_link(kit, t.clone(), LinkType::Contact, obj1, Some(rref.clone()), None, None, None);
                    }
                }

                "URI" => {
                    let scheme = rref.borrow().get_string_value("SCHEME").map(|s| s.to_string()).unwrap_or_default();
                    if scheme == "http" || scheme == "https" || scheme == "www" {
                        cur = next; continue;
                    }
                    let obj1 = cur_org.clone().or_else(|| cur_pers.clone()).or_else(|| main_pers.clone());
                    if obj1.is_some() {
                        set_object(&mut rref.borrow_mut());
                        emit_link(kit, t.clone(), LinkType::Contact, obj1, Some(rref.clone()), None, None, None);
                    }
                }

                "ADDRESS" | "STREET" => {
                    let obj1 = cur_org.clone().or_else(|| cur_pers.clone()).or_else(|| main_pers.clone());
                    if obj1.is_some() {
                        set_object(&mut rref.borrow_mut());
                        let is_org = obj1.as_ref().map_or(false, |o| o.borrow().type_name == "ORGANIZATION");
                        let param = detect_addr_param(&t, is_org, &sofa);
                        emit_link(kit, t.clone(), LinkType::Address, obj1, Some(rref.clone()), param.as_deref(), None, None);
                    }
                }

                "ORGANIZATION" => {
                    let p = cur_pers.clone().or_else(|| main_pers.clone());
                    cur_org = Some(rref.clone());
                    set_object(&mut rref.borrow_mut());
                    if p.is_some() {
                        let link_typ = if cur_typ == ResumeItemType::Study {
                            LinkType::Study
                        } else if cur_typ == ResumeItemType::Organization {
                            LinkType::Work
                        } else {
                            let study_prefixes: &[&str] = &["ОБРАЗОВАН", "ОКОНЧИТ", "ОБУЧАТ", "ЗАКОНЧИТ"];
                            let work_prefixes: &[&str]  = &["РАБОТАТ", "ПРАКТИК"];
                            if prev_term_starts_with(&t, study_prefixes, 5, &sofa) {
                                LinkType::Study
                            } else if prev_term_starts_with(&t, work_prefixes, 5, &sofa) {
                                LinkType::Work
                            } else {
                                LinkType::Undefined
                            }
                        };
                        let (df, dt) = find_adjacent_date(&t, &sofa);
                        if link_typ != LinkType::Undefined || df.is_some() || dt.is_some() {
                            emit_link(kit, t.clone(), link_typ, p, Some(rref.clone()), None, df, dt);
                        }
                    }
                }

                "RESUME" => {
                    let org_ref: Option<Rc<RefCell<Referent>>> = rref.borrow().slots.iter()
                        .find(|s| s.type_name == ATTR_REF)
                        .and_then(|s| s.value.as_ref())
                        .and_then(|v| v.as_referent())
                        .filter(|r| r.borrow().type_name == "ORGANIZATION");
                    let date_range: Option<Rc<RefCell<Referent>>> = rref.borrow().slots.iter()
                        .find(|s| s.type_name == ATTR_DATERANGE)
                        .and_then(|s| s.value.as_ref())
                        .and_then(|v| v.as_referent());
                    let typ = rr::get_typ(&rref.borrow());
                    let value_str = rr::get_value(&rref.borrow()).map(|s| s.to_string());
                    let p = cur_pers.clone().or_else(|| main_pers.clone());

                    if let (Some(p_ref), Some(org)) = (p, org_ref) {
                        set_object(&mut org.borrow_mut());
                        let link_typ = if typ == ResumeItemType::Study { LinkType::Study } else { LinkType::Work };
                        let (df, dt) = extract_date_range(date_range.as_ref());
                        cur_org = Some(org.clone());
                        emit_link(kit, t.clone(), link_typ, Some(p_ref), Some(org), value_str.as_deref(), df, dt);
                    }
                }

                "WEAPON" | "TRANSPORT" => {
                    let p = cur_pers.clone().or_else(|| main_pers.clone());
                    if p.is_some() {
                        set_object(&mut rref.borrow_mut());
                        emit_link(kit, t.clone(), LinkType::Undefined, p, Some(rref.clone()), None, None, None);
                    }
                }

                _ => {}
            }

            cur = next;
        }
    }
}

// ── Date extraction helpers ────────────────────────────────────────────────

/// Extract DateFrom/DateTo from a DateRange or plain Date referent.
fn extract_date_range(
    dr_ref: Option<&Rc<RefCell<Referent>>>,
) -> (Option<Rc<RefCell<Referent>>>, Option<Rc<RefCell<Referent>>>) {
    let dr = match dr_ref { Some(r) => r, None => return (None, None) };
    let rtype = dr.borrow().type_name.clone();
    if rtype == "DATERANGE" {
        let df = dr.borrow().slots.iter()
            .find(|s| s.type_name == "DATEFROM")
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_referent());
        let dt = dr.borrow().slots.iter()
            .find(|s| s.type_name == "DATETO")
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_referent());
        (df, dt)
    } else if rtype == "DATE" {
        (Some(dr.clone()), Some(dr.clone()))
    } else {
        (None, None)
    }
}

/// Look forward (within 5 tokens, no newlines) for a Date/DateRange referent.
fn find_adjacent_date(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> (Option<Rc<RefCell<Referent>>>, Option<Rc<RefCell<Referent>>>) {
    let mut cur = t.borrow().next.clone();
    let mut steps = 0;
    while let Some(tt) = cur.clone() {
        if steps >= 5 { break; }
        steps += 1;
        if tt.borrow().is_newline_before(sofa) { break; }
        if let Some(r) = tt.borrow().get_referent() {
            let rn = r.borrow().type_name.clone();
            if rn == "DATERANGE" || rn == "DATE" {
                return extract_date_range(Some(&r));
            }
        }
        cur = tt.borrow().next.clone();
    }
    (None, None)
}

// ── Address parameter detection ────────────────────────────────────────────

fn detect_addr_param(t: &TokenRef, is_org: bool, sofa: &SourceOfAnalysis) -> Option<String> {
    // Look backward a few tokens
    let mut prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    let mut steps = 0;
    while let Some(tt) = prev {
        if steps >= 5 { break; }
        steps += 1;
        if tt.borrow().is_newline_before(sofa) { break; }
        if tt.borrow().is_comma(sofa) { break; }
        if let Some(p) = check_addr_param(&tt, is_org) { return Some(p); }
        prev = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
    }
    None
}

fn check_addr_param(t: &TokenRef, is_org: bool) -> Option<String> {
    if let TokenKind::Text(td) = &t.borrow().kind {
        let term = td.term.as_str();
        if term == "АДРЕС" { return None; }
        if is_org {
            if term.starts_with("ЮР") { return Some("юридический".to_string()); }
            if term.starts_with("ФАКТ") { return Some("фактический".to_string()); }
        } else {
            if term.starts_with("ЗАРЕГ") || term.starts_with("РЕГИСТР") || term.starts_with("ПРОПИС") {
                return Some("регистрация".to_string());
            }
            if term.starts_with("ФАКТИЧ") || term.starts_with("ПРОЖИВ") {
                return Some("проживание".to_string());
            }
        }
    }
    None
}
