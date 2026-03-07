use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::referent::{Referent, SlotValue};
use crate::source_of_analysis::SourceOfAnalysis;

use super::phone_kind::PhoneKind;
use super::phone_item_token::{
    PhoneItemToken, PhoneItemType,
    try_attach_all, try_attach_alternate, try_attach_additional,
};
use super::phone_referent::{self as ph_ref, OBJ_TYPENAME};
use super::phone_helper;

/// Result of a successful phone number parse (before embedding)
struct PhoneMatch {
    referent: Rc<RefCell<Referent>>,
    begin: TokenRef,
    end: TokenRef,
}

/// Deduplication store for phone referents within one document
struct PhoneAnalyzerData {
    phones_hash: HashMap<String, Vec<Rc<RefCell<Referent>>>>,
}

impl PhoneAnalyzerData {
    fn new() -> Self {
        PhoneAnalyzerData { phones_hash: HashMap::new() }
    }

    fn register_referent(&mut self, referent: Rc<RefCell<Referent>>) -> Rc<RefCell<Referent>> {
        let key = {
            let rb = referent.borrow();
            let num = ph_ref::get_number(&rb).unwrap_or_default();
            if num.len() >= 10 { num[3..].to_string() } else { num }
        };
        let entry = self.phones_hash.entry(key).or_default();
        for existing in entry.iter() {
            if ph_ref::can_be_equals(&existing.borrow(), &referent.borrow()) {
                ph_ref::merge_slots(&mut existing.borrow_mut(), &referent.borrow());
                return existing.clone();
            }
        }
        entry.push(referent.clone());
        referent
    }
}

/// Phone number analyzer
pub struct PhoneAnalyzer;

