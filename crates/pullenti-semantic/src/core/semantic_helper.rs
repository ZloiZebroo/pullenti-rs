/// SemanticHelper — utility functions for semantic analysis.
/// Mirrors `SemanticHelper.cs` and `SemanticRole.cs`.
///
/// The main entry point is `try_create_links(master, slave)` which mirrors
/// `SemanticHelper.TryCreateLinks(master, slave, onto=null)`.

use pullenti_morph::{MorphCase, MorphNumber, MorphGenderFlags, MorphLang, MorphWordForm};
use pullenti_ner::deriv::deriv_service::find_groups_cloned;
use pullenti_ner::deriv::control_model::{
    ControlModelItem, ControlModelItemType, SemanticRole, items as cmq_items,
    IDX_BASE_NOM, IDX_BASE_GEN, IDX_BASE_ACC, IDX_BASE_INS, IDX_BASE_DAT,
};
use pullenti_ner::deriv::deriv_group::DerivateGroup;

use crate::core::semantic_link::SemanticLink;
use crate::internal::sent_item::{SentItem, SentItemSource, SentItemType, VerbMorphInfo, NounMorph};

// ── GetKeyword ────────────────────────────────────────────────────────────────

/// Get the canonical keyword (lemma) from a SentItem for derivate lookup.
pub fn get_keyword(si: &SentItem) -> Option<String> {
    match &si.source {
        SentItemSource::Verb(vpt) => {
            let lv = vpt.last_verb()?;
            let wf = lv.verb_morph()?;
            wf.normal_full.clone().or_else(|| wf.normal_case.clone())
        }
        SentItemSource::Noun(npt) => {
            let head = match &npt.noun {
                Some(ns) => ns.end_token.clone(),
                None => npt.end_token.clone(),
            };
            let tb = head.borrow();
            for wf in tb.morph.items() {
                if !wf.base.class.is_noun() { continue; }
                let norm = wf.normal_full.as_deref().or(wf.normal_case.as_deref());
                if let Some(s) = norm { if !s.is_empty() { return Some(s.to_string()); } }
            }
            for wf in tb.morph.items() {
                let norm = wf.normal_full.as_deref().or(wf.normal_case.as_deref());
                if let Some(s) = norm { if !s.is_empty() { return Some(s.to_string()); } }
            }
            None
        }
        _ => None,
    }
}

// ── FindControlItem ───────────────────────────────────────────────────────────

