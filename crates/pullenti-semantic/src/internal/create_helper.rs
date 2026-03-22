/// CreateHelper — creates SemObjects from NounPhraseToken / VerbPhraseToken / AdverbToken.
/// Mirrors `CreateHelper.cs` (simplified port).

use std::rc::Rc;
use pullenti_morph::{MorphGenderFlags, MorphNumber};
use pullenti_ner::token::{TokenRef, TokenKind};
use pullenti_ner::referent::SlotValue;
use pullenti_ner::core::noun_phrase::{NounPhraseToken, NounPhraseSpan};
use pullenti_ner::core::verb_phrase::VerbPhraseToken;
use pullenti_ner::source_of_analysis::SourceOfAnalysis;
use crate::sem_graph::{SemGraph, SemObject, SemObjectRef};
use crate::types::{SemObjectType, SemLinkType, SemAttributeType};
use super::adverb_token::{self, AdverbToken};

// ── Normal-text helpers ────────────────────────────────────────────────────

/// Extract a display string from a Referent (mirrors C# `r.ToString()`).
/// Tries common NAME/FIRSTNAME+LASTNAME slots; falls back to type_name.
fn referent_to_string(r: &pullenti_ner::referent::Referent) -> String {
    // Collect all NAME values
    let names: Vec<String> = r.slots.iter()
        .filter(|s| s.type_name == "NAME")
        .filter_map(|s| match &s.value {
            Some(SlotValue::Str(v)) => Some(v.clone()),
            _ => None,
        })
        .collect();
    if !names.is_empty() {
        return names[0].clone();
    }
    // PERSON: build from FIRSTNAME + LASTNAME
    let first = r.slots.iter().find(|s| s.type_name == "FIRSTNAME")
        .and_then(|s| match &s.value { Some(SlotValue::Str(v)) => Some(v.as_str()), _ => None });
    let last  = r.slots.iter().find(|s| s.type_name == "LASTNAME")
        .and_then(|s| match &s.value { Some(SlotValue::Str(v)) => Some(v.as_str()), _ => None });
    if last.is_some() || first.is_some() {
        let mut parts = Vec::new();
        if let Some(f) = first { parts.push(f); }
        if let Some(l) = last  { parts.push(l); }
        return parts.join(" ");
    }
    // Fallback: any string slot value
    for slot in &r.slots {
        if let Some(SlotValue::Str(v)) = &slot.value {
            if !v.is_empty() { return v.clone(); }
        }
    }
    r.type_name.clone()
}

/// Get the nominative singular form of a single token (first word form).
pub fn token_normal(tok: &TokenRef) -> String {
    let tb = tok.borrow();
    for wf in tb.morph.items() {
        if let Some(ref nf) = wf.normal_full { return nf.clone(); }
        if let Some(ref nc) = wf.normal_case { return nc.clone(); }
    }
    // Referent token: extract display name from the referent
    if let TokenKind::Referent(ref r) = tb.kind {
        return referent_to_string(&r.referent.borrow());
    }
    // Fallback: raw term
    if let TokenKind::Text(ref txt) = tb.kind {
        txt.term.clone()
    } else {
        String::new()
    }
}

/// Get the surface text for a span of tokens (begin..=end, using terms).
pub fn span_text(begin: &TokenRef, end: &TokenRef) -> String {
    let end_char = end.borrow().end_char;
    let mut parts = Vec::new();
    let mut cur = Some(begin.clone());
    while let Some(t) = cur {
        if t.borrow().end_char > end_char { break; }
        if let TokenKind::Text(ref txt) = t.borrow().kind {
            let s = txt.term.clone();
            if !s.is_empty() { parts.push(s); }
        }
        let next = t.borrow().next.clone();
        cur = next;
    }
    parts.join(" ")
}

/// Get nominative form of a noun span (single or multi-token).
pub fn span_normal(span: &NounPhraseSpan) -> String {
    // Single token case
    if Rc::ptr_eq(&span.begin_token, &span.end_token) {
        return token_normal(&span.begin_token);
    }
    // Multi-token: collect normal forms (use term as fallback)
    let end_char = span.end_token.borrow().end_char;
    let mut parts = Vec::new();
    let mut cur = Some(span.begin_token.clone());
    while let Some(t) = cur {
        if t.borrow().end_char > end_char { break; }
        parts.push(token_normal(&t));
        let next = t.borrow().next.clone();
        cur = next;
    }
    parts.join(" ")
}

// ── create_noun_group ──────────────────────────────────────────────────────

