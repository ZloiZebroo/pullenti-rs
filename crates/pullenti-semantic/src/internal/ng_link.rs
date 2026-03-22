/// NGLink — candidate semantic relationship between sentence items.
/// Mirrors `NGLink.cs` and `NGLinkType.cs`.
///
/// Link scoring uses `SemanticService.Params` defaults (all 1.0 or 2.0 as in AlgoParams.cs).

use pullenti_morph::{MorphCase, MorphNumber, MorphGenderFlags, MorphVoice, MorphWordForm, MorphLang};
use pullenti_morph::MorphMood;
use pullenti_ner::deriv::{find_verb_role, SemanticRole};
use super::sent_item::{SentItem, SentItemType, NounMorph, VerbMorphInfo};

// ── AlgoParams defaults (from AlgoParams.cs) ─────────────────────────────
const TRANSITIVE_COEF: f64 = 1.0;
const NG_LINK: f64 = 1.0;
const LIST: f64 = 2.0;
const VERB_PLURAL: f64 = 2.0;
const MORPH_ACCORD: f64 = 1.0;

// ── NGLinkType ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NGLinkType {
    #[default]
    Undefined,
    Agent,
    Pacient,
    Actant,
    Genetive,
    Name,
    Be,
    List,
    Participle,
    Adverb,
}

// ── NGLink ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NGLink {
    pub typ:              NGLinkType,
    /// index of the "from" item within its segment's items vec
    pub from_ord:         usize,
    /// index of the "to" item within the same segment's items vec (None if to_verb)
    pub to_ord:           Option<usize>,
    /// index of the verb in the *sentence* items vec (None if to_noun)
    pub to_verb_sent_idx: Option<usize>,
    pub coef:             f64,
    pub plural:           i32,   // -1 = unknown, 0 = singular, 1 = plural
    pub from_is_plural:   bool,
    pub reverce:          bool,
    pub to_all_list_items: bool,
    pub can_be_pacient:   bool,
    pub can_be_participle: bool,
}

impl Default for NGLink {
    fn default() -> Self {
        NGLink {
            typ:              NGLinkType::Undefined,
            from_ord:         0,
            to_ord:           None,
            to_verb_sent_idx: None,
            coef:             -1.0,
            plural:           -1,
            from_is_plural:   false,
            reverce:          false,
            to_all_list_items: false,
            can_be_pacient:   false,
            can_be_participle: false,
        }
    }
}

impl NGLink {
    /// Calculate the score for this link given the sentence items and segment items.
    /// `seg_items` is the list of SentItem indices in this segment (from the sentence),
    /// `sent_items` is the full sentence item list.
    pub fn calc_coef(
        &mut self,
        seg_items: &[usize],   // indices of SentItem in the segment
        sent_items: &[SentItem],
        noplural: bool,
    ) {
        self.coef = -1.0;
        self.can_be_pacient = false;
        self.to_all_list_items = false;
        self.plural = -1;

        match self.typ {
            NGLinkType::Genetive   => self.calc_genetive(seg_items, sent_items),
            NGLinkType::Name       => self.calc_name(seg_items, sent_items, noplural),
            NGLinkType::Be         => self.calc_be(seg_items, sent_items),
            NGLinkType::List       => self.calc_list(seg_items, sent_items),
            NGLinkType::Participle => self.calc_participle(seg_items, sent_items, noplural),
            NGLinkType::Agent      => self.calc_agent(seg_items, sent_items, noplural),
            NGLinkType::Pacient    => self.calc_pacient(seg_items, sent_items, noplural),
            NGLinkType::Actant     => self.calc_actant(seg_items, sent_items),
            NGLinkType::Adverb     => {
                // to verb
                if self.to_verb_sent_idx.is_some() {
                    self.coef = 1.0;
                } else if let Some(to_ord) = self.to_ord {
                    let to_idx = seg_items[to_ord];
                    if sent_items[to_idx].typ == SentItemType::Adverb {
                        self.coef = 1.0;
                    } else {
                        self.coef = 0.5;
                    }
                }
            }
            NGLinkType::Undefined  => {}
        }
    }

