/// Static lookup tables for Decree analyzer.
///
/// Type keywords and standard identifiers from DecreeToken.Initialize() in C# source.

use std::collections::HashMap;
use std::sync::OnceLock;
use super::decree_referent::DecreeKind;

pub struct TypeEntry {
    pub canonical: &'static str,
    pub kind: DecreeKind,
    /// True if the keyword itself IS the type (no adjective prefix expected)
    pub is_standalone: bool,
}

struct Tables {
    types: HashMap<String, TypeEntry>,
    /// Standard abbreviations (ГОСТ, ТУ, ISO, etc.) — matched case-sensitively
    standards: HashMap<String, &'static str>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn get_tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let mut types: HashMap<String, TypeEntry> = HashMap::new();
        let mut standards: HashMap<String, &'static str> = HashMap::new();

        // ── Law-type keywords ─────────────────────────────────────────────────
        macro_rules! add_type {
            ($key:expr, $canon:expr, $kind:expr) => {
                types.insert($key.to_uppercase(), TypeEntry {
                    canonical: $canon,
                    kind: $kind,
                    is_standalone: true,
                });
            };
        }

        // Laws
        add_type!("ЗАКОН",                     "закон",                    DecreeKind::Law);
        add_type!("ФЕДЕРАЛЬНЫЙ ЗАКОН",          "Федеральный закон",        DecreeKind::Law);
        add_type!("КОНСТИТУЦИОННЫЙ ЗАКОН",      "Конституционный закон",    DecreeKind::Law);
        add_type!("ФЕДЕРАЛЬНЫЙ КОНСТИТУЦИОННЫЙ ЗАКОН", "Федеральный конституционный закон", DecreeKind::Law);
        add_type!("ФЗ",                         "Федеральный закон",        DecreeKind::Law);
        add_type!("ФКЗ",                        "Федеральный конституционный закон", DecreeKind::Law);
        add_type!("ЗАКОНОПРОЕКТ",               "Законопроект",             DecreeKind::Project);

        // Orders/Decrees
        add_type!("ПРИКАЗ",                    "приказ",                   DecreeKind::Order);
        add_type!("УКАЗ",                      "указ",                     DecreeKind::Order);
        add_type!("УКАЗАНИЕ",                  "указание",                 DecreeKind::Order);
        add_type!("ПОСТАНОВЛЕНИЕ",             "постановление",            DecreeKind::Order);
        add_type!("РАСПОРЯЖЕНИЕ",              "распоряжение",             DecreeKind::Order);
        add_type!("ДИРЕКТИВА",                 "директива",                DecreeKind::Order);
        add_type!("РЕШЕНИЕ",                   "решение",                  DecreeKind::Order);
        add_type!("РЕЗОЛЮЦИЯ",                 "резолюция",                DecreeKind::Order);
        add_type!("ДЕКРЕТ",                    "декрет",                   DecreeKind::Order);
        add_type!("ПОРУЧЕНИЕ",                 "поручение",                DecreeKind::Order);

        // Codes
        add_type!("КОДЕКС",                    "Кодекс",                   DecreeKind::Kodex);
        add_type!("УК",                        "Уголовный кодекс",         DecreeKind::Kodex);
        add_type!("УПК",                       "Уголовно-процессуальный кодекс", DecreeKind::Kodex);
        add_type!("ГК",                        "Гражданский кодекс",       DecreeKind::Kodex);
        add_type!("ГПК",                       "Гражданский процессуальный кодекс", DecreeKind::Kodex);
        add_type!("ТК",                        "Трудовой кодекс",          DecreeKind::Kodex);
        add_type!("НК",                        "Налоговый кодекс",         DecreeKind::Kodex);
        add_type!("ЖК",                        "Жилищный кодекс",          DecreeKind::Kodex);
        add_type!("ЗК",                        "Земельный кодекс",         DecreeKind::Kodex);
        add_type!("КоАП",                      "КоАП",                     DecreeKind::Kodex);
        add_type!("КОАП",                      "КоАП",                     DecreeKind::Kodex);

        // Charter / Constitution
        add_type!("КОНСТИТУЦИЯ",               "Конституция",              DecreeKind::Ustav);
        add_type!("УСТАВ",                     "Устав",                    DecreeKind::Ustav);
        add_type!("ХАРТИЯ",                    "Хартия",                   DecreeKind::Ustav);

