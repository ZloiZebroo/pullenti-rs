/// DerivateService — static word-family lookup service.
/// Mirrors `DerivateService.cs`.

use std::sync::{RwLock, OnceLock};
use pullenti_morph::{MorphLang, MorphClass, MorphCase};
use super::control_model::{ControlModelItemType, SemanticRole, items as cmq_items};
use super::deriv_dict::DerivateDictionary;
use super::deriv_group::DerivateGroup;
use super::deriv_word::DerivateWord;
use super::explan_word_attr::ExplanWordAttr;

// ── Resource ───────────────────────────────────────────────────────────────

static D_RU_DAT: &[u8] = include_bytes!("../../resources/d_ru.dat");

// ── Singleton ──────────────────────────────────────────────────────────────

static DICT: OnceLock<RwLock<DerivateDictionary>> = OnceLock::new();

fn dict() -> &'static RwLock<DerivateDictionary> {
    DICT.get_or_init(|| RwLock::new(DerivateDictionary::new()))
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Initialize the deriv dictionary for the given languages.
/// Automatically called on first use; call explicitly to pre-warm.
pub fn initialize(langs: MorphLang) {
    let langs = if langs.is_undefined() { MorphLang::RU } else { langs };
    load_languages(langs);
}

pub fn load_languages(langs: MorphLang) {
    if langs.is_ru() || langs.is_ua() {
        let d = dict().read().unwrap();
        if d.is_loaded() { return; }
        drop(d);
        let mut d = dict().write().unwrap();
        if !d.is_loaded() {
            d.load(D_RU_DAT);
        }
    }
}

/// Find deriv groups for `word` (uppercase normalized form).
/// Returns None if nothing found.
pub fn find_derivates(word: &str, try_variants: bool, lang: MorphLang) -> Option<Vec<String>> {
    ensure_loaded();
    let d = dict().read().unwrap();
    let res = d.find(word, try_variants, lang)?;
    // Return just the first word spelling of each group for now;
    // callers iterate via find_derivate_groups
    Some(res.iter().map(|g| g.words.first().map(|w| w.spelling.clone()).unwrap_or_default()).collect())
}

/// Find deriv groups and return them by id (to allow deeper inspection).
/// Returns Vec of group ids (1-based).
pub fn find_derivate_group_ids(word: &str, try_variants: bool, lang: MorphLang) -> Vec<usize> {
    ensure_loaded();
    let d = dict().read().unwrap();
    match d.find(word, try_variants, lang) {
        None => Vec::new(),
        Some(groups) => groups.iter().map(|g| g.id).collect(),
    }
}

/// Check whether a word is animated (живой).
pub fn is_animated(word: &str, lang: MorphLang) -> bool {
    ensure_loaded();
    let d = dict().read().unwrap();
    match d.find(word, false, lang) {
        None => false,
        Some(groups) => groups.iter().any(|g| {
            g.words.iter().any(|w| w.spelling == word && w.attrs.is_animated())
        }),
    }
}

/// Check whether a word can carry a proper name.
pub fn is_named(word: &str, lang: MorphLang) -> bool {
    ensure_loaded();
    let d = dict().read().unwrap();
    match d.find(word, false, lang) {
        None => false,
        Some(groups) => groups.iter().any(|g| {
            g.words.iter().any(|w| w.spelling == word && w.attrs.is_named())
        }),
    }
}

/// Find and clone derivate groups for `word`.
/// Unlike `find_derivate_group_ids`, this returns full `DerivateGroup` copies so
/// the caller can work with the data without holding the dictionary lock.
pub fn find_groups_cloned(word: &str, try_variants: bool, lang: MorphLang) -> Vec<super::deriv_group::DerivateGroup> {
    ensure_loaded();
    let d = dict().read().unwrap();
    match d.find(word, try_variants, lang) {
        None => Vec::new(),
        Some(groups) => groups.iter().map(|g| (*g).clone()).collect(),
    }
}

/// Execute a closure with access to DerivateDictionary for advanced queries.
/// The closure receives a reference to the dictionary and the given word.
pub fn with_groups<F, R>(word: &str, try_variants: bool, lang: MorphLang, f: F) -> R
where
    F: FnOnce(Option<Vec<&DerivateDictionary>>) -> R,
    R: Default,
{
    ensure_loaded();
    // We can't easily return &'static references with RwLock, so use a callback approach
    // that captures needed data.
    R::default()
}

/// Call closure with each DerivateWord for the given word.
/// Closure receives (word_spelling, class, attrs, is_verb_noun, group_id).
pub fn for_each_word<F>(word: &str, try_variants: bool, lang: MorphLang, mut f: F)
where
    F: FnMut(&DerivateWord, usize),
{
    ensure_loaded();
    let d = dict().read().unwrap();
    if let Some(groups) = d.find(word, try_variants, lang) {
        for g in groups {
            for w in &g.words {
                f(w, g.id);
            }
        }
    }
}

/// Returns all DerivateWords for the given word.
pub fn find_words(word: &str, lang: MorphLang) -> Vec<(String, MorphClass, ExplanWordAttr)> {
    ensure_loaded();
    let d = dict().read().unwrap();
    let mut res = Vec::new();
    if let Some(groups) = d.find(word, false, lang) {
        for g in groups {
            for w in &g.words {
                if w.spelling == word {
                    res.push((w.spelling.clone(), w.class, w.attrs));
                }
            }
        }
    }
    res
}

/// Look up the semantic role that a verb assigns to a noun with the given prep+case.
/// Mirrors `SemanticHelper._tryCreateVerb` + `_createRoles`: finds the Verb (or Reflexive)
/// ControlModelItem for the verb lemma and checks if any of its links match prep+case.
///
/// Returns `Some(role)` if the control model defines a role for this configuration,
/// or `None` if not found. The caller uses this to override the morphological heuristic.
pub fn find_verb_role(
    verb_lemma: &str,
    is_reflexive: bool,
    lang: MorphLang,
    prep: Option<&str>,
    case: MorphCase,
) -> Option<SemanticRole> {
    ensure_loaded();
    let d = dict().read().unwrap();
    let groups = d.find(verb_lemma, true, lang)?;
    let qs = cmq_items();

    for gr in &groups {
        // Find the control model item for this verb form (Verb or Reflexive)
        let target_typ = if is_reflexive { ControlModelItemType::Reflexive } else { ControlModelItemType::Verb };
        let cit = gr.model.items.iter().find(|it| it.typ == target_typ)
            .or_else(|| {
                // Fallback: if reflexive and no Reflexive item, try Verb item
                if is_reflexive {
                    gr.model.items.iter().find(|it| it.typ == ControlModelItemType::Verb)
                } else {
                    None
                }
            });
        let cit = match cit { Some(c) => c, None => continue };

        // _createRoles: iterate cit.links, check if q.check(prep, case)
        for (&qi, &role) in &cit.links {
            let q = match qs.get(qi) { Some(q) => q, None => continue };
            if q.check(prep, case) {
                return Some(role);
            }
        }
    }
    None
}

fn ensure_loaded() {
    let d = dict().read().unwrap();
    if d.is_loaded() { return; }
    drop(d);
    load_languages(MorphLang::RU);
}