    fn from_nm<'a>(&self, seg_items: &[usize], sent_items: &'a [SentItem]) -> Option<&'a NounMorph> {
        seg_items.get(self.from_ord)
            .and_then(|&idx| sent_items.get(idx))
            .and_then(|si| si.noun_morph.as_ref())
    }

    fn to_nm<'a>(&self, seg_items: &[usize], sent_items: &'a [SentItem]) -> Option<&'a NounMorph> {
        self.to_ord.as_ref()
            .and_then(|&ord| seg_items.get(ord))
            .and_then(|&idx| sent_items.get(idx))
            .and_then(|si| si.noun_morph.as_ref())
    }

    fn to_verb_info<'a>(&self, sent_items: &'a [SentItem]) -> Option<&'a VerbMorphInfo> {
        self.to_verb_sent_idx.as_ref()
            .and_then(|&idx| sent_items.get(idx))
            .and_then(|si| si.verb_morph.as_ref())
    }

    fn from_prep<'a>(&self, seg_items: &[usize], sent_items: &'a [SentItem]) -> &'a str {
        seg_items.get(self.from_ord)
            .and_then(|&idx| sent_items.get(idx))
            .map(|si| si.prep.as_str())
            .unwrap_or("")
    }

    fn from_si<'a>(&self, seg_items: &[usize], sent_items: &'a [SentItem]) -> Option<&'a SentItem> {
        seg_items.get(self.from_ord).and_then(|&idx| sent_items.get(idx))
    }

    fn to_si<'a>(&self, seg_items: &[usize], sent_items: &'a [SentItem]) -> Option<&'a SentItem> {
        self.to_ord.as_ref()
            .and_then(|&ord| seg_items.get(ord))
            .and_then(|&idx| sent_items.get(idx))
    }

    fn calc_genetive(&mut self, seg_items: &[usize], sent_items: &[SentItem]) {
        let from_si = match self.from_si(seg_items, sent_items) {
            Some(s) => s,
            None    => return,
        };
        if !from_si.can_be_noun() { return; }
        let from_nm = match from_si.noun_morph.as_ref() {
            Some(m) => m,
            None    => return,
        };
        let from_prep = from_si.prep.as_str();

        let to_si = match self.to_si(seg_items, sent_items) {
            Some(s) => s,
            None    => return,
        };

        // Non-adjacent without preposition: skip
        let from_ord = self.from_ord;
        let to_ord   = self.to_ord.unwrap();
        let non_gen  = from_ord != to_ord + 1 && from_prep.is_empty();
        if non_gen && !from_prep.is_empty() { return; }
        if non_gen { return; }  // simplified: require adjacency for plain genitive
        if !from_prep.is_empty() { return; }  // prepositioned items use Actant, not Genetive

        let cas = from_nm.case;
        if cas.is_genitive() || cas.is_instrumental() || cas.value != 0 {
            // has some case
            if cas.is_genitive() {
                self.coef = NG_LINK;
                if cas.is_nominative() || from_si.typ == SentItemType::PartBefore {
                    self.coef /= 2.0;
                }
            } else if cas.is_instrumental() || cas == MorphCase::DATIVE {
                self.coef = NG_LINK / 2.0;
            } else if cas.value == 0 {
                // undefined case
                self.coef = 0.0;
            }
        }
    }

    fn calc_be(&mut self, seg_items: &[usize], sent_items: &[SentItem]) {
        let from_si = match self.from_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        let to_si = match self.to_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        if from_si.typ != SentItemType::Noun || to_si.typ != SentItemType::Noun { return; }
        let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
        let to_nm   = match to_si.noun_morph.as_ref()   { Some(m) => m, None => return };
        if !to_nm.is_nominative() { return; }
        if !from_si.prep.is_empty() { return; }
        if !from_nm.case.is_undefined() && !from_nm.is_nominative() { return; }
        self.coef = 0.0;
    }

    fn calc_name(&mut self, seg_items: &[usize], sent_items: &[SentItem], noplural: bool) {
        let from_si = match self.from_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        let to_si = match self.to_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        if !from_si.prep.is_empty() { return; }
        if from_si.typ != SentItemType::Noun || to_si.typ != SentItemType::Noun { return; }
        let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
        let to_nm   = match to_si.noun_morph.as_ref()   { Some(m) => m, None => return };
        if from_nm.is_lower { return; }  // must start uppercase
        let from_ord = self.from_ord;
        let to_ord = self.to_ord.unwrap();
        if from_ord != to_ord + 1 && !noplural { return; }
        // Check case intersection
        if !from_nm.case.is_undefined() && !to_nm.case.is_undefined() {
            if (from_nm.case & to_nm.case) == MorphCase::UNDEFINED { return; }
        }
        if from_nm.number == MorphNumber::PLURAL {
            if noplural {
                if !self.from_is_plural {
                    let n_inter = MorphNumber(from_nm.number.0 & to_nm.number.0);
                    if n_inter != MorphNumber::SINGULAR { return; }
                }
            }
            self.plural = 1;
            self.coef = VERB_PLURAL;
        } else {
            if from_nm.number == MorphNumber::SINGULAR { self.plural = 0; }
            if check_morph_accord(from_nm, false, to_nm) {
                self.coef = MORPH_ACCORD;
            }
        }
    }

    fn calc_list(&mut self, seg_items: &[usize], sent_items: &[SentItem]) {
        let from_si = match self.from_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        // to can be verb (to_verb_sent_idx) or noun
        if let Some(to_ord) = self.to_ord {
            let to_si = &sent_items[seg_items[to_ord]];
            // Type compatibility
            let types_ok = from_si.typ == to_si.typ
                || (matches!(from_si.typ, SentItemType::Noun | SentItemType::PartBefore | SentItemType::PartAfter)
                    && matches!(to_si.typ, SentItemType::Noun | SentItemType::PartBefore | SentItemType::PartAfter)
                    && from_si.prep == to_si.prep);
            if !types_ok { return; }
            let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
            let to_nm   = match to_si.noun_morph.as_ref()   { Some(m) => m, None => return };
            let cas_inter = from_nm.case & to_nm.case;
            if !cas_inter.is_undefined() {
                self.coef = LIST;
                if from_si.prep.is_empty() && !to_si.prep.is_empty() { self.coef /= 2.0; }
                else if !from_si.prep.is_empty() && to_si.prep.is_empty() { self.coef /= 4.0; }
            } else {
                if !from_nm.case.is_undefined() && !to_nm.case.is_undefined() { return; }
                if !from_si.prep.is_empty() && to_si.prep.is_empty() { return; }
                self.coef = LIST;
            }
            if from_si.typ != to_si.typ { self.coef /= 2.0; }
        }
        // to_verb case — list with verb head (similar items)
        // simplified: always valid
    }

    fn calc_participle(&mut self, seg_items: &[usize], sent_items: &[SentItem], noplural: bool) {
        let from_si = match self.from_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        let to_si = match self.to_si(seg_items, sent_items) {
            Some(s) => s, None => return,
        };
        if to_si.typ == SentItemType::PartBefore { self.coef = -1.0; return; }
        if from_si.typ != SentItemType::PartBefore && from_si.typ != SentItemType::SubSent
            && from_si.typ != SentItemType::Deepart { self.coef = -1.0; return; }
        let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
        let to_nm   = match to_si.noun_morph.as_ref()   { Some(m) => m, None => return };
        if from_si.typ == SentItemType::Deepart {
            if !from_si.prep.is_empty() { self.coef = -1.0; return; }
            if to_nm.is_nominative() { self.coef = MORPH_ACCORD; return; }
            if to_nm.case.is_undefined() { self.coef = 0.0; return; }
            self.coef = -1.0; return;
        }
        if !from_nm.case.is_undefined() && !to_nm.case.is_undefined() {
            if (from_nm.case & to_nm.case) == MorphCase::UNDEFINED {
                if from_si.typ == SentItemType::PartBefore { self.coef = -1.0; return; }
            }
        }
        if from_nm.number == MorphNumber::PLURAL {
            if noplural {
                if !self.from_is_plural {
                    let n_inter = MorphNumber(from_nm.number.0 & to_nm.number.0);
                    if n_inter != MorphNumber::PLURAL { self.coef = -1.0; return; }
                }
            }
            self.plural = 1;
            self.coef = VERB_PLURAL;
        } else {
            if from_nm.number == MorphNumber::SINGULAR { self.plural = 0; }
            if check_morph_accord_items(&from_nm.items, to_nm) {
                self.coef = MORPH_ACCORD;
            }
        }
    }

    fn calc_agent(&mut self, seg_items: &[usize], sent_items: &[SentItem], noplural: bool) {
        let from_prep = {
            let from_si = match self.from_si(seg_items, sent_items) {
                Some(s) => s, None => return,
            };
            from_si.prep.clone()
        };
        if !from_prep.is_empty() { self.coef = -1.0; return; }

        let vmi = match self.to_verb_info(sent_items) {
            Some(v) => v, None => return,
        };
        let vf = match vmi.word_form.as_ref() { Some(wf) => wf, None => return };

        let is_imperative = vf.misc.as_ref().map_or(false, |m| m.mood() == MorphMood::Imperative);
        if is_imperative { self.coef = -1.0; return; }
        if vmi.is_infinitive { self.coef = -1.0; return; }

        let from_si = self.from_si(seg_items, sent_items).unwrap();
        let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
        let morph = from_nm;

        // Passive voice — agent is instrumental
        if vmi.voice == MorphVoice::Passive || vmi.is_passive_str {
            if !morph.case.is_undefined() {
                if morph.is_instrumental() {
                    self.coef = TRANSITIVE_COEF;
                    return;
                }
                self.coef = -1.0; return;
            }
            self.coef = 0.0; return;
        }

        // Reflexive verb: no subject
        if is_rev_verb(vf) { self.coef = -1.0; return; }

        if vf.base.number == MorphNumber::PLURAL {
            if !morph.case.is_undefined() {
                if vf.base.case.is_undefined() {
                    if !morph.is_nominative() { self.coef = -1.0; return; }
                } else if (vf.base.case & morph.case) == MorphCase::UNDEFINED {
                    self.coef = -1.0; return;
                }
            }
            if noplural {
                if !self.from_is_plural {
                    let n_inter = MorphNumber(morph.number.0 & MorphNumber::PLURAL.0);
                    if n_inter != MorphNumber::PLURAL { self.coef = -1.0; return; }
                }
            }
            self.plural = 1;
            self.coef = VERB_PLURAL;
        } else {
            if vf.base.number == MorphNumber::SINGULAR {
                self.plural = 0;
                if self.from_is_plural { self.coef = -1.0; return; }
            }
            // Check morph agreement
            if !check_morph_accord_wf(morph, false, vf) { self.coef = -1.0; return; }
            if !morph.case.is_undefined() {
                if !morph.is_nominative() { self.coef = -1.0; return; }
            }
            self.coef = MORPH_ACCORD;
            if morph.case.is_undefined() { self.coef /= 4.0; }
        }
    }

    fn calc_pacient(&mut self, seg_items: &[usize], sent_items: &[SentItem], noplural: bool) {
        let from_prep = {
            let from_si = match self.from_si(seg_items, sent_items) {
                Some(s) => s, None => return,
            };
            from_si.prep.clone()
        };
        if !from_prep.is_empty() { self.coef = -1.0; return; }

        let vmi = match self.to_verb_info(sent_items) {
            Some(v) => v, None => return,
        };
        let vf = match vmi.word_form.as_ref() { Some(wf) => wf, None => return };

        let from_si = self.from_si(seg_items, sent_items).unwrap();
        let from_nm = match from_si.noun_morph.as_ref() { Some(m) => m, None => return };
        let morph = from_nm;

        // Reflexive verbs (возвр.) used as passive: nominative noun is the logical Pacient.
        // Approximates C# SemanticHelper.TryCreateLinks: when IsVerbReversive, nominative
        // case → Pacient role (e.g. "система разрабатывается программистом").
        // Positional rule: noun BEFORE the verb → Pacient; noun AFTER → -1 (excluded).
        // C# applies sl.Rank -= 0.5 for post-verb items; we go further and exclude them
        // to prevent cross-segment conflicts with the true pre-verb subject/patient.
        if vmi.is_reflexive && morph.is_nominative() {
            let noun_sent_idx = seg_items.get(self.from_ord).copied().unwrap_or(0);
            let after_verb = self.to_verb_sent_idx
                .map_or(false, |v_idx| noun_sent_idx > v_idx);
            if after_verb {
                // Not a patient — nominative after reflexive verb is not the passive subject
                return; // coef stays -1.0
            }
            self.coef = MORPH_ACCORD;
            return;
        }

        // Only valid for actual passive voice (страд.з.) — NOT for reflexive (возвр.).
        // C# NGLink._calcPacient checks Voice==Passive||ContainsAttr("страд.з.") only;
        // reflexive verbs (is_passive_str due to "возвр.") return -1 for Pacient.
        let is_actual_passive = vmi.voice == MorphVoice::Passive
            || (vmi.is_passive_str && !vmi.is_reflexive);

        if is_actual_passive {
            if vf.base.number == MorphNumber::PLURAL {
                if noplural {
                    if !self.from_is_plural {
                        if !check_morph_accord_wf(morph, false, vf) { return; }
                    }
                }
                self.coef = VERB_PLURAL;
                self.plural = 1;
            } else {
                if vf.base.number == MorphNumber::SINGULAR {
                    self.plural = 0;
                    if self.from_is_plural { return; }
                }
                if !check_morph_accord_wf(morph, false, vf) { return; }
                self.coef = MORPH_ACCORD;
            }
        }
        // Control model lookup (mirrors C# SemanticHelper.TryCreateLinks → _createRoles):
        // For active non-reflexive verbs, use the derivate dictionary to determine if
        // this prep+case combination maps to Pacient according to the verb's control model.
        else if !vmi.is_reflexive {
            if let Some(lemma) = &vmi.lemma {
                let prep_opt = if from_prep.is_empty() { None } else { Some(from_prep.as_str()) };
                if let Some(SemanticRole::Pacient) = find_verb_role(lemma, false, MorphLang::RU, prep_opt, morph.case) {
                    self.coef = MORPH_ACCORD; // strong model-based Pacient
                    return;
                }
            }
            // Fallback: accusative case is patient (weak heuristic for verbs not in dict)
            if morph.is_accusative() || morph.case.is_undefined() {
                self.coef = MORPH_ACCORD / 2.0;
            }
        }
    }

    fn calc_actant(&mut self, seg_items: &[usize], sent_items: &[SentItem]) {
        let from_prep = {
            let from_si = match self.from_si(seg_items, sent_items) {
                Some(s) => s, None => return,
            };
            from_si.prep.clone()
        };
        if from_prep.is_empty() { self.coef = 0.0; return; }
        // prepositioned actant — moderate score
        self.coef = 0.1;
    }
}

