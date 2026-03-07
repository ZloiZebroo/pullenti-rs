/// AnalyzeHelper — coordinates semantic analysis using NGSegment link scoring.
/// Mirrors `AnalyzeHelper.cs` + `NGSegment.cs` link creation logic.
///
/// Pipeline:
///   1. Block / sentence segmentation
///   2. parse_sent_items → Vec<SentItem>
///   3. If Delim tokens exist: split into sub-sentences, process each separately
///      and create SemFraglinks; otherwise process as one SemFragment.
///   4. NGSegment::create_segments → groups noun phrases around verbs
///   5. For each segment: create candidate NGLinks, pick best per item
///   6. Convert best links to SemObject + SemLink graph

use std::rc::Rc;
use std::collections::HashSet;
use pullenti_ner::analysis_result::AnalysisResult;
use pullenti_ner::token::TokenRef;
use pullenti_ner::source_of_analysis::SourceOfAnalysis;
use pullenti_ner::core::misc_helper::can_be_start_of_sentence;

use crate::types::{SemProcessParams, SemLinkType, SemFraglinkType, SemAttributeType};
use crate::sem_document::{SemDocument, SemBlock, SemFragment, SemFragmentRef};
use crate::sem_graph::{SemGraph, SemObject, SemObjectRef};

use crate::internal::sent_item::{SentItemType, SentItemSource, parse_sent_items};
use crate::internal::ng_link::NGLinkType;
use crate::internal::ng_segment::NGSegment;
use crate::internal::ng_segment_variant::{NGSegmentVariant, create_variants};
use crate::internal::sentence_variant::pick_best_sentence_variant;
use crate::internal::subsent::create_subsents;
use crate::internal::create_helper::{create_noun_group, create_verb_group, create_adverb};

/// Top-level entry: process the full AnalysisResult into a SemDocument.
pub fn process(ar: &AnalysisResult, pars: &SemProcessParams) -> SemDocument {
    let mut doc = SemDocument::new();
    let sofa = &ar.sofa;

    let mut cur_opt = ar.first_token.clone();
    while let Some(cur) = cur_opt {
        let block_start = cur.clone();
        let mut block_end = cur.clone();
        {
            let mut t = cur.borrow().next.clone();
            while let Some(tt) = t {
                let is_nl     = tt.borrow().is_newline_before(sofa);
                let can_start = can_be_start_of_sentence(&tt, sofa);
                if is_nl && can_start { break; }
                block_end = tt.clone();
                t = tt.borrow().next.clone();
            }
        }

        process_block(&mut doc, sofa, &block_start, &block_end, pars);

        let next = block_end.borrow().next.clone();
        cur_opt  = next;

        if pars.max_char > 0 && block_end.borrow().end_char as usize > pars.max_char {
            break;
        }
    }

    doc
}

fn process_block(
    doc:   &mut SemDocument,
    sofa:  &SourceOfAnalysis,
    t0:    &TokenRef,
    t1:    &TokenRef,
    _pars: &SemProcessParams,
) {
    let mut blk = SemBlock::new();
    let t1_end  = t1.borrow().end_char;

    let mut cur_opt = Some(t0.clone());
    while let Some(cur) = cur_opt {
        if cur.borrow().end_char > t1_end { break; }

        let sent_start = cur.clone();
        let mut sent_end = cur.clone();
        {
            let mut t = cur.borrow().next.clone();
            while let Some(tt) = t {
                if tt.borrow().end_char > t1_end { break; }
                if can_be_start_of_sentence(&tt, sofa) { break; }
                sent_end = tt.clone();
                t = tt.borrow().next.clone();
            }
        }

        process_sentence(&mut blk, sofa, &sent_start, &sent_end);

        let next = sent_end.borrow().next.clone();
        cur_opt  = next;
    }

    if !blk.fragments.is_empty() {
        doc.blocks.push(Rc::new(std::cell::RefCell::new(blk)));
    }
}

