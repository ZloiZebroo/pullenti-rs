/// SentItem — sentence element with morphological snapshot.
/// Mirrors `SentItem.cs` and `SentItemType.cs`.

use pullenti_morph::{MorphCase, MorphNumber, MorphGenderFlags, MorphWordForm, MorphVoice};
use pullenti_ner::core::noun_phrase::{NounPhraseToken, NounPhraseParseAttr, try_parse as npt_try_parse};
use pullenti_ner::core::verb_phrase::{VerbPhraseToken, try_parse as vpt_try_parse};
use pullenti_ner::core::conjunction::{ConjunctionToken, ConjunctionType, try_parse as cnj_try_parse};
use pullenti_ner::token::TokenRef;
use pullenti_ner::source_of_analysis::SourceOfAnalysis;
use super::adverb_token::{self, AdverbToken};
use super::delim_token::{self, DelimToken};

// ── NounPhraseParseAttr flags ─────────────────────────────────────────────

pub const NPT_ATTRS: NounPhraseParseAttr = NounPhraseParseAttr::from_bits_truncate(
    NounPhraseParseAttr::AdjectiveCanBeLast.bits()
    | NounPhraseParseAttr::IgnoreBrackets.bits()
    | NounPhraseParseAttr::ParseAdverbs.bits()
    | NounPhraseParseAttr::ParseNumericAsAdj.bits()
    | NounPhraseParseAttr::ParsePreposition.bits()
    | NounPhraseParseAttr::ParsePronouns.bits()
    | NounPhraseParseAttr::ParseVerbs.bits()
    | NounPhraseParseAttr::ReferentCanBeNoun.bits()
    | NounPhraseParseAttr::MultiNouns.bits()
);

// ── SentItemType ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentItemType {
    Undefined,
    Noun,
    Verb,
    Adverb,
    Conj,
    Delim,
    PartBefore,   // participle before its noun
    PartAfter,    // participle after its noun
    Deepart,      // deeparticiple (деепричастие)
    SubSent,      // sub-sentence (который, что, etc.)
    Formula,      // quantity formula (N раз)
}

// ── Cached morph snapshot for nouns ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NounMorph {
    pub case:   MorphCase,
    pub number: MorphNumber,
    pub gender: MorphGenderFlags,
    pub items:  Vec<MorphWordForm>,
    /// true if the surface word starts with lowercase (used for noun guards)
    pub is_lower: bool,
}

impl NounMorph {
    pub fn is_undefined(&self) -> bool {
        self.case == MorphCase::UNDEFINED
    }
    pub fn is_nominative(&self) -> bool {
        self.case.is_nominative()
    }
    pub fn is_genitive(&self) -> bool {
        self.case.is_genitive()
    }
    pub fn is_accusative(&self) -> bool {
        self.case.is_accusative()
    }
    pub fn is_instrumental(&self) -> bool {
        self.case.is_instrumental()
    }
    pub fn intersect_case(&self, other: &NounMorph) -> MorphCase {
        self.case & other.case
    }
    pub fn intersect_number(&self, other: &NounMorph) -> MorphNumber {
        MorphNumber(self.number.0 & other.number.0)
    }
}

fn extract_noun_morph(npt: &mut NounPhraseToken) -> NounMorph {
    let case   = npt.morph.case();
    let number = npt.morph.number();
    let gender = npt.morph.gender();
    let items  = npt.morph.items().to_vec();
    let is_lower = npt.begin_token.borrow().chars.is_all_lower();
    NounMorph { case, number, gender, items, is_lower }
}

// ── VerbMorph snapshot for verbs ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VerbMorphInfo {
    /// specific word form (from VerbMorph)
    pub word_form: Option<MorphWordForm>,
    /// voice from morph_collection
    pub voice: MorphVoice,
    /// true if has "инф." attribute (infinitive)
    pub is_infinitive: bool,
    /// true if has "страд.з." or is passive (reflexive-passive)
    pub is_passive_str: bool,
    /// true if has "возвр." attribute (reflexive)
    pub is_reflexive: bool,
    /// true if is dee-participle (деепричастие)
    pub is_deeparticiple: bool,
    /// verb lemma (normal_full or normal_case from VerbMorph), for control model lookup
    pub lemma: Option<String>,
}

