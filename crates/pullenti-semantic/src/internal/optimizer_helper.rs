/// OptimizerHelper — post-processing of the semantic graph.
/// Mirrors `OptimizerHelper.cs`.

use std::rc::Rc;
use std::collections::HashMap;
use pullenti_morph::MorphGenderFlags;
use crate::sem_document::{SemDocument, SemFragmentRef};
use crate::sem_graph::{SemGraph, SemObject, SemObjectRef, SemLinkRef};
use crate::types::{SemObjectType, SemLinkType, SemProcessParams};
use super::anafor_helper;

pub fn optimize(doc: &mut SemDocument, pars: &SemProcessParams) {
    for blk_rc in &doc.blocks {
        let mut blk = blk_rc.borrow_mut();

        // Collect all objects: block graph + all fragment graphs
        // Also build obj_ptr → fragment_index map for process_pointers/formulas
        let mut all_objs: Vec<SemObjectRef> = blk.graph.objects.clone();
        let mut obj_to_frag: HashMap<usize, usize> = HashMap::new();
        for (fi, fr_rc) in blk.fragments.iter().enumerate() {
            for o in &fr_rc.borrow().graph.objects {
                all_objs.push(o.clone());
                obj_to_frag.insert(Rc::as_ptr(o) as usize, fi);
            }
        }

        for fr_rc in &blk.fragments {
            let mut fr = fr_rc.borrow_mut();
            let gr = &mut fr.graph;

            // Sort object tokens
            optimize_graph(gr);

            // Remove dangling links (source or target not in all_objs)
            let to_remove: Vec<SemLinkRef> = gr.links.iter()
                .filter(|li| {
                    let lb = li.borrow();
                    !all_objs.iter().any(|o| Rc::ptr_eq(o, &lb.source))
                    || !all_objs.iter().any(|o| Rc::ptr_eq(o, &lb.target))
                })
                .cloned()
                .collect();
            for li in to_remove {
                gr.remove_link(&li);
            }

            process_participles(gr);
            process_links(gr);
        }

        sort_objects(&mut all_objs);
        process_pointers(&all_objs, &obj_to_frag, &blk.fragments);
        process_formulas(&all_objs, &obj_to_frag, &blk.fragments);

        if !pars.dont_create_anafor {
            anafor_helper::process_anafors(&all_objs);
            for fr_rc in &blk.fragments {
                let mut fr = fr_rc.borrow_mut();
                collapse_anafors(&mut fr.graph);
            }
        }
    }
}

fn optimize_graph(gr: &mut SemGraph) {
    sort_objects(&mut gr.objects);
}

fn sort_objects(objs: &mut Vec<SemObjectRef>) {
    objs.sort_by(|a, b| {
        a.borrow().compare_to(&b.borrow())
    });
}

fn process_participles(gr: &mut SemGraph) {
    let mut i = 0;
    while i < gr.objects.len() {
        let obj_rc = gr.objects[i].clone();
        if obj_rc.borrow().typ != SemObjectType::Participle {
            i += 1;
            continue;
        }

        // Find Participle-type own link and whether there are other links
        let own_link: Option<SemLinkRef> = {
            obj_rc.borrow().links_to.iter()
                .find(|li| li.borrow().typ == SemLinkType::Participle)
                .cloned()
        };
        let has_other = obj_rc.borrow().links_to.iter()
            .any(|li| li.borrow().typ != SemLinkType::Participle);

        if !has_other {
            i += 1;
            continue;
        }

        // If no own Participle link, create a dummy noun object for it
        let own_link = if let Some(li) = own_link {
            li
        } else {
            let mut dum = SemObject::new();
            dum.typ = SemObjectType::Noun;
            // Copy morph from the participle object
            {
                let ob = obj_rc.borrow();
                dum.gender = ob.gender;
                dum.number = ob.number;
            }
            let dum_ref = gr.add_object(dum);
            let new_link = gr.add_link(
                SemLinkType::Participle,
                dum_ref,
                obj_rc.clone(),
                Some("какой".to_string()),
                false,
                None,
            );
            new_link.unwrap()
        };

        // Re-route non-Participle links from participle → own_link.source
        let own_target = own_link.borrow().source.clone();
        let non_participle_links: Vec<SemLinkRef> = obj_rc.borrow().links_to.iter()
            .filter(|li| li.borrow().typ != SemLinkType::Participle)
            .cloned()
            .collect();

        for li in non_participle_links {
            let (link_src, link_typ, link_q, link_is_or, link_prep) = {
                let lb = li.borrow();
                (lb.source.clone(), lb.typ, lb.question.clone(), lb.is_or, lb.preposition.clone())
            };
            // Check if already exists
            let exists = link_src.borrow().links_from.iter().any(|ll| {
                Rc::ptr_eq(&ll.borrow().target, &own_target)
            });
            if exists {
                gr.remove_link(&li);
            } else {
                // Re-target: remove old link, add new link to own_target
                gr.remove_link(&li);
                gr.add_link(link_typ, link_src, own_target.clone(), link_q, link_is_or, link_prep);
            }
        }

        i += 1;
    }
}

