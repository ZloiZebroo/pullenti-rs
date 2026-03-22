/// ResumeAnalyzer — detects resume/CV structure.
/// Mirrors `ResumeAnalyzer.cs` (simplified port).
///
/// The analyzer recognizes work experience/education blocks
/// structured as: [ORG_entity] [DATE/DATERANGE_entity] [optional position].
///
/// Public helpers `parse_org` and `parse_org2` are also used by LinkAnalyzer.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::referent::SlotValue;
use super::resume_referent::{
    self as rr, ResumeItemType,
    OBJ_TYPENAME, ATTR_REF, ATTR_DATERANGE,
};

pub struct ResumeAnalyzer;

impl ResumeAnalyzer {
    pub fn new() -> Self { ResumeAnalyzer }
}

impl Default for ResumeAnalyzer {
    fn default() -> Self { ResumeAnalyzer }
}

impl Analyzer for ResumeAnalyzer {
    fn name(&self)       -> &'static str { "RESUME" }
    fn caption(&self)    -> &'static str { "Резюме" }
    fn is_specific(&self) -> bool        { true }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        let mut cur_typ = ResumeItemType::Undefined;

        while let Some(t) = cur.clone() {
            let next_tok = t.borrow().next.clone();

            // Track section type by keyword at line start
            if t.borrow().is_newline_before(&sofa) {
                if t.borrow().is_value("ОБРАЗОВАНИЕ", None) {
                    cur_typ = ResumeItemType::Study;
                } else if t.borrow().is_value("ОПЫТ", None) {
                    if next_tok.as_ref().map_or(false, |n| n.borrow().is_value("РАБОТЫ", None)) {
                        cur_typ = ResumeItemType::Organization;
                    }
                }

                // Try to parse PERSON entity at start of token chain
                if let Some(r) = t.borrow().get_referent() {
                    let rtype = r.borrow().type_name.clone();
                    if rtype == "PERSON" {
                        let mut referent = rr::new_resume_referent();
                        rr::set_typ(&mut referent, ResumeItemType::Person);
                        referent.add_slot(ATTR_REF, SlotValue::Referent(r), false);
                        let r_rc = Rc::new(RefCell::new(referent));
                        let r_rc = kit.add_entity(r_rc);
                        let tok = Rc::new(RefCell::new(Token::new_referent(t.clone(), t.clone(), r_rc)));
                        kit.embed_token(tok.clone());
                        cur = tok.borrow().next.clone();
                        continue;
                    }
                }

                // Try org pattern
                if let Some(rt) = parse_org(&t, cur_typ, &sofa) {
                    let end_next = rt.borrow().next.clone();
                    // Extract the referent and type before embedding
                    let (r_rc, new_typ) = {
                        let rb = rt.borrow();
                        if let TokenKind::Referent(rd) = &rb.kind {
                            let r = rd.referent.clone();
                            let typ = rr::get_typ(&r.borrow());
                            (Some(r), typ)
                        } else {
                            (None, cur_typ)
                        }
                    };
                    if let Some(r) = r_rc {
                        cur_typ = new_typ;
                        let canonical = kit.add_entity(r);
                        if let TokenKind::Referent(rd) = &mut rt.borrow_mut().kind {
                            rd.referent = canonical;
                        }
                    }
                    kit.embed_token(rt);
                    cur = end_next;
                    continue;
                }

                // Contact info: URI mailto/telegram, Phone, Address at line start before we've seen org
                if let Some(r) = t.borrow().get_referent() {
                    let rtype = r.borrow().type_name.clone();
                    if matches!(rtype.as_str(), "URI" | "PHONE" | "ADDRESS" | "STREET") {
                        let mut referent = rr::new_resume_referent();
                        rr::set_typ(&mut referent, ResumeItemType::Contact);
                        referent.add_slot(ATTR_REF, SlotValue::Referent(r), false);
                        let r_rc = Rc::new(RefCell::new(referent));
                        let r_rc = kit.add_entity(r_rc);
                        let tok = Rc::new(RefCell::new(Token::new_referent(t.clone(), t.clone(), r_rc)));
                        kit.embed_token(tok.clone());
                        cur = tok.borrow().next.clone();
                        continue;
                    }
                }
            }

            cur = next_tok;
        }
    }
}

// ── parse_org ──────────────────────────────────────────────────────────────
//
// Looks for: [optional prefix tokens] ORG_referent [DATE/DATERANGE_referent]
// at a newline-starting position.
//
// This is the public helper used by LinkAnalyzer.