fn extract_verb_morph(vpt: &VerbPhraseToken) -> VerbMorphInfo {
    let fv = vpt.first_verb();
    let lv = vpt.last_verb();
    let word_form = fv.and_then(|fv| fv.verb_morph()).cloned();
    let voice = {
        let lv_voice = lv.map_or(MorphVoice::Undefined, |lv| lv.verb_voice());
        if lv_voice == MorphVoice::Undefined {
            fv.map_or(MorphVoice::Undefined, |fv| fv.verb_voice())
        } else {
            lv_voice
        }
    };
    let is_infinitive = word_form.as_ref().map_or(false, |wf| {
        wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "инф."))
    });
    let is_passive_str = fv.map_or(false, |fv| {
        fv.verb_morph().map_or(false, |wf| {
            wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "страд.з." || a == "возвр."))
        })
    }) || voice == MorphVoice::Passive;
    let is_reflexive = fv.map_or(false, |fv| {
        fv.verb_morph().map_or(false, |wf| {
            wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "возвр."))
        })
    });
    // Deeparticiple (деепричастие) detection: check "дееприч." attr
    let is_deeparticiple = fv.map_or(false, |fv| {
        fv.morph.items().iter().any(|wf| {
            wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "дееприч."))
        })
    });
    // Verb lemma: prefer normal_full (infinitive), fall back to normal_case
    let lemma = lv.and_then(|lv| lv.verb_morph())
        .map(|wf| wf.normal_full.clone().or_else(|| wf.normal_case.clone()))
        .flatten()
        .or_else(|| {
            fv.and_then(|fv| fv.verb_morph())
                .map(|wf| wf.normal_full.clone().or_else(|| wf.normal_case.clone()))
                .flatten()
        });
    VerbMorphInfo { word_form, voice, is_infinitive, is_passive_str, is_reflexive, is_deeparticiple, lemma }
}

// ── SentItem source ───────────────────────────────────────────────────────

pub enum SentItemSource {
    Noun(NounPhraseToken),
    Verb(VerbPhraseToken),
    Adverb(AdverbToken),
    Conj(ConjunctionToken),
    Delim(DelimToken),
}

// ── SentItem ──────────────────────────────────────────────────────────────

pub struct SentItem {
    pub source:     SentItemSource,
    pub typ:        SentItemType,
    /// preposition normal form (empty if none)
    pub prep:       String,
    /// cached noun morphology (Some for Noun items)
    pub noun_morph: Option<NounMorph>,
    /// cached verb morphology (Some for Verb items)
    pub verb_morph: Option<VerbMorphInfo>,
}

impl SentItem {
    pub fn from_noun_npt(mut npt: NounPhraseToken) -> Self {
        let prep = npt.preposition.as_ref().map(|p| p.normal.clone()).unwrap_or_default();
        let nm = extract_noun_morph(&mut npt);

        // Relative pronouns (КОТОРЫЙ, ЧЕЙ, etc.) act as sub-sentence markers,
        // not standalone noun phrases.  Tag them as SubSent so that:
        //   - calc_list returns early for them (no comma-List link to preceding noun)
        //   - they get Participle links instead (КОТОРЫЙ comma-links to its antecedent)
        // This prevents КОТОРЫЙ's spurious List link from outscoring the Agent link of
        // the antecedent (e.g. МУЖЧИНА → Agent of РАБОТАЕТ in "Мужчина, который работает").
        let typ = if let Some(ref noun) = npt.noun {
            let is_personal = noun.morph.items().iter()
                .any(|wf| wf.base.class.is_personal_pronoun());
            let is_relative = !is_personal && noun.morph.items().iter().any(|wf| {
                let nf = wf.normal_full.as_deref()
                    .or(wf.normal_case.as_deref())
                    .unwrap_or("");
                nf.starts_with("КОТОР") || nf == "ЧЕЙ" || nf == "ЧЬЕГО"
            });
            if is_relative { SentItemType::SubSent } else { SentItemType::Noun }
        } else {
            SentItemType::Noun
        };

        SentItem {
            source: SentItemSource::Noun(npt),
            typ,
            prep,
            noun_morph: Some(nm),
            verb_morph: None,
        }
    }