fn process_links(gr: &mut SemGraph) {
    let objs: Vec<SemObjectRef> = gr.objects.clone();
    for obj_rc in &objs {
        let to_remove: Vec<SemLinkRef> = {
            let ob = obj_rc.borrow();
            ob.links_from.iter()
                .filter(|li| {
                    let lb = li.borrow();
                    if lb.typ != SemLinkType::Pacient { return false; }
                    // Check if same object also has Agent link to the same target
                    let tgt = lb.target.clone();
                    ob.links_from.iter().any(|ll| {
                        let llb = ll.borrow();
                        !Rc::ptr_eq(ll, li)
                            && llb.typ == SemLinkType::Agent
                            && Rc::ptr_eq(&llb.target, &tgt)
                    })
                    && obj_rc.borrow().begin_char > tgt.borrow().begin_char
                })
                .cloned()
                .collect()
        };
        for li in to_remove {
            gr.remove_link(&li);
        }
    }
}

fn collapse_anafors(gr: &mut SemGraph) {
    let mut i = 0;
    while i < gr.objects.len() {
        let obj_rc = gr.objects[i].clone();
        let should_collapse = {
            let ob = obj_rc.borrow();
            let is_pronoun = ob.typ == SemObjectType::PersonalPronoun;
            let is_kotory  = ob.normal_full == "КОТОРЫЙ";
            if !is_pronoun && !is_kotory {
                false
            } else if !ob.attrs.is_empty() || ob.quantity.is_some() {
                false
            } else {
                // Must have exactly 1 Anafor link, or 2 where first.alt_link == second
                let anafor_links: Vec<_> = ob.links_from.iter()
                    .filter(|l| l.borrow().typ == SemLinkType::Anafor)
                    .cloned()
                    .collect();
                if anafor_links.len() == 1 {
                    true
                } else if anafor_links.len() == 2 {
                    let alt_matches = anafor_links[0].borrow().alt_link.as_ref()
                        .map_or(false, |al| Rc::ptr_eq(al, &anafor_links[1]));
                    alt_matches
                } else {
                    false
                }
            }
        };

        if !should_collapse {
            i += 1;
            continue;
        }

        let alink: SemLinkRef = {
            obj_rc.borrow().links_from.iter()
                .find(|l| l.borrow().typ == SemLinkType::Anafor)
                .cloned()
                .unwrap()
        };
        let alink_target = alink.borrow().target.clone();
        let alink_alt   = alink.borrow().alt_link.clone();

        let links_to: Vec<SemLinkRef> = obj_rc.borrow().links_to.clone();
        for li in &links_to {
            let (lt, ls, lq, lor, lp) = {
                let lb = li.borrow();
                (lb.typ, lb.source.clone(), lb.question.clone(), lb.is_or, lb.preposition.clone())
            };
            let nli = gr.add_link(lt, ls.clone(), alink_target.clone(), lq.clone(), lor, lp.clone());
            if let (Some(nli), Some(alt)) = (nli, &alink_alt) {
                let alt_tgt = alt.borrow().target.clone();
                let nli2 = gr.add_link(lt, ls, alt_tgt, lq, lor, lp);
                if let Some(nli2) = nli2 {
                    nli2.borrow_mut().alt_link = Some(nli.clone());
                    nli.borrow_mut().alt_link = Some(nli2);
                }
            }
        }

        gr.remove_object(&obj_rc);
        // Don't increment i: object removed, next is at same index
    }
}

