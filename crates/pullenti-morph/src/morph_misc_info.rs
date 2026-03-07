use std::fmt;
use crate::{MorphPerson, MorphTense, MorphAspect, MorphMood, MorphVoice, MorphForm};

/// Additional morphological information (Дополнительная морфологическая информация)
#[derive(Clone, Default, Debug)]
pub struct MorphMiscInfo {
    pub attrs: Vec<String>,
    pub value: i16,
    pub id: i32,
}

impl MorphMiscInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_attr(&mut self, a: &str) {
        if !self.attrs.contains(&a.to_string()) {
            self.attrs.push(a.to_string());
        }
    }

    fn get_bool_value(&self, i: u32) -> bool {
        ((self.value >> i) & 1) != 0
    }

    fn set_bool_value(&mut self, i: u32, val: bool) {
        if val {
            self.value |= 1 << i;
        } else {
            self.value &= !(1 << i);
        }
    }

    pub fn copy_from(&mut self, src: &MorphMiscInfo) {
        self.value = src.value;
        self.attrs = src.attrs.clone();
    }

    pub fn person(&self) -> MorphPerson {
        let mut res = MorphPerson::UNDEFINED;
        if self.attrs.iter().any(|a| a == "1 л.") {
            res |= MorphPerson::FIRST;
        }
        if self.attrs.iter().any(|a| a == "2 л.") {
            res |= MorphPerson::SECOND;
        }
        if self.attrs.iter().any(|a| a == "3 л.") {
            res |= MorphPerson::THIRD;
        }
        res
    }

    pub fn set_person(&mut self, value: MorphPerson) {
        if (value & MorphPerson::FIRST) != MorphPerson::UNDEFINED {
            self.add_attr("1 л.");
        }
        if (value & MorphPerson::SECOND) != MorphPerson::UNDEFINED {
            self.add_attr("2 л.");
        }
        if (value & MorphPerson::THIRD) != MorphPerson::UNDEFINED {
            self.add_attr("3 л.");
        }
    }

    pub fn tense(&self) -> MorphTense {
        if self.attrs.iter().any(|a| a == "п.вр.") { return MorphTense::Past; }
        if self.attrs.iter().any(|a| a == "н.вр.") { return MorphTense::Present; }
        if self.attrs.iter().any(|a| a == "б.вр.") { return MorphTense::Future; }
        MorphTense::Undefined
    }

    pub fn set_tense(&mut self, value: MorphTense) {
        match value {
            MorphTense::Past => self.add_attr("п.вр."),
            MorphTense::Present => self.add_attr("н.вр."),
            MorphTense::Future => self.add_attr("б.вр."),
            _ => {}
        }
    }

    pub fn aspect(&self) -> MorphAspect {
        if self.attrs.iter().any(|a| a == "нес.в.") { return MorphAspect::Imperfective; }
        if self.attrs.iter().any(|a| a == "сов.в.") { return MorphAspect::Perfective; }
        MorphAspect::Undefined
    }

    pub fn set_aspect(&mut self, value: MorphAspect) {
        match value {
            MorphAspect::Imperfective => self.add_attr("нес.в."),
            MorphAspect::Perfective => self.add_attr("сов.в."),
            _ => {}
        }
    }

    pub fn mood(&self) -> MorphMood {
        if self.attrs.iter().any(|a| a == "пов.накл.") { return MorphMood::Imperative; }
        MorphMood::Undefined
    }

    pub fn set_mood(&mut self, value: MorphMood) {
        if value == MorphMood::Imperative {
            self.add_attr("пов.накл.");
        }
    }

    pub fn voice(&self) -> MorphVoice {
        if self.attrs.iter().any(|a| a == "дейст.з.") { return MorphVoice::Active; }
        if self.attrs.iter().any(|a| a == "страд.з.") { return MorphVoice::Passive; }
        MorphVoice::Undefined
    }

    pub fn set_voice(&mut self, value: MorphVoice) {
        match value {
            MorphVoice::Active => self.add_attr("дейст.з."),
            MorphVoice::Passive => self.add_attr("страд.з."),
            _ => {}
        }
    }

    pub fn form(&self) -> MorphForm {
        if self.attrs.iter().any(|a| a == "к.ф.") { return MorphForm::Short; }
        if self.attrs.iter().any(|a| a == "синоним.форма") { return MorphForm::Synonym; }
        if self.is_synonym_form() { return MorphForm::Synonym; }
        MorphForm::Undefined
    }

    pub fn is_synonym_form(&self) -> bool {
        self.get_bool_value(0)
    }

    pub fn set_synonym_form(&mut self, val: bool) {
        self.set_bool_value(0, val);
    }

    pub fn deserialize(&mut self, data: &[u8], pos: &mut usize) {
        use crate::internal::byte_array_wrapper::ByteArrayWrapper;
        let wrapper = ByteArrayWrapper::new(data);
        let sh = wrapper.deserialize_short(pos);
        self.value = sh;
        loop {
            let s = wrapper.deserialize_string(pos);
            if s.is_empty() {
                break;
            }
            if !self.attrs.contains(&s) {
                self.attrs.push(s);
            }
        }
    }
}

impl fmt::Display for MorphMiscInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.attrs.is_empty() && self.value == 0 {
            return Ok(());
        }
        let mut res = String::new();
        if self.is_synonym_form() {
            res.push_str("синоним.форма ");
        }
        for a in &self.attrs {
            res.push_str(a);
            res.push(' ');
        }
        write!(f, "{}", res.trim_end())
    }
}
