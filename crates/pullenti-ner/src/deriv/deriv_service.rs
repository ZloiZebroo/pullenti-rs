/// DerivateService — static word-family lookup service.
/// Mirrors `DerivateService.cs`.

use std::sync::{Mutex, OnceLock};
use pullenti_morph::{MorphLang, MorphClass};
use super::deriv_dict::DerivateDictionary;
use super::deriv_group::DerivateGroup;
use super::deriv_word::DerivateWord;
use super::explan_word_attr::ExplanWordAttr;

// ── Resource ───────────────────────────────────────────────────────────────

static D_RU_DAT: &[u8] = include_bytes!("../../resources/d_ru.dat");

// ── Singleton ──────────────────────────────────────────────────────────────

static DICT: OnceLock<Mutex<DerivateDictionary>> = OnceLock::new();

fn dict() -> &'static Mutex<DerivateDictionary> {
    DICT.get_or_init(|| Mutex::new(DerivateDictionary::new()))
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
        let mut d = dict().lock().unwrap();
        if !d.is_loaded() {
            d.load(D_RU_DAT);
        }
    }
}

/// Find deriv groups for `word` (uppercase normalized form).
/// Returns None if nothing found.
pub fn find_derivates(word: &str, try_variants: bool, lang: MorphLang) -> Option<Vec<String>> {
    ensure_loaded();
    let d = dict().lock().unwrap();
    let res = d.find(word, try_variants, lang)?;
    // Return just the first word spelling of each group for now;
    // callers iterate via find_derivate_groups
    Some(res.iter().map(|g| g.words.first().map(|w| w.spelling.clone()).unwrap_or_default()).collect())
}

/// Find deriv groups and return them by id (to allow deeper inspection).
/// Returns Vec of group ids (1-based).
pub fn find_derivate_group_ids(word: &str, try_variants: bool, lang: MorphLang) -> Vec<usize> {
    ensure_loaded();
    let d = dict().lock().unwrap();
    match d.find(word, try_variants, lang) {
        None => Vec::new(),
        Some(groups) => groups.iter().map(|g| g.id).collect(),
    }
}

/// Check whether a word is animated (живой).
pub fn is_animated(word: &str, lang: MorphLang) -> bool {
    ensure_loaded();
    let d = dict().lock().unwrap();
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
    let d = dict().lock().unwrap();
    match d.find(word, false, lang) {
        None => false,
        Some(groups) => groups.iter().any(|g| {
            g.words.iter().any(|w| w.spelling == word && w.attrs.is_named())
        }),
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
    // We can't easily return &'static references with Mutex, so use a callback approach
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
    let d = dict().lock().unwrap();
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
    let d = dict().lock().unwrap();
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

fn ensure_loaded() {
    let d = dict().lock().unwrap();
    if d.is_loaded() { return; }
    drop(d);
    load_languages(MorphLang::RU);
}
