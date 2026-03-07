use std::sync::OnceLock;
use std::sync::Mutex;
use crate::{MorphLang, MorphToken};
use crate::internal::inner_morphology::InnerMorphology;

static MORPH: OnceLock<Mutex<InnerMorphology>> = OnceLock::new();
static INITIALIZED: OnceLock<Mutex<bool>> = OnceLock::new();

pub struct MorphologyService;

impl MorphologyService {
    fn get_morph() -> &'static Mutex<InnerMorphology> {
        MORPH.get_or_init(|| Mutex::new(InnerMorphology::new()))
    }

    fn get_initialized() -> &'static Mutex<bool> {
        INITIALIZED.get_or_init(|| Mutex::new(false))
    }

    /// Initialize internal dictionaries.
    /// If langs is None or undefined, defaults to RU + EN.
    pub fn initialize(langs: Option<MorphLang>) {
        let init = Self::get_initialized();
        let mut inited = init.lock().unwrap();
        if *inited {
            return;
        }
        // Initialize unicode info
        crate::internal::unicode_info::UnicodeInfo::get_char('a');

        let langs = match langs {
            Some(l) if !l.is_undefined() => l,
            _ => MorphLang::RU | MorphLang::EN,
        };

        let morph = Self::get_morph();
        let mut m = morph.lock().unwrap();
        m.load_languages(langs, false);
        *inited = true;
    }

    pub fn loaded_languages() -> MorphLang {
        let morph = Self::get_morph();
        let m = morph.lock().unwrap();
        m.loaded_languages()
    }

    pub fn load_languages(langs: MorphLang) {
        let morph = Self::get_morph();
        let mut m = morph.lock().unwrap();
        m.load_languages(langs, false);
    }

    pub fn unload_languages(langs: MorphLang) {
        let morph = Self::get_morph();
        let mut m = morph.lock().unwrap();
        m.unload_languages(langs);
    }

    /// Perform pure tokenization without morphological analysis
    pub fn tokenize(text: &str) -> Option<Vec<MorphToken>> {
        if text.is_empty() {
            return None;
        }
        let morph = Self::get_morph();
        let m = morph.lock().unwrap();
        m.run(text, true, MorphLang::UNKNOWN, false)
    }

    /// Process text and produce morphological tokens
    pub fn process(text: &str, lang: Option<MorphLang>) -> Option<Vec<MorphToken>> {
        if text.is_empty() {
            return None;
        }
        let morph = Self::get_morph();
        let m = morph.lock().unwrap();
        let dlang = lang.unwrap_or(MorphLang::UNKNOWN);
        m.run(text, false, dlang, false)
    }
}
