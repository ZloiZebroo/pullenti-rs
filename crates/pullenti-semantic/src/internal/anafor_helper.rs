/// AnaforHelper — anaphora resolution for pronouns and "который"-nouns.
/// Mirrors `AnaforHelper.cs`.

use std::rc::Rc;
use pullenti_morph::{MorphGenderFlags, MorphNumber};
use crate::sem_graph::{SemGraph, SemObject, SemObjectRef, SemLinkRef};
use crate::types::{SemObjectType, SemLinkType};

pub fn process_anafors(objs: &[SemObjectRef]) {
    // Iterate in reverse: pronouns look back for antecedents
    for i in (0..objs.len()).rev() {
        let it_rc = objs[i].clone();
        {
            let it = it_rc.borrow();
            let is_personal = it.typ == SemObjectType::PersonalPronoun;
            let is_kotory   = it.normal_full == "КОТОРЫЙ"
                && it.links_from.iter().all(|l| l.borrow().typ != SemLinkType::Anafor);
            if !is_personal && !is_kotory {
                continue;
            }
        }

        let mut vars: Vec<AnaforLink> = Vec::new();
        for j in (0..i).rev() {
            if let Some(a) = AnaforLink::try_create(&it_rc, &objs[j]) {
                vars.push(a);
            }
        }

        if vars.is_empty() { continue; }

        // Sort descending by coef
        vars.sort_by(|a, b| b.coef.partial_cmp(&a.coef).unwrap_or(std::cmp::Ordering::Equal));

        // Apply corrections
        for v in &mut vars {
            v.correct();
        }
        // Re-sort after corrections
        vars.sort_by(|a, b| b.coef.partial_cmp(&a.coef).unwrap_or(std::cmp::Ordering::Equal));

        if vars[0].coef <= 0.1 { continue; }

        // Find the graph for it_rc (need to add links through its graph)
        // We'll look through each SemObject's links_from; but actually we need to
        // add a link to it_rc's graph. Since SemObject doesn't hold its graph ref,
        // we use a helper: inject the link via global add.
        // In Rust port: add link directly to the SemObject's links_from/links_to
        // (bypassing SemGraph's link list, since we don't have the graph ref here).
        // Instead, collect the graph via the existing links.
        // If it_rc has any existing links, use that graph; otherwise, skip.
        // NOTE: This is a simplification — the C# version has direct graph access.
        // We add anafor links by finding the SemGraph via obj.links_from/links_to.
        // For now, directly manipulate the Rc<RefCell<SemLink>> chain.

        if let Some(ref target_list) = vars[0].target_list {
            for tgt in target_list {
                add_anafor_link_direct(&it_rc, tgt);
            }
        } else {
            let li = add_anafor_link_direct(&it_rc, &vars[0].target);
            if vars.len() > 1
                && vars[0].coef <= vars[1].coef * 2.0
                && vars[1].target_list.is_none()
            {
                let li2 = add_anafor_link_direct(&it_rc, &vars[1].target);
                if let (Some(li), Some(li2)) = (li, li2) {
                    li2.borrow_mut().alt_link = Some(li.clone());
                    li.borrow_mut().alt_link = Some(li2);
                }
            }
        }
    }
}

/// Add an Anafor SemLink directly (without a SemGraph).
/// Returns the created link.
fn add_anafor_link_direct(src: &SemObjectRef, tgt: &SemObjectRef) -> Option<SemLinkRef> {
    // Dedup check
    for li in &src.borrow().links_from {
        let lb = li.borrow();
        if lb.typ == SemLinkType::Anafor && Rc::ptr_eq(&lb.target, tgt) {
            return Some(li.clone());
        }
    }
    use crate::sem_graph::SemLink;
    let link = SemLink::new(SemLinkType::Anafor, src.clone(), tgt.clone());
    Some(link)
}

// ── AnaforLink ────────────────────────────────────────────────────────────

struct AnaforLink {
    coef:        f64,
    target:      SemObjectRef,
    target_list: Option<Vec<SemObjectRef>>,
}

impl AnaforLink {
    fn try_create(src: &SemObjectRef, tgt: &SemObjectRef) -> Option<Self> {
        if tgt.borrow().typ != SemObjectType::Noun { return None; }

        let src_number = src.borrow().number;
        let tgt_number = tgt.borrow().number;
        let src_gender = src.borrow().gender;
        let tgt_gender = tgt.borrow().gender;

        if src_number == MorphNumber::PLURAL || (src_number.0 & MorphNumber::PLURAL.0) != 0 {
            // Plural pronoun
            if tgt_number != MorphNumber::UNDEFINED
                && (tgt_number.0 & MorphNumber::PLURAL.0) != 0
            {
                return Some(AnaforLink { coef: 1.0, target: tgt.clone(), target_list: None });
            }
            // Try to find a list via linksTo
            let mut res = AnaforLink { coef: 0.5, target: tgt.clone(), target_list: Some(Vec::new()) };
            let links_to: Vec<SemLinkRef> = tgt.borrow().links_to.clone();
            for li in &links_to {
                let frm = li.borrow().source.clone();
                let frm_links_from = frm.borrow().links_from.clone();
                for (k, li0) in frm_links_from.iter().enumerate() {
                    let li0_tgt = li0.borrow().target.clone();
                    if li0_tgt.borrow().typ != SemObjectType::Noun { continue; }
                    let li0_typ = li0.borrow().typ;
                    let li0_prep = li0.borrow().preposition.clone();
                    let mut candidates = vec![li0_tgt.clone()];
                    for li1 in frm_links_from.iter().skip(k + 1) {
                        let lb1 = li1.borrow();
                        if lb1.typ == li0_typ
                            && lb1.preposition == li0_prep
                            && lb1.target.borrow().typ == SemObjectType::Noun
                        {
                            candidates.push(lb1.target.clone());
                        }
                    }
                    if candidates.len() > 1 {
                        res.target_list = Some(candidates);
                        return Some(res);
                    }
                }
            }
            return None;
        }

        // Singular pronoun
        if tgt_number != MorphNumber::UNDEFINED
            && (tgt_number.0 & MorphNumber::SINGULAR.0) == 0
        {
            return None;
        }

        if tgt_gender != MorphGenderFlags::UNDEFINED {
            if (tgt_gender.0 & src_gender.0) == 0 { return None; }
            return Some(AnaforLink { coef: 1.0, target: tgt.clone(), target_list: None });
        }

        Some(AnaforLink { coef: 0.1, target: tgt.clone(), target_list: None })
    }

    fn correct(&mut self) {
        let links_to: Vec<SemLinkRef> = self.target.borrow().links_to.clone();
        for li in &links_to {
            let typ = li.borrow().typ;
            let prep = li.borrow().preposition.clone();
            match typ {
                SemLinkType::Naming  => self.coef = 0.0,
                SemLinkType::Agent   => self.coef *= 2.0,
                SemLinkType::Pacient => {
                    if li.borrow().alt_link.is_none() { self.coef *= 2.0; }
                }
                _ => {
                    if prep.is_some() { self.coef /= 2.0; }
                }
            }
        }
    }
}
