/// Bank data analyzer — ports BankAnalyzer.cs.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::{Referent, SlotValue};
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::bank::bank_referent as br;
use crate::uri::uri_referent as ur;

pub struct BankAnalyzer;

impl BankAnalyzer {
    pub fn new() -> Self { BankAnalyzer }
}

// ── Keyword detection ─────────────────────────────────────────────────────────

fn try_keyword(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(TokenRef, TokenRef)> {
    // Two-word: БАНКОВСКИЕ/ПЛАТЕЖНЫЕ РЕКВИЗИТЫ
    let is_prefix = t.borrow().is_value("БАНКОВСКИЕ", None)
        || t.borrow().is_value("ПЛАТЕЖНЫЕ", None);
    if is_prefix {
        let n1 = t.borrow().next.clone()?;
        if n1.borrow().is_value("РЕКВИЗИТЫ", None) {
            let kw_end = n1.clone();
            let after = kw_end.borrow().next.clone();
            let start = skip_colon(after, sofa)?;
            return Some((kw_end, start));
        }
    }
    // Single-word: РЕКВИЗИТЫ
    if t.borrow().is_value("РЕКВИЗИТЫ", None) {
        let kw_end = t.clone();
        let after = kw_end.borrow().next.clone();
        let start = skip_colon(after, sofa)?;
        return Some((kw_end, start));
    }
    None
}

fn skip_colon(t: Option<TokenRef>, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let tok = t?;
    if tok.borrow().is_char(':', sofa) {
        tok.borrow().next.clone()
    } else {
        Some(tok)
    }
}

// ── Analyzer impl ─────────────────────────────────────────────────────────────

impl Analyzer for BankAnalyzer {
    fn name(&self) -> &'static str { "BANKDATA" }
    fn caption(&self) -> &'static str { "Банковские данные" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();

        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                let next = t.borrow().next.clone();
                cur = next;
                continue;
            }

            let mut kw_begin: Option<TokenRef> = None;
            let mut attach_start: Option<TokenRef> = None;

            // Pattern A: keyword
            if t.borrow().chars.is_letter() {
                if let Some((_kw_end, start)) = try_keyword(&t, &sofa) {
                    kw_begin = Some(t.clone());
                    attach_start = Some(start);
                }
            }

            // Pattern B: ReferentToken or newline-start
            if attach_start.is_none() {
                let trigger = matches!(&t.borrow().kind, TokenKind::Referent(_))
                    || t.borrow().is_newline_before(&sofa);
                if trigger {
                    attach_start = Some(t.clone());
                }
            }

            if let Some(start) = attach_start {
                let keyword_mode = kw_begin.is_some();
                if let Some((referent, end)) = try_attach(&start, keyword_mode, &sofa) {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let begin = kw_begin.unwrap_or_else(|| start.clone());
                    let tok = Rc::new(RefCell::new(Token::new_referent(begin, end, r_rc)));
                    kit.embed_token(tok.clone());
                    let next = tok.borrow().next.clone();
                    cur = next;
                    continue;
                }
            }

            let next = t.borrow().next.clone();
            cur = next;
        }
    }
}

// ── try_attach ────────────────────────────────────────────────────────────────

macro_rules! advance {
    ($t:ident) => {{
        let _next = $t.borrow().next.clone();
        match _next { None => break, Some(x) => { $t = x; continue; } }
    }};
}