fn process_sentence(
    blk:  &mut SemBlock,
    sofa: &SourceOfAnalysis,
    t0:   &TokenRef,
    t1:   &TokenRef,
) {
    // 1. Parse sentence items
    let mut sent_items = parse_sent_items(t0, t1, sofa);

    // Limit to 70 tokens (mirrors C#'s 70-token cap)
    if sent_items.len() > 70 {
        sent_items.truncate(70);
    }

    if sent_items.is_empty() { return; }

    // 2. Check if there are any Delim tokens (ЕСЛИ/НО/ЧТО/etc.)
    let has_delim = sent_items.iter().any(|si| si.typ == SentItemType::Delim);

    if has_delim {
        // Full subsent splitting path
        process_sentence_with_subsents(blk, sofa, sent_items);
    } else {
        // Fast single-fragment path
        process_sentence_items(blk, sofa, t0, t1, sent_items);
    }
}

/// Process a sentence that contains Delim tokens, splitting into sub-sentences
/// and creating SemFraglinks between them.
fn process_sentence_with_subsents(
    blk:        &mut SemBlock,
    sofa:       &SourceOfAnalysis,
    sent_items: Vec<crate::internal::sent_item::SentItem>,
) {
    // Build a quick list-link char set (greedy links, no full enumeration)
    // so we don't split inside noun lists like "Иван и Петр".
    let list_char_set = build_list_char_set_greedy(&sent_items);

    // Create subsents
    let subsents = create_subsents(&sent_items, &list_char_set);

    if subsents.len() <= 1 {
        // Nothing to split: process as single fragment
        let (t0_sub, t1_sub) = match subsent_tokens(&subsents, 0, &sent_items) {
            Some(pair) => pair,
            None       => return,
        };
        let sub_items = parse_sent_items(&t0_sub, &t1_sub, sofa);
        if !sub_items.is_empty() {
            process_sentence_items(blk, sofa, &t0_sub, &t1_sub, sub_items);
        }
        return;
    }

    // Multiple subsents: process each independently and create SemFraglinks.
    let mut frag_refs: Vec<Option<SemFragmentRef>> = Vec::new();

    for (si, ss) in subsents.iter().enumerate() {
        if ss.item_indices.is_empty() {
            frag_refs.push(None);
            continue;
        }
        let (t0_sub, t1_sub) = match subsent_tokens(&subsents, si, &sent_items) {
            Some(pair) => pair,
            None       => { frag_refs.push(None); continue; }
        };
        let sub_items = parse_sent_items(&t0_sub, &t1_sub, sofa);
        if sub_items.is_empty() {
            frag_refs.push(None);
            continue;
        }
        let frag = build_fragment(sofa, &t0_sub, &t1_sub, sub_items);
        let frag_ref = Rc::new(std::cell::RefCell::new(frag));
        blk.fragments.push(frag_ref.clone());
        frag_refs.push(Some(frag_ref));
    }

    // Create SemFraglinks based on owner relationships
    for (i, ss) in subsents.iter().enumerate() {
        if ss.typ == SemFraglinkType::Undefined { continue; }
        let owner_idx = match ss.owner_idx { Some(o) => o, None => continue };
        let src_ref = match frag_refs.get(owner_idx).and_then(|o| o.as_ref()) {
            Some(r) => r.clone(),
            None    => continue,
        };
        let tgt_ref = match frag_refs.get(i).and_then(|o| o.as_ref()) {
            Some(r) => r.clone(),
            None    => continue,
        };
        blk.add_link(ss.typ, src_ref, tgt_ref, ss.question.clone());
    }
}

/// Return (begin_token, end_token) for the `si`-th subsent's item range.
fn subsent_tokens(
    subsents:   &[crate::internal::subsent::Subsent],
    si:         usize,
    sent_items: &[crate::internal::sent_item::SentItem],
) -> Option<(TokenRef, TokenRef)> {
    let ss = &subsents[si];
    let first = *ss.item_indices.first()?;
    let last  = *ss.item_indices.last()?;
    let t0 = sent_items[first].begin_token();
    let t1 = sent_items[last].end_token();
    Some((t0, t1))
}