// ── _processPointers ──────────────────────────────────────────────────────

/// Mirrors `OptimizerHelper._processPointers()`.
/// Nouns with quantity=="1" that appear alongside a same-type noun (with a
/// different quantity, or one decorated with ДРУГОЙ/ВТОРОЙ) are converted:
/// quantity is cleared and a ПЕРВЫЙ adjective Detail link is added.
/// In a second pass, nouns with ДРУГОЙ are renamed ВТОРОЙ when a ПЕРВЫЙ sibling exists.
fn process_pointers(
    all_objs:    &[SemObjectRef],
    obj_to_frag: &HashMap<usize, usize>,
    frags:       &[SemFragmentRef],
) {
    // ── Pass 1: add ПЕРВЫЙ adjective for qty=="1" nouns ──────────────────
    for i in 0..all_objs.len() {
        let o_rc = &all_objs[i];

        let (is_noun, qty_is_one, no_links_from, normal_full, gender) = {
            let ob = o_rc.borrow();
            let is_noun     = ob.typ == SemObjectType::Noun;
            let qty_is_one  = ob.quantity.as_ref().map(|q| q.spelling == "1").unwrap_or(false);
            let no_links    = ob.links_from.is_empty();
            (is_noun, qty_is_one, no_links, ob.normal_full.clone(), ob.gender)
        };
        if !is_noun || !qty_is_one || !no_links_from { continue; }

        // Look backwards for a Noun with same NormalFull but qty != "1"
        let mut ok = false;
        for j in (0..i).rev() {
            let oo = all_objs[j].borrow();
            if oo.typ != SemObjectType::Noun { continue; }
            if oo.normal_full != normal_full   { continue; }
            if oo.quantity.as_ref().map(|q| q.spelling.as_str()) != Some("1") {
                ok = true;
                break;
            }
        }
        if !ok {
            // Look forward for a Noun decorated with ДРУГОЙ or ВТОРОЙ
            for j in (i + 1)..all_objs.len() {
                let oo = all_objs[j].borrow();
                if oo.typ != SemObjectType::Noun { continue; }
                if oo.normal_full != normal_full   { continue; }
                let has_other = oo.find_from_object("ДРУГОЙ", SemLinkType::Undefined, SemObjectType::Undefined).is_some()
                    || oo.find_from_object("ВТОРОЙ", SemLinkType::Undefined, SemObjectType::Undefined).is_some();
                if has_other { ok = true; break; }
            }
        }
        if !ok { continue; }

        let frag_idx = match obj_to_frag.get(&(Rc::as_ptr(o_rc) as usize)) {
            Some(&fi) => fi,
            None      => continue,
        };

        let (normal_case, normal_full_adj) =
            if (gender.0 & MorphGenderFlags::FEMINIE.0) != 0 {
                ("ПЕРВАЯ".to_string(), "ПЕРВЫЙ".to_string())
            } else if (gender.0 & MorphGenderFlags::NEUTER.0) != 0 {
                ("ПЕРВОЕ".to_string(), "ПЕРВЫЙ".to_string())
            } else {
                ("ПЕРВЫЙ".to_string(), "ПЕРВЫЙ".to_string())
            };

        let (begin_char, end_char) = {
            let ob = o_rc.borrow();
            (ob.begin_char, ob.end_char)
        };

        let mut first = SemObject::new();
        first.typ         = SemObjectType::Adjective;
        first.normal      = normal_case;
        first.normal_full = normal_full_adj;
        first.gender      = gender;
        first.begin_char  = begin_char;
        first.end_char    = end_char;

        {
            let mut fr = frags[frag_idx].borrow_mut();
            let first_ref = fr.graph.add_object(first);
            fr.graph.add_link(SemLinkType::Detail, o_rc.clone(), first_ref, Some("какой".to_string()), false, None);
        }
        o_rc.borrow_mut().quantity = None;
    }

    // ── Pass 2: rename ДРУГОЙ → ВТОРОЙ where a ПЕРВЫЙ sibling exists ─────
    for i in 0..all_objs.len() {
        let o_rc = &all_objs[i];

        let (is_noun, qty_is_one, normal_full, gender, other_opt) = {
            let ob = o_rc.borrow();
            let is_noun    = ob.typ == SemObjectType::Noun;
            let qty_is_one = ob.quantity.as_ref().map(|q| q.spelling == "1").unwrap_or(false);
            let other      = ob.find_from_object("ДРУГОЙ", SemLinkType::Undefined, SemObjectType::Undefined);
            (is_noun, qty_is_one, ob.normal_full.clone(), ob.gender, other)
        };
        if !is_noun || !qty_is_one { continue; }
        let other = match other_opt { Some(r) => r, None => continue };

        // Look backward for a Noun with same NormalFull that already has ПЕРВЫЙ
        let mut ok = false;
        for j in (0..i).rev() {
            let oo = all_objs[j].borrow();
            if oo.typ != SemObjectType::Noun { continue; }
            if oo.normal_full != normal_full   { continue; }
            if oo.find_from_object("ПЕРВЫЙ", SemLinkType::Undefined, SemObjectType::Undefined).is_some() {
                ok = true;
                break;
            }
        }
        if !ok { continue; }

        let (new_case, new_full) =
            if (gender.0 & MorphGenderFlags::FEMINIE.0) != 0 {
                ("ВТОРАЯ".to_string(), "ВТОРОЙ".to_string())
            } else if (gender.0 & MorphGenderFlags::NEUTER.0) != 0 {
                ("ВТОРОЕ".to_string(), "ВТОРОЙ".to_string())
            } else {
                ("ВТОРОЙ".to_string(), "ВТОРОЙ".to_string())
            };
        let mut ob = other.borrow_mut();
        ob.normal_full = new_full;
        ob.normal      = new_case;
    }
}

