/// SemanticHelper — utility functions for semantic analysis.
/// Mirrors `SemanticHelper.cs`.

use pullenti_ner::token::{Token, TokenRef, TokenKind};
use pullenti_ner::deriv::deriv_service;
use pullenti_morph::MorphLang;

/// Find deriv groups for the given token (text token or verb phrase item).
/// Returns a list of (spelling, class, attrs) tuples.
pub fn find_derivates(t: &TokenRef) -> Vec<String> {
    let tb = t.borrow();
    let TokenKind::Text(ref txt) = tb.kind else { return Vec::new(); };
    let mc = tb.get_morph_class_in_dictionary();
    drop(tb);

    // Try each word form's normal_full / normal_case
    let mut result = Vec::new();
    let tb = t.borrow();
    for wf in tb.morph.items() {
        let word = wf.normal_full.as_deref()
            .or_else(|| wf.normal_case.as_deref())
            .unwrap_or("");
        if word.is_empty() { continue; }
        let ids = deriv_service::find_derivate_group_ids(word, true, MorphLang::new());
        if !ids.is_empty() {
            result.push(word.to_string());
            break;
        }
    }
    result
}
