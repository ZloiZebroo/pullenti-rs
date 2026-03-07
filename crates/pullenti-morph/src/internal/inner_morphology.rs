use std::collections::HashMap;
use crate::{MorphLang, MorphToken, MorphWordForm, CharsInfo, LanguageHelper};
use super::morph_engine::MorphEngine;
use super::text_wrapper::TextWrapper;
use super::uni_lex_wrap::UniLexWrap;
use super::unicode_info::UnicodeInfo;

pub struct InnerMorphology {
    engine_ru: MorphEngine,
    engine_en: MorphEngine,
    engine_ua: MorphEngine,
    engine_by: MorphEngine,
    engine_kz: MorphEngine,
}

impl InnerMorphology {
    pub fn new() -> Self {
        InnerMorphology {
            engine_ru: MorphEngine::new(),
            engine_en: MorphEngine::new(),
            engine_ua: MorphEngine::new(),
            engine_by: MorphEngine::new(),
            engine_kz: MorphEngine::new(),
        }
    }

    pub fn loaded_languages(&self) -> MorphLang {
        self.engine_ru.language | self.engine_en.language | self.engine_ua.language
            | self.engine_by.language | self.engine_kz.language
    }

    pub fn load_languages(&mut self, langs: MorphLang, lazy_load: bool) {
        if langs.is_ru() && !self.engine_ru.language.is_ru() {
            let data = include_bytes!("../../resources/m_RU.dat");
            self.engine_ru.initialize_from_bytes(data, MorphLang::RU, lazy_load);
        }
        if langs.is_en() && !self.engine_en.language.is_en() {
            let data = include_bytes!("../../resources/m_EN.dat");
            self.engine_en.initialize_from_bytes(data, MorphLang::EN, lazy_load);
        }
        if langs.is_ua() && !self.engine_ua.language.is_ua() {
            let data = include_bytes!("../../resources/m_UA.dat");
            self.engine_ua.initialize_from_bytes(data, MorphLang::UA, lazy_load);
        }
        if langs.is_by() && !self.engine_by.language.is_by() {
            // BY resource may not exist
            // let data = include_bytes!("../../resources/m_BY.dat");
            // self.engine_by.initialize_from_bytes(data, MorphLang::BY, lazy_load);
        }
        if langs.is_kz() && !self.engine_kz.language.is_kz() {
            // KZ resource may not exist
            // let data = include_bytes!("../../resources/m_KZ.dat");
            // self.engine_kz.initialize_from_bytes(data, MorphLang::KZ, lazy_load);
        }
    }

    pub fn unload_languages(&mut self, langs: MorphLang) {
        if langs.is_ru() && self.engine_ru.language.is_ru() {
            self.engine_ru = MorphEngine::new();
        }
        if langs.is_en() && self.engine_en.language.is_en() {
            self.engine_en = MorphEngine::new();
        }
        if langs.is_ua() && self.engine_ua.language.is_ua() {
            self.engine_ua = MorphEngine::new();
        }
        if langs.is_by() && self.engine_by.language.is_by() {
            self.engine_by = MorphEngine::new();
        }
        if langs.is_kz() && self.engine_kz.language.is_kz() {
            self.engine_kz = MorphEngine::new();
        }
    }

    fn get_char_typ(ui: &UnicodeInfo) -> i32 {
        if ui.is_letter() { return 1; }
        if ui.is_digit() { return 2; }
        if ui.is_whitespace() { return 0; }
        if ui.is_udaren() { return 1; }
        ui.code
    }