/// Build a list-char-set using greedy (non-enumerating) link assignment.
/// Covers char positions that belong to list NGLinks, preventing splits there.
fn build_list_char_set_greedy(
    sent_items: &[crate::internal::sent_item::SentItem],
) -> HashSet<i32> {
    let mut set = HashSet::new();
    let segs = NGSegment::create_segments(sent_items);
    for seg in &segs {
        let greedy = seg.best_links(sent_items);
        for (idx, link_opt) in greedy.iter().enumerate() {
            let link = match link_opt { Some(l) => l, None => continue };
            if link.typ != NGLinkType::List { continue; }

            let from_sent_idx = seg.items[idx].sent_item_idx;
            let from_end      = sent_items[from_sent_idx].end_char();

            let to_begin = if let Some(verb_idx) = link.to_verb_sent_idx {
                sent_items[verb_idx].begin_char()
            } else if let Some(to_ord) = link.to_ord {
                let to_sent_idx = seg.items[to_ord].sent_item_idx;
                sent_items[to_sent_idx].begin_char()
            } else {
                continue
            };

            for pos in to_begin..=from_end {
                set.insert(pos);
            }
        }
    }
    set
}

/// Process a sentence (represented as `sent_items`) and add one SemFragment to `blk`.
fn process_sentence_items(
    blk:        &mut SemBlock,
    sofa:       &SourceOfAnalysis,
    t0:         &TokenRef,
    t1:         &TokenRef,
    sent_items: Vec<crate::internal::sent_item::SentItem>,
) {
    let frag = build_fragment(sofa, t0, t1, sent_items);
    blk.fragments.push(Rc::new(std::cell::RefCell::new(frag)));
}

