/// Sentence — sequential sentence item parser.
/// Simplified port of `Sentence.cs` / `SentItem.cs`.
///
/// The full C# version uses an N-best variant algorithm (NGSegment/NGLink).
/// This port collects items sequentially and creates SemObjects without the
/// full link-scoring algorithm.

use pullenti_ner::token::TokenRef;
use pullenti_ner::source_of_analysis::SourceOfAnalysis;
use pullenti_ner::core::noun_phrase::{
    NounPhraseToken, NounPhraseParseAttr, try_parse as noun_phrase_try_parse,
};
use pullenti_ner::core::verb_phrase::{
    VerbPhraseToken, try_parse as verb_phrase_try_parse,
};
use pullenti_ner::core::conjunction::{
    ConjunctionToken, try_parse as conjunction_try_parse,
};
use super::adverb_token::{self, AdverbToken};
use super::delim_token::{self, DelimToken};

// ── ParsedItem ─────────────────────────────────────────────────────────────

pub enum ParsedItem {
    Noun  { npt: NounPhraseToken },
    Verb  { vpt: VerbPhraseToken },
    Adverb{ adv: AdverbToken },
    Conj  { cnj: ConjunctionToken },
    Delim { dlm: DelimToken },
}

impl ParsedItem {
    pub fn begin_char(&self) -> i32 {
        match self {
            ParsedItem::Noun  { npt } => npt.begin_token.borrow().begin_char,
            ParsedItem::Verb  { vpt } => vpt.begin_token.borrow().begin_char,
            ParsedItem::Adverb{ adv } => adv.begin_char(),
            ParsedItem::Conj  { cnj } => cnj.begin_token.borrow().begin_char,
            ParsedItem::Delim { dlm } => dlm.begin_char(),
        }
    }
    pub fn end_char(&self) -> i32 {
        match self {
            ParsedItem::Noun  { npt } => npt.end_token.borrow().end_char,
            ParsedItem::Verb  { vpt } => vpt.end_token.borrow().end_char,
            ParsedItem::Adverb{ adv } => adv.end_char(),
            ParsedItem::Conj  { cnj } => cnj.end_token.borrow().end_char,
            ParsedItem::Delim { dlm } => dlm.end_char(),
        }
    }
    pub fn end_token(&self) -> TokenRef {
        match self {
            ParsedItem::Noun  { npt } => npt.end_token.clone(),
            ParsedItem::Verb  { vpt } => vpt.end_token.clone(),
            ParsedItem::Adverb{ adv } => adv.end_token.clone(),
            ParsedItem::Conj  { cnj } => cnj.end_token.clone(),
            ParsedItem::Delim { dlm } => dlm.end_token.clone(),
        }
    }
}

// ── NounPhraseParseAttr flags for sentence analysis ───────────────────────

const NPT_ATTRS: NounPhraseParseAttr = NounPhraseParseAttr::from_bits_truncate(
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

// ── parse_variants ─────────────────────────────────────────────────────────

/// Parse a sentence span [t0..t1] into a sequence of semantic items.
/// Simplified: sequential, no variant expansion.
pub fn parse_variants(
    t0:   &TokenRef,
    t1:   &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let t1_end = t1.borrow().end_char;

    let mut cur = Some(t0.clone());
    while let Some(t) = cur.take() {
        if t.borrow().end_char > t1_end { break; }

        // Skip open-parenthesis spans
        if t.borrow().is_char('(', sofa) {
            // Skip until ')' or end
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
            // If no close found, advance past this token
            cur = t.borrow().next.clone();
            continue;
        }

        // 1. Delimiter
        if let Some(dlm) = delim_token::try_parse(&t, sofa) {
            let next = dlm.end_token.borrow().next.clone();
            items.push(ParsedItem::Delim { dlm });
            cur = next;
            continue;
        }

        // 2. Conjunction
        if let Some(cnj) = conjunction_try_parse(&t, sofa) {
            let next = cnj.end_token.borrow().next.clone();
            items.push(ParsedItem::Conj { cnj });
            cur = next;
            continue;
        }

        // 3. Noun phrase
        let npt_opt = noun_phrase_try_parse(&t, NPT_ATTRS, 0, sofa);

        // 4. Adverb
        let adv_opt = adverb_token::try_parse(&t, sofa);

        // 5. Verb phrase
        let vpt_opt = verb_phrase_try_parse(&t, true, false, false, sofa);

        // Resolve noun vs adverb ambiguity (take longer span)
        let use_npt = match (&npt_opt, &adv_opt) {
            (Some(npt), Some(adv)) => {
                adv.end_char() <= npt.end_token.borrow().end_char
            }
            (Some(_), None) => true,
            _ => false,
        };

        if use_npt {
            let npt = npt_opt.unwrap();
            // Check if a participle verb phrase covers the same span
            let _npt_end = npt.end_token.borrow().end_char;
            let vpt_is_participle = vpt_opt.as_ref()
                .and_then(|v| v.first_verb())
                .map_or(false, |fv| fv.is_participle());
            if npt.adjectives.is_empty() || !vpt_is_participle {
                let next = npt.end_token.borrow().next.clone();
                items.push(ParsedItem::Noun { npt });
                cur = next;
                continue;
            }
        }

        // Try verb
        if let Some(vpt) = vpt_opt {
            let first_is_part = vpt.first_verb().map_or(false, |fv| fv.is_participle());
            if !first_is_part {
                let next = vpt.end_token.borrow().next.clone();
                items.push(ParsedItem::Verb { vpt });
                cur = next;
                continue;
            }
        }

        // Try adverb
        if let Some(adv) = adv_opt {
            let next = adv.end_token.borrow().next.clone();
            items.push(ParsedItem::Adverb { adv });
            cur = next;
            continue;
        }

        // Skip token
        cur = t.borrow().next.clone();
    }

    items
}
