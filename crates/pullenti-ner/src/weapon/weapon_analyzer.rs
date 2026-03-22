/// Weapon analyzer — ports WeaponAnalyzer.cs (first pass only).

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::{Referent, SlotValue};
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;

use super::weapon_referent as wr;
use super::weapon_item_token::{WeaponItemToken, try_parse_list};
use super::weapon_table::WeaponItemTyp;

pub struct WeaponAnalyzer;
impl WeaponAnalyzer { pub fn new() -> Self { WeaponAnalyzer } }

impl Analyzer for WeaponAnalyzer {
    fn name(&self) -> &'static str { "WEAPON" }
    fn caption(&self) -> &'static str { "Оружие" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();

        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            if let Some(its) = try_parse_list(&t, &sofa) {
                if let Some((referent, end)) = try_attach(&its, &sofa) {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let begin = its[0].begin.clone();
                    let tok = Rc::new(RefCell::new(Token::new_referent(begin, end, r_rc)));
                    kit.embed_token(tok.clone());
                    cur = tok.borrow().next.clone();
                    continue;
                }
            }

            cur = t.borrow().next.clone();
        }
    }
}

// ── try_attach ────────────────────────────────────────────────────────────────

fn try_attach(its: &[WeaponItemToken], sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let mut r = wr::new_weapon_referent();
    let mut t1: Option<TokenRef> = None;
    let mut noun: Option<&WeaponItemToken> = None;
    let mut brand: Option<&WeaponItemToken> = None;
    let mut model: Option<&WeaponItemToken> = None;

    for item in its {
        match item.typ {
            WeaponItemTyp::Noun => {
                // Single noun alone → no weapon
                if its.len() == 1 { return None; }
                // If we already have a different type set, stop
                if r.find_slot(wr::ATTR_TYPE, None).is_some() {
                    if r.find_slot(wr::ATTR_TYPE, Some(&item.value)).is_none() {
                        break;
                    }
                }
                if !item.is_internal { noun = Some(item); }
                r.add_slot(wr::ATTR_TYPE, SlotValue::Str(item.value.clone()), false);
                if let Some(ref alt) = item.alt_value {
                    r.add_slot(wr::ATTR_TYPE, SlotValue::Str(alt.clone()), false);
                }
                t1 = Some(item.end.clone());
            }
            WeaponItemTyp::Brand => {
                if r.find_slot(wr::ATTR_BRAND, None).is_some() {
                    if r.find_slot(wr::ATTR_BRAND, Some(&item.value)).is_none() {
                        break;
                    }
                }
                if !item.is_internal {
                    if let Some(n) = noun { if n.is_doubt { /* clear doubt */ } }
                }
                brand = Some(item);
                r.add_slot(wr::ATTR_BRAND, SlotValue::Str(item.value.clone()), false);
                t1 = Some(item.end.clone());
            }
            WeaponItemTyp::Model => {
                if r.find_slot(wr::ATTR_MODEL, None).is_some() {
                    if r.find_slot(wr::ATTR_MODEL, Some(&item.value)).is_none() {
                        break;
                    }
                }
                model = Some(item);
                r.add_slot(wr::ATTR_MODEL, SlotValue::Str(item.value.clone()), false);
                if let Some(ref alt) = item.alt_value {
                    r.add_slot(wr::ATTR_MODEL, SlotValue::Str(alt.clone()), false);
                }
                t1 = Some(item.end.clone());
            }
            WeaponItemTyp::Name => {
                if r.find_slot(wr::ATTR_NAME, None).is_some() { break; }
                r.add_slot(wr::ATTR_NAME, SlotValue::Str(item.value.clone()), false);
                if let Some(ref alt) = item.alt_value {
                    r.add_slot(wr::ATTR_NAME, SlotValue::Str(alt.clone()), false);
                }
                t1 = Some(item.end.clone());
            }
            WeaponItemTyp::Number => {
                if r.find_slot(wr::ATTR_NUMBER, None).is_some() { break; }
                r.add_slot(wr::ATTR_NUMBER, SlotValue::Str(item.value.clone()), false);
                t1 = Some(item.end.clone());
            }
            WeaponItemTyp::Caliber => {
                if r.find_slot(wr::ATTR_CALIBER, None).is_some() { break; }
                r.add_slot(wr::ATTR_CALIBER, SlotValue::Str(item.value.clone()), false);
                t1 = Some(item.end.clone());
            }
            _ => {}
        }
    }

    let t1 = t1?;

    // Validity check — mirrors C# TryAttach logic
    let has_noun_with_doubt_cleared = noun.map_or(false, |n| !n.is_doubt);
    let has_model = model.is_some();
    let has_brand_no_doubt = brand.map_or(false, |b| !b.is_doubt);

    if has_noun_with_doubt_cleared {
        // Good non-doubtful noun — fine as long as there's something else
        // (single-noun-alone is already rejected above with its.len()==1)
    } else if noun.is_some() {
        // Doubtful noun: requires model or non-doubtful brand
        if !has_model && !has_brand_no_doubt { return None; }
    } else {
        // No noun at all: need model
        if !has_model { return None; }
        // Check for "оружие"/"вооружение" context in preceding tokens
        let start = &its[0].begin;
        if !has_weapon_context_before(start, sofa) { return None; }
    }

    Some((r, t1))
}

fn has_weapon_context_before(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let mut cur_opt = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    let mut count = 0;
    while let Some(cur) = cur_opt {
        if count > 20 { break; }
        count += 1;
        if cur.borrow().is_value("ОРУЖИЕ", None)
            || cur.borrow().is_value("ВООРУЖЕНИЕ", None)
            || cur.borrow().is_value("ВЫСТРЕЛ", None)
            || cur.borrow().is_value("ВЫСТРЕЛИТЬ", None)
        {
            return true;
        }
        cur_opt = cur.borrow().prev.as_ref().and_then(|w| w.upgrade());
    }
    false
}
