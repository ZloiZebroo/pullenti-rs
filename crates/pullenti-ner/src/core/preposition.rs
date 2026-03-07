/// PrepositionToken — a meta-token representing a preposition
/// (may span multiple tokens, e.g. "в соответствии с").
///
/// Mirrors `PrepositionToken.cs` / `PrepositionHelper.cs`.

use std::sync::OnceLock;
use pullenti_morph::{MorphCase, LanguageHelper};
use crate::token::{Token, TokenRef, TokenKind};
use crate::core::termin::{Termin, TerminCollection};

// ── PrepositionToken ───────────────────────────────────────────────────────

/// The matched preposition span (begin_token … end_token) plus metadata.
#[derive(Clone)]
pub struct PrepositionToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    /// Normalized preposition string (e.g. "ПОД", "В СООТВЕТСТВИИ С")
    pub normal: String,
    /// Grammatical case that follows this preposition
    pub next_case: MorphCase,
}

impl PrepositionToken {
    pub fn new(begin: TokenRef, end: TokenRef, normal: String, next_case: MorphCase) -> Self {
        PrepositionToken { begin_token: begin, end_token: end, normal, next_case }
    }
}

// ── Ontology ───────────────────────────────────────────────────────────────

static ONTOLOGY: OnceLock<TerminCollection> = OnceLock::new();

fn ontology() -> &'static TerminCollection {
    ONTOLOGY.get_or_init(|| {
        let mut tc = TerminCollection::new();

        // Genitive case prepositions
        for s in &[
            "близко от", "в виде", "в зависимости от", "в интересах", "в качестве",
            "в лице", "в отличие от", "в отношении", "в пандан", "в пользу",
            "в преддверии", "в продолжение", "в результате", "в роли", "в силу",
            "в случае", "в течение", "в целях", "в честь", "во имя", "вплоть до",
            "впредь до", "за вычетом", "за исключением", "за счет", "исходя из",
            "на благо", "на виду у", "на глазах у", "начиная с", "невзирая на",
            "недалеко от", "независимо от", "от имени", "от лица", "по линии",
            "по мере", "по поводу", "по причине", "по случаю", "поблизости от",
            "под видом", "под эгидой", "при помощи", "с ведома", "с помощью",
            "с точки зрения", "с целью",
        ] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::GENITIVE));
            tc.add(t);
        }

        // Dative case
        for s in &["вдоль по", "по направлению к", "применительно к", "смотря по", "судя по"] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::DATIVE));
            tc.add(t);
        }

        // Accusative case
        for s in &["несмотря на", "с прицелом на"] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::ACCUSATIVE));
            tc.add(t);
        }

        // Genitive | Dative
        for s in &["во славу"] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::GENITIVE | MorphCase::DATIVE));
            tc.add(t);
        }

        // Genitive | Accusative
        for s in &["не считая"] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::GENITIVE | MorphCase::ACCUSATIVE));
            tc.add(t);
        }

        // Instrumental case
        for s in &[
            "в связи с", "в соответствии с", "вслед за", "лицом к лицу с",
            "наряду с", "по сравнению с", "рядом с", "следом за",
        ] {
            let mut t = Termin::new(*s);
            t.tag = Some(std::sync::Arc::new(MorphCase::INSTRUMENTAL));
            tc.add(t);
        }

        tc
    })
}

// ── PrepositionHelper ──────────────────────────────────────────────────────

/// Try to parse a preposition starting at token `t`.
///
/// Returns `Some(PrepositionToken)` or `None`.
pub fn try_parse(t: &TokenRef, sofa: &crate::source_of_analysis::SourceOfAnalysis) -> Option<PrepositionToken> {
    let tb = t.borrow();
    let TokenKind::Text(_) = &tb.kind else { return None; };
    drop(tb);

    // Try multi-word prepositions from ontology
    if let Some(tok) = ontology().try_parse(t) {
        let next_case = tok.termin.tag.as_ref()
            .and_then(|a| a.downcast_ref::<MorphCase>())
            .copied()
            .unwrap_or(MorphCase::UNDEFINED);
        let normal = tok.termin.canonic_text.clone();
        return Some(PrepositionToken::new(t.clone(), tok.end_token.clone(), normal, next_case));
    }

    // Check morph dictionary for single preposition
    let mc = t.borrow().get_morph_class_in_dictionary();
    if !mc.is_preposition() { return None; }

    let surf = {
        let tb = t.borrow();
        sofa.substring(tb.begin_char, tb.end_char)
    };

    let normal = LanguageHelper::normalize_preposition(&surf.to_uppercase());
    let next_case = LanguageHelper::get_case_after_preposition(&normal);

    // Check for hyphenated compound preposition (ПО-НАД, etc.)
    let end = t.clone();
    let tb = t.borrow();
    if let Some(next) = tb.next.clone() {
        let nb = next.borrow();
        if nb.whitespaces_before_count(sofa) == 0 {
            if let TokenKind::Text(_) = &nb.kind {
                let ch = sofa.char_at(tb.end_char);
                // check hyphen continuation like "из-за" is already in ontology, skip
                drop(nb);
                drop(tb);
                return Some(PrepositionToken::new(t.clone(), end, normal, next_case));
            }
        }
        drop(nb);
    }
    drop(tb);

    Some(PrepositionToken::new(t.clone(), end, normal, next_case))
}
