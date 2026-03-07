/// NounPhraseToken, NounPhraseParseAttr, NounPhraseHelper.
/// Mirrors NounPhraseToken.cs, NounPhraseParseAttr.cs, NounPhraseHelper.cs,
///         _NounPraseHelperInt.cs, and Core/Internal/NounPhraseItem.cs.

use std::sync::OnceLock;
use std::rc::Rc;

use pullenti_morph::{MorphBaseInfo, MorphClass, MorphCase, MorphNumber,
                     MorphWordForm, LanguageHelper};

use crate::token::{TokenRef, TokenKind};
use crate::morph_collection::MorphCollection;
use crate::source_of_analysis::SourceOfAnalysis;
use super::preposition::{PrepositionToken, try_parse as preposition_try_parse};
use super::termin::{Termin, TerminCollection};

// ── NounPhraseParseAttr ───────────────────────────────────────────────────

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct NounPhraseParseAttr: u32 {
        const No                = 0;
        const ParsePronouns     = 0x01;
        const ParsePreposition  = 0x02;
        const IgnoreAdjBest     = 0x04;
        const IgnoreParticiples = 0x08;
        const ReferentCanBeNoun = 0x10;
        const CanNotHasCommaAnd = 0x20;
        const AdjectiveCanBeLast= 0x40;
        const ParseAdverbs      = 0x80;
        const ParseVerbs        = 0x100;
        const ParseNumericAsAdj = 0x200;
        const Multilines        = 0x400;
        const IgnoreBrackets    = 0x800;
        const MultiNouns        = 0x1000;
        const ParseNot          = 0x2000;
    }
}

// ── NounPhraseSpan (adjective / noun slot in the public result) ───────────

#[derive(Clone)]
pub struct NounPhraseSpan {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub morph:       MorphCollection,
}

// ── NounPhraseToken (public result) ──────────────────────────────────────

#[derive(Clone)]
pub struct NounPhraseToken {
    pub begin_token:   TokenRef,
    pub end_token:     TokenRef,
    pub morph:         MorphCollection,
    pub noun:          Option<NounPhraseSpan>,
    pub adjectives:    Vec<NounPhraseSpan>,
    pub internal_noun: Option<Box<NounPhraseToken>>,
    pub preposition:   Option<PrepositionToken>,
    pub multi_nouns:   bool,
}