    pub fn run(&self, text: &str, only_tokenizing: bool, dlang: MorphLang, good_text: bool) -> Option<Vec<MorphToken>> {
        if text.is_empty() {
            return None;
        }

        let twr = TextWrapper::new(text, good_text);
        let text_chars: Vec<char> = text.chars().collect();
        let mut res: Vec<MorphToken> = Vec::with_capacity(text.len() / 6);
        let mut uni_lex: HashMap<String, UniLexWrap> = HashMap::new();

        let mut _term0: Option<String> = None;
        let mut pure_rus_words = 0i32;
        let mut pure_ukr_words = 0i32;
        let mut pure_by_words = 0i32;
        let mut pure_kz_words = 0i32;
        let mut tot_rus_words = 0i32;
        let mut tot_ukr_words = 0i32;
        let mut tot_by_words = 0i32;
        let mut tot_kz_words = 0i32;

        let mut i = 0;
        while i < twr.length {
            let ui = twr.get_char(i);
            let ty = Self::get_char_typ(&ui);
            if ty == 0 {
                i += 1;
                continue;
            }

            let j;
            if ty > 2 {
                j = i + 1;
            } else {
                j = {
                    let mut jj = i + 1;
                    while jj < twr.length {
                        if Self::get_char_typ(&twr.get_char(jj)) != ty {
                            break;
                        }
                        jj += 1;
                    }
                    jj
                };
            }

            let wstr: String = text_chars[i..j].iter().collect();
            let term = if good_text {
                Some(wstr.clone())
            } else {
                let trstr = LanguageHelper::transliteral_correction(&wstr, _term0.as_deref(), false);
                LanguageHelper::correct_word(&trstr)
            };

            let term = match term {
                Some(t) if !t.is_empty() => t,
                _ => {
                    i = j;
                    continue;
                }
            };

            let lang = LanguageHelper::get_word_lang(&term);
            if term.len() > 2 {
                if lang == MorphLang::UA { pure_ukr_words += 1; }
                else if lang == MorphLang::RU { pure_rus_words += 1; }
                else if lang == MorphLang::BY { pure_by_words += 1; }
                else if lang == MorphLang::KZ { pure_kz_words += 1; }
            }
            if lang.is_ru() { tot_rus_words += 1; }
            if lang.is_ua() { tot_ukr_words += 1; }
            if lang.is_by() { tot_by_words += 1; }
            if lang.is_kz() { tot_kz_words += 1; }

            if ty == 1 {
                _term0 = Some(term.clone());
            }

            if ty == 1 && !only_tokenizing {
                if !uni_lex.contains_key(&term) {
                    uni_lex.insert(term.clone(), UniLexWrap::new(lang));
                }
            }

            let mut tok = MorphToken::new();
            tok.term = Some(term);
            tok.begin_char = i as i32;
            tok.end_char = (j - 1) as i32;
            res.push(tok);

            i = j;
        }

        // Determine default language
        let mut def_lang = dlang;
        if pure_rus_words > pure_ukr_words && pure_rus_words > pure_by_words && pure_rus_words > pure_kz_words {
            def_lang = MorphLang::RU;
        } else if tot_rus_words > tot_ukr_words && tot_rus_words > tot_by_words && tot_rus_words > tot_kz_words {
            def_lang = MorphLang::RU;
        } else if pure_ukr_words > pure_rus_words && pure_ukr_words > pure_by_words {
            def_lang = MorphLang::UA;
        }

        // Process word forms
        for (term, wrap) in uni_lex.iter_mut() {
            let mut lang = def_lang;
            wrap.word_forms = self.process_one_word(term, &mut lang);
            wrap.lang = lang;
        }

        // Assign word forms to tokens
        let empty_list: Vec<MorphWordForm> = Vec::new();
        for r in res.iter_mut() {
            if let Some(ref term) = r.term {
                if let Some(uni) = uni_lex.get(term.as_str()) {
                    if let Some(ref wfs) = uni.word_forms {
                        if !wfs.is_empty() {
                            r.word_forms = Some(wfs.clone());
                        } else {
                            r.word_forms = Some(empty_list.clone());
                            r.set_language(uni.lang);
                        }
                    } else {
                        r.word_forms = Some(empty_list.clone());
                        r.set_language(uni.lang);
                    }
                } else {
                    r.word_forms = Some(empty_list.clone());
                }
            }
        }

        // Set char info for each token
        for tok in res.iter_mut() {
            let mut ci = CharsInfo::new();
            let begin = tok.begin_char as usize;
            let end = tok.end_char as usize;

            let ui0 = twr.get_char(begin);
            if let Some(ref term) = tok.term {
                let first_char = term.chars().next().unwrap_or('?');
                let ui00 = UnicodeInfo::get_char(first_char);

                if ui0.is_letter() {
                    ci.set_letter(true);
                    if ui00.is_latin() { ci.set_latin_letter(true); }
                    else if ui00.is_cyrillic() { ci.set_cyrillic_letter(true); }

                    if tok.language().is_undefined() {
                        if LanguageHelper::is_cyrillic(term) {
                            tok.set_language(if def_lang.is_undefined() { MorphLang::RU } else { def_lang });
                        }
                    }

                    if !good_text {
                        let mut all_up = true;
                        let mut all_lo = true;
                        for j in begin..=end {
                            let ch = twr.get_char(j);
                            if ch.is_upper() || ch.is_digit() { all_lo = false; }
                            else { all_up = false; }
                        }
                        if all_up { ci.set_all_upper(true); }
                        else if all_lo { ci.set_all_lower(true); }
                        else if (ui0.is_upper() || twr.get_char(begin).is_digit()) && end > begin {
                            let mut rest_lo = true;
                            for j in (begin + 1)..=end {
                                if twr.get_char(j).is_upper() || twr.get_char(j).is_digit() {
                                    rest_lo = false;
                                    break;
                                }
                            }
                            if rest_lo { ci.set_capital_upper(true); }
                            else if twr.get_char(end).is_lower() && (end - begin) > 1 {
                                let mut init_up = true;
                                for j in begin..end {
                                    if twr.get_char(j).is_lower() {
                                        init_up = false;
                                        break;
                                    }
                                }
                                if init_up { ci.set_last_lower(true); }
                            }
                        }
                    }
                }
            }

            tok.char_info = ci;
        }

        // Set normal_case for forms that don't have one
        for r in res.iter_mut() {
            if let Some(ref mut wfs) = r.word_forms {
                for wf in wfs.iter_mut() {
                    if wf.normal_case.is_none() {
                        wf.normal_case = r.term.clone();
                    }
                }
            }
        }

        Some(res)
    }

