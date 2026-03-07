/// VerbPhraseToken, VerbPhraseItemToken, and VerbPhraseHelper.
/// Mirrors `VerbPhraseToken.cs`, `VerbPhraseItemToken.cs`, `VerbPhraseHelper.cs`.

use pullenti_morph::{MorphVoice, MorphWordForm};
use crate::token::{Token, TokenRef, TokenKind};
use crate::morph_collection::MorphCollection;
use crate::source_of_analysis::SourceOfAnalysis;
use super::preposition::{PrepositionToken, try_parse as prep_try_parse};
use super::misc_helper::can_be_start_of_sentence;
use crate::deriv::deriv_service;

// ── VerbPhraseItemToken ───────────────────────────────────────────────────

#[derive(Clone)]
pub struct VerbPhraseItemToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub morph:       MorphCollection,
    pub not:         bool,
    pub is_adverb:   bool,
    pub normal:      String,
}

impl VerbPhraseItemToken {
    pub fn new(begin: TokenRef, end: TokenRef, morph: MorphCollection) -> Self {
        VerbPhraseItemToken {
            begin_token: begin,
            end_token:   end,
            morph,
            not:       false,
            is_adverb: false,
            normal:    String::new(),
        }
    }

    pub fn is_participle(&self) -> bool {
        for wf in self.morph.items() {
            if wf.base.class.is_adjective() && !wf.contains_attr("к.ф.") {
                return true;
            }
            if wf.base.class.is_verb() && !wf.base.case.is_undefined() {
                return true;
            }
        }
        false
    }

    pub fn verb_voice(&self) -> MorphVoice {
        for wf in self.morph.items() {
            if wf.base.class.is_verb() {
                if let Some(ref misc) = wf.misc {
                    return misc.voice();
                }
            }
        }
        MorphVoice::Undefined
    }

    pub fn verb_morph(&self) -> Option<&MorphWordForm> {
        self.morph.items().iter().find(|wf| wf.base.class.is_verb())
    }
}

// ── VerbPhraseToken ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct VerbPhraseToken {
    pub begin_token:  TokenRef,
    pub end_token:    TokenRef,
    pub morph:        MorphCollection,
    pub items:        Vec<VerbPhraseItemToken>,
    pub preposition:  Option<PrepositionToken>,
}

impl VerbPhraseToken {
    pub fn new(begin: TokenRef, end: TokenRef) -> Self {
        VerbPhraseToken {
            begin_token: begin,
            end_token:   end,
            morph:       MorphCollection::new(),
            items:       Vec::new(),
            preposition: None,
        }
    }

    pub fn first_verb(&self) -> Option<&VerbPhraseItemToken> {
        self.items.iter().find(|it| !it.is_adverb)
    }

    pub fn last_verb(&self) -> Option<&VerbPhraseItemToken> {
        self.items.iter().rev().find(|it| !it.is_adverb)
    }

    pub fn is_verb_passive(&self) -> bool {
        self.first_verb().map_or(false, |fv| fv.verb_voice() == MorphVoice::Passive)
    }
}

// ── VerbPhraseHelper ──────────────────────────────────────────────────────

pub fn try_parse(
    t:                    &TokenRef,
    can_be_partition:     bool,
    can_be_adj_partition: bool,
    force_parse:          bool,
    sofa:                 &SourceOfAnalysis,
) -> Option<VerbPhraseToken> {
    let is_cyrillic = {
        let tb = t.borrow();
        let TokenKind::Text(_) = &tb.kind else { return None; };
        if !tb.chars.is_letter() { return None; }
        tb.chars.is_cyrillic_letter()
    };
    if is_cyrillic {
        try_parse_ru(t, can_be_partition, can_be_adj_partition, force_parse, sofa)
    } else {
        None
    }
}

