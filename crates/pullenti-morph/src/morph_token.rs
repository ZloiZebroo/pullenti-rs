use std::fmt;
use crate::{MorphLang, MorphWordForm, CharsInfo, LanguageHelper, MorphNumber};

/// Morphological token - element of text decomposition (Морфологический токен)
#[derive(Clone, Default)]
pub struct MorphToken {
    pub begin_char: i32,
    pub end_char: i32,
    pub term: Option<String>,
    pub word_forms: Option<Vec<MorphWordForm>>,
    pub char_info: CharsInfo,
    m_language: Option<MorphLang>,
    m_lemma: Option<String>,
}

impl MorphToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn length(&self) -> usize {
        match &self.term {
            Some(t) => t.len(),
            None => 0,
        }
    }

    pub fn get_source_text<'a>(&self, text: &'a str) -> &'a str {
        // begin_char/end_char are char (Unicode scalar) positions, not byte offsets
        let begin = self.begin_char as usize;
        let end = (self.end_char + 1) as usize;
        let char_to_byte: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        let byte_begin = char_to_byte.get(begin).copied().unwrap_or(text.len());
        let byte_end = char_to_byte.get(end).copied().unwrap_or(text.len());
        if byte_begin >= byte_end { return ""; }
        &text[byte_begin..byte_end]
    }

    pub fn language(&self) -> MorphLang {
        if let Some(ref lang) = self.m_language {
            if !lang.is_undefined() {
                return *lang;
            }
        }
        let mut lang = MorphLang::new();
        if let Some(ref wfs) = self.word_forms {
            for wf in wfs {
                if !wf.base.language.is_undefined() {
                    lang |= wf.base.language;
                }
            }
        }
        lang
    }

    pub fn set_language(&mut self, value: MorphLang) {
        self.m_language = Some(value);
    }

    pub fn get_lemma(&self) -> String {
        if let Some(ref lemma) = self.m_lemma {
            return lemma.clone();
        }

        let mut res: Option<String> = None;

        if let Some(ref wfs) = self.word_forms {
            if !wfs.is_empty() {
                if wfs.len() == 1 {
                    res = wfs[0].normal_full.clone().or_else(|| wfs[0].normal_case.clone());
                }

                if res.is_none() && !self.char_info.is_all_lower() {
                    for m in wfs {
                        if m.base.class.is_proper_surname() {
                            let s = m.normal_full.as_ref().or(m.normal_case.as_ref());
                            if let Some(s) = s {
                                if LanguageHelper::ends_with_ex(s, &["ОВ", "ЕВ"]) {
                                    res = Some(s.clone());
                                    break;
                                }
                            }
                        } else if m.base.class.is_proper_name() && m.is_in_dictionary() {
                            return m.normal_case.clone().unwrap_or_default();
                        }
                    }
                }

                if res.is_none() {
                    let mut best: Option<&MorphWordForm> = None;
                    for m in wfs {
                        if best.is_none() || Self::compare_forms(best.unwrap(), m, &self.char_info) > 0 {
                            best = Some(m);
                        }
                    }
                    if let Some(b) = best {
                        res = b.normal_full.clone().or_else(|| b.normal_case.clone());
                    }
                }
            }
        }

        if let Some(ref r) = res {
            if LanguageHelper::ends_with_ex(r, &["АНЫЙ", "ЕНЫЙ"]) {
                return format!("{}ННЫЙ", &r[..r.len() - 3]);
            } else if LanguageHelper::ends_with(r, "ЙСЯ") {
                return r[..r.len() - 2].to_string();
            } else if LanguageHelper::ends_with(r, "АНИЙ") {
                if let Some(ref term) = self.term {
                    if r == term {
                        if let Some(ref wfs) = self.word_forms {
                            for wf in wfs {
                                if wf.is_in_dictionary() {
                                    return r.clone();
                                }
                            }
                        }
                        return format!("{}Е", &r[..r.len() - 1]);
                    }
                }
            }
            return r.clone();
        }

        self.term.clone().unwrap_or_else(|| "?".to_string())
    }

    fn compare_forms(x: &MorphWordForm, y: &MorphWordForm, char_info: &CharsInfo) -> i32 {
        let vx = x.normal_full.as_ref().or(x.normal_case.as_ref());
        let vy = y.normal_full.as_ref().or(y.normal_case.as_ref());
        if vx == vy { return 0; }
        let vx = match vx { Some(s) if !s.is_empty() => s, _ => return 1 };
        let vy = match vy { Some(s) if !s.is_empty() => s, _ => return -1 };

        if x.base.class.is_proper_surname() && !char_info.is_all_lower() {
            if LanguageHelper::ends_with_ex(vx, &["ОВ", "ЕВ", "ИН"]) {
                if !y.base.class.is_proper_surname() {
                    return -1;
                }
            }
        }
        if y.base.class.is_proper_surname() && !char_info.is_all_lower() {
            if LanguageHelper::ends_with_ex(vy, &["ОВ", "ЕВ", "ИН"]) {
                if !x.base.class.is_proper_surname() {
                    return 1;
                }
                if vx.len() > vy.len() { return -1; }
                if vx.len() < vy.len() { return 1; }
                return 0;
            }
        }
        if x.base.class == y.base.class {
            if x.base.class.is_adjective() {
                let lastx = vx.chars().last().unwrap_or('\0');
                let lasty = vy.chars().last().unwrap_or('\0');
                if lastx == 'Й' && lasty != 'Й' { return -1; }
                if lastx != 'Й' && lasty == 'Й' { return 1; }
                if !LanguageHelper::ends_with(vx, "ОЙ") && LanguageHelper::ends_with(vy, "ОЙ") { return -1; }
                if LanguageHelper::ends_with(vx, "ОЙ") && !LanguageHelper::ends_with(vy, "ОЙ") { return 1; }
            }
            if x.base.class.is_noun() {
                if x.base.number == MorphNumber::SINGULAR && y.base.number == MorphNumber::PLURAL && vx.len() <= vy.len() + 1 {
                    return -1;
                }
                if x.base.number == MorphNumber::PLURAL && y.base.number == MorphNumber::SINGULAR && vx.len() + 1 >= vy.len() {
                    return 1;
                }
            }
            if vx.len() < vy.len() { return -1; }
            if vx.len() > vy.len() { return 1; }
            return 0;
        }
        if x.base.class.is_adverb() { return 1; }
        if x.base.class.is_noun() && x.is_in_dictionary() {
            if y.base.class.is_adjective() && y.is_in_dictionary() {
                if let Some(ref misc) = y.misc {
                    if !misc.attrs.iter().any(|a| a == "к.ф.") {
                        return 1;
                    }
                }
            }
            return -1;
        }
        if x.base.class.is_adjective() {
            if !x.is_in_dictionary() && y.base.class.is_noun() && y.is_in_dictionary() {
                return 1;
            }
            return -1;
        }
        if x.base.class.is_verb() {
            if y.base.class.is_noun() || y.base.class.is_adjective() || y.base.class.is_preposition() {
                return 1;
            }
            return -1;
        }
        if y.base.class.is_adverb() { return -1; }
        if y.base.class.is_noun() && y.is_in_dictionary() { return 1; }
        if y.base.class.is_adjective() {
            if (x.base.class.is_noun() || x.base.class.is_proper_secname()) && x.is_in_dictionary() {
                return -1;
            }
            if x.base.class.is_noun() && !y.is_in_dictionary() {
                if vx.len() < vy.len() { return -1; }
            }
            return 1;
        }
        if y.base.class.is_verb() {
            if x.base.class.is_noun() || x.base.class.is_adjective() || x.base.class.is_preposition() {
                return -1;
            }
            if x.base.class.is_proper() { return -1; }
            return 1;
        }
        if vx.len() < vy.len() { return -1; }
        if vx.len() > vy.len() { return 1; }
        0
    }
}

impl fmt::Display for MorphToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let term = match &self.term {
            Some(t) => t.as_str(),
            None => return write!(f, "Null"),
        };

        let mut str = if self.char_info.is_all_lower() {
            term.to_lowercase()
        } else if self.char_info.is_capital_upper() && !term.is_empty() {
            let first: String = term.chars().take(1).collect();
            let rest: String = term.chars().skip(1).collect();
            format!("{}{}", first, rest.to_lowercase())
        } else if self.char_info.is_last_lower() && term.len() > 1 {
            let init: String = term.chars().take(term.len() - 1).collect();
            let last: String = term.chars().last().unwrap().to_lowercase().collect();
            format!("{}{}", init, last)
        } else {
            term.to_string()
        };

        if let Some(ref wfs) = self.word_forms {
            for l in wfs {
                str.push_str(&format!(", {}", l));
            }
        }
        write!(f, "{}", str)
    }
}

