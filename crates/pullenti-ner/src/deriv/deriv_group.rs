/// DerivateGroup — a group of related words (same root, different POS/aspects).
/// Mirrors `DerivateGroup.cs`.

use pullenti_morph::{MorphLang, MorphAspect, MorphVoice, MorphTense};
use pullenti_morph::internal::byte_array_wrapper::ByteArrayWrapper;
use super::explan_word_attr::ExplanWordAttr;
use super::deriv_word::DerivateWord;
use super::control_model::ControlModel;

#[derive(Clone, Debug)]
pub struct DerivateGroup {
    pub words:        Vec<DerivateWord>,
    pub prefix:       Option<String>,
    pub is_dummy:     bool,
    pub not_generate: bool,
    pub is_generated: bool,
    pub model:        ControlModel,
    pub lazy_pos:     usize,
    pub id:           usize,
}

impl DerivateGroup {
    pub fn new() -> Self {
        DerivateGroup {
            words:        Vec::new(),
            prefix:       None,
            is_dummy:     false,
            not_generate: false,
            is_generated: false,
            model:        ControlModel::new(),
            lazy_pos:     0,
            id:           0,
        }
    }

    pub fn contains_word(&self, word: &str, lang: MorphLang) -> bool {
        self.words.iter().any(|w| {
            w.spelling == word && (lang.is_undefined() || !(lang & w.lang).is_undefined())
        })
    }

    pub fn create_by_prefix(&self, pref: &str, lang: MorphLang) -> DerivateGroup {
        let mut res = DerivateGroup::new();
        res.is_generated = true;
        res.prefix = Some(pref.to_string());
        for w in &self.words {
            if !lang.is_undefined() && (lang & w.lang).is_undefined() { continue; }
            let mut rw = DerivateWord::new();
            rw.spelling  = format!("{}{}", pref, w.spelling);
            rw.lang      = w.lang;
            rw.class     = w.class;
            rw.aspect    = w.aspect;
            rw.reflexive = w.reflexive;
            rw.tense     = w.tense;
            rw.voice     = w.voice;
            rw.attrs     = w.attrs;
            res.words.push(rw);
        }
        res
    }

    pub fn deserialize(&mut self, buf: &ByteArrayWrapper, pos: &mut usize) {
        let attr = buf.deserialize_short(pos) as i32;
        if (attr & 1) != 0 { self.is_dummy     = true; }
        if (attr & 2) != 0 { self.not_generate = true; }
        let pref = buf.deserialize_string(pos);
        self.prefix = if pref.is_empty() { None } else { Some(pref) };

        self.model.deserialize(buf, pos);

        let mut cou = buf.deserialize_short(pos) as i32;
        while cou > 0 {
            cou -= 1;
            let mut w = DerivateWord::new();
            w.spelling = buf.deserialize_string(pos);
            let sh = buf.deserialize_short(pos);
            w.class.value = sh;
            let sh = buf.deserialize_short(pos);
            w.lang.value = sh;
            let sh = buf.deserialize_short(pos);
            w.attrs.value = sh;
            let b = buf.deserialize_byte(pos);
            w.aspect = match b { 1 => MorphAspect::Perfective, 2 => MorphAspect::Imperfective, _ => MorphAspect::Undefined };
            let b = buf.deserialize_byte(pos);
            w.tense = match b { 1 => MorphTense::Past, 2 => MorphTense::Present, 4 => MorphTense::Future, _ => MorphTense::Undefined };
            let b = buf.deserialize_byte(pos);
            w.voice = match b { 1 => MorphVoice::Active, 2 => MorphVoice::Passive, 4 => MorphVoice::Middle, _ => MorphVoice::Undefined };
            let b = buf.deserialize_byte(pos);
            let mut cou1 = b as i32;
            while cou1 > 0 {
                cou1 -= 1;
                let n = buf.deserialize_string(pos);
                if !n.is_empty() {
                    w.next_words.get_or_insert_with(Vec::new).push(n);
                }
            }
            self.words.push(w);
        }
    }
}