impl Analyzer for PhoneAnalyzer {
    fn name(&self) -> &'static str { "PHONE" }
    fn caption(&self) -> &'static str { "Телефоны" }
    fn progress_weight(&self) -> i32 { 2 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut ad = PhoneAnalyzerData::new();

        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            // Advance cursor regardless (will be overridden on match)
            cur = t.borrow().next.clone();

            if t.borrow().is_ignored(&sofa) { continue; }

            let pli_opt = try_attach_all(&t, &sofa, 15);
            let pli = match pli_opt {
                None => continue,
                Some(p) if p.is_empty() => continue,
                Some(p) => p,
            };

            // Check previous context for ГОСТ/ОСТ (not phone numbers)
            let mut prev_phone: Option<Rc<RefCell<Referent>>> = None;
            let mut not_phone = false;
            let mut kkk = 0i32;
            {
                let mut tt_cur: Option<TokenRef> = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                loop {
                    let tt = match tt_cur.take() { None => break, Some(x) => x };

                    // Extract all flags from the borrowed token
                    let ref_r = tt.borrow().get_referent();
                    if let Some(r) = ref_r {
                        if r.borrow().type_name == OBJ_TYPENAME {
                            prev_phone = Some(r);
                            break;
                        }
                        // Other ReferentToken: advance to prev
                        tt_cur = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        continue;
                    }

                    let is_close_paren = tt.borrow().is_char(')', &sofa);
                    if is_close_paren {
                        let start = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        let mut count = 0;
                        let mut inner = start;
                        let mut found_prev: Option<TokenRef> = None;
                        while let Some(inner_tok) = inner.take() {
                            count += 1;
                            if count > 100 { break; }
                            if inner_tok.borrow().is_char('(', &sofa) {
                                found_prev = inner_tok.borrow().prev.as_ref().and_then(|w| w.upgrade());
                                break;
                            }
                            inner = inner_tok.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        }
                        if found_prev.is_none() && count > 0 { break; }
                        tt_cur = found_prev;
                        continue;
                    }

                    let is_gost = tt.borrow().is_value("ГОСТ", None) || tt.borrow().is_value("ОСТ", None);
                    if is_gost {
                        not_phone = true;
                        break;
                    }

                    let is_separator = tt.borrow().is_char_of(",;/\\", &sofa) || tt.borrow().is_and(&sofa);
                    if !is_separator {
                        kkk += 1;
                        if kkk > 5 { break; }
                        let nl = tt.borrow().is_newline_before(&sofa) || tt.borrow().is_newline_after(&sofa);
                        if nl { break; }
                    }

                    tt_cur = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                }
            }

            if not_phone {
                cur = pli.last().unwrap().end.borrow().next.clone();
                continue;
            }

            // Extract prefix kinds from pli
            let mut j = 0usize;
            let mut pli = pli;
            let mut is_phone_before = prev_phone.is_some();
            let mut is_pref = false;
            let mut ki = PhoneKind::Undefined;
            let mut ki2 = PhoneKind::Undefined;

            while j < pli.len() {
                if pli[j].item_type == PhoneItemType::Prefix {
                    if ki == PhoneKind::Undefined { ki = pli[j].kind; }
                    is_pref = true;
                    if ki2 == PhoneKind::Undefined { ki2 = pli[j].kind2; }
                    is_phone_before = true;
                    j += 1;
                    if j < pli.len() && pli[j].item_type == PhoneItemType::Delim { j += 1; }
                } else if j == 0 && pli.len() > 1 && pli[1].item_type == PhoneItemType::Prefix {
                    if ki == PhoneKind::Undefined { ki = pli[0].kind; }
                    is_pref = true;
                    if ki2 == PhoneKind::Undefined { ki2 = pli[0].kind2; }
                    pli.remove(0);
                } else {
                    break;
                }
            }

            // Skip single 6-char number after URI scheme
            if pli.len() == 1 && pli[0].item_type == PhoneItemType::Number {
                if pli[0].length_char() == 6 {
                    let t_prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    if t_prev.map_or(false, |p| matches!(p.borrow().kind, TokenKind::Text(_))) {
                        continue;
                    }
                }
            }

            let rts_opt = self.try_attach_list(&pli, j, is_phone_before, prev_phone.as_ref().map(|r| r.borrow()).as_deref(), &sofa);

            let rts = match rts_opt {
                None => {
                    // Try searching for prefix inside
                    let mut found = None;
                    for jj in 1..pli.len() {
                        if pli[jj].item_type == PhoneItemType::Prefix {
                            let sub = pli[jj..].to_vec();
                            found = self.try_attach_list(&sub, 1, true, prev_phone.as_ref().map(|r| r.borrow()).as_deref(), &sofa);
                            break;
                        }
                    }
                    match found {
                        None => {
                            cur = pli.last().unwrap().end.borrow().next.clone();
                            continue;
                        }
                        Some(f) => f,
                    }
                }
                Some(r) => r,
            };

            // Apply kind to each matched phone referent
            for (idx, m) in rts.iter().enumerate() {
                let mut rb = m.referent.borrow_mut();
                if ki2 != PhoneKind::Undefined {
                    if idx == 0 {
                        ph_ref::set_tag(&mut rb, ki2);
                    } else {
                        ph_ref::set_kind(&mut rb, ki2);
                    }
                } else if ki != PhoneKind::Undefined {
                    ph_ref::set_kind(&mut rb, ki);
                } else {
                    // Check for T/Ф/M letter before first match
                    if idx == 0 && m.begin.borrow().whitespaces_before_count(&sofa) < 3 {
                        let prev_t = m.begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        if let Some(pt) = prev_t {
                            let ptb = pt.borrow();
                            if ptb.is_newline_before(&sofa)
                                || ptb.prev.as_ref().and_then(|w| w.upgrade())
                                    .map_or(false, |pp| pp.borrow().is_table_control_char(&sofa))
                            {
                                let term = match &ptb.kind {
                                    TokenKind::Text(td) => Some(td.term.clone()),
                                    _ => None,
                                };
                                if let Some(term) = term {
                                    match term.as_str() {
                                        "Ф" | "F" => {
                                            drop(ptb);
                                            ph_ref::set_kind(&mut rb, PhoneKind::Fax);
                                        }
                                        "M" | "М" => {
                                            drop(ptb);
                                            ph_ref::set_kind(&mut rb, PhoneKind::Mobile);
                                        }
                                        _ => { drop(ptb); }
                                    }
                                }
                            }
                        }
                    }
                    ph_ref::correct(&mut rb);
                }
                drop(rb);
            }

            // Register and embed
            let last_end = rts.last().unwrap().end.clone();
            for m in rts {
                let new_ref = m.referent.clone();
                let registered = ad.register_referent(new_ref.clone());
                // Add to kit entities only when the referent is first registered
                if Rc::ptr_eq(&registered, &new_ref) {
                    kit.add_entity(registered.clone());
                }
                let tok = Rc::new(RefCell::new(
                    Token::new_referent(m.begin.clone(), m.end.clone(), registered)
                ));
                kit.embed_token(tok.clone());
                cur = tok.borrow().next.clone();
            }
            if cur.is_none() {
                cur = last_end.borrow().next.clone();
            }
        }
    }
}