    fn process_one_word(&self, wstr: &str, def_lang: &mut MorphLang) -> Option<Vec<MorphWordForm>> {
        let lang = LanguageHelper::get_word_lang(wstr);
        if lang.is_undefined() {
            *def_lang = MorphLang::UNKNOWN;
            return None;
        }
        if lang == MorphLang::EN {
            return self.engine_en.process(wstr, false);
        }
        if *def_lang == MorphLang::RU && lang.is_ru() {
            return self.engine_ru.process(wstr, false);
        }
        if lang == MorphLang::RU {
            *def_lang = lang;
            return self.engine_ru.process(wstr, false);
        }
        if *def_lang == MorphLang::UA && lang.is_ua() {
            return self.engine_ua.process(wstr, false);
        }
        if lang == MorphLang::UA {
            *def_lang = lang;
            return self.engine_ua.process(wstr, false);
        }

        // Try all applicable engines
        let ru = if lang.is_ru() { self.engine_ru.process(wstr, false) } else { None };
        let ua = if lang.is_ua() { self.engine_ua.process(wstr, false) } else { None };

        let has_ru = ru.as_ref().map_or(false, |wfs| wfs.iter().any(|wf| wf.is_in_dictionary()));
        let has_ua = ua.as_ref().map_or(false, |wfs| wfs.iter().any(|wf| wf.is_in_dictionary()));

        if has_ru && !has_ua {
            *def_lang = MorphLang::RU;
            return ru;
        }
        if has_ua && !has_ru {
            *def_lang = MorphLang::UA;
            return ua;
        }

        if ru.is_none() && ua.is_none() { return None; }
        if ru.is_some() && ua.is_none() { return ru; }
        if ua.is_some() && ru.is_none() { return ua; }

        // Merge results
        let mut result = Vec::new();
        if let Some(r) = ru { result.extend(r); }
        if let Some(u) = ua { result.extend(u); }
        Some(result)
    }
}