/// Find the ControlModelItem within a group that applies to this SentItem.
fn find_control_item<'a>(si: &SentItem, gr: &'a DerivateGroup) -> Option<&'a ControlModelItem> {
    match &si.source {
        SentItemSource::Verb(vpt) => {
            let lv = vpt.last_verb()?;
            // is_reflexive: check "возвр." in attrs
            let is_rev = lv.verb_morph().map_or(false, |wf| {
                wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "возвр."))
            });
            if is_rev {
                gr.model.items.iter()
                    .find(|it| it.typ == ControlModelItemType::Reflexive)
                    .or_else(|| gr.model.items.iter().find(|it| it.typ == ControlModelItemType::Verb))
            } else {
                gr.model.items.iter().find(|it| it.typ == ControlModelItemType::Verb)
            }
        }
        SentItemSource::Noun(npt) => {
            let head = match &npt.noun {
                Some(ns) => ns.end_token.clone(),
                None => npt.end_token.clone(),
            };
            let tb = head.borrow();
            // Check if head noun is a verbal noun in this group
            let term_up = tb.term().unwrap_or("").to_uppercase();
            for w in &gr.words {
                if w.attrs.is_verb_noun() && is_value_match_term(&term_up, w.spelling.as_str(), &tb) {
                    return gr.model.items.iter().find(|it| it.typ == ControlModelItemType::Noun);
                }
            }
            // Check control model items with matching word
            for cit in &gr.model.items {
                if let Some(ref w) = cit.word {
                    if is_value_match_term(&term_up, w, &tb) { return Some(cit); }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_value_match_term(
    term_upper: &str,
    word: &str,
    tb: &std::cell::Ref<'_, pullenti_ner::token::Token>,
) -> bool {
    if term_upper.eq_ignore_ascii_case(word) { return true; }
    for wf in tb.morph.items() {
        let norm = wf.normal_full.as_deref().or(wf.normal_case.as_deref());
        if let Some(n) = norm {
            if n.eq_ignore_ascii_case(word) { return true; }
        }
    }
    false
}

// ── FindWordInGroup ───────────────────────────────────────────────────────────

fn find_word_in_group_for_noun<'a>(
    npt: &pullenti_ner::core::noun_phrase::NounPhraseToken,
    gr: &'a DerivateGroup,
) -> Option<&'a pullenti_ner::deriv::deriv_word::DerivateWord> {
    let head = match &npt.noun {
        Some(ns) => ns.end_token.clone(),
        None => npt.end_token.clone(),
    };
    let tb = head.borrow();
    let term_upper = tb.term().unwrap_or("").to_uppercase();
    for w in &gr.words {
        if w.class.is_noun() && (w.lang.is_undefined() || w.lang.is_ru()) {
            if is_value_match_term(&term_upper, &w.spelling, &tb) {
                return Some(w);
            }
        }
    }
    None
}

// ── _createRoles ──────────────────────────────────────────────────────────────

/// Apply the control model item's links for prep+case → fill `res`.
/// Mirrors `_createRoles(cit, prep, cas, res, ignoreNomCase, ignoreInstrCase)`.
fn create_roles(
    cit: &ControlModelItem,
    prep: Option<&str>,
    cas: MorphCase,
    res: &mut Vec<SemanticLink>,
    no_nomin: bool,
    no_instr: bool,
) {
    let qs = cmq_items();
    let mut roles: Vec<(usize, SemanticRole)> = Vec::new();

    for (&qi, &role) in &cit.links {
        let q = match qs.get(qi) { Some(q) => q, None => continue };
        if !q.check(prep, cas) { continue; }
        if no_nomin && q.case.is_nominative() && q.preposition.is_none() { continue; }
        if no_instr && q.case.is_instrumental() && q.preposition.is_none() { continue; }

        // For abstract questions, use qi as-is (role becomes Common per C# CheckAbstract logic)
        let (final_qi, final_role) = if q.is_abstract { (qi, SemanticRole::Common) } else { (qi, role) };

        if !roles.iter().any(|(rqi, _)| *rqi == final_qi) {
            roles.push((final_qi, final_role));
        } else if final_role != SemanticRole::Common {
            if let Some(entry) = roles.iter_mut().find(|(rqi, _)| *rqi == final_qi) {
                entry.1 = final_role;
            }
        }
    }

    for (qi, role) in roles {
        let q = &qs[qi];
        let mut sl = SemanticLink::new();
        sl.role = role;
        sl.rank = 2.0;
        sl.question = Some(q.spelling.clone());
        if role == SemanticRole::Agent && !q.is_base {
            sl.role = SemanticRole::Common;
        }
        if sl.role == SemanticRole::Strong { sl.rank += 2.0; }
        res.push(sl);
    }
}

// ── CheckMorphAccord ──────────────────────────────────────────────────────────

/// Check morphological agreement between noun morph and verb form.
/// Mirrors `CheckMorphAccord(m, plural, vf, checkCase=false)`.
pub fn check_morph_accord(m: &NounMorph, plural: bool, vf: &MorphWordForm, check_case: bool) -> bool {
    if check_case && !m.case.is_undefined() && !vf.base.case.is_undefined() {
        if (m.case & vf.base.case).is_undefined() { return false; }
    }
    let mut coef: f64 = 0.0;
    let vf_num = vf.base.number;
    let vf_gender = vf.base.gender;

    if vf_num == MorphNumber::PLURAL {
        if plural {
            coef += 1.0;
        } else if m.number != MorphNumber::UNDEFINED {
            if (m.number & MorphNumber::PLURAL) == MorphNumber::PLURAL {
                coef += 1.0;
            } else {
                return false;
            }
        }
    } else if vf_num == MorphNumber::SINGULAR {
        if plural { return false; }
        if m.number != MorphNumber::UNDEFINED {
            if (m.number & MorphNumber::SINGULAR) == MorphNumber::SINGULAR {
                coef += 1.0;
            } else {
                return false;
            }
        }
        if m.gender != MorphGenderFlags::UNDEFINED {
            if vf_gender != MorphGenderFlags::UNDEFINED {
                if m.gender == MorphGenderFlags::FEMINIE {
                    if (vf_gender & MorphGenderFlags::FEMINIE) != MorphGenderFlags::UNDEFINED {
                        coef += 1.0;
                    } else {
                        return false;
                    }
                } else if (m.gender & vf_gender) != MorphGenderFlags::UNDEFINED {
                    coef += 1.0;
                } else if m.gender == MorphGenderFlags::MASCULINE && vf_gender == MorphGenderFlags::FEMINIE {
                    // allowed
                } else {
                    return false;
                }
            }
        }
    }
    coef >= 0.0
}

// ── _tryCreateInf ─────────────────────────────────────────────────────────────

fn try_create_inf(cit: Option<&ControlModelItem>, res: &mut Vec<SemanticLink>) {
    let qs = cmq_items();
    let todo_q = &qs[5]; // IDX_TODO
    let role = cit.and_then(|c| c.links.get(&5).copied());
    if cit.is_some() && role.is_none() { return; }
    let r = role.unwrap_or(SemanticRole::Common);
    let mut sl = SemanticLink::new();
    sl.rank = if r != SemanticRole::Common { 2.0 } else { 1.0 };
    sl.question = Some(todo_q.spelling.clone());
    res.push(sl);
}

// ── _tryCreateVerb ────────────────────────────────────────────────────────────

fn try_create_verb(
    vmi: &VerbMorphInfo,
    master_begin: i32,
    slave_prep: Option<&str>,
    slave_morph: &NounMorph,
    slave_begin: i32,
    gr: Option<&DerivateGroup>,
    slave_head_normal: Option<&str>,
    res: &mut Vec<SemanticLink>,
) {
    let qs = cmq_items();
    let prep = slave_prep;
    let cas = slave_morph.case;

    let cit = gr.and_then(|g| {
        let is_rev = vmi.is_reflexive;
        if is_rev {
            g.model.items.iter()
                .find(|it| it.typ == ControlModelItemType::Reflexive)
                .or_else(|| g.model.items.iter().find(|it| it.typ == ControlModelItemType::Verb))
        } else {
            g.model.items.iter().find(|it| it.typ == ControlModelItemType::Verb)
        }
    });

    let is_rev1 = vmi.is_reflexive;
    let mut no_nomin = false;
    let mut no_instr = false;

    // ── Nominative case: morph agreement → Agent/Pacient ──────────────────
    if prep.is_none() && cas.is_nominative() {
        if let Some(wf) = &vmi.word_form {
            let mut ok = true;
            if wf.base.number == MorphNumber::SINGULAR && slave_morph.number == MorphNumber::PLURAL {
                ok = false;
            }
            if ok && !check_morph_accord(slave_morph, false, wf, false) { ok = false; }
            no_nomin = true;
            if ok {
                let mut sl = SemanticLink::new();
                sl.modelled = cit.is_none();
                sl.role = if is_rev1 { SemanticRole::Pacient } else { SemanticRole::Agent };
                sl.is_passive = is_rev1;
                sl.rank = 1.0;
                sl.question = Some(qs[IDX_BASE_NOM].spelling.clone());
                if cas.is_accusative() { sl.rank -= 0.5; }
                if slave_begin > master_begin { sl.rank -= 0.5; }
                res.push(sl);
            }
        } else {
            no_nomin = true;
        }
    }

    // ── Instrumental + reflexive: passive logical subject ──────────────────
    if prep.is_none() && is_rev1 && cas.is_instrumental() {
        no_instr = true;
        let mut sl = SemanticLink::new();
        let mut found = false;
        if let Some(c) = cit {
            for (&qi, &role) in &c.links {
                let q = match qs.get(qi) { Some(q) => q, None => continue };
                if q.check(None, MorphCase::INSTRUMENTAL) {
                    sl.role = role;
                    sl.rank = 2.0;
                    sl.question = Some(q.spelling.clone());
                    if sl.role == SemanticRole::Agent { sl.is_passive = true; }
                    found = true;
                    break;
                }
            }
        }
        if !found {
            sl.modelled = true;
            sl.role = SemanticRole::Agent;
            sl.is_passive = true;
            sl.rank = 1.0;
            sl.question = Some(qs[IDX_BASE_INS].spelling.clone());
        }
        if cas.is_nominative() { sl.rank -= 0.5; }
        if cas.is_accusative() { sl.rank -= 0.5; }
        if slave_begin < master_begin { sl.rank -= 0.5; }

        // Check if verb dict item says instrumental→role → insert at head
        let verb_role_ins = gr.and_then(|g| {
            g.model.items.iter()
                .find(|it| it.typ == ControlModelItemType::Verb)
                .and_then(|vc| vc.links.get(&IDX_BASE_INS).copied())
        });
        if let Some(vr) = verb_role_ins {
            let q_str = sl.question.clone();
            res.push(sl);
            let last_idx = res.len() - 1;
            res[last_idx].rank = 0.0;
            let mut sl0 = SemanticLink::new();
            sl0.question = q_str;
            sl0.rank = 1.0;
            sl0.role = vr;
            res.insert(0, sl0);
        } else {
            res.push(sl);
        }
    }

    // ── Dative fallback ────────────────────────────────────────────────────
    if prep.is_none() && cas.is_dative() {
        let cit_has_dat = cit.map_or(false, |c| c.links.contains_key(&IDX_BASE_DAT));
        if !cit_has_dat {
            let mut sl = SemanticLink::new();
            sl.modelled = cit.is_none();
            sl.role = SemanticRole::Strong;
            sl.rank = 1.0;
            sl.question = Some(qs[IDX_BASE_DAT].spelling.clone());
            if cit.is_some() { sl.rank -= 0.5; }
            sl.rank -= 0.3; // simplified: assume not adjacent
            res.push(sl);
        }
    }

    // ── Control model roles ────────────────────────────────────────────────
    if let Some(c) = cit {
        create_roles(c, prep, cas, res, no_nomin, no_instr);
    }

    // ── Idiom: model.pacients list ─────────────────────────────────────────
    if let Some(g) = gr {
        if !g.model.pacients.is_empty() {
            if let Some(head_norm) = slave_head_normal {
                let head_up = head_norm.to_uppercase();
                if g.model.pacients.iter().any(|p| p.eq_ignore_ascii_case(&head_up)) {
                    if res.is_empty() {
                        let idx = if is_rev1 { IDX_BASE_NOM } else { IDX_BASE_ACC };
                        let mut sl = SemanticLink::new();
                        sl.role = SemanticRole::Pacient;
                        sl.question = Some(qs[idx].spelling.clone());
                        sl.idiom = true;
                        res.push(sl);
                    } else {
                        for sl in res.iter_mut() {
                            sl.rank += 4.0;
                            if sl.role == SemanticRole::Common { sl.role = SemanticRole::Strong; }
                            sl.idiom = true;
                        }
                    }
                }
            }
        }
    }
}

// ── _tryCreateNoun ────────────────────────────────────────────────────────────

fn try_create_noun(
    slave_prep: Option<&str>,
    slave_morph: &NounMorph,
    gr: Option<&DerivateGroup>,
    slave_head_normal: Option<&str>,
    master_npt: &pullenti_ner::core::noun_phrase::NounPhraseToken,
    master_si: &SentItem,
    res: &mut Vec<SemanticLink>,
) {
    let qs = cmq_items();
    let prep = slave_prep;
    let cas = slave_morph.case;

    let cit = gr.and_then(|g| find_control_item(master_si, g));

    if let Some(c) = cit {
        create_roles(c, prep, cas, res, false, false);

        // If only Agent+Instrumental → downgrade to verb dict's instrumental role
        if res.len() == 1 && res[0].role == SemanticRole::Agent
            && res[0].question.as_deref() == Some(qs[IDX_BASE_INS].spelling.as_str())
        {
            if let Some(g) = gr {
                if let Some(&vr) = g.model.items.iter()
                    .find(|it| it.typ == ControlModelItemType::Verb)
                    .and_then(|vc| vc.links.get(&IDX_BASE_INS))
                {
                    res[0].role = vr;
                }
            }
        }
    }

    // Idiom: NextWords / model.pacients
    if let Some(shn) = slave_head_normal {
        let shn_up = shn.to_uppercase();
        let ok_idiom = gr.map_or(false, |g| {
            let via_next = find_word_in_group_for_noun(master_npt, g)
                .and_then(|w| w.next_words.as_ref())
                .map_or(false, |nws| nws.iter().any(|nw| nw.eq_ignore_ascii_case(&shn_up)));
            let via_pacient = g.model.pacients.iter().any(|p| p.eq_ignore_ascii_case(&shn_up));
            via_next || via_pacient
        });
        if ok_idiom {
            if res.is_empty() {
                let mut sl = SemanticLink::new();
                sl.question = Some(qs[IDX_BASE_GEN].spelling.clone());
                sl.role = SemanticRole::Pacient;
                sl.idiom = true;
                res.push(sl);
            } else {
                for sl in res.iter_mut() {
                    sl.rank += 4.0;
                    if sl.role == SemanticRole::Common { sl.role = SemanticRole::Strong; }
                    sl.idiom = true;
                }
            }
        }
    }
}

// ── try_create_links (public) ─────────────────────────────────────────────────

/// Compute semantic link candidates between a master phrase (verb or noun)
/// and a slave noun phrase. Mirrors `SemanticHelper.TryCreateLinks(master, slave)`.
///
/// Returns links sorted descending by rank (best first).
pub fn try_create_links(master: &SentItem, slave: &SentItem) -> Vec<SemanticLink> {
    let mut res: Vec<SemanticLink> = Vec::new();

    // Slave must be a noun phrase (or we can't determine its morphology)
    let slave_nm = match slave.noun_morph.as_ref() {
        Some(nm) => nm,
        None => return res,
    };
    let slave_prep: Option<&str> = if slave.prep.is_empty() { None } else { Some(&slave.prep) };
    let slave_begin = slave.begin_char();
    let master_begin = master.begin_char();

    // Get slave head word normal form for idiom checks
    let slave_head_normal: Option<String> = slave.noun_phrase().and_then(|npt| {
        let head = match &npt.noun {
            Some(ns) => ns.end_token.clone(),
            None => npt.end_token.clone(),
        };
        let tb = head.borrow();
        tb.morph.items().iter()
            .filter(|wf| wf.base.class.is_noun())
            .find_map(|wf| wf.normal_full.clone().or_else(|| wf.normal_case.clone()))
    });

    match &master.source {
        // ── Verb master ───────────────────────────────────────────────────
        SentItemSource::Verb(_) => {
            let vmi = match master.verb_morph.as_ref() { Some(v) => v, None => return res };
            let lemma = vmi.lemma.as_deref().unwrap_or("");
            let groups = if lemma.is_empty() { Vec::new() }
                         else { find_groups_cloned(lemma, true, MorphLang::RU) };

            // Slave is verb infinitive → _tryCreateInf
            if slave.typ == SentItemType::Verb
                && slave.verb_morph.as_ref().map_or(false, |vm| vm.is_infinitive)
            {
                if groups.is_empty() {
                    try_create_inf(None, &mut res);
                } else {
                    for gr in &groups {
                        let cit = find_control_item(master, gr);
                        try_create_inf(cit, &mut res);
                    }
                }
                return sort_and_finalize(res);
            }

            if groups.is_empty() {
                try_create_verb(vmi, master_begin, slave_prep, slave_nm,
                    slave_begin, None, slave_head_normal.as_deref(), &mut res);
            } else {
                for gr in &groups {
                    let mut local = Vec::new();
                    try_create_verb(vmi, master_begin, slave_prep, slave_nm,
                        slave_begin, Some(gr), slave_head_normal.as_deref(), &mut local);
                    res.extend(local);
                }
            }
        }

        // ── Noun master ───────────────────────────────────────────────────
        SentItemSource::Noun(npt) => {
            let kw = get_keyword(master).unwrap_or_default();
            let groups = if kw.is_empty() { Vec::new() }
                         else { find_groups_cloned(&kw, true, MorphLang::RU) };

            if groups.is_empty() {
                try_create_noun(slave_prep, slave_nm, None,
                    slave_head_normal.as_deref(), npt, master, &mut res);
            } else {
                for gr in &groups {
                    let mut local = Vec::new();
                    try_create_noun(slave_prep, slave_nm, Some(gr),
                        slave_head_normal.as_deref(), npt, master, &mut local);
                    res.extend(local);
                }
            }

            // Genitive fallback
            if slave_prep.is_none() && slave_nm.case.is_genitive() {
                let qs = cmq_items();
                let has_gen = res.iter().any(|r| {
                    r.question.as_deref() == Some(qs[IDX_BASE_GEN].spelling.as_str())
                });
                let is_pronoun = npt.noun.as_ref()
                    .map(|ns| {
                        let tb = ns.begin_token.borrow();
                        tb.morph.items().iter().any(|wf| wf.base.class.is_personal_pronoun())
                    })
                    .unwrap_or(false);
                if !has_gen && !is_pronoun {
                    let mut sl = SemanticLink::new();
                    sl.modelled = true;
                    sl.rank = 0.5;
                    sl.question = Some(qs[IDX_BASE_GEN].spelling.clone());
                    res.push(sl);
                }
            }

            // Pronoun/anafor: downgrade genitive links
            let is_pronoun = npt.noun.as_ref()
                .map(|ns| {
                    let tb = ns.begin_token.borrow();
                    tb.morph.items().iter().any(|wf| wf.base.class.is_pronoun())
                })
                .unwrap_or(false);
            if is_pronoun {
                let qs = cmq_items();
                for sl in res.iter_mut() {
                    if sl.question.as_deref() == Some(qs[IDX_BASE_GEN].spelling.as_str()) {
                        sl.rank -= 0.5;
                        if sl.role == SemanticRole::Strong { sl.role = SemanticRole::Common; }
                    }
                }
            }
        }

        _ => return res,
    }

    sort_and_finalize(res)
}

fn sort_and_finalize(mut res: Vec<SemanticLink>) -> Vec<SemanticLink> {
    // Strong boost: halve non-Strong ranks
    if res.iter().any(|r| r.role == SemanticRole::Strong) {
        for sl in res.iter_mut() {
            if sl.role != SemanticRole::Strong { sl.rank /= 2.0; }
        }
    }
    // Sort descending by rank (best first, as per C# CompareTo which returns -1 for rank > other)
    res.sort_by(|a, b| b.rank.partial_cmp(&a.rank).unwrap_or(std::cmp::Ordering::Equal));
    res
}