impl NounPhraseToken {
    fn new(begin: TokenRef, end: TokenRef) -> Self {
        NounPhraseToken {
            begin_token:   begin,
            end_token:     end,
            morph:         MorphCollection::new(),
            noun:          None,
            adjectives:    Vec::new(),
            internal_noun: None,
            preposition:   None,
            multi_nouns:   false,
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════
//  Internal types (mirrors C# NounPhraseItemTextVar + NounPhraseItem)
// ═════════════════════════════════════════════════════════════════════════

/// Morphological variant for a noun phrase item.
/// Mirrors `NounPhraseItemTextVar` (extends MorphBaseInfo in C#).
#[derive(Clone, Debug)]
struct NptTextVar {
    base:               MorphBaseInfo,
    normal_value:       Option<String>,
    single_number_value: Option<String>,
    undef_coef:         i16,
}

impl NptTextVar {
    fn from_word_form(wf: &MorphWordForm, tok_normal: Option<String>) -> Self {
        let mut tv = NptTextVar {
            base:               wf.base.clone(),
            normal_value:       wf.normal_case.clone().or(tok_normal),
            single_number_value: None,
            undef_coef:         wf.undef_coef,
        };
        // For plural forms, store the singular nominative in single_number_value
        if wf.base.number == MorphNumber::PLURAL {
            if let Some(ref nf) = wf.normal_full {
                tv.single_number_value = Some(nf.clone());
            }
        }
        // If case is undefined and word is invariable ("неизм."), allow all cases
        if tv.base.case.is_undefined() && wf.contains_attr("неизм.") {
            tv.base.case = MorphCase::ALL_CASES;
        }
        tv
    }

    fn from_base_with_normal(base: MorphBaseInfo, normal: Option<String>) -> Self {
        NptTextVar { base, normal_value: normal, single_number_value: None, undef_coef: 0 }
    }

    /// Agreement check: does `self` (adj form) agree with `other` (noun/adj form)?
    /// Wraps MorphBaseInfo::check_accord.
    fn check_accord(&self, other: &NptTextVar, ignore_gender: bool, multinouns: bool) -> bool {
        self.base.check_accord(&other.base, ignore_gender, multinouns)
    }

    fn check_accord_base(&self, other: &MorphBaseInfo, ignore_gender: bool, multinouns: bool) -> bool {
        self.base.check_accord(other, ignore_gender, multinouns)
    }

    /// Prepend a prefix (from a preceding token) to normal_value/single_number_value.
    fn correct_prefix(&mut self, prefix_term: &str, prefix_normal: Option<&str>) {
        let pfx = prefix_normal.unwrap_or(prefix_term);
        if let Some(ref nv) = self.normal_value.clone() {
            self.normal_value = Some(format!("{}-{}", pfx, nv));
        }
        if let Some(ref sv) = self.single_number_value.clone() {
            self.single_number_value = Some(format!("{}-{}", pfx, sv));
        }
    }

    fn to_word_form(&self) -> MorphWordForm {
        let mut wf = MorphWordForm::new();
        wf.base = self.base.clone();
        wf.normal_case = self.normal_value.clone();
        wf.normal_full = self.single_number_value.clone();
        wf.undef_coef = self.undef_coef;
        wf
    }
}

// ─────────────────────────────────────────────────────────────────────────

/// Internal item in noun phrase parsing.
/// Mirrors C# `NounPhraseItem` (extends MetaToken).
struct NptItem {
    begin_token:       TokenRef,
    end_token:         TokenRef,
    morph:             MorphCollection,
    conj_before:       bool,
    can_be_adj:        bool,
    adj_morph:         Vec<NptTextVar>,
    can_be_noun:       bool,
    noun_morph:        Vec<NptTextVar>,
    multi_nouns:       bool,
    can_be_surname:    bool,
    is_std_adjective:  bool,
    is_doubt_adjective: bool,
}

impl NptItem {
    fn new(begin: TokenRef, end: TokenRef) -> Self {
        let morph = begin.borrow().morph.clone_collection();
        NptItem {
            begin_token:       begin,
            end_token:         end,
            morph,
            conj_before:       false,
            can_be_adj:        false,
            adj_morph:         Vec::new(),
            can_be_noun:       false,
            noun_morph:        Vec::new(),
            multi_nouns:       false,
            can_be_surname:    false,
            is_std_adjective:  false,
            is_doubt_adjective: false,
        }
    }

    fn begin_char(&self) -> i32 { self.begin_token.borrow().begin_char }
    fn end_char(&self) -> i32   { self.end_token.borrow().end_char }

    fn is_pronoun(&self) -> bool {
        self.begin_token.borrow().morph.items().iter()
            .any(|wf| wf.base.class.is_pronoun())
    }
    fn is_personal_pronoun(&self) -> bool {
        self.begin_token.borrow().morph.items().iter()
            .any(|wf| wf.base.class.is_personal_pronoun())
    }
    fn is_verb(&self) -> bool {
        self.begin_token.borrow().morph.items().iter()
            .any(|wf| wf.base.class.is_verb())
    }
    fn is_adverb(&self) -> bool {
        self.begin_token.borrow().morph.items().iter()
            .any(|wf| wf.base.class.is_adverb())
    }

    fn whitespaces_after_count(&self, sofa: &SourceOfAnalysis) -> i32 {
        self.end_token.borrow().whitespaces_before_count(sofa)
            .max(if self.end_token.borrow().is_whitespace_after(sofa) { 1 } else { 0 })
    }

    fn newlines_after_count(&self, sofa: &SourceOfAnalysis) -> i32 {
        if self.end_token.borrow().is_newline_after(sofa) { 1 } else { 0 }
    }

    fn can_be_numeric_adj(&self) -> bool {
        let tb = self.begin_token.borrow();
        if let TokenKind::Number(ref n) = tb.kind {
            if let Ok(val) = n.value.parse::<i64>() {
                return val > 1;
            }
            return false;
        }
        drop(tb);
        self.begin_token.borrow().is_value("НЕСКОЛЬКО", None)
            || self.begin_token.borrow().is_value("МНОГО", None)
            || self.begin_token.borrow().is_value("ПАРА", None)
            || self.begin_token.borrow().is_value("ПОЛТОРА", None)
    }

    fn can_be_adj_for_personal_pronoun(&self) -> bool {
        if self.is_pronoun() && self.can_be_adj {
            return self.begin_token.borrow().is_value("ВСЕ", None)
                || self.begin_token.borrow().is_value("ВЕСЬ", None)
                || self.begin_token.borrow().is_value("САМ", None);
        }
        false
    }

    /// Does any adj_morph form agree with `v`?
    fn try_accord_var(&self, v: &MorphBaseInfo, multinouns: bool) -> bool {
        for vv in &self.adj_morph {
            if vv.check_accord_base(v, false, multinouns) { return true; }
            if vv.normal_value.as_deref() == Some("СКОЛЬКО") { return true; }
        }
        // Numeric adjective special cases
        if self.can_be_numeric_adj() {
            if v.number == MorphNumber::PLURAL { return true; }
            // Genitive case with 2/3/4 endings
            if let TokenKind::Number(ref n) = self.begin_token.borrow().kind {
                if let Ok(val) = n.value.parse::<i64>() {
                    if let Some(ch) = n.value.chars().last() {
                        if (ch == '2' || ch == '3' || ch == '4') && (val < 10 || val > 20) {
                            if v.case.is_genitive() { return true; }
                        }
                    }
                }
            }
        }
        // Personal pronoun (3rd person) acts as adj for next noun
        if !self.adj_morph.is_empty() && self.begin_token.borrow().morph.items().iter()
                .any(|wf| wf.base.class.is_personal_pronoun() && wf.contains_attr("3 л."))
        {
            return true;
        }
        false
    }

    fn to_span(&self) -> NounPhraseSpan {
        NounPhraseSpan {
            begin_token: self.begin_token.clone(),
            end_token:   self.end_token.clone(),
            morph:       self.morph.clone_collection(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────

/// Check that `v` agrees with ALL items[0..count].
fn try_accord_variant(items: &[NptItem], count: usize, v: &MorphBaseInfo, multinouns: bool) -> bool {
    let n = count.min(items.len());
    for i in 0..n {
        if !items[i].try_accord_var(v, multinouns) { return false; }
    }
    true
}

/// Check that adj and noun items agree with each other.
fn try_accord_adj_and_noun(adj: &NptItem, noun: &NptItem) -> bool {
    for v in &adj.adj_morph {
        for vv in &noun.noun_morph {
            if v.check_accord(vv, false, false) { return true; }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────
//  Standard adjectives (СЕВЕРНЫЙ, ЮЖНЫЙ, ...) — used to detect IsStdAdjective
// ─────────────────────────────────────────────────────────────────────────

static STD_ADJECTIVES: OnceLock<TerminCollection> = OnceLock::new();
fn std_adjectives() -> &'static TerminCollection {
    STD_ADJECTIVES.get_or_init(|| {
        let mut tc = TerminCollection::new();
        for s in ["СЕВЕРНЫЙ", "ЮЖНЫЙ", "ЗАПАДНЫЙ", "ВОСТОЧНЫЙ"] {
            tc.add(Termin::new(s));
        }
        tc
    })
}

// ─────────────────────────────────────────────────────────────────────────
//  Skip matching closing bracket from '(' (simplified BracketHelper)
// ─────────────────────────────────────────────────────────────────────────

fn skip_bracket(open: &TokenRef, max_len: i32, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let start_char = open.borrow().begin_char;
    let mut cur = open.borrow().next.clone()?;
    let mut depth = 1i32;
    loop {
        let (bc, ec) = { let tb = cur.borrow(); (tb.begin_char, tb.end_char) };
        if max_len > 0 && ec - start_char > max_len { return None; }
        {
            let tb = cur.borrow();
            if tb.is_char('(', sofa) { depth += 1; }
            else if tb.is_char(')', sofa) {
                depth -= 1;
                if depth == 0 { return Some(cur.clone()); }
            }
        }
        let next = cur.borrow().next.clone()?;
        cur = next;
    }
}

// ═════════════════════════════════════════════════════════════════════════
//  NptItem::try_parse — core item classification
// ═════════════════════════════════════════════════════════════════════════

fn npt_item_try_parse(
    t0:    &TokenRef,
    items: &[NptItem],
    attrs: NounPhraseParseAttr,
    sofa:  &SourceOfAnalysis,
) -> Option<NptItem> {

    // ── ReferentToken ──────────────────────────────────────────────────────
    if matches!(t0.borrow().kind, TokenKind::Referent(_)) {
        if attrs.contains(NounPhraseParseAttr::ReferentCanBeNoun) {
            let mut it = NptItem::new(t0.clone(), t0.clone());
            for wf in t0.borrow().morph.items() {
                it.noun_morph.push(NptTextVar::from_word_form(wf, None));
            }
            it.can_be_noun = true;
            return Some(it);
        }
        return None;
    }

    let tb0 = t0.borrow();
    let is_number_tok = matches!(tb0.kind, TokenKind::Number(_));
    let is_text_tok   = matches!(tb0.kind, TokenKind::Text(_));

    if is_text_tok && !tb0.chars.is_letter() {
        return None;
    }

    // ── Pre-scan for special verb/adverb forms ─────────────────────────────
    let mut has_legal_verb = false;
    let mut is_doubt_adj   = false;
    let mut can_be_surname = false;

    if is_text_tok {
        let term = match &tb0.kind { TokenKind::Text(t) => t.term.as_str(), _ => "" };
        let last_ch = term.chars().last().unwrap_or('\0');
        if last_ch == 'А' || last_ch == 'О' {
            for wf in tb0.morph.items() {
                if wf.is_in_dictionary() {
                    if wf.base.class.is_verb() {
                        let mc1 = tb0.get_morph_class_in_dictionary();
                        if !mc1.is_noun()
                            && !attrs.contains(NounPhraseParseAttr::IgnoreParticiples)
                            && !LanguageHelper::ends_with_ex(term, &["ОГО", "ЕГО"])
                        {
                            return None;
                        }
                        has_legal_verb = true;
                    }
                    if wf.base.class.is_adverb() {
                        let next_is_hiphen = tb0.next.as_ref()
                            .map_or(false, |n| n.borrow().is_hiphen(sofa));
                        if !next_is_hiphen {
                            let ok = matches!(term, "ВСЕГО" | "ДОМА" | "НЕСКОЛЬКО" | "МНОГО" | "ПОРЯДКА");
                            if !ok { return None; }
                        }
                    }
                    if wf.base.class.is_adjective() && wf.contains_attr("к.ф.") {
                        let mc1 = tb0.get_morph_class_in_dictionary();
                        if mc1.value == MorphClass::ADJECTIVE.value
                            && !tb0.morph.contains_attr("неизм.", None)
                        {
                            // ok
                        } else {
                            is_doubt_adj = true;
                        }
                    }
                }
            }
        }
        // Surname / proper-name early-out
        let mc0 = tb0.morph.items().iter()
            .fold(MorphClass::new(), |mut a, wf| { a.value |= wf.base.class.value; a });
        if mc0.is_proper_surname() && !tb0.chars.is_all_lower() {
            for wf in tb0.morph.items() {
                if wf.base.class.is_proper_surname() && wf.base.number != MorphNumber::PLURAL {
                    let s = wf.normal_full.as_deref().or(wf.normal_case.as_deref()).unwrap_or("");
                    if LanguageHelper::ends_with_ex(s, &["ИН", "ЕН", "ЫН"]) {
                        if !wf.is_in_dictionary() { can_be_surname = true; }
                        else { return None; }
                    }
                    if wf.is_in_dictionary() && LanguageHelper::ends_with(s, "ОВ") {
                        can_be_surname = true;
                    }
                }
            }
        }
        if mc0.is_proper_name() && !tb0.chars.is_all_lower() {
            for wf in tb0.morph.items() {
                if wf.base.class.is_proper_name() && wf.is_in_dictionary() {
                    let nc = wf.normal_case.as_deref().unwrap_or("");
                    if nc == "ГОР" || nc == "ГОРЫ" || nc == "ПОЛ" { continue; }
                    if nc.starts_with("ЛЮБ") { continue; }
                    if mc0.is_adjective() && tb0.morph.contains_attr("неизм.", None) { continue; }
                    if attrs.contains(NounPhraseParseAttr::ReferentCanBeNoun) { continue; }
                    if mc0.is_proper_geo() && !attrs.contains(NounPhraseParseAttr::No) { continue; }
                    if tb0.is_value("ПОЛЕ", None) { continue; }
                    if items.is_empty() { return None; }
                    if !items[0].is_std_adjective { return None; }
                }
            }
        }
        // Comparative degree → skip
        if mc0.is_adjective() {
            let items_cnt = tb0.morph.items().iter().count();
            if items_cnt == 1 && tb0.morph.items()[0].contains_attr("в.ср.ст.") {
                return None;
            }
        }
        let mc1 = tb0.get_morph_class_in_dictionary();
        if mc1.value == MorphClass::VERB.value && tb0.morph.items().iter()
                .all(|wf| wf.base.case.is_undefined()) {
            return None;
        }
        // IgnoreParticiples check
        if attrs.contains(NounPhraseParseAttr::IgnoreParticiples)
            && mc0.is_verb() && !mc0.is_noun() && !mc0.is_proper()
        {
            for wf in tb0.morph.items() {
                if wf.base.class.is_verb() && wf.contains_attr("дейст.з.") {
                    let t_term = match &tb0.kind { TokenKind::Text(t) => t.term.as_str(), _ => "" };
                    if !LanguageHelper::ends_with(t_term, "СЯ") { return None; }
                }
            }
        }
    }

    drop(tb0);

    // ── Handle hyphenated compound (e.g., тёмно-синий, бизнес-центр) ──────
    // Check t0.next = '-' and t0.next.next = some word (no whitespace around hyphen)
    let t_end: TokenRef = {
        let mut t_end_cand = t0.clone();
        if is_text_tok {
            // Extract next2 candidate before holding borrows
            let nxt2_opt: Option<TokenRef> = {
                let tb = t0.borrow();
                tb.next.as_ref().and_then(|nxt| {
                    let nb = nxt.borrow();
                    if nb.is_hiphen(sofa) && !tb.is_whitespace_after(sofa) && !nb.is_whitespace_after(sofa) {
                        nb.next.clone()
                    } else {
                        None
                    }
                })
            };
            if let Some(nxt2) = nxt2_opt {
                let ok = {
                    let tb = t0.borrow();
                    let n2b = nxt2.borrow();
                    let same_script = tb.chars.is_cyrillic_letter() == n2b.chars.is_cyrillic_letter();
                    let not_pronoun = !tb.morph.items().iter().any(|wf| wf.base.class.is_pronoun());
                    same_script && not_pronoun && !matches!(n2b.kind, TokenKind::Number(_))
                };
                if ok { t_end_cand = nxt2; }
            }
        }
        t_end_cand
    };

    // ── Build NptItem from t (= t_end for hyphen compounds) ───────────────
    let t = &t_end;
    let mut it = NptItem::new(t0.clone(), t.clone());
    it.can_be_surname = can_be_surname;

    let tb = t.borrow();
    let mut can_be_prepos = false;

    for wf in tb.morph.items() {
        // Verb with case → participial adjective
        if wf.base.class.is_verb() && !wf.base.case.is_undefined() {
            if try_accord_variant(items, items.len(), &wf.base, false) {
                it.adj_morph.push(NptTextVar::from_word_form(wf, None));
                it.can_be_adj = true;
            }
            continue;
        }
        if wf.base.class.is_preposition() { can_be_prepos = true; }

        // ── Adjective / pronoun(non-personal) / noun(number-token) ──────────
        let is_adj_candidate = wf.base.class.is_adjective()
            || (wf.base.class.is_pronoun()
                && !wf.base.class.is_personal_pronoun()
                && !wf.contains_attr("неизм."))
            || (wf.base.class.is_noun() && is_number_tok);

        if is_adj_candidate {
            if try_accord_variant(items, items.len(), &wf.base, false) {
                if wf.contains_attr("к.ф.") { continue; }
                if wf.contains_attr("неизм.") { continue; }
                if wf.contains_attr("сравн.") { continue; }
                // High undef_coef (uncertain form) — skip unless ends with 'Ы'
                let mc0_ok = {
                    let mc0 = tb.morph.items().iter()
                        .fold(MorphClass::new(), |mut a, w| { a.value |= w.base.class.value; a });
                    !mc0.is_undefined()
                };
                if mc0_ok && wf.undef_coef > 0 && wf.undef_coef < 3 {
                    let nc = wf.normal_case.as_deref().unwrap_or("");
                    if !LanguageHelper::ends_with(nc, "Ы") { continue; }
                }
                if wf.contains_attr("собир.") && !is_number_tok {
                    if wf.is_in_dictionary() { return None; }
                    continue;
                }
                // "ПРАВО" / "ПРАВА" should not be adj
                let bad_term = if is_text_tok {
                    let t_term = match &tb.kind { TokenKind::Text(t) => t.term.as_str(), _ => "" };
                    t_term == "ПРАВО" || t_term == "ПРАВА"
                        || (LanguageHelper::ends_with(t_term, "ОВ")
                            && tb.get_morph_class_in_dictionary().is_noun())
                } else { false };
                if bad_term { continue; }

                let mut tv = NptTextVar::from_word_form(wf, None);
                if is_doubt_adj && std::ptr::eq(t.as_ptr(), t0.as_ptr()) {
                    it.is_doubt_adjective = true;
                }
                // If verb+adj combo and in dictionary → also can be noun
                if has_legal_verb && wf.is_in_dictionary() {
                    it.can_be_noun = true;
                }
                // If pronoun in adj position → can also be noun
                if wf.base.class.is_pronoun() {
                    it.can_be_noun = true;
                    it.noun_morph.push(NptTextVar::from_word_form(wf, None));
                }
                it.adj_morph.push(tv);
                it.can_be_adj = true;
            }
        }

        // ── Noun / personal-pronoun / special-pronoun ─────────────────────
        let can_be_noun = if is_number_tok {
            false
        } else if wf.base.class.is_noun()
            || wf.normal_case.as_deref() == Some("САМ")
        {
            true
        } else if wf.base.class.is_personal_pronoun() {
            if items.is_empty() {
                true
            } else {
                let prev_verb = items.iter().any(|it| it.is_verb());
                if prev_verb {
                    items.len() == 1 && !wf.base.case.is_nominative()
                } else if items.len() == 1 && items[0].can_be_adj_for_personal_pronoun() {
                    true
                } else {
                    false
                }
            }
        } else if wf.base.class.is_pronoun() {
            // Special pronouns that can be noun heads
            let nc = wf.normal_case.as_deref().unwrap_or("");
            let nf = wf.normal_full.as_deref().unwrap_or("");
            (nc == "ТОТ" || nf == "ТО" || nc == "ТО" || nc == "ЭТО"
                || nc == "ВСЕ" || nc == "ЧТО" || nc == "КТО"
                || nf == "КОТОРЫЙ" || nc == "КОТОРЫЙ")
                && !{
                    // "ВСЕ РАВНО" → not noun
                    nc == "ВСЕ" && t.borrow().next.as_ref()
                        .map_or(false, |n| n.borrow().is_value("РАВНО", None))
                }
        } else if wf.base.class.is_proper() && is_text_tok {
            tb.length_char() > 4 || wf.base.class.is_proper_name()
        } else {
            false
        };

        if can_be_noun && try_accord_variant(items, items.len(), &wf.base, false) {
            let tv = NptTextVar::from_word_form(wf, None);
            it.noun_morph.push(tv);
            it.can_be_noun = true;
        }
    }
    drop(tb);

    // ── Correct prefix for hyphen compounds ───────────────────────────────
    if !Rc::ptr_eq(t, t0) {
        let prefix_term = match &t0.borrow().kind {
            TokenKind::Text(tx) => tx.term.clone(),
            _ => String::new(),
        };
        let prefix_normal = t0.borrow().morph.items().first()
            .and_then(|wf| wf.normal_case.clone());
        for tv in it.adj_morph.iter_mut() {
            tv.correct_prefix(&prefix_term, prefix_normal.as_deref());
        }
        for tv in it.noun_morph.iter_mut() {
            tv.correct_prefix(&prefix_term, prefix_normal.as_deref());
        }
    }

    // ── STD_ADJECTIVES check ──────────────────────────────────────────────
    if it.can_be_adj {
        if std_adjectives().try_parse(t0).is_some() {
            it.is_std_adjective = true;
        }
    }

    // ── Preposition which is also noun → check if really preposition ──────
    if can_be_prepos && it.can_be_noun && !items.is_empty() {
        // Simplified: just skip deep recursion, assume it's not a preposition in NP context
    }

    // ── Pronoun suffix handling (же, бы, ли, нибудь, либо, то) ───────────
    if (it.can_be_noun || it.can_be_adj) && it.begin_token.borrow().morph.items()
            .iter().any(|wf| wf.base.class.is_pronoun())
    {
        let mut end_ext: Option<TokenRef> = None;
        {
            let itb = it.end_token.borrow();
            if let Some(ref nxt) = itb.next {
                let nb = nxt.borrow();
                let is_hiphen = nb.is_hiphen(sofa) && !nb.is_whitespace_before(sofa) && !nb.is_whitespace_after(sofa);
                drop(nb);
                let check_tok = if is_hiphen {
                    itb.next.as_ref().and_then(|n| n.borrow().next.clone())
                } else {
                    itb.next.clone()
                };
                if let Some(ref ct) = check_tok {
                    let ctb = ct.borrow();
                    if let TokenKind::Text(ref tx) = ctb.kind {
                        match tx.term.as_str() {
                            "ЖЕ" | "БЫ" | "ЛИ" | "Ж" => { end_ext = Some(ct.clone()); }
                            "НИБУДЬ" | "ЛИБО" => {
                                // Append to normal values
                                for tv in it.adj_morph.iter_mut() {
                                    tv.correct_prefix(
                                        &format!("{}-{}", tv.normal_value.as_deref().unwrap_or(""), tx.term),
                                        None,
                                    );
                                }
                                end_ext = Some(ct.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if let Some(ext) = end_ext {
            it.end_token = ext;
        }
    }

    if it.can_be_noun || it.can_be_adj {
        // Update it.morph based on collected variants
        let mut mc = MorphCollection::new();
        for tv in &it.noun_morph {
            mc.add_item(tv.to_word_form());
        }
        if mc.items_count() == 0 {
            for tv in &it.adj_morph {
                mc.add_item(tv.to_word_form());
            }
        }
        if mc.items_count() > 0 { it.morph = mc; }
        return Some(it);
    }

    // БИЗНЕС special case (try to extend to next word)
    if is_text_tok && t0.borrow().is_value("БИЗНЕС", None) {
        if let Some(ref nxt) = t0.borrow().next.clone() {
            let nb = nxt.borrow();
            if nb.chars.is_cyrillic_letter() {
                drop(nb);
                // Return minimal item pointing to next token
                let mut it2 = NptItem::new(t0.clone(), nxt.clone());
                // Use next token's morph
                for wf in nxt.borrow().morph.items() {
                    if wf.base.class.is_noun() {
                        it2.noun_morph.push(NptTextVar::from_word_form(wf, None));
                        it2.can_be_noun = true;
                    }
                    if wf.base.class.is_adjective() {
                        it2.adj_morph.push(NptTextVar::from_word_form(wf, None));
                        it2.can_be_adj = true;
                    }
                }
                if it2.can_be_noun || it2.can_be_adj { return Some(it2); }
            }
        }
    }

    None
}

// ═════════════════════════════════════════════════════════════════════════
//  try_parse_ru — Russian noun phrase parsing
// ═════════════════════════════════════════════════════════════════════════

fn try_parse_ru(
    first:    &TokenRef,
    typ:      NounPhraseParseAttr,
    max_char: i32,
    sofa:     &SourceOfAnalysis,
) -> Option<NounPhraseToken> {

    let mut items: Vec<NptItem> = Vec::new();
    let mut adverbs: Vec<TokenRef> = Vec::new();
    let mut conj_before = false;

    let mut cur: Option<TokenRef> = Some(first.clone());

    while let Some(t) = cur.take() {
        {
            let tb = t.borrow();
            if max_char > 0 && tb.begin_char > max_char { break; }

            // Conjunction (И/ИЛИ) between adjectives
            if tb.morph.items().iter().any(|wf| wf.base.class.is_conjunction())
                && !tb.morph.items().iter().any(|wf| wf.base.class.is_adjective())
                && !tb.morph.items().iter().any(|wf| wf.base.class.is_pronoun())
                && !tb.morph.items().iter().any(|wf| wf.base.class.is_noun())
            {
                if conj_before { break; }
                if typ.contains(NounPhraseParseAttr::CanNotHasCommaAnd) { break; }
                if !items.is_empty() && (tb.is_and(sofa) || tb.is_or(sofa)) {
                    conj_before = true;
                    cur = tb.next.clone();
                    // Skip 'или/(' shorthand
                    continue;
                }
                break;
            }
            // Comma
            if tb.is_comma(sofa) {
                if conj_before || items.is_empty() { break; }
                if typ.contains(NounPhraseParseAttr::CanNotHasCommaAnd) { break; }
                let mc_prev = match &tb.prev {
                    Some(pw) => pw.upgrade().map(|p| p.borrow().get_morph_class_in_dictionary()),
                    None => None,
                };
                if let Some(mc) = mc_prev {
                    if mc.is_proper_surname() || mc.is_proper_secname() { break; }
                }
                if items.last().map_or(false, |it| {
                    it.can_be_noun && it.is_pronoun()
                        && it.begin_token.borrow().morph.items().iter()
                            .any(|wf| wf.base.class.is_personal_pronoun())
                }) {
                    break;
                }
                conj_before = true;
                cur = tb.next.clone();
                continue;
            }
            // Opening bracket → skip to closing ')'
            if tb.is_char('(', sofa) {
                if items.is_empty() { break; }
                drop(tb);
                let closing = skip_bracket(&t, 100, sofa);
                if let Some(cl) = closing {
                    let end_char = cl.borrow().end_char;
                    if end_char - t.borrow().begin_char <= 100 {
                        cur = cl.borrow().next.clone();
                        continue;
                    }
                }
                break;
            }
            // НЕ particle
            if tb.is_value("НЕ", None) && typ.contains(NounPhraseParseAttr::ParseNot) {
                cur = tb.next.clone();
                continue;
            }
            // Skip Latin letters
            if tb.chars.is_latin_letter() { break; }
            // Referent without ReferentCanBeNoun
            if matches!(tb.kind, TokenKind::Referent(_))
                && !typ.contains(NounPhraseParseAttr::ReferentCanBeNoun)
            {
                break;
            }

            // Newline check (for non-Multilines mode)
            let is_newline = tb.is_newline_before(sofa);
            if is_newline && !Rc::ptr_eq(&t, first) {
                if !typ.contains(NounPhraseParseAttr::Multilines) {
                    if let Some(last) = items.last() {
                        let same_chars = tb.chars == last.begin_token.borrow().chars;
                        if !same_chars {
                            let ok = tb.chars.is_all_lower()
                                && last.begin_token.borrow().chars.is_capital_upper();
                            if !ok { break; }
                        }
                    } else if !items.is_empty() { break; }
                }
            }
        }

        // Try to parse item
        let it_opt = npt_item_try_parse(&t, &items, typ, sofa);
        if let Some(it) = it_opt {
            if !it.can_be_adj && !it.can_be_noun {
                // Adverb fallback
                if typ.contains(NounPhraseParseAttr::ParseAdverbs)
                    && t.borrow().morph.items().iter()
                        .any(|wf| wf.base.class.is_adverb() || wf.contains_attr("неизм."))
                {
                    adverbs.push(t.clone());
                    let next = {
                        let tb = t.borrow();
                        if tb.next.as_ref().map_or(false, |n| n.borrow().is_hiphen(sofa)) {
                            tb.next.as_ref().and_then(|n| n.borrow().next.clone())
                        } else {
                            tb.next.clone()
                        }
                    };
                    cur = next;
                    continue;
                }
                break;
            }
            // Newline-after check
            {
                let tb = it.end_token.borrow();
                if tb.is_newline_after(sofa) {
                    let mc = tb.get_morph_class_in_dictionary();
                    if mc.is_proper_surname()
                        || (tb.morph.items().iter().any(|wf| wf.base.class.is_proper_surname())
                            && mc.is_undefined())
                    {
                        break;
                    }
                }
            }
            let next = it.end_token.borrow().next.clone();
            let mut it = it;
            it.conj_before = conj_before;
            conj_before = false;

            // Check if previous item was personal pronoun and this one is another pronoun
            if let Some(prev_it) = items.last() {
                if prev_it.can_be_noun && prev_it.is_personal_pronoun() {
                    if it.is_pronoun() && !it.can_be_adj_for_personal_pronoun() { break; }
                }
            }
            items.push(it);
            cur = next;
        } else {
            // Try adverb
            if typ.contains(NounPhraseParseAttr::ParseAdverbs)
                && t.borrow().morph.items().iter()
                    .any(|wf| wf.base.class.is_adverb() || wf.contains_attr("неизм."))
            {
                adverbs.push(t.clone());
                let next = {
                    let tb = t.borrow();
                    if tb.next.as_ref().map_or(false, |n| n.borrow().is_hiphen(sofa)) {
                        tb.next.as_ref().and_then(|n| n.borrow().next.clone())
                    } else {
                        tb.next.clone()
                    }
                };
                cur = next;
                continue;
            }
            break;
        }
    }

    if items.is_empty() { return None; }

    // ── Find the noun (search from right to left) ─────────────────────────
    let mut noun_idx: Option<usize> = None;
    'outer: for i in (0..items.len()).rev() {
        if !items[i].can_be_noun { continue; }
        if items[i].conj_before { continue; }
        // Skip if a non-adj noun already exists to the left
        let has_noun_left = (0..i).any(|j| items[j].can_be_noun && !items[j].can_be_adj);
        if has_noun_left { continue; }
        // Skip if previous item is a non-adj
        if i > 0 && !items[i - 1].can_be_adj { continue; }
        // Skip if previous item is also a noun (and not doubtful adj)
        if i > 0 && items[i - 1].can_be_noun {
            if items[i - 1].is_doubt_adjective { continue; }
            if items[i - 1].is_pronoun() && items[i].is_pronoun() {
                if items[i - 1].can_be_adj_for_personal_pronoun() {
                    // ok
                } else {
                    continue;
                }
            }
        }
        noun_idx = Some(i);
        break;
    }

    let noun_idx = noun_idx?;

    // Items after noun_idx are adjectives that come AFTER noun (e.g. "член моржовый")
    // Truncate: everything from noun_idx+1 onward is discarded (or kept as adj-after if AdjectiveCanBeLast)
    let mut noun = items.remove(noun_idx);
    // items now contains [0..noun_idx-1] = adjectives
    if noun_idx < items.len() {
        items.truncate(noun_idx);
    }

    // ── Build NounPhraseToken ─────────────────────────────────────────────
    let res_end = noun.end_token.clone();
    let mut res = NounPhraseToken::new(first.clone(), res_end);

    // Build noun morph from noun_morph variants
    let mut noun_mc = MorphCollection::new();
    for tv in &noun.noun_morph {
        noun_mc.add_item(tv.to_word_form());
    }
    if noun_mc.items_count() == 0 {
        noun_mc = noun.begin_token.borrow().morph.clone_collection();
    }
    noun.morph = noun_mc.clone_collection();
    res.morph = noun_mc;

    // Pronoun-only check (if ParsePronouns not set)
    if !typ.contains(NounPhraseParseAttr::ParsePronouns) {
        let is_pron = { let mc = res.morph.items().iter()
            .fold(MorphClass::new(), |mut a, wf| { a.value |= wf.base.class.value; a });
            mc.is_pronoun() || mc.is_personal_pronoun()
        };
        if is_pron && !noun.end_token.borrow().is_value("ДАННЫЙ", None) {
            return None;
        }
    }

    // Remove nominative case from morph if preceded by preposition
    {
        let has_prep_before = noun.begin_token.borrow().prev.as_ref()
            .and_then(|w| w.upgrade())
            .map_or(false, |p| p.borrow().morph.items().iter()
                .any(|wf| wf.base.class.is_preposition()));
        if has_prep_before && res.morph.items().iter().any(|wf| wf.base.case.is_nominative()) {
            res.morph.remove_items_by_case(MorphCase::NOMINATIVE);
        }
    }

    // ── Narrow adjective morph and add adjectives ─────────────────────────
    let mut has_adj_err = false;
    for i in 0..items.len() {
        // Check whitespace between adjectives
        if i + 1 < items.len() {
            let ws = items[i].whitespaces_after_count(sofa);
            if ws > 5 {
                let same_chars = items[i].begin_token.borrow().chars
                    == items[i+1].begin_token.borrow().chars;
                if !same_chars {
                    let next_all_lower = items[i+1].begin_token.borrow().chars.is_all_lower();
                    if !next_all_lower { has_adj_err = true; break; }
                }
                if ws > 10 {
                    has_adj_err = true;
                    break;
                }
            }
        }

        // Narrow adj_morph to those that agree with the noun
        let mut agreed_adj_morph: Vec<NptTextVar> = Vec::new();
        for av in &items[i].adj_morph {
            let agrees = noun.noun_morph.iter()
                .any(|nv| av.check_accord(nv, false, false));
            if agrees { agreed_adj_morph.push(av.clone()); }
        }
        if agreed_adj_morph.is_empty() && !items[i].adj_morph.is_empty() {
            // No agreement found; keep original (might still be useful)
            agreed_adj_morph = items[i].adj_morph.clone();
        }

        // Check IgnoreAdjBest
        let mut err = false;
        let tb = items[i].begin_token.borrow();
        if let TokenKind::Text(ref tx) = tb.kind {
            if !tx.term.starts_with("ВЫСШ") {
                for wf in tb.morph.items() {
                    if wf.base.class.is_adjective() && wf.contains_attr("прев.") {
                        if typ.contains(NounPhraseParseAttr::IgnoreAdjBest) { err = true; }
                    }
                    if wf.base.class.is_adjective() && wf.contains_attr("к.ф.")
                        && tb.morph.items().iter().any(|w| w.base.class.is_personal_pronoun())
                    {
                        return None;
                    }
                }
            }
        }
        drop(tb);
        if err { continue; }

        // Build adj span morph from agreed variants
        let mut adj_mc = MorphCollection::new();
        for tv in &agreed_adj_morph {
            adj_mc.add_item(tv.to_word_form());
        }
        if adj_mc.items_count() == 0 {
            adj_mc = items[i].begin_token.borrow().morph.clone_collection();
        }
        items[i].morph = adj_mc;

        let adj_span = items[i].to_span();
        if adj_span.end_token.borrow().end_char > res.end_token.borrow().end_char {
            res.end_token = adj_span.end_token.clone();
        }

        // Handle pronoun-as-anafor (ignore if ParsePronouns not set)
        let is_pron_adj = items[i].is_pronoun() || items[i].is_personal_pronoun();
        if is_pron_adj && !typ.contains(NounPhraseParseAttr::ParsePronouns) {
            continue;
        }

        res.adjectives.push(adj_span);
    }

    if has_adj_err && !typ.contains(NounPhraseParseAttr::CanNotHasCommaAnd) {
        // Retry with CanNotHasCommaAnd
        return try_parse_ru(first, typ | NounPhraseParseAttr::CanNotHasCommaAnd, max_char, sofa);
    }

    // Validate conjunction/comma usage between adjectives
    if res.adjectives.len() > 1 {
        let mut zap = 0i32;
        let mut and_ = 0i32;
        let mut last_and = false;
        for i in 0..(res.adjectives.len() - 1) {
            let next_tok = res.adjectives[i].end_token.borrow().next.clone();
            if let Some(nt) = next_tok {
                let nb = nt.borrow();
                if nb.is_comma(sofa) { zap += 1; last_and = false; }
                else if nb.is_and(sofa) || nb.is_or(sofa) { and_ += 1; last_and = true; }
            }
        }
        if zap + and_ > 0 {
            let mut err = false;
            if and_ > 1 { err = true; }
            else if and_ == 1 && !last_and { err = true; }
            else if zap > 0 && and_ == 0 { /* ok — only commas */ }
            if err && !typ.contains(NounPhraseParseAttr::CanNotHasCommaAnd) {
                return try_parse_ru(first, typ | NounPhraseParseAttr::CanNotHasCommaAnd, max_char, sofa);
            }
            if err { return None; }
        }
    }

    // Check: single-token result is an adverb → reject
    if Rc::ptr_eq(&res.begin_token, &res.end_token) {
        let mc = res.begin_token.borrow().get_morph_class_in_dictionary();
        let is_adv = mc.is_adverb();
        if is_adv {
            let ok = mc.is_noun() && !mc.is_preposition() && !mc.is_conjunction();
            let is_ves = res.begin_token.borrow().is_value("ВЕСЬ", None);
            let has_prep_before = res.begin_token.borrow().prev.as_ref()
                .and_then(|w| w.upgrade())
                .map_or(false, |p| p.borrow().morph.items().iter()
                    .any(|wf| wf.base.class.is_preposition()));
            if !ok && !is_ves && !has_prep_before {
                return None;
            }
        }
    }

    res.noun = Some(noun.to_span());
    res.multi_nouns = noun.multi_nouns;
    res.begin_token = first.clone();

    Some(res)
}

// ═════════════════════════════════════════════════════════════════════════
//  try_parse_en — English noun phrase parsing (simplified)
// ═════════════════════════════════════════════════════════════════════════

fn try_parse_en(
    first:    &TokenRef,
    typ:      NounPhraseParseAttr,
    max_char: i32,
    sofa:     &SourceOfAnalysis,
) -> Option<NounPhraseToken> {
    use super::misc_helper::is_eng_article;

    let mut items: Vec<NptItem> = Vec::new();
    let mut has_article = false;
    let has_prop = first.borrow().prev.as_ref()
        .and_then(|w| w.upgrade())
        .map_or(false, |p| {
            p.borrow().morph.items().iter()
                .any(|wf| wf.base.class.is_preposition())
                && first.borrow().whitespaces_before_count(sofa) < 3
        });

    let mut cur: Option<TokenRef> = Some(first.clone());
    while let Some(t) = cur.take() {
        {
            let tb = t.borrow();
            if max_char > 0 && tb.begin_char > max_char { break; }
            if !tb.chars.is_latin_letter() { break; }
            if !Rc::ptr_eq(&t, first) && tb.whitespaces_before_count(sofa) > 2 {
                if !typ.contains(NounPhraseParseAttr::Multilines) {
                    if !is_eng_article(&t) { break; }
                }
            }
        }

        // Article
        if Rc::ptr_eq(&t, first) && is_eng_article(&t) {
            has_article = true;
            cur = t.borrow().next.clone();
            continue;
        }

        // ReferentToken
        if matches!(t.borrow().kind, TokenKind::Referent(_)) {
            if !typ.contains(NounPhraseParseAttr::ReferentCanBeNoun) { break; }
        }

        let mc = t.borrow().get_morph_class_in_dictionary();
        if mc.is_conjunction() || mc.is_preposition() { break; }
        if mc.is_pronoun() || mc.is_personal_pronoun() {
            if !typ.contains(NounPhraseParseAttr::ParsePronouns) { break; }
        }

        let mut it = NptItem::new(t.clone(), t.clone());
        if mc.is_noun() || mc.is_undefined() || has_article || has_prop
            || matches!(t.borrow().kind, TokenKind::Referent(_))
        {
            it.can_be_noun = true;
            for wf in t.borrow().morph.items() {
                if !wf.base.class.is_verb() {
                    it.noun_morph.push(NptTextVar::from_word_form(wf, None));
                }
            }
        }
        if mc.is_adjective() || mc.is_pronoun() {
            it.can_be_adj = true;
            for wf in t.borrow().morph.items() {
                it.adj_morph.push(NptTextVar::from_word_form(wf, None));
            }
        }
        if !it.can_be_noun && !it.can_be_adj { break; }

        let next = t.borrow().next.clone();
        items.push(it);
        cur = next;
    }

    if items.is_empty() { return None; }

    let noun = items.remove(items.len() - 1);
    let mut res = NounPhraseToken::new(first.clone(), noun.end_token.clone());

    let mut mc = MorphCollection::new();
    for wf in noun.end_token.borrow().morph.items() {
        if wf.base.class.is_verb() { continue; }
        mc.add_item(wf.clone());
    }
    if mc.items_count() == 0 && has_article {
        let mut wf = MorphWordForm::new();
        wf.base.class.value = MorphClass::NOUN.value;
        wf.base.number = MorphNumber::SINGULAR;
        mc.add_item(wf);
    }
    res.morph = mc;
    res.noun = Some(noun.to_span());

    for it in items {
        res.adjectives.push(it.to_span());
    }
    Some(res)
}

// ═════════════════════════════════════════════════════════════════════════
//  Public entry point
// ═════════════════════════════════════════════════════════════════════════

/// Try to parse a noun phrase beginning at token `t`.
/// Mirrors `NounPhraseHelper.TryParse()`.
pub fn try_parse(
    t:        &TokenRef,
    attr:     NounPhraseParseAttr,
    max_char: i32,
    sofa:     &SourceOfAnalysis,
) -> Option<NounPhraseToken> {
    {
        let tb = t.borrow();
        if attr == NounPhraseParseAttr::No {
            if matches!(tb.kind, TokenKind::Text(_)) && tb.not_noun_phrase() {
                return None;
            }
        }
    }

    // Determine language from first token and dispatch
    let mut lang_checked = false;
    let mut cur_check: Option<TokenRef> = Some(t.clone());
    let mut cou = 0;
    while let Some(tc) = cur_check.take() {
        {
            let tb = tc.borrow();
            if max_char > 0 && tb.begin_char > max_char { break; }
            if tb.morph.items().iter().any(|wf| wf.base.language.is_cyrillic())
                || (matches!(tb.kind, TokenKind::Number(_))
                    && tb.morph.items().iter().any(|wf| wf.base.class.is_adjective())
                    && !tb.chars.is_latin_letter())
                || (matches!(tb.kind, TokenKind::Referent(_))
                    && attr.contains(NounPhraseParseAttr::ReferentCanBeNoun)
                    && !tb.chars.is_latin_letter())
            {
                lang_checked = true;
                break;
            } else if tb.chars.is_latin_letter() {
                let res = try_parse_en(t, attr, max_char, sofa);
                if res.is_none() && attr == NounPhraseParseAttr::No {
                    t.borrow().set_not_noun_phrase(true);
                }
                return res;
            } else {
                cou += 1;
                if cou > 0 { break; }
            }
            cur_check = tb.next.clone();
        }
    }
    if !lang_checked { return None; }

    // Main dispatch: try with ParsePreposition if requested
    let res = try_parse_ru(t, attr, max_char, sofa);

    if let Some(ref r) = res {
        if attr == NounPhraseParseAttr::No {
            // Cache positive result via not_noun_phrase=false (we already handle None case below)
        }
        // Handle ParsePreposition: if result is single token that's a preposition, try after it
        if attr.contains(NounPhraseParseAttr::ParsePreposition) {
            if r.begin_token.borrow().end_char == r.end_token.borrow().end_char {
                if r.begin_token.borrow().morph.items().iter()
                        .any(|wf| wf.base.class.is_preposition()) {
                    let prep = preposition_try_parse(t, sofa);
                    if let Some(p) = prep {
                        let next = p.end_token.borrow().next.clone();
                        if let Some(nt) = next {
                            let mut res2 = try_parse_ru(&nt, attr, max_char, sofa);
                            if let Some(ref mut r2) = res2 {
                                let prep_case = p.next_case;
                                if !(prep_case & r2.morph.items().iter()
                                        .fold(MorphCase::new(), |mut a, wf| { a |= wf.base.case; a }))
                                    .is_undefined()
                                {
                                    r2.morph.remove_items_by_case(prep_case);
                                    r2.preposition = Some(p);
                                    r2.begin_token = t.clone();
                                    return res2;
                                }
                            }
                        }
                    }
                }
            }
        }
        return res;
    }

    // No result: try preposition path
    if attr.contains(NounPhraseParseAttr::ParsePreposition) {
        let prep = preposition_try_parse(t, sofa);
        if let Some(p) = prep {
            let nl_after = p.end_token.borrow().newlines_before_count(sofa);
            // We check newlines after the preposition
            if nl_after < 2 {
                let next = p.end_token.borrow().next.clone();
                if let Some(nt) = next {
                    let mut res2 = try_parse_ru(&nt, attr, max_char, sofa);
                    if let Some(ref mut r2) = res2 {
                        let prep_case = p.next_case;
                        let r2_case = r2.morph.items().iter()
                            .fold(MorphCase::new(), |mut a, wf| { a |= wf.base.case; a });
                        if !(prep_case & r2_case).is_undefined() {
                            r2.morph.remove_items_by_case(prep_case);
                        } else if t.borrow().morph.items().iter()
                                .any(|wf| wf.base.class.is_adverb()) {
                            return None;
                        }
                        r2.preposition = Some(p);
                        r2.begin_token = t.clone();
                        return res2;
                    }
                }
            }
        }
    }

    if attr == NounPhraseParseAttr::No {
        t.borrow().set_not_noun_phrase(true);
    }
    None
}
