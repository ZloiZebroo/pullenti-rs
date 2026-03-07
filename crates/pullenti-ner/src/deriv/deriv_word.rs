/// DerivateWord — a single word entry within a DerivateGroup.
/// Mirrors `DerivateWord.cs`.

use pullenti_morph::{MorphClass, MorphLang, MorphAspect, MorphVoice, MorphTense};
use super::explan_word_attr::ExplanWordAttr;

#[derive(Clone, Debug)]
pub struct DerivateWord {
    pub spelling:  String,
    pub class:     MorphClass,
    pub aspect:    MorphAspect,
    pub voice:     MorphVoice,
    pub tense:     MorphTense,
    pub reflexive: bool,
    pub lang:      MorphLang,
    pub attrs:     ExplanWordAttr,
    pub next_words: Option<Vec<String>>,
}

impl DerivateWord {
    pub fn new() -> Self {
        DerivateWord {
            spelling:   String::new(),
            class:      MorphClass::new(),
            aspect:     MorphAspect::Undefined,
            voice:      MorphVoice::Undefined,
            tense:      MorphTense::Undefined,
            reflexive:  false,
            lang:       MorphLang::new(),
            attrs:      ExplanWordAttr::default(),
            next_words: None,
        }
    }
}