fn try_attach(
    t_start: &TokenRef,
    keyword_mode: bool,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let mut t = t_start.clone();

    let mut uris: Vec<Rc<RefCell<Referent>>> = Vec::new();
    let mut uri_schemes: Vec<String> = Vec::new();
    let mut org: Option<Rc<RefCell<Referent>>> = None;
    let mut org_is_bank = false;
    let mut cor_org: Option<Rc<RefCell<Referent>>> = None;
    let mut t1 = t_start.clone();
    let mut empty: i32 = 0;
    let mut last_uri_scheme: Option<String> = None;

    loop {
        // Table-control char breaks (except at the start)
        if !Rc::ptr_eq(&t, t_start) && t.borrow().is_table_control_char(sofa) { break; }

        // Skip commas and slashes
        if t.borrow().is_char(',', sofa) || t.borrow().is_char_of("/\\", sofa) {
            advance!(t);
        }

        // Skip prepositions
        if matches!(&t.borrow().kind, TokenKind::Text(_)) {
            let is_prep = t.borrow().morph.items().iter()
                .any(|wf| wf.base.class.is_preposition());
            if is_prep { advance!(t); }
        }

        // Skip "ПОЛНЫЙ НАИМЕНОВАНИЕ/НАЗВАНИЕ"
        if t.borrow().is_value("ПОЛНЫЙ", None) {
            let n = t.borrow().next.clone();
            if let Some(ref n1) = n {
                if n1.borrow().is_value("НАИМЕНОВАНИЕ", None)
                    || n1.borrow().is_value("НАЗВАНИЕ", None)
                {
                    let nn = n1.borrow().next.clone();
                    t = match nn { None => break, Some(x) => x };
                    continue;
                }
            }
        }

        let ref_opt = get_referent(&t);

        if let Some(ref r_rc) = ref_opt {
            let type_name = r_rc.borrow().type_name.clone();

            if type_name == "ORGANIZATION" {
                let is_bank = is_bank_org(r_rc);
                let prev_is_v = {
                    let prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    prev.map_or(false, |p| p.borrow().is_value("В", None))
                };
                if last_uri_scheme.as_deref() == Some("К/С") && prev_is_v {
                    cor_org = Some(r_rc.clone());
                    t1 = t.clone();
                } else if org.is_none() || (!org_is_bank && is_bank) {
                    org = Some(r_rc.clone());
                    t1 = t.clone();
                    org_is_bank = is_bank;
                    if is_bank { advance!(t); }
                }
                if uris.is_empty() && !keyword_mode { return None; }
                advance!(t);
            }

            if type_name == "URI" {
                let scheme = {
                    let rb = r_rc.borrow();
                    ur::get_scheme(&rb).map(|s| s.to_string())
                };
                let scheme = match scheme { None => { advance!(t); } Some(s) => s };

                if uris.is_empty() {
                    if !br::is_bank_req_scheme(&scheme) { return None; }
                    if scheme == "ИНН" && t.borrow().is_newline_after(sofa) { return None; }
                } else {
                    if !br::is_bank_req_scheme(&scheme) { break; }
                    if uri_schemes.contains(&scheme) { break; }
                    if scheme == "ИНН" && empty > 0 { break; }
                }

                last_uri_scheme = Some(scheme.clone());
                uri_schemes.push(scheme);
                uris.push(r_rc.clone());
                t1 = t.clone();
                empty = 0;
                advance!(t);
            }

            if type_name == "GEO" || type_name == "ADDRESS" {
                empty += 1;
                advance!(t);
            }
        } else if uris.is_empty() && !keyword_mode && !org_is_bank {
            return None;
        }

        // Text token
        if matches!(&t.borrow().kind, TokenKind::Text(_)) {
            let is_name_word = t.borrow().is_value("ПОЛНЫЙ", None)
                || t.borrow().is_value("НАИМЕНОВАНИЕ", None)
                || t.borrow().is_value("НАЗВАНИЕ", None);
            if is_name_word { advance!(t); }
            if t.borrow().chars.is_letter() {
                empty += 1;
                if t.borrow().is_newline_before(sofa) && next_is_colon_newline(&t, sofa) { break; }
                if uris.is_empty() { break; }
            }
        }

        // Number at newline + non-letter next
        if matches!(&t.borrow().kind, TokenKind::Number(_)) {
            if t.borrow().is_newline_before(sofa) {
                let nxt_non_letter = {
                    let next = t.borrow().next.clone();
                    next.map_or(true, |n| !n.borrow().chars.is_letter())
                };
                if nxt_non_letter { break; }
            }
        }

        if empty > 2 { break; }
        if empty > 0 && t.borrow().is_char(':', sofa) && t.borrow().is_newline_after(sofa) { break; }

        let next = t.borrow().next.clone();
        t = match next { None => break, Some(x) => x };
    }

    if uris.is_empty() { return None; }
    if !uri_schemes.contains(&"Р/С".to_string()) && !uri_schemes.contains(&"Л/С".to_string()) {
        return None;
    }
    if uris.len() < 2 && org.is_none() { return None; }

    let mut bdr = br::new_bank_data_referent();
    for uri in uris {
        br::add_item(&mut bdr, uri);
    }
    if let Some(bank) = org {
        br::set_bank(&mut bdr, bank);
    }
    if let Some(cb) = cor_org {
        br::set_corbank(&mut bdr, cb);
    }

    Some((bdr, t1))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_referent(t: &TokenRef) -> Option<Rc<RefCell<Referent>>> {
    if let TokenKind::Referent(rd) = &t.borrow().kind {
        Some(rd.referent.clone())
    } else {
        None
    }
}

fn is_bank_org(r_rc: &Rc<RefCell<Referent>>) -> bool {
    r_rc.borrow().get_string_value("KIND")
        .map_or(false, |k| k.eq_ignore_ascii_case("bank"))
}

fn next_is_colon_newline(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let mut cur = t.borrow().next.clone();
    let mut steps = 0;
    while let Some(n) = cur {
        if steps > 8 { break; }
        if n.borrow().is_char(':', sofa) && n.borrow().is_newline_after(sofa) {
            return true;
        }
        if n.borrow().chars.is_letter() || matches!(&n.borrow().kind, TokenKind::Number(_)) {
            break;
        }
        cur = n.borrow().next.clone();
        steps += 1;
    }
    false
}