/// Core: build a SemFragment from a list of SentItems.
fn build_fragment(
    sofa:       &SourceOfAnalysis,
    t0:         &TokenRef,
    t1:         &TokenRef,
    sent_items: Vec<crate::internal::sent_item::SentItem>,
) -> SemFragment {
    let begin_char = t0.borrow().begin_char as usize;
    let end_char   = t1.borrow().end_char   as usize;

    let mut frag = SemFragment::new();
    frag.begin_token = Some(t0.clone());
    frag.end_token   = Some(t1.clone());
    frag.begin_char  = begin_char;
    frag.end_char    = end_char;

    // Create SemObjects for each item
    let gr = &mut frag.graph;
    let mut obj_map: Vec<Option<SemObjectRef>> = vec![None; sent_items.len()];

    for (idx, si) in sent_items.iter().enumerate() {
        match si.typ {
            SentItemType::Noun | SentItemType::PartBefore | SentItemType::PartAfter
            | SentItemType::Deepart | SentItemType::SubSent | SentItemType::Formula => {
                if let Some(npt) = si.noun_phrase() {
                    if let Some(sem_ref) = create_noun_group(gr, npt, sofa) {
                        obj_map[idx] = Some(sem_ref);
                    }
                }
            }
            SentItemType::Verb => {
                if let Some(vpt) = si.verb_phrase() {
                    if let Some(sem_ref) = create_verb_group(gr, vpt, sofa) {
                        obj_map[idx] = Some(sem_ref);
                    }
                }
            }
            SentItemType::Adverb => {
                if let crate::internal::sent_item::SentItemSource::Adverb(ref adv) = si.source {
                    let sem_ref = create_adverb(gr, adv, sofa);
                    obj_map[idx] = Some(sem_ref);
                }
            }
            _ => {} // Conj / Delim — no SemObject
        }
    }

    // Create NGSegments and pick best cross-segment link combination
    let mut segs = NGSegment::create_segments(&sent_items);
    let mut all_seg_variants: Vec<Vec<NGSegmentVariant>> = segs.iter_mut()
        .map(|seg| {
            let vars = create_variants(seg, &sent_items, 5);
            if vars.is_empty() {
                let best = seg.best_links(&sent_items);
                let coef: f64 = best.iter().filter_map(|l| l.as_ref()).map(|l| l.coef).sum();
                vec![NGSegmentVariant {
                    coef,
                    links: best,
                    before_verb_sent_idx: seg.before_verb_idx,
                }]
            } else {
                vars
            }
        })
        .collect();

    let best_seg_variants = pick_best_sentence_variant(&all_seg_variants);

    // Build list-group map: root sent_item_idx → Vec<member sent_item_idx>
    // Mirrors _createLists(): when item i has a List link to root j, the root's group
    // gets item i added, so Agent/Pacient links on the root propagate to all members.
    use std::collections::HashMap;
    let mut list_groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (seg_i, best_opt) in best_seg_variants.iter().enumerate() {
        let bv = match best_opt { Some(v) => v, None => continue };
        let seg = &segs[seg_i];
        for (i, opt_link) in bv.links.iter().enumerate() {
            let link = match opt_link { Some(l) => l, None => continue };
            if link.typ != NGLinkType::List { continue; }
            let to_ord = match link.to_ord { Some(t) => t, None => continue };
            let root_sent_idx   = seg.items[to_ord].sent_item_idx;
            let member_sent_idx = seg.items[i].sent_item_idx;
            list_groups.entry(root_sent_idx).or_default().push(member_sent_idx);
        }
    }

    for (seg_i, best_opt) in best_seg_variants.iter().enumerate() {
        let best_links = match best_opt {
            Some(v) => &v.links,
            None    => continue,
        };
        let seg = &segs[seg_i];

        for (i, opt_link) in best_links.iter().enumerate() {
            let link = match opt_link { Some(l) => l, None => continue };
            let from_sent_idx = seg.items[i].sent_item_idx;
            let from_sem = match &obj_map[from_sent_idx] { Some(r) => r, None => continue };

            match link.typ {
                NGLinkType::Agent | NGLinkType::Pacient => {
                    let verb_idx = match link.to_verb_sent_idx { Some(v) => v, None => continue };
                    let verb_sem = match &obj_map[verb_idx] { Some(r) => r, None => continue };
                    let sem_link_type = if link.typ == NGLinkType::Agent {
                        SemLinkType::Agent
                    } else {
                        SemLinkType::Pacient
                    };
                    gr.add_link(sem_link_type, verb_sem.clone(), from_sem.clone(), None, false, None);
                    // Expand list group: also create links for all members of this root
                    if let Some(members) = list_groups.get(&from_sent_idx) {
                        for &member_sent_idx in members {
                            if let Some(member_sem) = &obj_map[member_sent_idx] {
                                gr.add_link(sem_link_type, verb_sem.clone(), member_sem.clone(), None, false, None);
                            }
                        }
                    }
                }
                NGLinkType::Actant => {
                    let verb_idx = match link.to_verb_sent_idx { Some(v) => v, None => continue };
                    let verb_sem = match &obj_map[verb_idx] { Some(r) => r, None => continue };
                    let prep_str = sent_items[from_sent_idx].prep.clone();
                    let ques = if prep_str.is_empty() { None } else { Some(prep_str) };
                    gr.add_link(SemLinkType::Detail, verb_sem.clone(), from_sem.clone(), ques, false, None);
                }
                NGLinkType::Genetive => {
                    let to_ord  = match link.to_ord { Some(t) => t, None => continue };
                    let to_sent = seg.items[to_ord].sent_item_idx;
                    let to_sem  = match &obj_map[to_sent] { Some(r) => r, None => continue };
                    let (source, target) = if link.reverce {
                        (from_sem.clone(), to_sem.clone())
                    } else {
                        (to_sem.clone(), from_sem.clone())
                    };
                    gr.add_link(SemLinkType::Detail, source, target, Some("чего".to_string()), false, None);
                }
                NGLinkType::Name => {
                    let to_ord  = match link.to_ord { Some(t) => t, None => continue };
                    let to_sent = seg.items[to_ord].sent_item_idx;
                    let to_sem  = match &obj_map[to_sent] { Some(r) => r, None => continue };
                    gr.add_link(SemLinkType::Naming, to_sem.clone(), from_sem.clone(), None, false, None);
                }
                NGLinkType::Participle => {
                    let to_ord  = match link.to_ord { Some(t) => t, None => continue };
                    let to_sent = seg.items[to_ord].sent_item_idx;
                    let to_sem  = match &obj_map[to_sent] { Some(r) => r, None => continue };
                    gr.add_link(SemLinkType::Participle, to_sem.clone(), from_sem.clone(), Some("какой".to_string()), false, None);
                }
                NGLinkType::Be => {
                    let to_ord  = match link.to_ord { Some(t) => t, None => continue };
                    let to_sent = seg.items[to_ord].sent_item_idx;
                    let to_sem  = match &obj_map[to_sent] { Some(r) => r, None => continue };
                    // БЫТЬ copula: create a dummy verb-object linking the two nouns
                    let mut be_sem = SemObject::new();
                    be_sem.typ         = crate::types::SemObjectType::Verb;
                    be_sem.normal      = "БЫТЬ".to_string();
                    be_sem.normal_full = "БЫТЬ".to_string();
                    be_sem.begin_char  = sent_items[from_sent_idx].begin_char() as usize;
                    be_sem.end_char    = sent_items[from_sent_idx].end_char()   as usize;
                    let be_ref = gr.add_object(be_sem);
                    gr.add_link(SemLinkType::Agent,  be_ref.clone(), to_sem.clone(),   None, false, None);
                    gr.add_link(SemLinkType::Pacient, be_ref,        from_sem.clone(), None, false, None);
                }
                NGLinkType::Adverb => {
                    if let Some(verb_idx) = link.to_verb_sent_idx {
                        if let Some(verb_sem) = &obj_map[verb_idx] {
                            gr.add_link(SemLinkType::Detail, verb_sem.clone(), from_sem.clone(), Some("как".to_string()), false, None);
                        }
                    }
                }
                NGLinkType::List => { /* handled by merge system */ }
                _ => {}
            }
        }
    }

    // Fallback: if no segments produced links, do naive first-noun=agent assignment
    if gr.links.is_empty() {
        let verb_refs: Vec<SemObjectRef> = sent_items.iter().enumerate()
            .filter(|(_, si)| si.typ == SentItemType::Verb)
            .filter_map(|(idx, _)| obj_map[idx].clone())
            .collect();
        let noun_refs: Vec<SemObjectRef> = sent_items.iter().enumerate()
            .filter(|(_, si)| si.can_be_noun())
            .filter_map(|(idx, _)| obj_map[idx].clone())
            .collect();
        if verb_refs.len() == 1 {
            let verb_ref = &verb_refs[0];
            let mut agent_assigned = false;
            for noun_ref in &noun_refs {
                if !agent_assigned {
                    gr.add_link(SemLinkType::Agent,  verb_ref.clone(), noun_ref.clone(), None, false, None);
                    agent_assigned = true;
                } else {
                    gr.add_link(SemLinkType::Pacient, verb_ref.clone(), noun_ref.clone(), None, false, None);
                }
            }
        }
    }

    // ── Adverb Detail links (Sentence.cs lines 311-397) ─────────────────────
    // For Undefined-type adverbs (free manner adverbs like "быстро", "хорошо")
    // that haven't been linked by NGLink::Adverb, connect them with a Detail link
    // to the nearest Verb (preferred) or Noun in the sentence.
    for i in 0..sent_items.len() {
        if sent_items[i].typ != SentItemType::Adverb { continue; }
        let adv_sem = match &obj_map[i] { Some(r) => r.clone(), None => continue };

        // Only handle Undefined-type adverbs
        let is_undefined_adv = match &sent_items[i].source {
            SentItemSource::Adverb(adv) => adv.typ == SemAttributeType::Undefined,
            _ => false,
        };
        if !is_undefined_adv { continue; }

        // Skip if already has a Detail link targeting this adverb (from NGLink::Adverb)
        let already_linked = gr.links.iter().any(|l| {
            let lb = l.borrow();
            lb.typ == SemLinkType::Detail && Rc::ptr_eq(&lb.target, &adv_sem)
        });
        if already_linked { continue; }

        // Pass 1: scan backward through Adverb/Noun items for a Verb
        let mut before: Option<usize> = None;
        for ii in (0..i).rev() {
            match sent_items[ii].typ {
                SentItemType::Verb => { before = Some(ii); break; }
                SentItemType::Adverb | SentItemType::Noun => {}
                _ => break,
            }
        }
        // Pass 2: if no Verb found, scan backward for any Verb or Noun
        if before.is_none() {
            for ii in (0..i).rev() {
                match sent_items[ii].typ {
                    SentItemType::Verb | SentItemType::Noun => { before = Some(ii); break; }
                    SentItemType::Adverb => {}
                    _ => break,
                }
            }
        }

        // Scan forward for a Verb or Noun, respecting comma boundaries
        let mut after: Option<usize> = None;
        let mut comma_after = false;
        for ii in (i + 1)..sent_items.len() {
            match sent_items[ii].typ {
                SentItemType::Verb | SentItemType::Noun => { after = Some(ii); break; }
                SentItemType::Adverb => {}
                _ => {
                    if sent_items[ii].can_be_comma_end() {
                        // Stop scanning forward if before is a Verb
                        if before.map_or(false, |b| sent_items[b].typ == SentItemType::Verb) {
                            break;
                        }
                        let next_is_ok = (ii + 1 < sent_items.len())
                            && matches!(
                                sent_items[ii + 1].typ,
                                SentItemType::Adverb | SentItemType::Verb
                            );
                        if !next_is_ok {
                            comma_after = true;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Resolve: prefer after over before, handle comma_after
        if before.is_some() && after.is_some() {
            if comma_after {
                after = None;
            } else {
                let bt = sent_items[before.unwrap()].typ;
                let at = sent_items[after.unwrap()].typ;
                if bt == SentItemType::Noun && at == SentItemType::Verb {
                    before = None; // forward verb wins
                } else if bt == SentItemType::Verb && at == SentItemType::Noun {
                    after = None; // backward verb wins
                }
            }
        }

        let target_idx = after.or(before);
        if let Some(t_idx) = target_idx {
            if let Some(target_sem) = &obj_map[t_idx] {
                gr.add_link(
                    SemLinkType::Detail,
                    target_sem.clone(),
                    adv_sem,
                    Some("как".to_string()),
                    false,
                    None,
                );
            }
        }
    }

    // ── Predicate chaining (Sentence.cs lines 398-483) ──────────────────────
    // When a verb has no Agent link but the previous verb did, propagate the agent.
    // "Иван бежит и прыгает." → прыгает inherits Иван as Agent.
    {
        let mut prev_agent_sem: Option<SemObjectRef> = None;
        for i in 0..sent_items.len() {
            if sent_items[i].typ != SentItemType::Verb { continue; }
            let verb_sem = match &obj_map[i] { Some(r) => r.clone(), None => continue };

            // Check if this verb already has an Agent link (source = this verb)
            let existing_agent = gr.links.iter()
                .find(|l| {
                    let lb = l.borrow();
                    lb.typ == SemLinkType::Agent && Rc::ptr_eq(&lb.source, &verb_sem)
                })
                .map(|l| l.borrow().target.clone());

            if let Some(agent_sem) = existing_agent {
                // Verb already has an Agent — record it for next verb
                prev_agent_sem = Some(agent_sem);
                continue;
            }

            // No Agent — try to inherit from previous verb's agent
            if let Some(ref agent_sem) = prev_agent_sem.clone() {
                // Don't inherit for passive verbs
                let is_passive = sent_items[i].verb_morph.as_ref()
                    .map_or(false, |vm| vm.is_passive_str);
                if !is_passive {
                    gr.add_link(
                        SemLinkType::Agent,
                        verb_sem.clone(),
                        agent_sem.clone(),
                        None,
                        false,
                        None,
                    );
                    // Keep same agent for further chaining
                    prev_agent_sem = Some(agent_sem.clone());
                }
            }
        }
    }

    // ── Deepart Agent links (Sentence.cs lines 486-548) ──────────────────────
    // Deepart (деепричастие) items: find nearby nominative nouns that already
    // have Agent or Pacient links and add an Agent link from the deepart to them.
    for i in 0..sent_items.len() {
        if sent_items[i].typ != SentItemType::Deepart { continue; }
        let deepart_sem = match &obj_map[i] { Some(r) => r.clone(), None => continue };

        // Scan backward for nominative nouns that have Agent/Pacient links
        let mut found_link: Option<SemObjectRef> = None;
        'back: for j in (0..i).rev() {
            if sent_items[j].typ != SentItemType::Noun { continue; }
            let nm = match sent_items[j].noun_morph.as_ref() {
                Some(m) => m,
                None    => continue,
            };
            if !nm.is_nominative() { continue; }
            let noun_sem = match &obj_map[j] { Some(r) => r.clone(), None => continue };
            let has_agent_or_pac = gr.links.iter().any(|l| {
                let lb = l.borrow();
                (lb.typ == SemLinkType::Agent || lb.typ == SemLinkType::Pacient)
                    && Rc::ptr_eq(&lb.target, &noun_sem)
            });
            if !has_agent_or_pac { continue; }
            gr.add_link(SemLinkType::Agent, deepart_sem.clone(), noun_sem.clone(), None, false, None);
            if found_link.is_none() {
                found_link = Some(noun_sem);
            } else {
                break 'back;
            }
        }
        // If nothing found backward, scan forward
        if found_link.is_none() {
            for j in (i + 1)..sent_items.len() {
                if sent_items[j].typ != SentItemType::Noun { continue; }
                let nm = match sent_items[j].noun_morph.as_ref() {
                    Some(m) => m,
                    None    => continue,
                };
                if !nm.is_nominative() { continue; }
                let noun_sem = match &obj_map[j] { Some(r) => r.clone(), None => continue };
                let has_agent_or_pac = gr.links.iter().any(|l| {
                    let lb = l.borrow();
                    (lb.typ == SemLinkType::Agent || lb.typ == SemLinkType::Pacient)
                        && Rc::ptr_eq(&lb.target, &noun_sem)
                });
                if !has_agent_or_pac { continue; }
                gr.add_link(SemLinkType::Agent, deepart_sem.clone(), noun_sem.clone(), None, false, None);
                if found_link.is_none() {
                    found_link = Some(noun_sem);
                } else {
                    break;
                }
            }
        }
    }

    // ── Question detection (Sentence.cs lines 549-562) ──────────────────────
    // If the first object is "КАКОЙ" or "СКОЛЬКО" and the sentence ends with '?',
    // mark it as a Question.
    use crate::types::SemObjectType;
    if let Some(first_obj) = gr.objects.first().cloned() {
        let nf = first_obj.borrow().normal_full.clone();
        if nf.eq_ignore_ascii_case("КАКОЙ") || nf.eq_ignore_ascii_case("СКОЛЬКО") {
            // Check if the sentence end token is '?'
            let last_char_is_q = t1.borrow().end_char;
            let _ = last_char_is_q; // used indirectly via sofa
            // Simple check: see if the end token is a question mark
            let ends_with_q = {
                let tb = t1.borrow();
                // Check if end token text contains '?'
                sofa.substring(tb.begin_char, tb.end_char).contains('?')
            };
            if ends_with_q {
                first_obj.borrow_mut().typ = SemObjectType::Question;
            }
        }
    }

    frag
}
