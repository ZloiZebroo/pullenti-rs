use crate::{MorphLang, MorphWordForm};

pub struct UniLexWrap {
    pub lang: MorphLang,
    pub word_forms: Option<Vec<MorphWordForm>>,
}

impl UniLexWrap {
    pub fn new(lang: MorphLang) -> Self {
        UniLexWrap {
            lang,
            word_forms: None,
        }
    }
}