pub fn parse_org(t: &TokenRef, typ: ResumeItemType, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    // Skip leading separators (colon, hyphen, table control)
    let mut cur = t.clone();
    loop {
        let cb = cur.borrow();
        if cb.is_char_of(":-", sofa) || cb.is_table_control_char(sofa) {
            let next = cb.next.clone()?;
            drop(cb);
            cur = next;
        } else {
            break;
        }
    }

    // Check for number + "." or ")" prefix (list item) → skip
    {
        let is_num_prefix = {
            let cb = cur.borrow();
            matches!(&cb.kind, TokenKind::Number(_))
                && cb.is_newline_before(sofa)
                && cb.next.as_ref().map_or(false, |nx| nx.borrow().is_char_of(".)", sofa))
        };
        if is_num_prefix {
            let after = cur.borrow().next.clone()
                .and_then(|n| n.borrow().next.clone());
            match after {
                Some(a) => { cur = a; }
                None => return None,
            }
        }
    }

    // Case 1: Org referent immediately at this position
    let org_ref = cur.borrow().get_referent()
        .filter(|r| r.borrow().type_name == "ORGANIZATION");

    if let Some(org) = org_ref {
        let org_tok = cur.clone();
        // Advance past the org token
        let after_org = cur.borrow().next.clone()?;

        // Look for an adjacent date or date range (within 3 tokens, at most 1 newline)
        let (date_ref, end_tok) = find_date_after(&after_org, sofa)?;

        // Determine actual type
        let actual_typ = if typ == ResumeItemType::Undefined {
            // Check org profile
            let org_type = org.borrow().get_string_value("TYPE").unwrap_or("").to_string();
            if org_type.contains("обр") || org_type.contains("универ") || org_type.contains("институт") {
                ResumeItemType::Study
            } else {
                ResumeItemType::Organization
            }
        } else {
            typ
        };

        let mut referent = rr::new_resume_referent();
        rr::set_typ(&mut referent, actual_typ);
        referent.add_slot(ATTR_REF, SlotValue::Referent(org), false);
        if let Some(dr) = date_ref {
            referent.add_slot(ATTR_DATERANGE, SlotValue::Referent(dr), false);
        }
        let r_rc = Rc::new(RefCell::new(referent));
        let tok = Rc::new(RefCell::new(Token::new_referent(org_tok, end_tok, r_rc)));
        return Some(tok);
    }

    None
}

// ── parse_org2 ─────────────────────────────────────────────────────────────
//
// Looks for "ОПЫТ РАБОТЫ" section header, then parses org+date pairs.
// Returns the last embedded ReferentToken, or None if none found.

pub fn parse_org2(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    // Must start with "ОПЫТ РАБОТЫ"
    if !t.borrow().is_value("ОПЫТ", None) { return None; }
    let next = t.borrow().next.clone()?;
    if !next.borrow().is_value("РАБОТЫ", None) { return None; }

    // Skip to next newline
    let mut cur = next.borrow().next.clone()?;
    while !cur.borrow().is_newline_before(sofa) {
        let n = cur.borrow().next.clone()?;
        cur = n;
    }

    let mut res: Option<TokenRef> = None;
    loop {
        let c = cur.clone();
        {
            let cb = c.borrow();
            if let Some(r) = cb.get_referent() {
                let rtype = r.borrow().type_name.clone();
                if rtype == "ORGANIZATION" {
                    drop(cb);
                    // Look for date after org
                    let after = cur.borrow().next.clone();
                    if let Some(af) = after {
                        if let Some((date_ref, end_tok)) = find_date_after(&af, sofa) {
                            let mut referent = rr::new_resume_referent();
                            rr::set_typ(&mut referent, ResumeItemType::Organization);
                            referent.add_slot(ATTR_REF, SlotValue::Referent(r), false);
                            if let Some(dr) = date_ref {
                                referent.add_slot(ATTR_DATERANGE, SlotValue::Referent(dr), false);
                            }
                            let r_rc = Rc::new(RefCell::new(referent));
                            let tok = Rc::new(RefCell::new(
                                Token::new_referent(cur.clone(), end_tok.clone(), r_rc)
                            ));
                            res = Some(tok.clone());
                            cur = end_tok.borrow().next.clone()
                                .unwrap_or_else(|| tok.clone());
                            continue;
                        }
                    }
                }
            }
        }
        let next = c.borrow().next.clone();
        match next {
            None => break,
            Some(n) => { cur = n; }
        }
        // Stop at new sections
        if cur.borrow().is_value("ОБРАЗОВАНИЕ", None) { break; }
    }

    res
}

// ── Helper: find date referent within a few tokens ────────────────────────

fn find_date_after(start: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Option<Rc<RefCell<crate::referent::Referent>>>, TokenRef)> {
    let mut cur = start.clone();
    let mut steps = 0;
    loop {
        {
            let cb = cur.borrow();
            let rtype = cb.get_referent()
                .map(|r| r.borrow().type_name.clone())
                .unwrap_or_default();
            if rtype == "DATE" || rtype == "DATERANGE" {
                let r = cb.get_referent();
                return Some((r, cur.clone()));
            }
            // Skip short separators: "(", ")", "–", "-", ","
            if cb.length_char() <= 1 || cb.is_char_of("()-,", sofa) {
                steps += 1;
            } else if cb.chars.is_letter() {
                steps += 1;
            } else {
                return None;
            }
        }
        if steps > 5 { return None; }
        let next = cur.borrow().next.clone();
        match next {
            None => return None,
            Some(n) => { cur = n; }
        }
    }
}