// ── Morphological agreement helpers ──────────────────────────────────────

fn is_rev_verb(vf: &MorphWordForm) -> bool {
    if vf.misc.as_ref().map_or(false, |m| m.attrs.iter().any(|a| a == "возвр.")) { return true; }
    if let Some(ref nc) = vf.normal_case {
        if nc.ends_with("СЯ") || nc.ends_with("СЬ") { return true; }
    }
    false
}

/// Check morph agreement between a noun's morph and a target morph (generic).
pub fn check_morph_accord(from: &NounMorph, plural: bool, to: &NounMorph) -> bool {
    check_morph_accord_base(from.number, from.gender, plural, to.number, to.case)
}

pub fn check_morph_accord_wf(from: &NounMorph, plural: bool, vf: &MorphWordForm) -> bool {
    check_morph_accord_base(from.number, from.gender, plural, vf.base.number, vf.base.case)
}

pub fn check_morph_accord_items(items: &[MorphWordForm], to: &NounMorph) -> bool {
    items.iter().any(|wf| check_morph_accord_base(wf.base.number, wf.base.gender, false, to.number, to.case))
}

fn check_morph_accord_base(
    from_num: MorphNumber,
    from_gen: MorphGenderFlags,
    plural:   bool,
    to_num:   MorphNumber,
    to_case:  MorphCase,
) -> bool {
    let _ = to_case; // simplified: only check number/gender
    if to_num == MorphNumber::PLURAL {
        if plural { return true; }
        if from_num != MorphNumber::UNDEFINED {
            if (MorphNumber(from_num.0 & MorphNumber::PLURAL.0)) == MorphNumber::PLURAL {
                return true;
            } else {
                return false;
            }
        }
    } else if to_num == MorphNumber::SINGULAR {
        if plural { return false; }
        if from_num != MorphNumber::UNDEFINED {
            if (MorphNumber(from_num.0 & MorphNumber::SINGULAR.0)) == MorphNumber::SINGULAR {
                // Check gender agreement
                if from_gen != MorphGenderFlags::UNDEFINED && to_num == MorphNumber::SINGULAR {
                    // simplified: skip gender check in this helper
                }
                return true;
            } else {
                return false;
            }
        }
    }
    true
}