        // Conventions / Agreements
        add_type!("КОНВЕНЦИЯ",                 "конвенция",                DecreeKind::Konvention);
        add_type!("ПАКТ",                      "пакт",                     DecreeKind::Konvention);

        // Contracts
        add_type!("ДОГОВОР",                   "договор",                  DecreeKind::Contract);
        add_type!("КОНТРАКТ",                  "контракт",                 DecreeKind::Contract);
        add_type!("СОГЛАШЕНИЕ",                "соглашение",               DecreeKind::Contract);
        add_type!("ГОСУДАРСТВЕННЫЙ КОНТРАКТ",  "Государственный контракт", DecreeKind::Contract);
        add_type!("ПРОТОКОЛ",                  "протокол",                 DecreeKind::Contract);
        add_type!("ДОВЕРЕННОСТЬ",              "доверенность",             DecreeKind::Contract);
        add_type!("ДЕКЛАРАЦИЯ",                "декларация",               DecreeKind::Contract);

        // Programs
        add_type!("ПРОГРАММА",                 "программа",                DecreeKind::Program);
        add_type!("ГОСУДАРСТВЕННАЯ ПРОГРАММА", "программа",                DecreeKind::Program);

        // Instructions / Regulations
        add_type!("ИНСТРУКЦИЯ",               "инструкция",               DecreeKind::Order);
        add_type!("ПОЛОЖЕНИЕ",                "положение",                DecreeKind::Order);
        add_type!("РЕГЛАМЕНТ",                "регламент",                DecreeKind::Order);
        add_type!("ПРАВИЛА",                  "правила",                  DecreeKind::Order);
        add_type!("ПОРЯДОК",                  "порядок",                  DecreeKind::Order);

        // Technical specs
        add_type!("ТЗ",                       "Техническое задание",      DecreeKind::Tz);

        // Classifiers
        add_type!("КЛАССИФИКАТОР",            "классификатор",            DecreeKind::Classifier);
        add_type!("ОБЩЕРОССИЙСКИЙ КЛАССИФИКАТОР", "классификатор",        DecreeKind::Classifier);

        // Licenses
        add_type!("ЛИЦЕНЗИЯ",                 "лицензия",                 DecreeKind::License);
        add_type!("СЕРТИФИКАТ",               "сертификат",               DecreeKind::License);

        // Named document types
        add_type!("ПИСЬМО",                   "письмо",                   DecreeKind::Order);
        add_type!("ИНФОРМАЦИОННОЕ ПИСЬМО",    "информационное письмо",    DecreeKind::Order);
        add_type!("ЗАКЛЮЧЕНИЕ",               "заключение",               DecreeKind::Order);
        add_type!("ПРИГОВОР",                 "приговор",                 DecreeKind::Order);
        add_type!("ОПРЕДЕЛЕНИЕ",              "определение",              DecreeKind::Order);

        // ── Standards ─────────────────────────────────────────────────────────
        for (abbr, canon) in &[
            ("ГОСТ", "ГОСТ"),
            ("ОСТ", "ОСТ"),
            ("ПНСТ", "ПНСТ"),
            ("РСТ", "РСТ"),
            ("РМГ", "РМГ"),
            ("ПБУ", "ПБУ"),
            ("ISO", "ISO"),
            ("ISO/IEC", "ISO/IEC"),
            ("ТУ", "ТУ"),       // Технические условия (only when followed by number)
        ] {
            standards.insert(abbr.to_string(), canon);
            types.insert(abbr.to_string(), TypeEntry {
                canonical: canon,
                kind: DecreeKind::Standard,
                is_standalone: true,
            });
        }
        // ГОСТ Р variant
        standards.insert("ГОСТ Р".to_string(), "ГОСТ Р");
        types.insert("ГОСТ Р".to_string(), TypeEntry {
            canonical: "ГОСТ Р",
            kind: DecreeKind::Standard,
            is_standalone: true,
        });

        Tables { types, standards }
    })
}

/// Look up a decree type keyword (uppercase).
pub fn lookup_type(key: &str) -> Option<&'static TypeEntry> {
    get_tables().types.get(key).map(|e| {
        unsafe { &*(e as *const TypeEntry) }
    })
}

/// Check if a string is a known standard abbreviation.
pub fn is_standard_abbr(key: &str) -> bool {
    get_tables().standards.contains_key(key)
}
