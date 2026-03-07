/// street_table.rs — Russian/Ukrainian/English street type keywords.
use std::collections::HashMap;
use std::sync::OnceLock;

/// Maps uppercase surface/abbreviation → canonical street type name.
pub struct StreetTypeEntry {
    pub canonical: String, // e.g. "улица"
}

static TABLE: OnceLock<HashMap<String, StreetTypeEntry>> = OnceLock::new();

fn table() -> &'static HashMap<String, StreetTypeEntry> {
    TABLE.get_or_init(build)
}

fn build() -> HashMap<String, StreetTypeEntry> {
    let mut m: HashMap<String, StreetTypeEntry> = HashMap::new();

    let entries: &[(&[&str], &str)] = &[
        // Улица
        (&["УЛ", "УЛ.", "УЛИЦА", "ВУЛИЦЯ"], "улица"),
        // Переулок
        (&["ПЕР", "ПЕР.", "ПЕРЕУЛОК", "ПРОВУЛОК"], "переулок"),
        // Проспект
        (&["ПР", "ПР.", "ПРОСП", "ПРОСП.", "ПРОСПЕКТ", "ПР-Т", "ПР-КТ", "ПРСПЕКТ", "ПРКТ", "ПРОСПЭКТ"], "проспект"),
        // Площадь
        (&["ПЛ", "ПЛ.", "ПЛОЩАДЬ", "МАЙДАН"], "площадь"),
        // Шоссе
        (&["Ш", "Ш.", "ШОС", "ШОС.", "ШОССЕ", "ШОСЕ"], "шоссе"),
        // Набережная
        (&["НАБ", "НАБ.", "НАБЕРЕЖНАЯ", "НАБЕРЕЖНА"], "набережная"),
        // Бульвар
        (&["БУЛ", "БУЛ.", "БУЛЬВАР"], "бульвар"),
        // Проезд
        (&["ПРД", "ПРД.", "ПРОЕЗД", "ПР-ЗД", "ПР-Д"], "проезд"),
        // Тупик
        (&["ТУП", "ТУП.", "ТУПИК"], "тупик"),
        // Линия
        (&["ЛИН", "ЛИН.", "ЛИНИЯ"], "линия"),
        // Аллея
        (&["АЛ", "АЛЛ.", "АЛЛЕЯ"], "аллея"),
        // Переулок (alternative)
        (&["ПРКТ"], "проспект"),
        // Квартал
        (&["КВ-Л", "КВАРТАЛ"], "квартал"),
        // Микрорайон
        (&["МКР", "МКР.", "МКРН", "МКРН.", "МИКРОРАЙОН"], "микрорайон"),
        // Дорога
        (&["ДОР", "ДОР.", "ДОРОГА"], "дорога"),
        // Тракт
        (&["ТР", "ТР.", "ТРАКТ"], "тракт"),
        // Магистраль
        (&["МАГИСТРАЛЬ", "МАГІСТРАЛЬ"], "магистраль"),
        // English
        (&["ST", "ST.", "STREET", "STR"], "street"),
        (&["AVE", "AVE.", "AVENUE"], "avenue"),
        (&["BLVD", "BLVD.", "BOULEVARD"], "boulevard"),
        (&["RD", "RD.", "ROAD"], "road"),
        (&["LN", "LN.", "LANE"], "lane"),
        (&["DR", "DR.", "DRIVE"], "drive"),
    ];

    for (keys, canonical) in entries {
        for k in *keys {
            m.entry(k.to_string()).or_insert_with(|| StreetTypeEntry {
                canonical: canonical.to_string(),
            });
        }
    }
    m
}

/// Returns Some(canonical type name) if `key` is a known street type abbreviation or full name.
pub fn lookup_street_type(key: &str) -> Option<&'static StreetTypeEntry> {
    table().get(key)
}

/// Abbreviations for house number
pub const HOUSE_ABBRS: &[&str] = &["Д", "Д.", "ДОМ", "ДОМОВЛ", "ДОМОВЛАДЕНИЕ"];
/// Abbreviations for apartment
pub const FLAT_ABBRS: &[&str] = &["КВ", "КВ.", "КВАРТИРА", "ПОМЕЩЕНИЕ", "ПОМ", "ПОМ."];
/// Abbreviations for corpus
pub const CORPUS_ABBRS: &[&str] = &["КОРП", "КОРП.", "КОРПУС", "К."];
/// Abbreviations for building/structure
pub const BUILDING_ABBRS: &[&str] = &["СТР", "СТР.", "СТРОЕНИЕ"];
/// Abbreviations for office
pub const OFFICE_ABBRS: &[&str] = &["ОФ", "ОФ.", "ОФИС"];
/// Abbreviations for floor
pub const FLOOR_ABBRS: &[&str] = &["ЭТ", "ЭТ.", "ЭТАЖ"];
