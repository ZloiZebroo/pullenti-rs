use std::fmt;
use crate::{MorphBaseInfo, MorphMiscInfo, MorphPerson, LanguageHelper};
use crate::internal::morph_rule_variant::MorphRuleVariant;

/// Word form - a variant of morphological analysis (Словоформа)
#[derive(Clone, Default, Debug)]
pub struct MorphWordForm {
    pub base: MorphBaseInfo,
    /// Full normal form (nominative singular masculine for adjectives, infinitive for verbs)
    pub normal_full: Option<String>,
    /// Case-normalized form (nominative case, other characteristics unchanged)
    pub normal_case: Option<String>,
    pub misc: Option<MorphMiscInfo>,
    /// Coefficient for unknown word forms (0 = in dictionary)
    pub undef_coef: i16,
}

impl MorphWordForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_in_dictionary(&self) -> bool {
        self.undef_coef == 0
    }

    pub fn from_rule_variant(v: &MorphRuleVariant, word: &str, mi: MorphMiscInfo) -> Self {
        let mut wf = MorphWordForm::new();
        wf.base.class = v.base.class;
        wf.base.gender = v.base.gender;
        wf.base.number = v.base.number;
        wf.base.case = v.base.case;
        wf.base.language = v.base.language;
        wf.misc = Some(mi);

        if let Some(ref normal_tail) = v.normal_tail {
            let word_begin = if LanguageHelper::ends_with(word, &v.tail) {
                &word[..word.len() - v.tail.len()]
            } else {
                word
            };
            if !normal_tail.is_empty() {
                wf.normal_case = Some(format!("{}{}", word_begin, normal_tail));
            } else {
                wf.normal_case = Some(word_begin.to_string());
            }
        }

        if let Some(ref full_normal_tail) = v.full_normal_tail {
            let word_begin = if LanguageHelper::ends_with(word, &v.tail) {
                &word[..word.len() - v.tail.len()]
            } else {
                word
            };
            if !full_normal_tail.is_empty() {
                wf.normal_full = Some(format!("{}{}", word_begin, full_normal_tail));
            } else {
                wf.normal_full = Some(word_begin.to_string());
            }
        }

        wf
    }

    pub fn copy_from_word_form(&mut self, src: &MorphWordForm) {
        self.base.copy_from(&src.base);
        self.undef_coef = src.undef_coef;
        self.normal_case = src.normal_case.clone();
        self.normal_full = src.normal_full.clone();
        self.misc = src.misc.clone();
    }

    pub fn contains_attr(&self, attr_value: &str) -> bool {
        if let Some(ref misc) = self.misc {
            return misc.attrs.iter().any(|a| a == attr_value);
        }
        false
    }

    pub fn has_morph_equals(&self, list: &mut Vec<MorphWordForm>) -> bool {
        // First pass: merge cases
        for mr in list.iter_mut() {
            if self.base.class == mr.base.class
                && self.base.number == mr.base.number
                && self.base.gender == mr.base.gender
                && self.normal_case == mr.normal_case
                && self.normal_full == mr.normal_full
            {
                mr.base.case |= self.base.case;
                if let Some(ref misc) = self.misc {
                    let p = misc.person();
                    if let Some(ref mr_misc) = mr.misc {
                        if p != MorphPerson::UNDEFINED && p != mr_misc.person() {
                            let mut mi = MorphMiscInfo::new();
                            mi.copy_from(mr_misc);
                            mi.set_person(mr_misc.person() | p);
                            mr.misc = Some(mi);
                        }
                    }
                }
                return true;
            }
        }
        // Second pass: merge genders
        for mr in list.iter_mut() {
            if self.base.class == mr.base.class
                && self.base.number == mr.base.number
                && self.base.case == mr.base.case
                && self.normal_case == mr.normal_case
                && self.normal_full == mr.normal_full
            {
                mr.base.gender |= self.base.gender;
                return true;
            }
        }
        // Third pass: merge numbers
        for mr in list.iter_mut() {
            if self.base.class == mr.base.class
                && self.base.gender == mr.base.gender
                && self.base.case == mr.base.case
                && self.normal_case == mr.normal_case
                && self.normal_full == mr.normal_full
            {
                mr.base.number |= self.base.number;
                return true;
            }
        }
        false
    }
}

impl fmt::Display for MorphWordForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_ex(f, false)
    }
}

impl MorphWordForm {
    pub fn fmt_ex(&self, f: &mut fmt::Formatter<'_>, ignore_normals: bool) -> fmt::Result {
        let mut res = String::new();
        if !ignore_normals {
            if let Some(ref nc) = self.normal_case {
                res.push_str(nc);
            }
            if let Some(ref nf) = self.normal_full {
                if self.normal_full != self.normal_case {
                    res.push('\\');
                    res.push_str(nf);
                }
            }
            if !res.is_empty() {
                res.push(' ');
            }
        }
        res.push_str(&self.base.to_string());
        if let Some(ref misc) = self.misc {
            let s = misc.to_string();
            if !s.is_empty() {
                res.push(' ');
                res.push_str(&s);
            }
        }
        if self.undef_coef > 0 {
            res.push_str(&format!(" (? {})", self.undef_coef));
        }
        write!(f, "{}", res)
    }
}