    pub fn from_verb_vpt(vpt: VerbPhraseToken) -> Self {
        let prep = vpt.preposition.as_ref().map(|p| p.normal.clone()).unwrap_or_default();
        let is_deepart = vpt.first_verb().map_or(false, |fv| {
            fv.morph.items().iter().any(|wf| {
                wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "дееприч."))
            })
        });
        let is_participle = vpt.first_verb().map_or(false, |fv| fv.is_participle());
        let vm = extract_verb_morph(&vpt);
        let typ = if is_deepart {
            SentItemType::Deepart
        } else if is_participle {
            SentItemType::PartBefore
        } else {
            SentItemType::Verb
        };
        SentItem {
            source: SentItemSource::Verb(vpt),
            typ,
            prep,
            noun_morph: None,
            verb_morph: Some(vm),
        }
    }

    pub fn from_adverb(adv: AdverbToken) -> Self {
        SentItem {
            source: SentItemSource::Adverb(adv),
            typ: SentItemType::Adverb,
            prep: String::new(),
            noun_morph: None,
            verb_morph: None,
        }
    }

    pub fn from_conj(cnj: ConjunctionToken) -> Self {
        SentItem {
            source: SentItemSource::Conj(cnj),
            typ: SentItemType::Conj,
            prep: String::new(),
            noun_morph: None,
            verb_morph: None,
        }
    }

    pub fn from_delim(dlm: DelimToken) -> Self {
        SentItem {
            source: SentItemSource::Delim(dlm),
            typ: SentItemType::Delim,
            prep: String::new(),
            noun_morph: None,
            verb_morph: None,
        }
    }

    pub fn begin_char(&self) -> i32 {
        match &self.source {
            SentItemSource::Noun(npt)  => npt.begin_token.borrow().begin_char,
            SentItemSource::Verb(vpt)  => vpt.begin_token.borrow().begin_char,
            SentItemSource::Adverb(adv) => adv.begin_char(),
            SentItemSource::Conj(cnj)  => cnj.begin_token.borrow().begin_char,
            SentItemSource::Delim(dlm) => dlm.begin_char(),
        }
    }

    pub fn end_char(&self) -> i32 {
        match &self.source {
            SentItemSource::Noun(npt)  => npt.end_token.borrow().end_char,
            SentItemSource::Verb(vpt)  => vpt.end_token.borrow().end_char,
            SentItemSource::Adverb(adv) => adv.end_char(),
            SentItemSource::Conj(cnj)  => cnj.end_token.borrow().end_char,
            SentItemSource::Delim(dlm) => dlm.end_char(),
        }
    }

    pub fn begin_token(&self) -> TokenRef {
        match &self.source {
            SentItemSource::Noun(npt)  => npt.begin_token.clone(),
            SentItemSource::Verb(vpt)  => vpt.begin_token.clone(),
            SentItemSource::Adverb(adv) => adv.begin_token.clone(),
            SentItemSource::Conj(cnj)  => cnj.begin_token.clone(),
            SentItemSource::Delim(dlm) => dlm.begin_token.clone(),
        }
    }

    pub fn end_token(&self) -> TokenRef {
        match &self.source {
            SentItemSource::Noun(npt)  => npt.end_token.clone(),
            SentItemSource::Verb(vpt)  => vpt.end_token.clone(),
            SentItemSource::Adverb(adv) => adv.end_token.clone(),
            SentItemSource::Conj(cnj)  => cnj.end_token.clone(),
            SentItemSource::Delim(dlm) => dlm.end_token.clone(),
        }
    }

    pub fn can_be_noun(&self) -> bool {
        matches!(
            self.typ,
            SentItemType::Noun
            | SentItemType::Deepart
            | SentItemType::PartAfter
            | SentItemType::PartBefore
            | SentItemType::SubSent
            | SentItemType::Formula
        )
    }

    pub fn can_be_comma_end(&self) -> bool {
        if let SentItemSource::Conj(cnj) = &self.source {
            return matches!(
                cnj.typ,
                ConjunctionType::Comma | ConjunctionType::And | ConjunctionType::Or
            );
        }
        false
    }

    pub fn is_conj_or_type(&self, typ: ConjunctionType) -> bool {
        if let SentItemSource::Conj(cnj) = &self.source {
            return cnj.typ == typ;
        }
        false
    }

    pub fn noun_phrase(&self) -> Option<&NounPhraseToken> {
        if let SentItemSource::Noun(npt) = &self.source {
            Some(npt)
        } else {
            None
        }
    }

    pub fn verb_phrase(&self) -> Option<&VerbPhraseToken> {
        if let SentItemSource::Verb(vpt) = &self.source {
            Some(vpt)
        } else {
            None
        }
    }
}

