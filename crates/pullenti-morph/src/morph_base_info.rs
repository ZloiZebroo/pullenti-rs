use std::fmt;
use crate::{MorphClass, MorphCase, MorphLang, MorphGenderFlags, MorphNumber};

/// Base morphological information (Базовая часть морфологической информации)
#[derive(Clone, Default, Debug)]
pub struct MorphBaseInfo {
    pub class: MorphClass,
    pub gender: MorphGenderFlags,
    pub number: MorphNumber,
    pub case: MorphCase,
    pub language: MorphLang,
}

impl MorphBaseInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copy_from(&mut self, src: &MorphBaseInfo) {
        self.class = src.class;
        self.gender = src.gender;
        self.number = src.number;
        self.case = src.case;
        self.language = src.language;
    }

    pub fn contains_attr(&self, _attr_value: &str, _cla: Option<&MorphClass>) -> bool {
        false
    }

    pub fn check_accord(&self, v: &MorphBaseInfo, ignore_gender: bool, ignore_number: bool) -> bool {
        if v.language != self.language {
            if v.language.is_undefined() && self.language.is_undefined() {
                return false;
            }
        }
        let num = v.number & self.number;
        if num == MorphNumber::UNDEFINED && !ignore_number {
            if v.number != MorphNumber::UNDEFINED && self.number != MorphNumber::UNDEFINED {
                if v.number == MorphNumber::SINGULAR && v.case.is_genitive() {
                    if self.number == MorphNumber::PLURAL && self.case.is_genitive() {
                        if (v.gender & MorphGenderFlags::MASCULINE) == MorphGenderFlags::MASCULINE {
                            return true;
                        }
                    }
                }
                return false;
            }
        }
        if !ignore_gender && num != MorphNumber::PLURAL {
            if (v.gender & self.gender) == MorphGenderFlags::UNDEFINED {
                if v.gender != MorphGenderFlags::UNDEFINED && self.gender != MorphGenderFlags::UNDEFINED {
                    return false;
                }
            }
        }
        if (v.case & self.case).is_undefined() {
            if !v.case.is_undefined() && !self.case.is_undefined() {
                return false;
            }
        }
        true
    }
}

impl fmt::Display for MorphBaseInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = String::new();
        if !self.class.is_undefined() {
            res.push_str(&format!("{} ", self.class));
        }
        if self.number != MorphNumber::UNDEFINED {
            if self.number == MorphNumber::SINGULAR {
                res.push_str("ед.ч. ");
            } else if self.number == MorphNumber::PLURAL {
                res.push_str("мн.ч. ");
            } else {
                res.push_str("ед.мн.ч. ");
            }
        }
        if self.gender != MorphGenderFlags::UNDEFINED {
            if self.gender == MorphGenderFlags::MASCULINE {
                res.push_str("муж.р. ");
            } else if self.gender == MorphGenderFlags::NEUTER {
                res.push_str("ср.р. ");
            } else if self.gender == MorphGenderFlags::FEMINIE {
                res.push_str("жен.р. ");
            } else if self.gender == (MorphGenderFlags::MASCULINE | MorphGenderFlags::NEUTER) {
                res.push_str("муж.ср.р. ");
            } else if self.gender == (MorphGenderFlags::FEMINIE | MorphGenderFlags::NEUTER) {
                res.push_str("жен.ср.р. ");
            } else if self.gender.0 == 7 {
                res.push_str("муж.жен.ср.р. ");
            } else if self.gender == (MorphGenderFlags::FEMINIE | MorphGenderFlags::MASCULINE) {
                res.push_str("муж.жен.р. ");
            }
        }
        if !self.case.is_undefined() {
            res.push_str(&format!("{} ", self.case));
        }
        if !self.language.is_undefined() && self.language != MorphLang::RU {
            res.push_str(&format!("{} ", self.language));
        }
        write!(f, "{}", res.trim_end())
    }
}