fn try_parse_ru(
    first:               &TokenRef,
    can_be_partition:    bool,
    can_be_adj_partition: bool,
    force_parse:         bool,
    sofa:                &SourceOfAnalysis,
) -> Option<VerbPhraseToken> {
    let mut res: Option<VerbPhraseToken> = None;
    let t0 = first.clone();
    let mut cur = first.clone();
    let mut not_tok: Option<TokenRef> = None;
    let mut has_verb    = false;
    let mut verb_be_before = false;
    let mut prep_opt: Option<PrepositionToken> = None;

    'main: loop {
        // Read term — if not a text token, stop
        let (term, is_first_tok) = {
            let tb = cur.borrow();
            let TokenKind::Text(ref t) = tb.kind else { break; };
            (t.term.clone(), std::rc::Rc::ptr_eq(&cur, &t0))
        };

        // НЕ — negation particle
        if term == "НЕ" {
            not_tok = Some(cur.clone());
            let next = cur.borrow().next.clone();
            match next { None => break, Some(n) => { cur = n; continue; } }
        }

        let mc = cur.borrow().get_morph_class_in_dictionary();
        let is_pure_verb = cur.borrow().is_pure_verb();
        let is_verb_be   = cur.borrow().is_verb_be();
        let chars_all_lower = cur.borrow().chars.is_all_lower();
        let has_kf = cur.borrow().morph.contains_attr("к.ф.", None);
        let has_inf = cur.borrow().morph.contains_attr("инф.", None);

        // Determine ty (1=verb, 2=adverb, 3=participle-like-adj, 0=stop)
        let mut ty: i32 = 0;
        let mut norm_override: Option<String> = None;

        if term == "НЕТ" {
            if has_verb { break; }
            ty = 1;
        } else if term == "ДОПУСТИМО" {
            ty = 3;
        } else if mc.is_adverb() && !mc.is_verb() {
            ty = 2;
        } else if is_pure_verb || is_verb_be {
            ty = 1;
            if has_verb && !has_inf {
                if !verb_be_before { break; }
            }
        } else if mc.is_verb() {
            if mc.is_preposition() || mc.is_misc() || mc.is_pronoun() {
                // ty = 0
            } else if mc.is_noun() {
                let excl = matches!(
                    term.as_str(),
                    "СТАЛИ"|"СТЕКЛО"|"БЫЛИ"|"ДАМ"|"ПОДАТЬ"|"ГОТОВ"
                );
                if excl {
                    ty = 1;
                } else if !chars_all_lower && !can_be_start_of_sentence(&cur, sofa) {
                    ty = 1;
                } else if mc.is_adjective() && can_be_partition {
                    ty = 1;
                } else if force_parse {
                    ty = 1;
                }
            } else if mc.is_proper() {
                if chars_all_lower { ty = 1; }
            } else {
                ty = 1;
            }

            // Participle check
            let is_part = mc.is_adjective();
            if !can_be_partition && is_part { break; }
            if ty == 1 && has_verb {
                if has_inf {
                    // ok (infinitive can follow)
                } else if !is_part {
                    // ok (plain verb sequence allowed)
                } else {
                    break;
                }
            }
            if ty == 1 && is_part { ty = 1; } // stays 1 but is_participle = true
        } else if mc.is_adjective() && has_kf {
            // Short-form adjective ending in О → adverb-like
            if term.ends_with('О') {
                ty = 2;
            }
        } else if mc.is_adjective() && (can_be_partition || can_be_adj_partition) {
            if has_kf && !can_be_adj_partition { break; }
            let norm_adj = cur.borrow().get_normal_case_text(sofa);
            if !norm_adj.ends_with("ЙШИЙ") {
                let mut h_verb = false;
                let mut h_part = false;
                deriv_service::for_each_word(
                    &norm_adj, true, pullenti_morph::MorphLang::new(),
                    |w, _gid| {
                        if w.class.is_adjective() && w.class.is_verb() && w.spelling == norm_adj {
                            h_part = true;
                        } else if w.class.is_verb() {
                            h_verb = true;
                        }
                    }
                );
                if h_part && h_verb {
                    ty = 3;
                    norm_override = Some(norm_adj);
                } else if can_be_adj_partition {
                    ty = 3;
                    norm_override = Some(norm_adj);
                }
            }
        } else if is_first_tok && can_be_partition {
            // Try to parse preposition
            if let Some(p) = prep_try_parse(&cur, sofa) {
                let end_tok = p.end_token.clone();
                prep_opt = Some(p);
                let next = end_tok.borrow().next.clone();
                cur = end_tok;
                match next { None => break, Some(n) => { cur = n; continue; } }
            }
        }

        if ty == 0 { break; }

        // Build result
        if res.is_none() {
            res = Some(VerbPhraseToken::new(t0.clone(), cur.clone()));
        }
        let vpt = res.as_mut().unwrap();
        vpt.end_token = cur.clone();

        let item_morph = cur.borrow().morph.clone_collection();
        let mut item = VerbPhraseItemToken::new(cur.clone(), cur.clone(), item_morph);

        if let Some(ref nt) = not_tok {
            item.begin_token = nt.clone();
            item.not = true;
            not_tok = None;
        }
        item.is_adverb = ty == 2;

        // Preposition case check (only for first item)
        if let Some(ref prep) = prep_opt {
            if vpt.items.is_empty() {
                let token_case = cur.borrow_mut().morph.case();
                if !token_case.is_undefined() {
                    let prep_next_case = prep.next_case;
                    if (prep_next_case & token_case).is_undefined() {
                        return None;
                    }
                    vpt.preposition = prep_opt.clone();
                }
            }
        }

        item.normal = norm_override.unwrap_or_else(|| cur.borrow().get_normal_case_text(sofa));
        vpt.items.push(item);

        if !has_verb && (ty == 1 || ty == 3) {
            vpt.morph = cur.borrow().morph.clone_collection();
            has_verb = true;
        }
        if ty == 1 || ty == 3 {
            verb_be_before = ty == 1 && is_verb_be;
        }

        let next = cur.borrow().next.clone();
        match next { None => break, Some(n) => cur = n }
    }

    if !has_verb { return None; }

    let vpt = res.as_mut()?;
    // Remove trailing adverbs
    while vpt.items.len() > 1 && vpt.items.last().map_or(false, |it| it.is_adverb) {
        vpt.items.pop();
        if let Some(last) = vpt.items.last() {
            vpt.end_token = last.end_token.clone();
        }
    }

    res
}