// ── parse_sent_items ──────────────────────────────────────────────────────

/// Parse [t0..t1] into SentItem list (replaces parse_variants for internal use).
pub fn parse_sent_items(
    t0:   &TokenRef,
    t1:   &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Vec<SentItem> {
    let mut items = Vec::new();
    let t1_end = t1.borrow().end_char;

    let mut cur = Some(t0.clone());
    while let Some(t) = cur.take() {
        if t.borrow().end_char > t1_end { break; }

        // Skip parenthesised spans
        if t.borrow().is_char('(', sofa) {
            let mut inner = t.borrow().next.clone();
            let mut found_close = false;
            while let Some(it) = inner.take() {
                if it.borrow().end_char > t1_end { break; }
                if it.borrow().is_char(')', sofa) {
                    cur = it.borrow().next.clone();
                    found_close = true;
                    break;
                }
                inner = it.borrow().next.clone();
            }
            if found_close { continue; }
            cur = t.borrow().next.clone();
            continue;
        }

        // 1. Delimiter
        if let Some(dlm) = delim_token::try_parse(&t, sofa) {
            let next = dlm.end_token.borrow().next.clone();
            items.push(SentItem::from_delim(dlm));
            cur = next;
            continue;
        }

        // 2. Conjunction
        if let Some(cnj) = cnj_try_parse(&t, sofa) {
            let next = cnj.end_token.borrow().next.clone();
            items.push(SentItem::from_conj(cnj));
            cur = next;
            continue;
        }

        // 3. Noun phrase
        let npt_opt = npt_try_parse(&t, NPT_ATTRS, 0, sofa);

        // 4. Adverb
        let adv_opt = adverb_token::try_parse(&t, sofa);

        // 5. Verb phrase
        let vpt_opt = vpt_try_parse(&t, true, false, false, sofa);

        // Resolve noun vs adverb ambiguity (take longer span)
        let use_npt = match (&npt_opt, &adv_opt) {
            (Some(npt), Some(adv)) => adv.end_char() <= npt.end_token.borrow().end_char,
            (Some(_), None)        => true,
            _                      => false,
        };

        if use_npt {
            let npt = npt_opt.unwrap();
            let vpt_is_participle = vpt_opt.as_ref()
                .and_then(|v| v.first_verb())
                .map_or(false, |fv| fv.is_participle());
            if npt.adjectives.is_empty() || !vpt_is_participle {
                let next = npt.end_token.borrow().next.clone();
                items.push(SentItem::from_noun_npt(npt));
                cur = next;
                continue;
            }
        }

        // Try verb
        if let Some(vpt) = vpt_opt {
            let first_is_deepart = vpt.first_verb().map_or(false, |fv| {
                fv.morph.items().iter().any(|wf| {
                    wf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "дееприч."))
                })
            });
            let first_is_part    = vpt.first_verb().map_or(false, |fv| fv.is_participle());
            // Accept all verbs including participles at the sentence level
            if !first_is_part || first_is_deepart {
                let next = vpt.end_token.borrow().next.clone();
                items.push(SentItem::from_verb_vpt(vpt));
                cur = next;
                continue;
            }
        }

        // Try adverb
        if let Some(adv) = adv_opt {
            let next = adv.end_token.borrow().next.clone();
            items.push(SentItem::from_adverb(adv));
            cur = next;
            continue;
        }

        // Skip token
        cur = t.borrow().next.clone();
    }

    items
}