// ── _processFormulas ──────────────────────────────────────────────────────

/// Mirrors `OptimizerHelper._processFormulas()`.
/// Handles "РАЗ" (times/once) noun objects with a quantity:
/// Re-routes Detail/Pacient links that come "after" the РАЗ object in the text.
fn process_formulas(
    all_objs:    &[SemObjectRef],
    obj_to_frag: &HashMap<usize, usize>,
    frags:       &[SemFragmentRef],
) {
    for o_rc in all_objs {
        // Check: Noun, IsValue("РАЗ"), has quantity, 0 outgoing links, exactly 1 incoming link
        let source_rc: SemObjectRef = {
            let ob = o_rc.borrow();
            if ob.typ != SemObjectType::Noun { continue; }
            if !ob.is_value("РАЗ", SemObjectType::Undefined) { continue; }
            if ob.quantity.is_none() { continue; }
            if !ob.links_from.is_empty() { continue; }
            if ob.links_to.len() != 1 { continue; }
            let x = ob.links_to[0].borrow().source.clone(); x
        };

        let mut frm = source_rc;
        for _ in 0..5 {
            let frm_links_from: Vec<SemLinkRef> = frm.borrow().links_from.clone();
            let mut found: Option<(SemLinkRef, SemObjectRef)> = None;
            let mut next_frm: Option<SemObjectRef> = None;

            for li in &frm_links_from {
                let (typ, is_o, tgt) = {
                    let lb = li.borrow();
                    (lb.typ, Rc::ptr_eq(&lb.target, o_rc), lb.target.clone())
                };
                if !(typ == SemLinkType::Detail || typ == SemLinkType::Pacient) { continue; }
                if is_o { continue; }

                let o_begin      = o_rc.borrow().begin_char;
                let frm_end_char = frm.borrow().end_char;
                let tgt_begin    = tgt.borrow().begin_char;

                if o_begin > frm_end_char && o_begin < tgt_begin {
                    found = Some((li.clone(), tgt));
                } else {
                    next_frm = Some(tgt);
                }
                break; // Only consider the first matching link (mirrors C# `break`)
            }

            if found.is_some() {
                let (li, tgt) = found.unwrap();
                let frag_idx = match obj_to_frag.get(&(Rc::as_ptr(o_rc) as usize)) {
                    Some(&fi) => fi,
                    None      => break,
                };
                let mut fr = frags[frag_idx].borrow_mut();
                fr.graph.add_link(SemLinkType::Detail, o_rc.clone(), tgt, Some("чего".to_string()), false, None);
                fr.graph.remove_link(&li);
                break;
            } else if let Some(next) = next_frm {
                frm = next;
            } else {
                break;
            }
        }
    }
}
