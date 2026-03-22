use std::sync::OnceLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::{MorphLang, MorphToken};
use crate::internal::inner_morphology::InnerMorphology;

static MORPH: OnceLock<RwLock<InnerMorphology>> = OnceLock::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct MorphologyService;

impl MorphologyService {
    fn get_morph() -> &'static RwLock<InnerMorphology> {
        MORPH.get_or_init(|| RwLock::new(InnerMorphology::new()))
    }

    /// Initialize internal dictionaries.
    /// If langs is None or undefined, defaults to RU + EN.
    /// Idempotent: if already initialized, ensures requested languages are loaded.
    pub fn initialize(langs: Option<MorphLang>) {
        // Initialize unicode info (idempotent)
        crate::internal::unicode_info::UnicodeInfo::get_char('a');

        let langs = match langs {
            Some(l) if !l.is_undefined() => l,
            _ => MorphLang::RU | MorphLang::EN,
        };

        // Fast path: check if requested languages are already loaded (read lock only)
        let morph = Self::get_morph();
        {
            let m = morph.read().unwrap();
            let loaded = m.loaded_languages();
            if (langs.is_ru() && !loaded.is_ru())
                || (langs.is_en() && !loaded.is_en())
                || (langs.is_ua() && !loaded.is_ua())
            {
                // Need to load — fall through to write path
            } else {
                INITIALIZED.store(true, Ordering::Release);
                return;
            }
        }

        // Slow path: acquire write lock and load
        let mut m = morph.write().unwrap();
        m.load_languages(langs, false);
        INITIALIZED.store(true, Ordering::Release);
    }

    pub fn loaded_languages() -> MorphLang {
        let morph = Self::get_morph();
        let m = morph.read().unwrap();
        m.loaded_languages()
    }

    pub fn load_languages(langs: MorphLang) {
        let morph = Self::get_morph();
        let mut m = morph.write().unwrap();
        m.load_languages(langs, false);
    }

    pub fn unload_languages(langs: MorphLang) {
        let morph = Self::get_morph();
        let mut m = morph.write().unwrap();
        m.unload_languages(langs);
    }

    /// Perform pure tokenization without morphological analysis
    pub fn tokenize(text: &str) -> Option<Vec<MorphToken>> {
        if text.is_empty() {
            return None;
        }
        let morph = Self::get_morph();
        let m = morph.read().unwrap();
        m.run(text, true, MorphLang::UNKNOWN, false)
    }

    /// Process text and produce morphological tokens
    pub fn process(text: &str, lang: Option<MorphLang>) -> Option<Vec<MorphToken>> {
        if text.is_empty() {
            return None;
        }
        let morph = Self::get_morph();
        let m = morph.read().unwrap();
        let dlang = lang.unwrap_or(MorphLang::UNKNOWN);
        m.run(text, false, dlang, false)
    }
}
