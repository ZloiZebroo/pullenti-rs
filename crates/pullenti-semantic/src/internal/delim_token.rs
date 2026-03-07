/// DelimToken — discourse connective token (if/then/because/but/what…).
/// Mirrors `DelimToken.cs` and `DelimType.cs`.

use std::sync::{Arc, OnceLock};
use pullenti_ner::token::{TokenRef, TokenKind};
use pullenti_ner::core::termin::{Termin, TerminCollection, TerminToken};
use pullenti_ner::core::noun_phrase::{try_parse as noun_phrase_try_parse, NounPhraseParseAttr};
use pullenti_ner::source_of_analysis::SourceOfAnalysis;

// ── DelimType ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum DelimType {
    #[default]
    Undefined = 0,
    And       = 1,
    But       = 2,
    If        = 4,
    Then      = 8,
    Else      = 0x10,
    Because   = 0x20,
    For       = 0x40,
    What      = 0x80,
}

// ── DelimToken ─────────────────────────────────────────────────────────────

pub struct DelimToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub typ:         DelimType,
    /// True when the term itself is "double" (e.g. КОГДА which can be If or Adverb)
    pub doublt:      bool,
}

impl DelimToken {
    pub fn begin_char(&self) -> i32 { self.begin_token.borrow().begin_char }
    pub fn end_char(&self)   -> i32 { self.end_token.borrow().end_char }
}

impl std::fmt::Display for DelimToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}{}", self.typ, if self.doublt { "?" } else { "" })
    }
}

// ── TerminCollection init ──────────────────────────────────────────────────

static M_ONTO: OnceLock<TerminCollection> = OnceLock::new();

fn onto() -> &'static TerminCollection {
    M_ONTO.get_or_init(|| {
        let mut tc = TerminCollection::new();

        let make = |text: &str, typ: DelimType| -> Termin {
            let mut t = Termin::new(text);
            t.tag = Some(Arc::new(typ));
            t
        };

        // НО / А / ОДНАКО / ХОТЯ
        let mut t = make("НО", DelimType::But);
        t.add_variant("А");
        t.add_variant("ОДНАКО");
        t.add_variant("ХОТЯ");
        tc.add(t);

        // ЕСЛИ / В СЛУЧАЕ ЕСЛИ
        let mut t = make("ЕСЛИ", DelimType::If);
        t.add_variant("В СЛУЧАЕ ЕСЛИ");
        tc.add(t);

        // КОГДА — doublt (can be adverb when)
        let mut t = make("КОГДА", DelimType::If);
        t.tag2 = Some(Arc::new(true_val())); // doublt marker
        tc.add(t);

        // ТО / ТОГДА
        let mut t = make("ТО", DelimType::Then);
        t.add_variant("ТОГДА");
        tc.add(t);

        // ИНАЧЕ / В ПРОТИВНОМ СЛУЧАЕ
        let mut t = make("ИНАЧЕ", DelimType::Else);
        t.add_variant("В ПРОТИВНОМ СЛУЧАЕ");
        tc.add(t);

        // ТАК КАК / ПОТОМУ ЧТО / etc.
        let mut t = make("ТАК КАК", DelimType::Because);
        t.add_variant("ПОТОМУ ЧТО");
        t.add_variant("ПО ПРИЧИНЕ ТОГО ЧТО");
        t.add_variant("ИЗ ЗА ТОГО ЧТО");
        t.add_variant("ИЗ-ЗА ТОГО ЧТО");
        t.add_variant("ТО ЕСТЬ");
        tc.add(t);

        // ЧТОБЫ / ДЛЯ ТОГО ЧТОБЫ
        let mut t = make("ЧТОБЫ", DelimType::For);
        t.add_variant("ДЛЯ ТОГО ЧТОБЫ");
        tc.add(t);

        // ЧТО
        tc.add(make("ЧТО", DelimType::What));

        tc
    })
}

/// A unit value stored in tag2 to mark "doublt" terms.
fn true_val() -> bool { true }

fn termin_typ(tok: &TerminToken) -> DelimType {
    tok.termin.tag.as_ref()
        .and_then(|t| t.downcast_ref::<DelimType>())
        .copied()
        .unwrap_or(DelimType::Undefined)
}

fn termin_doublt(tok: &TerminToken) -> bool {
    tok.termin.tag2.as_ref()
        .and_then(|t| t.downcast_ref::<bool>())
        .copied()
        .unwrap_or(false)
}

// ── try_parse ──────────────────────────────────────────────────────────────

pub fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<DelimToken> {
    // Must be text token
    match &t.borrow().kind {
        TokenKind::Text(_) => {}
        _ => return None,
    }

    // Handle comma+and prefix (IsCommaAnd): try recursively after it
    if t.borrow().is_comma_and(sofa) {
        let next = t.borrow().next.clone()?;
        let mut res = try_parse(&next, sofa)?;
        res.begin_token = t.clone();
        return Some(res);
    }

    // Look up in ontology
    let tok = onto().try_parse(t)?;

    // Special ХОТЕТЬ/ХОТЯ disambiguation:
    // "ХОТЯ" is a valid delimiter; other forms of ХОТЕТЬ are not.
    if t.borrow().is_value("ХОТЕТЬ", None) {
        if let TokenKind::Text(ref txt) = t.borrow().kind {
            if txt.term != "ХОТЯ" {
                return None;
            }
        } else {
            return None;
        }
    }

    let typ    = termin_typ(&tok);
    let doublt = termin_doublt(&tok);
    let mut res = DelimToken {
        begin_token: t.clone(),
        end_token:   tok.end_token.clone(),
        typ,
        doublt,
    };

    // Try to chain a matching delimiter (e.g. ТО after ЕСЛИ → IfThen)
    {
        let end_next = res.end_token.borrow().next.clone();
        if let Some(next) = end_next {
            if let Some(res2) = try_parse(&next, sofa) {
                if res2.typ == res.typ {
                    res.end_token = res2.end_token.clone();
                    res.doublt = false;
                }
            }
        }
    }

    // Pronoun conflict: if the triggering token is a pronoun, check if it's
    // better explained as a noun phrase (e.g. "что нибудь" vs delimiter "что").
    let is_pronoun = {
        let mc = t.borrow().get_morph_class_in_dictionary();
        mc.is_pronoun()
    };
    if is_pronoun {
        let npt = noun_phrase_try_parse(t, NounPhraseParseAttr::ParseAdverbs, 0, sofa);
        if let Some(ref npt) = npt {
            if npt.end_token.borrow().end_char > res.end_char() {
                return None;
            }
        }
    }

    Some(res)
}
