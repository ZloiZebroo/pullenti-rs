use std::collections::HashMap;
use std::sync::OnceLock;
use crate::internal::unicode_info::UnicodeInfo;
use crate::{MorphLang, MorphCase};

static PREP_CASES: OnceLock<HashMap<String, MorphCase>> = OnceLock::new();
static PREP_NORMS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub struct LanguageHelper;

impl LanguageHelper {
    pub fn get_language_for_text(text: &str) -> Option<&'static str> {
        if text.is_empty() {
            return None;
        }
        let mut ru_chars = 0;
        let mut en_chars = 0;
        for ch in text.chars() {
            if !ch.is_alphabetic() { continue; }
            let j = ch as u32;
            if j >= 0x400 && j < 0x500 {
                ru_chars += 1;
            } else if j < 0x80 {
                en_chars += 1;
            }
        }
        if ru_chars > en_chars * 2 && ru_chars > 10 {
            return Some("ru");
        }
        if ru_chars > 0 && en_chars == 0 {
            return Some("ru");
        }
        if en_chars > 0 {
            return Some("en");
        }
        None
    }

    pub fn get_word_lang(word: &str) -> MorphLang {
        let mut cyr = 0;
        let mut lat = 0;
        let mut undef = 0;
        for ch in word.chars() {
            let ui = UnicodeInfo::get_char(ch);
            if ui.is_letter() {
                if ui.is_cyrillic() {
                    cyr += 1;
                } else if ui.is_latin() {
                    lat += 1;
                } else {
                    undef += 1;
                }
            }
        }
        if undef > 0 { return MorphLang::UNKNOWN; }
        if cyr == 0 && lat == 0 { return MorphLang::UNKNOWN; }
        if cyr == 0 { return MorphLang::EN; }
        if lat > 0 { return MorphLang::UNKNOWN; }

        let mut lang = MorphLang::UA | MorphLang::RU | MorphLang::BY | MorphLang::KZ;
        for ch in word.chars() {
            let ui = UnicodeInfo::get_char(ch);
            if ui.is_letter() {
                match ch {
                    'Ґ' | 'Є' | 'Ї' => {
                        lang.set_ru(false);
                        lang.set_by(false);
                    }
                    'І' => { lang.set_ru(false); }
                    'Ё' | 'Э' => {
                        lang.set_ua(false);
                        lang.set_kz(false);
                    }
                    'Ы' => { lang.set_ua(false); }
                    'Ў' => {
                        lang.set_ru(false);
                        lang.set_ua(false);
                    }
                    'Щ' => { lang.set_by(false); }
                    'Ъ' => {
                        lang.set_by(false);
                        lang.set_ua(false);
                        lang.set_kz(false);
                    }
                    'Ә' | 'Ғ' | 'Қ' | 'Ң' | 'Ө' | 'Ү' | 'Һ' => {
                        lang.set_by(false);
                        lang.set_ua(false);
                        lang.set_ru(false);
                    }
                    'Ұ' if word.len() > 1 => {
                        lang.set_by(false);
                        lang.set_ua(false);
                        lang.set_ru(false);
                    }
                    'В' | 'Ф' | 'Ц' | 'Ч' | 'Ь' => {
                        lang.set_kz(false);
                    }
                    _ => {}
                }
            }
        }
        lang
    }

    pub fn is_latin_char(ch: char) -> bool {
        UnicodeInfo::get_char(ch).is_latin()
    }

    pub fn is_latin(s: &str) -> bool {
        for ch in s.chars() {
            if !Self::is_latin_char(ch) {
                if !ch.is_whitespace() && ch != '-' {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_cyrillic_char(ch: char) -> bool {
        UnicodeInfo::get_char(ch).is_cyrillic()
    }

    pub fn is_cyrillic(s: &str) -> bool {
        for ch in s.chars() {
            if !Self::is_cyrillic_char(ch) {
                if !ch.is_whitespace() && ch != '-' {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_hiphen(ch: char) -> bool {
        UnicodeInfo::get_char(ch).is_hiphen()
    }

    pub fn is_cyrillic_vowel(ch: char) -> bool {
        let ui = UnicodeInfo::get_char(ch);
        ui.is_cyrillic() && ui.is_vowel()
    }

    pub fn is_latin_vowel(ch: char) -> bool {
        let ui = UnicodeInfo::get_char(ch);
        ui.is_latin() && ui.is_vowel()
    }

    pub fn is_quote(ch: char) -> bool {
        UnicodeInfo::get_char(ch).is_quot()
    }

    pub fn is_apos(ch: char) -> bool {
        UnicodeInfo::get_char(ch).is_apos()
    }

    const LAT_CHARS: &'static str = "ABEKMHOPCTYXIaekmopctyxi";
    const CYR_CHARS: &'static str = "АВЕКМНОРСТУХІаекморстухі";
    const GREEK_CHARS: &'static str = "ΑΒΓΕΗΙΚΛΜΟΠΡΤΥΦΧ";
    const CYR_GREEK_CHARS: &'static str = "АВГЕНІКЛМОПРТУФХ";
    const UDAR_CHARS: &'static str = "ÀÁÈÉËÒÓàáèéëýÝòóЀѐЍѝỲỳ";
    const UDAR_CYR_CHARS: &'static str = "ААЕЕЕООааеееуУооЕеИиУу";

    pub fn get_cyr_for_lat(lat: char) -> Option<char> {
        if let Some(i) = Self::LAT_CHARS.find(lat) {
            Self::CYR_CHARS.chars().nth(i)
        } else if let Some(i) = Self::GREEK_CHARS.find(lat) {
            Self::CYR_GREEK_CHARS.chars().nth(i)
        } else {
            None
        }
    }

    pub fn get_lat_for_cyr(cyr: char) -> Option<char> {
        if let Some(i) = Self::CYR_CHARS.find(cyr) {
            Self::LAT_CHARS.chars().nth(i)
        } else {
            None
        }
    }

    pub fn ends_with(s: &str, substr: &str) -> bool {
        s.ends_with(substr)
    }

    pub fn ends_with_ex(s: &str, substrs: &[&str]) -> bool {
        for sub in substrs {
            if s.ends_with(sub) {
                return true;
            }
        }
        false
    }

    const RUS0: &'static str = "–ЁѐЀЍѝЎўӢӣ";
    const RUS1: &'static str = "-ЕЕЕИИУУЙЙ";

    pub fn correct_word(w: &str) -> Option<String> {
        if w.is_empty() {
            return None;
        }
        let mut res: String = w.to_uppercase();

        // Replace special Russian characters
        let rus0_chars: Vec<char> = Self::RUS0.chars().collect();
        let rus1_chars: Vec<char> = Self::RUS1.chars().collect();
        let needs_fix = res.chars().any(|ch| rus0_chars.contains(&ch));
        if needs_fix {
            let mut tmp: Vec<char> = res.chars().collect();
            for c in tmp.iter_mut() {
                if let Some(j) = rus0_chars.iter().position(|&x| x == *c) {
                    if j < rus1_chars.len() {
                        *c = rus1_chars[j];
                    }
                }
            }
            res = tmp.into_iter().collect();
        }

        // Replace soft hyphen
        if res.contains('\u{00AD}') {
            res = res.replace('\u{00AD}', "-");
        }

        // Special fix
        if res.starts_with("АГЕНС") {
            res = format!("АГЕНТС{}", &res[("АГЕНС".len())..]);
        }

        Some(res)
    }

    pub fn transliteral_correction(value: &str, prev_value: Option<&str>, always: bool) -> String {
        let lat_chars: Vec<char> = Self::LAT_CHARS.chars().collect();
        let cyr_chars: Vec<char> = Self::CYR_CHARS.chars().collect();
        let udar_chars: Vec<char> = Self::UDAR_CHARS.chars().collect();
        let udar_cyr_chars: Vec<char> = Self::UDAR_CYR_CHARS.chars().collect();

        let mut pure_cyr = 0i32;
        let mut pure_lat = 0i32;
        let mut ques_cyr = 0i32;
        let mut ques_lat = 0i32;
        let mut udar_cyr = 0i32;
        let mut y = false;
        let mut udaren = false;

        let chars: Vec<char> = value.chars().collect();

        for i in 0..chars.len() {
            let ch = chars[i];
            let ui = UnicodeInfo::get_char(ch);
            if !ui.is_letter() {
                if ui.is_udaren() {
                    udaren = true;
                    continue;
                }
                if ui.is_apos() && chars.len() > 2 {
                    let cleaned: String = chars.iter().filter(|&&c| c != ch).collect();
                    return Self::transliteral_correction(&cleaned, prev_value, false);
                }
                return value.to_string();
            }
            if ui.is_cyrillic() {
                if cyr_chars.contains(&ch) {
                    ques_cyr += 1;
                } else {
                    pure_cyr += 1;
                }
            } else if ui.is_latin() {
                if lat_chars.contains(&ch) {
                    ques_lat += 1;
                } else {
                    pure_lat += 1;
                }
            } else if udar_chars.contains(&ch) {
                udar_cyr += 1;
            } else {
                return value.to_string();
            }
            if ch == 'Ь' && i + 1 < chars.len() && chars[i + 1] == 'I' {
                y = true;
            }
        }

        let mut to_rus = false;
        let mut to_lat = false;

        if pure_lat > 0 && pure_cyr > 0 {
            return value.to_string();
        }
        if (pure_lat > 0 || always) && ques_cyr > 0 {
            to_lat = true;
        } else if (pure_cyr > 0 || always) && ques_lat > 0 {
            to_rus = true;
        } else if pure_cyr == 0 && pure_lat == 0 {
            if ques_cyr > 0 && ques_lat > 0 {
                if let Some(pv) = prev_value {
                    if !pv.is_empty() {
                        let first = pv.chars().next().unwrap();
                        if Self::is_cyrillic_char(first) {
                            to_rus = true;
                        } else if Self::is_latin_char(first) {
                            to_lat = true;
                        }
                    }
                }
                if !to_lat && !to_rus {
                    if ques_cyr > ques_lat {
                        to_rus = true;
                    } else if ques_cyr < ques_lat {
                        to_lat = true;
                    }
                }
            }
        }

        if !to_rus && !to_lat {
            if !y && !udaren && udar_cyr == 0 {
                return value.to_string();
            }
        }

        let mut tmp: Vec<char> = chars;
        let mut i = 0;
        while i < tmp.len() {
            if tmp[i] == 'Ь' && i + 1 < tmp.len() && tmp[i + 1] == 'I' {
                tmp[i] = 'Ы';
                tmp.remove(i + 1);
                continue;
            }
            let cod = tmp[i] as u32;
            if cod >= 0x300 && cod < 0x370 {
                tmp.remove(i);
                continue;
            }
            if to_rus {
                if let Some(ii) = lat_chars.iter().position(|&c| c == tmp[i]) {
                    tmp[i] = cyr_chars[ii];
                } else if let Some(ii) = udar_chars.iter().position(|&c| c == tmp[i]) {
                    if ii < udar_cyr_chars.len() {
                        tmp[i] = udar_cyr_chars[ii];
                    }
                }
            } else if to_lat {
                if let Some(ii) = cyr_chars.iter().position(|&c| c == tmp[i]) {
                    tmp[i] = lat_chars[ii];
                }
            } else {
                if let Some(ii) = udar_chars.iter().position(|&c| c == tmp[i]) {
                    if ii < udar_cyr_chars.len() {
                        tmp[i] = udar_cyr_chars[ii];
                    }
                }
            }
            i += 1;
        }
        tmp.into_iter().collect()
    }

    pub fn get_case_after_preposition(prep: &str) -> MorphCase {
        let map = PREP_CASES.get_or_init(Self::init_prep_cases);
        map.get(prep).copied().unwrap_or(MorphCase::UNDEFINED)
    }

    pub fn normalize_preposition(prep: &str) -> String {
        let map = PREP_NORMS.get_or_init(Self::init_prep_norms);
        map.get(prep).cloned().unwrap_or_else(|| prep.to_string())
    }

    fn init_prep_cases() -> HashMap<String, MorphCase> {
        let preps = [
            ("БЕЗ;ДО;ИЗ;ИЗЗА;ОТ;У;ДЛЯ;РАДИ;ВОЗЛЕ;ПОЗАДИ;ВПЕРЕДИ;БЛИЗ;ВБЛИЗИ;ВГЛУБЬ;ВВИДУ;ВДОЛЬ;ВЗАМЕН;ВКРУГ;ВМЕСТО;\
              ВНЕ;ВНИЗУ;ВНУТРИ;ВНУТРЬ;ВОКРУГ;ВРОДЕ;ВСЛЕД;ВСЛЕДСТВИЕ;ЗАМЕСТО;ИЗНУТРИ;КАСАТЕЛЬНО;КРОМЕ;\
              МИМО;НАВРОДЕ;НАЗАД;НАКАНУНЕ;НАПОДОБИЕ;НАПРОТИВ;НАСЧЕТ;ОКОЛО;ОТНОСИТЕЛЬНО;\
              ПОВЕРХ;ПОДЛЕ;ПОМИМО;ПОПЕРЕК;ПОРЯДКА;ПОСЕРЕДИНЕ;ПОСРЕДИ;ПОСЛЕ;ПРЕВЫШЕ;ПРЕЖДЕ;ПРОТИВ;СВЕРХ;\
              СВЫШЕ;СНАРУЖИ;СРЕДИ;СУПРОТИВ;ПУТЕМ;ПОСРЕДСТВОМ", MorphCase::GENITIVE),
            ("К;БЛАГОДАРЯ;ВОПРЕКИ;НАВСТРЕЧУ;СОГЛАСНО;СООБРАЗНО;ПАРАЛЛЕЛЬНО;ПОДОБНО;СООТВЕТСТВЕННО;СОРАЗМЕРНО", MorphCase::DATIVE),
            ("ПРО;ЧЕРЕЗ;СКВОЗЬ;СПУСТЯ", MorphCase::ACCUSATIVE),
            ("НАД;ПЕРЕД;ПРЕД", MorphCase::INSTRUMENTAL),
            ("ПРИ", MorphCase::PREPOSITIONAL),
            ("В;НА;О;ВКЛЮЧАЯ", MorphCase::from_value(MorphCase::ACCUSATIVE.value | MorphCase::PREPOSITIONAL.value)),
            ("МЕЖДУ", MorphCase::from_value(MorphCase::GENITIVE.value | MorphCase::INSTRUMENTAL.value)),
            ("ЗА;ПОД", MorphCase::from_value(MorphCase::ACCUSATIVE.value | MorphCase::INSTRUMENTAL.value)),
            ("ПО", MorphCase::from_value(MorphCase::DATIVE.value | MorphCase::ACCUSATIVE.value | MorphCase::PREPOSITIONAL.value)),
            ("С", MorphCase::from_value(MorphCase::GENITIVE.value | MorphCase::ACCUSATIVE.value | MorphCase::INSTRUMENTAL.value)),
        ];
        let mut map = HashMap::new();
        for (prep_str, case) in &preps {
            for v in prep_str.split(';') {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    map.insert(trimmed.to_string(), *case);
                }
            }
        }
        map
    }

    fn init_prep_norms() -> HashMap<String, String> {
        let norms_src = [
            "БЕЗ;БЕЗО",
            "ВБЛИЗИ;БЛИЗ",
            "В;ВО",
            "ВОКРУГ;ВКРУГ",
            "ВНУТРИ;ВНУТРЬ;ВОВНУТРЬ",
            "ВПЕРЕДИ;ВПЕРЕД",
            "ВСЛЕД;ВОСЛЕД",
            "ВМЕСТО;ЗАМЕСТО",
            "ИЗ;ИЗО",
            "К;КО",
            "МЕЖДУ;МЕЖ;ПРОМЕЖДУ;ПРОМЕЖ",
            "НАД;НАДО",
            "О;ОБ;ОБО",
            "ОТ;ОТО",
            "ПЕРЕД;ПРЕД;ПРЕДО;ПЕРЕДО",
            "ПОД;ПОДО",
            "ПОСЕРЕДИНЕ;ПОСРЕДИ;ПОСЕРЕДЬ",
            "С;СО",
            "СРЕДИ;СРЕДЬ;СЕРЕДЬ",
            "ЧЕРЕЗ;ЧРЕЗ",
        ];
        let mut map = HashMap::new();
        for s in &norms_src {
            let vars: Vec<&str> = s.split(';').collect();
            for i in 1..vars.len() {
                map.insert(vars[i].to_string(), vars[0].to_string());
            }
        }
        map
    }
}
