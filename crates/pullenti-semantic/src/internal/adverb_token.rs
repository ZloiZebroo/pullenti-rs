/// AdverbToken — adverb/quantifier token for semantic analysis.
/// Mirrors `AdverbToken.cs`.

use std::sync::{Arc, OnceLock};
use pullenti_ner::token::TokenRef;
use pullenti_ner::core::termin::{Termin, TerminCollection, TerminToken};
use pullenti_ner::source_of_analysis::SourceOfAnalysis;
use crate::types::SemAttributeType;

// ── AdverbToken ────────────────────────────────────────────────────────────

pub struct AdverbToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub typ:         SemAttributeType,
    pub not:         bool,
    /// Cached spelling (set for multi-token spans like "ДРУГ ДРУГА")
    pub spelling:    Option<String>,
}

impl AdverbToken {
    pub fn begin_char(&self) -> i32 { self.begin_token.borrow().begin_char }
    pub fn end_char(&self)   -> i32 { self.end_token.borrow().end_char }

    /// Get the adverb spelling (from cache or source text)
    pub fn get_spelling(&self, sofa: &SourceOfAnalysis) -> String {
        if let Some(ref s) = self.spelling { return s.clone(); }
        let begin = self.begin_token.borrow().begin_char;
        let end   = self.end_token.borrow().end_char;
        sofa.substring(begin, end).to_string()
    }
}

impl std::fmt::Display for AdverbToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.typ == SemAttributeType::Undefined {
            write!(f, "(adverb)")
        } else {
            write!(f, "{:?}: {}", self.typ, if self.not { "НЕ " } else { "" })
        }
    }
}

// ── Term initialisation ────────────────────────────────────────────────────

static M_TERMINS: OnceLock<TerminCollection> = OnceLock::new();

fn termins() -> &'static TerminCollection {
    M_TERMINS.get_or_init(|| {
        let mut tc = TerminCollection::new();

        let make = |text: &str, typ: SemAttributeType| -> Termin {
            let mut t = Termin::new(text);
            t.tag = Some(Arc::new(typ));
            t
        };

        tc.add(make("ЕЩЕ",      SemAttributeType::Still));
        tc.add(make("УЖЕ",      SemAttributeType::Already));
        tc.add(make("ВСЕ",      SemAttributeType::All));

        let mut t = make("ЛЮБОЙ", SemAttributeType::Any);
        t.add_variant("КАЖДЫЙ");
        t.add_variant("ЧТО УГОДНО");
        t.add_variant("ВСЯКИЙ");
        tc.add(t);

        let mut t = make("НЕКОТОРЫЙ", SemAttributeType::Some);
        t.add_variant("НЕКИЙ");
        tc.add(t);

        let mut t = make("ДРУГОЙ", SemAttributeType::Other);
        t.add_variant("ИНОЙ");
        tc.add(t);

        let mut t = make("ВЕСЬ", SemAttributeType::Whole);
        t.add_variant("ЦЕЛИКОМ");
        t.add_variant("ПОЛНОСТЬЮ");
        tc.add(t);

        tc.add(make("ОЧЕНЬ", SemAttributeType::Very));

        let mut t = make("МЕНЬШЕ", SemAttributeType::Less);
        t.add_variant("МЕНЕЕ");
        tc.add(t);

        let mut t = make("БОЛЬШЕ", SemAttributeType::Great);
        t.add_variant("БОЛЕЕ");
        t.add_variant("СВЫШЕ");
        tc.add(t);

        tc
    })
}

/// Extract SemAttributeType from a TerminToken's tag.
fn termin_typ(tok: &TerminToken) -> SemAttributeType {
    tok.termin.tag.as_ref()
        .and_then(|t| t.downcast_ref::<SemAttributeType>())
        .copied()
        .unwrap_or(SemAttributeType::Undefined)
}

// ── try_parse ──────────────────────────────────────────────────────────────

pub fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<AdverbToken> {
    // Handle НЕ prefix
    if t.borrow().is_value("НЕ", None) {
        let next = t.borrow().next.clone()?;
        let mut res = try_parse(&next, sofa)?;
        res.not = true;
        res.begin_token = t.clone();
        return Some(res);
    }

    let t0 = t.clone();

    // Skip leading preposition
    let t_eff: TokenRef = {
        let tb = t.borrow();
        let mc = tb.get_morph_class_in_dictionary();
        if mc.is_preposition() {
            tb.next.clone().unwrap_or_else(|| t.clone())
        } else {
            t.clone()
        }
    };

    // "ДРУГ ДРУГА" / "САМ СЕБЯ" patterns
    {
        let is_drug = t_eff.borrow().is_value("ДРУГ", None);
        let is_sam  = t_eff.borrow().is_value("САМ",  None);
        if is_drug || is_sam {
            let t1_opt = t_eff.borrow().next.clone();
            if let Some(t1) = t1_opt {
                // Skip optional preposition
                let t1_eff: TokenRef = {
                    let tb = t1.borrow();
                    let mc = tb.get_morph_class_in_dictionary();
                    if mc.is_preposition() {
                        tb.next.clone().unwrap_or(t1.clone())
                    } else {
                        t1.clone()
                    }
                };
                if is_drug && t1_eff.borrow().is_value("ДРУГА", None) {
                    return Some(AdverbToken {
                        begin_token: t0,
                        end_token:   t1_eff,
                        typ:         SemAttributeType::EachOther,
                        not:         false,
                        spelling:    None,
                    });
                }
                if is_sam && t1_eff.borrow().is_value("СЕБЯ", None) {
                    return Some(AdverbToken {
                        begin_token: t0,
                        end_token:   t1_eff,
                        typ:         SemAttributeType::Himself,
                        not:         false,
                        spelling:    None,
                    });
                }
            }
        }
    }

    // TerminCollection lookup
    if let Some(tok) = termins().try_parse(&t_eff) {
        let typ     = termin_typ(&tok);
        let mut end = tok.end_token.clone();

        // For Less/Great, extend to "ЧЕМ" if present
        if typ == SemAttributeType::Less || typ == SemAttributeType::Great {
            let skip = end.borrow().next.clone();
            // skip optional comma
            let skip = if let Some(ref sk) = skip {
                if sk.borrow().is_comma(sofa) {
                    sk.borrow().next.clone()
                } else {
                    skip.clone()
                }
            } else {
                None
            };
            if let Some(sk) = skip {
                if sk.borrow().is_value("ЧЕМ", None) {
                    end = sk;
                }
            }
        }

        return Some(AdverbToken {
            begin_token: t0,
            end_token:   end,
            typ,
            not:         false,
            spelling:    None,
        });
    }

    // Dictionary adverb check
    let mc = t_eff.borrow().get_morph_class_in_dictionary();
    if mc.is_adverb() {
        return Some(AdverbToken {
            begin_token: t0,
            end_token:   t_eff,
            typ:         SemAttributeType::Undefined,
            not:         false,
            spelling:    None,
        });
    }

    None
}