/// Create a SemObject for a noun phrase (noun + adjectives → Detail links).
/// Mirrors `CreateHelper.CreateNounGroup`.
pub fn create_noun_group(
    gr:   &mut SemGraph,
    npt:  &NounPhraseToken,
    sofa: &SourceOfAnalysis,
) -> Option<SemObjectRef> {
    let noun_span = npt.noun.as_ref()?;

    // Determine token type
    let is_personal_pronoun = noun_span.morph.items().iter()
        .any(|wf| wf.base.class.is_personal_pronoun());
    let is_pronoun = noun_span.morph.items().iter()
        .any(|wf| wf.base.class.is_pronoun());

    let obj_typ = if is_personal_pronoun {
        SemObjectType::PersonalPronoun
    } else if is_pronoun {
        SemObjectType::Pronoun
    } else {
        SemObjectType::Noun
    };

    // Build normal forms
    let (normal, normal_full) = if Rc::ptr_eq(&noun_span.begin_token, &noun_span.end_token) {
        // Single token: find word form matching overall case/number
        let tb = noun_span.begin_token.borrow();
        let mut nc = String::new();
        let mut nf = String::new();
        for wf in tb.morph.items() {
            if !nc.is_empty() { break; }
            if let Some(ref n) = wf.normal_case { nc = n.clone(); }
            if let Some(ref n) = wf.normal_full  { nf = n.clone(); }
        }
        if nc.is_empty() {
            nc = token_normal(&noun_span.begin_token);
        }
        if nf.is_empty() { nf = nc.clone(); }
        (nc, nf)
    } else {
        let s = span_normal(noun_span);
        (s.clone(), s)
    };

    let begin_char = noun_span.begin_token.borrow().begin_char as usize;
    let end_char   = noun_span.end_token.borrow().end_char as usize;

    // Extract gender/number from noun span morph collection
    let mut gender = MorphGenderFlags::UNDEFINED;
    let mut number = MorphNumber::UNDEFINED;
    {
        for wf in noun_span.morph.items() {
            let wg = wf.base.gender;
            if wg != MorphGenderFlags::UNDEFINED && gender == MorphGenderFlags::UNDEFINED {
                gender = wg;
            }
            let wn = wf.base.number;
            if wn != MorphNumber::UNDEFINED && number == MorphNumber::UNDEFINED {
                number = wn;
            }
            if gender != MorphGenderFlags::UNDEFINED && number != MorphNumber::UNDEFINED { break; }
        }
    }

    let mut sem = SemObject::new();
    sem.typ        = obj_typ;
    sem.normal     = normal;
    sem.normal_full = normal_full;
    sem.gender     = gender;
    sem.number     = number;
    sem.begin_char = begin_char;
    sem.end_char   = end_char;
    let sem_ref = gr.add_object(sem);

    // Adjectives → Detail links
    for adj in &npt.adjectives {
        if let Some(adj_ref) = create_npt_adj(gr, npt, adj) {
            gr.add_link(
                SemLinkType::Detail,
                sem_ref.clone(),
                adj_ref,
                Some("какой".to_string()),
                false,
                None,
            );
        }
    }

    // InternalNoun → Detail link
    if let Some(ref internal_npt) = npt.internal_noun {
        if let Some(inner_ref) = create_noun_group(gr, internal_npt, sofa) {
            gr.add_link(
                SemLinkType::Detail,
                sem_ref.clone(),
                inner_ref,
                None,
                false,
                None,
            );
        }
    }

    Some(sem_ref)
}

/// Create a SemObject for a noun-phrase adjective slot.
fn create_npt_adj(
    gr:  &mut SemGraph,
    _npt: &NounPhraseToken,
    adj: &NounPhraseSpan,
) -> Option<SemObjectRef> {
    let is_pronoun = adj.morph.items().iter().any(|wf| wf.base.class.is_pronoun());
    let is_personal_pronoun = adj.morph.items().iter().any(|wf| wf.base.class.is_personal_pronoun());
    let is_verb   = adj.morph.items().iter().any(|wf| wf.base.class.is_verb());

    if is_pronoun {
        // Pronoun adjective (e.g. possessive pronoun)
        let normal = span_normal(adj);
        let mut sem = SemObject::new();
        sem.typ = if is_personal_pronoun {
            SemObjectType::PersonalPronoun
        } else {
            SemObjectType::Pronoun
        };
        sem.normal     = normal.clone();
        sem.normal_full = normal;
        sem.begin_char = adj.begin_token.borrow().begin_char as usize;
        sem.end_char   = adj.end_token.borrow().end_char as usize;
        return Some(gr.add_object(sem));
    }

    if is_verb {
        // Verb-adjective (participle) — skip for now
        return None;
    }

    // Regular adjective
    let normal_case = {
        let tb = adj.begin_token.borrow();
        let mut nc = String::new();
        for wf in tb.morph.items() {
            if wf.base.class.is_adjective() {
                nc = wf.normal_case.clone().unwrap_or_default();
                break;
            }
        }
        if nc.is_empty() { span_normal(adj) } else { nc }
    };
    let normal_full = {
        let tb = adj.begin_token.borrow();
        let mut nf = String::new();
        for wf in tb.morph.items() {
            if wf.base.class.is_adjective() {
                nf = wf.normal_full.clone()
                    .or_else(|| wf.normal_case.clone())
                    .unwrap_or_default();
                break;
            }
        }
        if nf.is_empty() { normal_case.clone() } else { nf }
    };

    let mut sem = SemObject::new();
    sem.typ        = SemObjectType::Adjective;
    sem.normal     = normal_case;
    sem.normal_full = normal_full;
    sem.begin_char = adj.begin_token.borrow().begin_char as usize;
    sem.end_char   = adj.end_token.borrow().end_char as usize;
    Some(gr.add_object(sem))
}