impl PhoneAnalyzer {
    pub fn new() -> Self { PhoneAnalyzer }

    fn try_attach_list(
        &self,
        pli: &[PhoneItemToken],
        ind: usize,
        is_phone_before: bool,
        prev_phone: Option<&Referent>,
        sofa: &SourceOfAnalysis,
    ) -> Option<Vec<PhoneMatch>> {
        let mut pli = pli.to_vec();
        let rt = self._try_attach_(&mut pli, ind, is_phone_before, prev_phone, 0, sofa)?;

        let mut res = Vec::new();
        res.push(rt);

        // Try alternate digit variants
        for _ in 0..5 {
            let last = res.last().unwrap();
            if ph_ref::get_add_number(&last.referent.borrow()).is_some() { break; }

            let ph0_template = {
                let rb = last.referent.borrow();
                ph_ref::get_template(&rb).map(|s| s.to_string())
            };
            let alt_opt = try_attach_alternate(
                last.end.borrow().next.clone(),
                ph0_template.as_deref(),
                &pli,
                sofa,
            );
            let alt = match alt_opt { None => break, Some(a) => a };

            // Clone slots and modify number
            let new_ref = ph_ref::new_phone_referent();
            {
                let last_rb = last.referent.borrow();
                let mut new_rb = new_ref.borrow_mut();
                for slot in &last_rb.slots {
                    new_rb.add_slot(slot.type_name.clone(), slot.value.clone().unwrap_or(SlotValue::Str(String::new())), false);
                }
                let num = ph_ref::get_number(&last_rb).unwrap_or_default();
                if !num.is_empty() && num.len() > alt.value.len() {
                    let new_num = format!("{}{}", &num[..num.len() - alt.value.len()], &alt.value);
                    ph_ref::set_number(&mut new_rb, &new_num);
                }
                if let Some(tmpl) = ph_ref::get_template(&last_rb) {
                    ph_ref::set_template(&mut new_rb, tmpl);
                }
            }

            let alt_match = PhoneMatch {
                referent: new_ref,
                begin: alt.begin.clone(),
                end: alt.end.clone(),
            };
            res.push(alt_match);
        }

        // Try to attach additional number after last match
        let last_end = res.last().unwrap().end.clone();
        if let Some(add) = try_attach_additional(last_end.borrow().next.clone(), sofa) {
            for m in &res {
                ph_ref::set_add_number(&mut m.referent.borrow_mut(), &add.value);
            }
            res.last_mut().unwrap().end = add.end.clone();
        }

        Some(res)
    }