// ── create_adverb ──────────────────────────────────────────────────────────

/// Create a SemObject for an adverb token.
pub fn create_adverb(
    gr:   &mut SemGraph,
    adv:  &AdverbToken,
    sofa: &SourceOfAnalysis,
) -> SemObjectRef {
    let spelling = adv.get_spelling(sofa);
    let mut sem = SemObject::new();
    sem.typ        = SemObjectType::Adverb;
    sem.normal     = spelling.clone();
    sem.normal_full = spelling;
    sem.not        = adv.not;
    sem.begin_char = adv.begin_char() as usize;
    sem.end_char   = adv.end_char() as usize;
    gr.add_object(sem)
}

// ── create_verb_group ──────────────────────────────────────────────────────

/// Create SemObject(s) for a verb phrase.
/// Returns the first (main) verb SemObject, chaining others via Detail links.
pub fn create_verb_group(
    gr:   &mut SemGraph,
    vpt:  &VerbPhraseToken,
    sofa: &SourceOfAnalysis,
) -> Option<SemObjectRef> {
    let mut verb_sems: Vec<SemObjectRef> = Vec::new();
    let mut pending_adverbs: Vec<SemObjectRef> = Vec::new();

    for item in &vpt.items {
        if item.is_adverb {
            // Parse as adverb
            if let Some(adv) = adverb_token::try_parse(&item.begin_token, sofa) {
                if adv.typ != SemAttributeType::Undefined {
                    // Attribute-style adverb: will be added to next verb
                    // (simplified: skip for now)
                } else {
                    pending_adverbs.push(create_adverb(gr, &adv, sofa));
                }
            }
            continue;
        }

        // Skip БЫТЬ if not the last verb
        let is_last = {
            let idx = vpt.items.iter().position(|i| {
                i.begin_token.borrow().begin_char == item.begin_token.borrow().begin_char
            });
            idx.map_or(true, |i| {
                vpt.items[i+1..].iter().all(|j| j.is_adverb)
            })
        };
        if item.normal == "БЫТЬ" && !is_last {
            continue;
        }

        // Build normal text from verb_morph or normal field
        let normal = item.normal.clone();
        let normal_full = {
            if let Some(wf) = item.verb_morph() {
                wf.normal_full.clone()
                    .or_else(|| wf.normal_case.clone())
                    .unwrap_or_else(|| normal.clone())
            } else {
                normal.clone()
            }
        };

        let obj_typ = if item.is_participle() {
            SemObjectType::Participle
        } else {
            SemObjectType::Verb
        };

        let mut sem = SemObject::new();
        sem.typ        = obj_typ;
        sem.normal     = normal.clone();
        sem.normal_full = normal_full;
        sem.not        = item.not;
        sem.begin_char = item.begin_token.borrow().begin_char as usize;
        sem.end_char   = item.end_token.borrow().end_char as usize;
        let sem_ref = gr.add_object(sem);

        // Attach pending adverbs as Detail links
        for adv_ref in pending_adverbs.drain(..) {
            gr.add_link(
                SemLinkType::Detail,
                sem_ref.clone(),
                adv_ref,
                Some("как".to_string()),
                false,
                None,
            );
        }

        verb_sems.push(sem_ref);
    }

    if verb_sems.is_empty() { return None; }

    // Attach remaining adverbs to last verb
    let last = verb_sems.last().unwrap().clone();
    for adv_ref in pending_adverbs {
        gr.add_link(
            SemLinkType::Detail,
            last.clone(),
            adv_ref,
            Some("как".to_string()),
            false,
            None,
        );
    }

    // Chain multiple verbs: v[n-1] --Detail("что делать?")--> v[n]
    for i in 1..verb_sems.len() {
        gr.add_link(
            SemLinkType::Detail,
            verb_sems[i-1].clone(),
            verb_sems[i].clone(),
            Some("что делать?".to_string()),
            false,
            None,
        );
    }

    Some(verb_sems[0].clone())
}