    /// Core phone number parsing from a list of item tokens
    fn _try_attach_(
        &self,
        pli: &mut Vec<PhoneItemToken>,
        ind: usize,
        is_phone_before: bool,
        prev_phone: Option<&Referent>,
        lev: i32,
        sofa: &SourceOfAnalysis,
    ) -> Option<PhoneMatch> {
        if ind >= pli.len() || lev > 4 { return None; }

        let mut country_code: Option<String> = None;
        let mut city_code: Option<String> = None;
        let mut j = ind;

        // Check if prev_phone has a template and current starts with Number
        if let Some(pp) = prev_phone {
            let tmpl = ph_ref::get_template(pp);
            if tmpl.is_some() && pli[j].item_type == PhoneItemType::Number {
                let tmpl = tmpl.unwrap().to_string();
                let mut tmp = String::new();
                let mut jj = j;
                while jj < pli.len() {
                    match pli[jj].item_type {
                        PhoneItemType::Number => {
                            tmp.push_str(&pli[jj].value.len().to_string());
                        }
                        PhoneItemType::Delim => {
                            if pli[jj].value == " " { break; }
                            tmp.push_str(&pli[jj].value);
                            jj += 1;
                            continue;
                        }
                        _ => break,
                    }
                    let templ0 = tmp.clone();
                    if templ0 == tmpl {
                        if jj + 1 < pli.len() {
                            if !(pli[jj + 1].item_type == PhoneItemType::Prefix && jj + 2 == pli.len()) {
                                pli.drain(jj + 1..);
                            }
                        }
                        break;
                    }
                    jj += 1;
                }
                // Reject common false patterns
                let vv = tmp.clone();
                if vv == "4-2-2" || vv == "2-2-4" { return None; }
            }
        }

        // CountryCode item
        if j < pli.len() && pli[j].item_type == PhoneItemType::CountryCode {
            country_code = Some(pli[j].value.clone());
            let cc = country_code.as_ref().unwrap();
            if cc != "8" {
                if let Some(prefix) = phone_helper::get_country_prefix(cc) {
                    if prefix.len() < cc.len() {
                        city_code = Some(cc[prefix.len()..].to_string());
                        country_code = Some(prefix);
                    }
                }
            }
            j += 1;
        } else if j < pli.len() && pli[j].can_be_country_prefix(sofa) {
            let k = if j + 1 < pli.len() && pli[j + 1].item_type == PhoneItemType::Delim { j + 2 } else { j + 1 };
            let sub_rt = self._try_attach_(pli, k, is_phone_before, None, lev + 1, sofa);
            if sub_rt.is_some() {
                let is_false_pattern = j + 1 < pli.len()
                    && pli[j + 1].item_type == PhoneItemType::Delim
                    && pli[j + 1].begin.borrow().is_hiphen(sofa)
                    && pli[j].item_type == PhoneItemType::Number
                    && pli[j].value.len() == 3
                    && j + 2 < pli.len()
                    && pli[j + 2].item_type == PhoneItemType::Number
                    && pli[j + 2].value.len() == 3
                    && is_phone_before;
                if !is_false_pattern {
                    country_code = Some(pli[j].value.clone());
                    j += 1;
                }
            }
        }

        // 8 or 7 leading digit as country code
        if country_code.is_none() && j < pli.len() && pli[j].item_type == PhoneItemType::Number {
            let first = pli[j].value.chars().next().unwrap_or('0');
            if first == '8' || first == '7' {
                let len = pli[j].value.len();
                if len == 1 {
                    country_code = Some(pli[j].value.clone());
                    j += 1;
                } else if len == 4 {
                    country_code = Some(pli[j].value[..1].to_string());
                    let rest = pli[j].value[1..].to_string();
                    city_code = Some(city_code.map_or(rest.clone(), |cc| cc + &rest));
                    j += 1;
                } else if len == 11 && j == pli.len() - 1 && is_phone_before {
                    let ph = ph_ref::new_phone_referent();
                    {
                        let mut rb = ph.borrow_mut();
                        if first != '8' {
                            ph_ref::set_country_code(&mut rb, &pli[j].value[..1]);
                        }
                        ph_ref::set_number(&mut rb, &format!("{}{}", &pli[j].value[1..4], &pli[j].value[4..]));
                    }
                    return Some(PhoneMatch {
                        referent: ph,
                        begin: pli[0].begin.clone(),
                        end: pli[j].end.clone(),
                    });
                } else if city_code.is_none() && len > 3 && j + 1 < pli.len() {
                    let sum: i32 = pli.iter()
                        .filter(|it| it.item_type == PhoneItemType::Number)
                        .map(|it| it.value.len() as i32)
                        .sum();
                    if sum == 11 {
                        city_code = Some(pli[j].value[1..].to_string());
                        j += 1;
                    }
                }
            }
        }

        // CityCode item
        if j < pli.len() && pli[j].item_type == PhoneItemType::CityCode {
            let val = pli[j].value.clone();
            city_code = Some(city_code.map_or(val.clone(), |cc| cc + &val));
            j += 1;
        }

        // Skip delimiter
        if j < pli.len() && pli[j].item_type == PhoneItemType::Delim { j += 1; }

        // Pull city code from "8 + 3/4 digit" when country code is "8"
        if country_code.as_deref() == Some("8") && city_code.is_none() && j + 3 < pli.len()
            && pli[j].item_type == PhoneItemType::Number
        {
            let len = pli[j].value.len();
            if len == 3 || len == 4 {
                city_code = Some(pli[j].value.clone());
                j += 1;
                if j < pli.len() && pli[j].item_type == PhoneItemType::Delim { j += 1; }
            }
        }

        let normal_num_len: usize = if country_code.as_deref() == Some("421") { 9 } else { 0 };
        let mut num = String::new();
        let mut templ = String::new();
        let mut part_length: Vec<usize> = Vec::new();
        let mut delim: Option<String> = None;
        let mut ok = false;
        let mut additional: Option<String> = None;
        let mut is_std = false;

        // Try standard format: cc-NNN-NN-NN or similar
        if country_code.is_some() && j + 4 < pli.len() && j > 0 {
            let prev_is_cc_or_dash =
                pli[j - 1].value == "-"
                || pli[j - 1].item_type == PhoneItemType::CountryCode;
            if prev_is_cc_or_dash
                && pli[j].item_type == PhoneItemType::Number
                && pli[j + 1].item_type == PhoneItemType::Delim
                && pli[j + 2].item_type == PhoneItemType::Number
                && pli[j + 3].item_type == PhoneItemType::Delim
                && pli[j + 4].item_type == PhoneItemType::Number
            {
                let l0 = pli[j].value.len();
                let l2 = pli[j + 2].value.len();
                let l4 = pli[j + 4].value.len();
                if (l0 + l2 == 6 || (l0 == 4 && l2 == 5)) && (l4 == 4 || l4 == 1) {
                    num.push_str(&pli[j].value);
                    num.push_str(&pli[j + 2].value);
                    num.push_str(&pli[j + 4].value);
                    templ = format!("{}{}{}{}{}", l0, pli[j + 1].value, l2, pli[j + 3].value, l4);
                    is_std = true;
                    ok = true;
                    j += 5;
                }
            }
        }

        // Main loop: collect number parts
        while j < pli.len() && !is_std {
            match pli[j].item_type {
                PhoneItemType::Delim => {
                    if pli[j].is_in_brackets { j += 1; continue; }
                    if j > 0 && pli[j - 1].is_in_brackets { j += 1; continue; }
                    if !templ.is_empty() { templ.push_str(&pli[j].value); }
                    if delim.is_none() {
                        delim = Some(pli[j].value.clone());
                    } else if pli[j].value != *delim.as_ref().unwrap() {
                        // Different delimiter — try to split city code
                        if part_length.len() == 2
                            && (part_length[0] == 3 || part_length[0] == 4)
                            && city_code.is_none()
                            && part_length[1] == 3
                        {
                            city_code = Some(num[..part_length[0]].to_string());
                            num = num[part_length[0]..].to_string();
                            part_length.remove(0);
                            delim = Some(pli[j].value.clone());
                            j += 1;
                            continue;
                        }
                        if is_phone_before && j + 1 < pli.len() && pli[j + 1].item_type == PhoneItemType::Number {
                            if num.len() < 6 {
                                j += 1;
                                continue;
                            }
                            if normal_num_len > 0 && num.len() + pli[j + 1].value.len() == normal_num_len {
                                j += 1;
                                continue;
                            }
                        }
                        break;
                    } else {
                        j += 1;
                        continue;
                    }
                    ok = false;
                    j += 1;
                }
                PhoneItemType::Number => {
                    if num.is_empty() {
                        // Check: if previous token is table control and followed by another number → not phone
                        let begin_prev = pli[j].begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        if begin_prev.map_or(false, |p| p.borrow().is_table_control_char(sofa)) {
                            let last_end = pli.last().unwrap().end.clone();
                            let last_next = last_end.borrow().next.clone();
                            if let Some(ln) = last_next {
                                let lnb = ln.borrow();
                                let is_comma = lnb.is_char_of(",.", sofa);
                                let nxt = if is_comma { lnb.next.clone() } else { Some(ln.clone()) };
                                drop(lnb);
                                if let Some(nn) = nxt {
                                    if nn.borrow().is_number_token() { return None; }
                                }
                            }
                        }
                    }
                    if num.len() + pli[j].value.len() > 13 {
                        if j > 0 && pli[j - 1].item_type == PhoneItemType::Delim {
                            j -= 1;
                        }
                        ok = true;
                        break;
                    }
                    num.push_str(&pli[j].value);
                    part_length.push(pli[j].value.len());
                    templ.push_str(&pli[j].value.len().to_string());
                    ok = true;
                    if num.len() > 10 {
                        j += 1;
                        if j < pli.len() && pli[j].item_type == PhoneItemType::AddNumber {
                            additional = Some(pli[j].value.clone());
                            j += 1;
                        }
                        break;
                    }
                    j += 1;
                }
                PhoneItemType::AddNumber => {
                    additional = Some(pli[j].value.clone());
                    j += 1;
                    break;
                }
                _ => break,
            }
        }

        // Last item in-brackets with 3-4 digits → additional
        if j == pli.len().saturating_sub(1)
            && pli[j].is_in_brackets
            && (pli[j].value.len() == 3 || pli[j].value.len() == 4)
            && additional.is_none()
        {
            additional = Some(pli[j].value.clone());
            j += 1;
        }

        // Skip trailing in-brackets prefix
        if j < pli.len() && pli[j].item_type == PhoneItemType::Prefix && pli[j].is_in_brackets {
            j += 1;
        }

        // Try extracting country code from oversized city code
        if country_code.is_none() {
            if let Some(ref cc) = city_code {
                if cc.len() > 3 && num.len() < 8 && !cc.starts_with('8') {
                    if (cc.len() + num.len()) != 10 {
                        if let Some(prefix) = phone_helper::get_country_prefix(cc) {
                            if prefix.len() > 1 && (cc.len() - prefix.len()) > 1 {
                                let rest = cc[prefix.len()..].to_string();
                                country_code = Some(prefix);
                                city_code = Some(rest);
                            }
                        }
                    }
                }
            }
        }

        // "00" prefix → international code
        if country_code.is_none() {
            if let Some(ref cc) = city_code {
                if cc.starts_with("00") {
                    if let Some(prefix) = phone_helper::get_country_prefix(&cc[2..]) {
                        if cc.len() > prefix.len() + 3 {
                            let new_cc = prefix.clone();
                            let new_city = cc[prefix.len() + 2..].to_string();
                            country_code = Some(new_cc);
                            city_code = Some(new_city);
                        }
                    }
                }
            }
        }

        // Promote city code to number if city code alone has 10 digits
        if num.is_empty() {
            if let Some(ref cc) = city_code {
                if cc.len() == 10 {
                    num = cc[3..].to_string();
                    part_length.push(num.len());
                    city_code = Some(cc[..3].to_string());
                    ok = true;
                } else if (cc.len() == 9 || cc.len() == 11 || cc.len() == 8)
                    && (is_phone_before || country_code.is_some())
                {
                    num = cc.clone();
                    part_length.push(num.len());
                    city_code = None;
                    ok = true;
                }
            }
        }

        if num.len() < 4 { ok = false; }

        // Validate number length
        if num.len() < 7 {
            if let Some(ref cc) = city_code {
                if cc.len() + num.len() > 7 {
                    if !is_phone_before && cc.len() == 3 {
                        let all_3 = part_length.iter().all(|&l| l == 3)
                            || part_length.iter().enumerate().all(|(i, &l)| l == 3 || (i == part_length.len() - 1 && l >= 2));
                        if !all_3 || country_code.as_deref() != Some("61") {
                            ok = false;
                        }
                    }
                } else if (num.len() == 6 || num.len() == 5)
                    && part_length.len() >= 1
                    && part_length.len() <= 3
                    && is_phone_before
                {
                    if pli[0].item_type == PhoneItemType::Prefix && pli[0].kind == PhoneKind::Home {
                        ok = false;
                    } else if part_length.len() == 1 && num.len() < 7 && pli[0].kind != PhoneKind::Undefined && pli[0].length_char() < 3 {
                        ok = false;
                    }
                } else if let Some(pp) = prev_phone {
                    let pp_num = ph_ref::get_number(pp).unwrap_or_default();
                    let pp_len = pp_num.len();
                    if pp_len == num.len() || pp_len == num.len() + 3 || pp_len == num.len() + 4 {
                        // ok remains
                    } else if num.len() > 4 && ph_ref::get_template(pp).map_or(false, |t| t == templ) {
                        ok = true;
                    } else {
                        ok = false;
                    }
                } else {
                    ok = false;
                }
            }
        }

        if delim.as_deref() == Some(".") && country_code.is_none() && city_code.is_none() {
            ok = false;
        }

        // Try to pull country code from long num
        if is_phone_before && country_code.is_none() && city_code.is_none() && num.len() > 10 {
            if let Some(prefix) = phone_helper::get_country_prefix(&num) {
                if num.len() - prefix.len() == 9 {
                    let rest = num[prefix.len()..].to_string();
                    country_code = Some(prefix);
                    num = rest;
                    ok = true;
                }
            }
        }

        if ok {
            // Structural validation
            let is_valid_pattern = part_length.len() == 3 && part_length[0] == 3 && part_length[1] == 2 && part_length[2] == 2
                || part_length.len() == 3 && is_phone_before
                || part_length.len() == 4 && (part_length[0] + part_length[1] == 3) && part_length[2] == 2 && part_length[3] == 2
                || part_length.len() == 4 && part_length[0] == 3 && part_length[1] == 3 && part_length[2] == 2 && part_length[3] == 2
                || part_length.len() == 5 && (part_length[1] + part_length[2] == 4) && (part_length[3] + part_length[4] == 4)
                || is_std;

            let has_context = is_phone_before || city_code.is_some() || country_code.is_some() || additional.is_some();

            if !is_valid_pattern {
                if part_length.len() > 4 {
                    ok = false;
                } else if part_length.len() > 3 && city_code.is_some() {
                    ok = false;
                } else if has_context {
                    // ok
                } else if let Some(pp) = prev_phone {
                    let pp_num = ph_ref::get_number(pp).unwrap_or_default();
                    let pp_len = pp_num.len();
                    if !(pp_len == num.len() || pp_len == num.len() + 3 || pp_len == num.len() + 4
                        || ph_ref::get_template(pp).map_or(false, |t| t == templ))
                    {
                        ok = false;
                    }
                } else {
                    ok = false;
                    // Lookahead: check next phone
                    if (num.len() == 6 || num.len() == 7) && part_length.len() < 4 && j > 0 {
                        let after = pli.get(j.saturating_sub(1)).map(|it| it.end.borrow().next.clone()).flatten();
                        if let Some(next_ph) = self.get_next_phone(after, lev + 1, sofa) {
                            let next_num = ph_ref::get_number(&next_ph.borrow()).unwrap_or_default();
                            let d = next_num.len() as i32 - num.len() as i32;
                            if d == 0 || d == 3 || d == 4 { ok = true; }
                        }
                    }
                }
            }
        }

        // Find end token
        let end_tok = if j > 0 { pli[j - 1].end.clone() } else { return None; };
        if !ok { return None; }

        // Reject "4 3.4" or similar IP-address-like patterns
        let stempl = templ.trim().to_string();
        if (stempl == "4 3.4" || stempl == "2.4" || stempl == "3.4") {
            if pli[0].length_char() == 4 {
                let begin_prev = pli[0].begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
                if let Some(bp) = begin_prev {
                    if bp.borrow().is_char('.', sofa)
                        && bp.borrow().prev.as_ref().and_then(|w| w.upgrade())
                            .map_or(false, |pp| pp.borrow().is_number_token())
                    {
                        return None;
                    }
                }
            }
        }

        // Strip trailing non-digit from template
        if !templ.is_empty() && !templ.chars().last().unwrap().is_ascii_digit() {
            templ.pop();
        }

        // Try to extract country code from oversized city code (post-validation)
        if country_code.is_none() {
            if let Some(ref cc) = city_code {
                if cc.len() > 3 && num.len() > 6 {
                    if let Some(prefix) = phone_helper::get_country_prefix(cc) {
                        if prefix.len() + 1 < cc.len() {
                            let rest = cc[prefix.len()..].to_string();
                            country_code = Some(prefix);
                            city_code = Some(rest);
                        }
                    }
                }
            }
        }

        // Reject ГОСТ/ТУ patterns
        {
            let begin_prev = pli[0].begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
            if let Some(bp) = begin_prev {
                if bp.borrow().is_value("ГОСТ", None) || bp.borrow().is_value("ТУ", None) {
                    return None;
                }
            }
        }

        // Build the final number string
        let mut number = num.clone();
        if city_code.is_none() && number.len() > 7 && !part_length.is_empty() && part_length[0] < 5 {
            let cou = part_length[0];
            city_code = Some(number[..cou].to_string());
            number = number[cou..].to_string();
        }
        if city_code.is_none() && number.len() == 11 && number.starts_with('8') {
            city_code = Some(number[1..4].to_string());
            number = number[4..].to_string();
        }
        if city_code.is_none() && number.len() == 10 {
            city_code = Some(number[..3].to_string());
            number = number[3..].to_string();
        }
        if let Some(ref cc) = city_code {
            number = format!("{}{}", cc, number);
        } else if country_code.is_none() {
            if let Some(pp) = prev_phone {
                let pp_num = ph_ref::get_number(pp).unwrap_or_default();
                let ok1 = pp_num.len() >= number.len() + 2
                    || (templ.len() > 0 && ph_ref::get_template(pp).map_or(false, |t| t.ends_with(&templ)));
                if ok1 && pp_num.len() > number.len() {
                    number = format!("{}{}", &pp_num[..pp_num.len() - number.len()], number);
                }
            }
        }

        // Copy country code from prev phone if lengths match
        if country_code.is_none() {
            if let Some(pp) = prev_phone {
                if let Some(pp_cc) = ph_ref::get_country_code(pp) {
                    if ph_ref::get_number(pp).map_or(0, |n| n.len()) == number.len() {
                        country_code = Some(pp_cc);
                    }
                }
            }
        }

        // Validate: at least one non-zero digit
        if !number.chars().any(|c| c != '0') { return None; }

        // Validate length
        if let Some(ref cc) = country_code {
            if number.len() < 7 { return None; }
            if cc == "7" && number.len() != 10 { return None; }
        } else {
            // Try to strip embedded country code
            if let Some(prefix) = phone_helper::get_country_prefix(&number) {
                let rest = &number[prefix.len()..];
                if rest.len() >= 10 && rest.len() <= 11 {
                    number = rest.to_string();
                    if prefix != "7" {
                        country_code = Some(prefix);
                    }
                }
            }
            if number.len() == 8 && prev_phone.is_none() { return None; }
        }

        if number.len() > 11 {
            let is_long_ok = matches!(country_code.as_deref(), Some("1") | Some("43")) && number.len() < 14;
            if !is_long_ok { return None; }
        }

        let ph = ph_ref::new_phone_referent();
        {
            let mut rb = ph.borrow_mut();
            if let Some(ref cc) = country_code {
                ph_ref::set_country_code(&mut rb, cc);
            }
            ph_ref::set_number(&mut rb, &number);
            if let Some(ref add) = additional {
                ph_ref::set_add_number(&mut rb, add);
            }
            ph_ref::set_template(&mut rb, &templ);
        }

        // Check for invalid trailing chars
        let end_next = end_tok.borrow().next.clone();
        if !is_phone_before {
            if let Some(ref en) = end_next {
                let enb = en.borrow();
                if !end_tok.borrow().is_newline_after(sofa) {
                    if enb.is_char_of("+=", sofa) { return None; }
                    if enb.is_hiphen(sofa) {
                        if enb.next.as_ref().map_or(false, |n| n.borrow().is_number_token()) {
                            return None;
                        }
                    }
                }
            }
        }

        // Extend end if last item is Prefix
        let end_final = if j < pli.len() && pli[j].item_type == PhoneItemType::Prefix && !pli[j].is_newline_before(sofa) {
            if pli[j].kind != PhoneKind::Undefined {
                ph_ref::set_kind(&mut ph.borrow_mut(), pli[j].kind);
            }
            if pli[j].kind2 == PhoneKind::Fax {
                ph_ref::set_tag(&mut ph.borrow_mut(), PhoneKind::Fax);
            }
            pli[j].end.clone()
        } else {
            end_tok.clone()
        };

        let mut begin = pli[0].begin.clone();

        // Adjust begin if first item is Prefix followed by table control char
        if pli[0].item_type == PhoneItemType::Prefix {
            if let Some(p0_next) = pli[0].end.borrow().next.clone() {
                if p0_next.borrow().is_table_control_char(sofa) && pli.len() > 1 {
                    begin = pli[1].begin.clone();
                }
            }
        }

        // Extend end if ending with ')'  and beginning with '('
        let end_final = {
            if let Some(en) = end_final.borrow().next.clone() {
                if en.borrow().is_char(')', sofa) && begin.borrow().is_char('(', sofa) {
                    en.clone()
                } else {
                    end_final.clone()
                }
            } else {
                end_final.clone()
            }
        };

        Some(PhoneMatch { referent: ph, begin, end: end_final })
    }

    fn get_next_phone(&self, t_opt: Option<TokenRef>, lev: i32, sofa: &SourceOfAnalysis) -> Option<Rc<RefCell<Referent>>> {
        if lev > 3 { return None; }
        let t_init = t_opt?;
        let t = if t_init.borrow().is_char(',', sofa) {
            let next = t_init.borrow().next.clone()?;
            next
        } else {
            t_init
        };
        let mut its = try_attach_all(&t, sofa, 15)?;
        let rt = self._try_attach_(&mut its, 0, false, None, lev + 1, sofa)?;
        Some(rt.referent)
    }
}
