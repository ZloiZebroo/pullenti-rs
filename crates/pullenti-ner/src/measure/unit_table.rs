/// Static unit lookup table — mirrors UnitsHelper.Initialize() from C#.
///
/// Maps uppercase unit name / abbreviation → (MeasureKind, canonical_abbreviation).
/// SI-prefixed variants are generated automatically for all base units that support them.

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::measure::measure_kind::MeasureKind;

#[derive(Clone)]
pub struct UnitInfo {
    pub kind:      MeasureKind,
    /// Short canonical abbreviation (e.g. "км", "кг")
    pub canonical: &'static str,
    /// Full canonical name (e.g. "километр")
    pub fullname:  &'static str,
}

static TABLE: OnceLock<HashMap<String, UnitInfo>> = OnceLock::new();

/// Return `(kind, canonical, fullname)` for a unit name, or `None`.
pub fn lookup(name: &str) -> Option<&'static UnitInfo> {
    TABLE.get_or_init(build_table).get(&name.to_uppercase())
}

// ── SI prefix definitions ─────────────────────────────────────────────────────

struct SiPrefix {
    name_cyr: &'static str,
    name_lat: &'static str,
    prefix_ru: &'static str,
    abbr_cyr:  &'static str,
    abbr_lat:  &'static str,
}

const SI_PREFIXES: &[SiPrefix] = &[
    SiPrefix { name_cyr: "кило",  name_lat: "kilo",  prefix_ru: "кило",  abbr_cyr: "К",  abbr_lat: "K"  },
    SiPrefix { name_cyr: "мега",  name_lat: "mega",  prefix_ru: "мега",  abbr_cyr: "М",  abbr_lat: "M"  },
    SiPrefix { name_cyr: "гига",  name_lat: "giga",  prefix_ru: "гига",  abbr_cyr: "Г",  abbr_lat: "G"  },
    SiPrefix { name_cyr: "тера",  name_lat: "tera",  prefix_ru: "тера",  abbr_cyr: "Т",  abbr_lat: "T"  },
    SiPrefix { name_cyr: "деци",  name_lat: "deci",  prefix_ru: "деци",  abbr_cyr: "Д",  abbr_lat: "D"  },
    SiPrefix { name_cyr: "санти", name_lat: "centi", prefix_ru: "санти", abbr_cyr: "С",  abbr_lat: "C"  },
    SiPrefix { name_cyr: "милли", name_lat: "milli", prefix_ru: "милли", abbr_cyr: "М",  abbr_lat: "M"  },
    SiPrefix { name_cyr: "микро", name_lat: "micro", prefix_ru: "микро", abbr_cyr: "МК", abbr_lat: "MK" },
    SiPrefix { name_cyr: "нано",  name_lat: "nano",  prefix_ru: "нано",  abbr_cyr: "Н",  abbr_lat: "N"  },
    SiPrefix { name_cyr: "пико",  name_lat: "pico",  prefix_ru: "пико",  abbr_cyr: "П",  abbr_lat: "P"  },
];

// ── Base unit definition ──────────────────────────────────────────────────────

struct BaseUnit {
    /// All recognized name/abbreviation strings (uppercase)
    names: &'static [&'static str],
    kind: MeasureKind,
    canonical: &'static str,
    fullname: &'static str,
    /// SI prefixes to generate (indices into SI_PREFIXES)
    si_prefixes: &'static [usize],
    /// Abbr suffixes for SI prefix generation: (cyr, lat)
    si_abbr_cyr: &'static str,
    si_abbr_lat: &'static str,
    /// Base name suffix for SI full names: (ru_names, en_names)
    si_base_names_ru: &'static [&'static str],
    si_base_names_en: &'static [&'static str],
}

// SI prefix indices
const KILO:  usize = 0;
const MEGA:  usize = 1;
const GIGA:  usize = 2;
const TERA:  usize = 3;
const DECI:  usize = 4;
const CENTI: usize = 5;
const MILLI: usize = 6;
const MICRO: usize = 7;
const NANO:  usize = 8;
const PICO:  usize = 9;

static BASES: &[BaseUnit] = &[
    // ── Count ────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["РАЗ", "TIMES", "РАЗОВ", "РАЗА"],
        kind: MeasureKind::Count, canonical: "раз", fullname: "раз",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Length ───────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["МЕТР", "МЕТРА", "МЕТРОВ", "МЕТРАХ", "МЕТРОВЫЙ", "МЕТРОВИЙ",
                 "M.", "М.", "METER", "METRE"],
        kind: MeasureKind::Length, canonical: "м", fullname: "метр",
        si_prefixes: &[KILO, DECI, CENTI, MILLI, MICRO, NANO],
        si_abbr_cyr: "М.", si_abbr_lat: "M.",
        si_base_names_ru: &["МЕТР", "МЕТРОВЫЙ"], si_base_names_en: &["METER", "METRE"],
    },
    BaseUnit {
        names: &["МИЛЯ", "МИЛЬ", "MILE", "MILES", "МОРСКАЯ МИЛЯ", "NMI"],
        kind: MeasureKind::Length, canonical: "миль", fullname: "морская миля",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ФУТ", "FT.", "FT", "FOOT", "FEET"],
        kind: MeasureKind::Length, canonical: "фут", fullname: "фут",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ДЮЙМ", "IN", "INCH", "INCHES", "\""],
        kind: MeasureKind::Length, canonical: "дюйм", fullname: "дюйм",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Area ─────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["АР", "ARE", "СОТКА"],
        kind: MeasureKind::Area, canonical: "ар", fullname: "ар",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГЕКТАР", "ГА", "GA", "HECTARE"],
        kind: MeasureKind::Area, canonical: "га", fullname: "гектар",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["КВ.М.", "КВ.МЕТР", "КВМ", "М²", "M²", "SQM", "КВМ.",
                 "КВАДРАТНЫЙ МЕТР", "КВАДРАТНИЙ МЕТР"],
        kind: MeasureKind::Area, canonical: "кв.м.", fullname: "квадратный метр",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["КВ.КМ.", "КМ²", "KM²", "КВАДРАТНЫЙ КИЛОМЕТР"],
        kind: MeasureKind::Area, canonical: "кв.км.", fullname: "квадратный километр",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Weight ───────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ГРАММ", "ГРАММОВ", "ГРАММА", "ГРАММАХ", "ГРАММОВЫЙ", "ГРАМОВИЙ",
                 "ГР.", "ГР", "G.", "GR.", "GR", "GRAM", "GRAMME"],
        kind: MeasureKind::Weight, canonical: "г", fullname: "грамм",
        si_prefixes: &[KILO, MILLI],
        si_abbr_cyr: "Г.;ГР.", si_abbr_lat: "G.;GR.",
        si_base_names_ru: &["ГРАММ", "ГРАММНЫЙ"], si_base_names_en: &["GRAM", "GRAMME"],
    },
    BaseUnit {
        names: &["ЦЕНТНЕР", "Ц.", "Ц", "CENTNER", "QUINTAL"],
        kind: MeasureKind::Weight, canonical: "ц", fullname: "центнер",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ТОННА", "ТОНН", "ТОННЫ", "ТОННЫЙ", "ТОННИХ", "Т.", "Т",
                 "T.", "T", "TONNE", "TON", "TONS"],
        kind: MeasureKind::Weight, canonical: "т", fullname: "тонна",
        si_prefixes: &[KILO, MEGA],
        si_abbr_cyr: "Т.", si_abbr_lat: "T.",
        si_base_names_ru: &["ТОННА", "ТОННЫЙ"], si_base_names_en: &["TONNE", "TON"],
    },
    // ── Volume ───────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ЛИТР", "ЛИТРА", "ЛИТРОВ", "ЛИТРАХ", "ЛИТРОВЫЙ", "ЛІТР", "ЛІТРОВИЙ",
                 "Л.", "Л", "L.", "L", "LITER", "LITRE"],
        kind: MeasureKind::Volume, canonical: "л", fullname: "литр",
        si_prefixes: &[MILLI, CENTI],
        si_abbr_cyr: "Л.", si_abbr_lat: "L.",
        si_base_names_ru: &["ЛИТР", "ЛИТРОВЫЙ"], si_base_names_en: &["LITER", "LITRE"],
    },
    BaseUnit {
        names: &["КУБ.М.", "КУБ.МЕТР", "М³", "M³", "CBM",
                 "КУБИЧЕСКИЙ МЕТР", "КУБІЧНИЙ МЕТР"],
        kind: MeasureKind::Volume, canonical: "куб.м.", fullname: "кубический метр",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГАЛЛОН", "ГАЛОН", "GALLON", "GAL"],
        kind: MeasureKind::Volume, canonical: "галлон", fullname: "галлон",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["БАРРЕЛЬ", "BBLS", "BARREL", "BARREL OF OIL"],
        kind: MeasureKind::Volume, canonical: "баррель", fullname: "баррель нефти",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Time ─────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["СЕКУНДА", "СЕКУНДЫ", "СЕКУНД", "СЕКУНДНЫЙ", "СЕКУНДНИЙ",
                 "С.", "С", "СЕК.", "СЕК", "S.", "SEC", "SECOND", "SECONDS"],
        kind: MeasureKind::Time, canonical: "сек", fullname: "секунда",
        si_prefixes: &[MILLI, MICRO],
        si_abbr_cyr: "С.;СЕК.", si_abbr_lat: "S.;SEC.",
        si_base_names_ru: &["СЕКУНДА", "СЕКУНДНЫЙ"], si_base_names_en: &["SECOND"],
    },
    BaseUnit {
        names: &["МИНУТА", "МИНУТЫ", "МИНУТ", "МИНУТНЫЙ", "ХВИЛИНА",
                 "МИН.", "МИН", "MIN.", "MIN", "MINUTE", "MINUTES"],
        kind: MeasureKind::Time, canonical: "мин", fullname: "минута",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ЧАС", "ЧАСА", "ЧАСОВ", "ЧАСОВОЙ", "ГОДИНА",
                 "Ч.", "Ч", "H.", "H", "HOUR", "HOURS"],
        kind: MeasureKind::Time, canonical: "ч", fullname: "час",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ДЕНЬ", "ДНЯ", "ДНЕЙ", "СУТОК", "СУТКИ",
                 "ДН.", "ДН", "Д.", "Д", "DAY", "DAYS"],
        kind: MeasureKind::Time, canonical: "дн", fullname: "день",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["НЕДЕЛЯ", "НЕДЕЛИ", "НЕДЕЛЬ", "НЕД.", "НЕД", "WEEK", "WEEKS"],
        kind: MeasureKind::Time, canonical: "нед", fullname: "неделя",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["МЕСЯЦ", "МЕСЯЦА", "МЕСЯЦЕВ", "МЕС.", "МЕС", "МІСЯЦЬ",
                 "MON", "MONTH", "MONTHS"],
        kind: MeasureKind::Time, canonical: "мес", fullname: "месяц",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["КВАРТАЛ", "КВАРТАЛОВ", "QUARTER"],
        kind: MeasureKind::Time, canonical: "квартал", fullname: "квартал",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГОД", "ГОДА", "ГОДОВ", "ЛЕТ", "Г.", "РІК", "YEAR", "YEARS"],
        kind: MeasureKind::Time, canonical: "г", fullname: "год",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Temperature ───────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ГРАДУС", "ГРАДУСА", "ГРАДУСОВ", "°", "DEGREE", "DEG", "DEG."],
        kind: MeasureKind::Temperature, canonical: "°", fullname: "градус",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГРАДУС ЦЕЛЬСИЯ", "ГРАДУС ПО ЦЕЛЬСИЮ", "°C", "°С", "CELSIUS", "CELSIUS DEGREE"],
        kind: MeasureKind::Temperature, canonical: "°C", fullname: "градус Цельсия",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГРАДУС ФАРЕНГЕЙТА", "°F", "FAHRENHEIT"],
        kind: MeasureKind::Temperature, canonical: "°F", fullname: "градус Фаренгейта",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ГРАДУС КЕЛЬВИНА", "КЕЛЬВИН", "°K", "°К", "K", "К", "KELVIN"],
        kind: MeasureKind::Temperature, canonical: "°К", fullname: "градус Кельвина",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Percent ───────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ПРОЦЕНТ", "ПРОЦЕНТА", "ПРОЦЕНТОВ", "ПРОЦ.", "ПРОЦ", "%", "PERCENT", "PERC"],
        kind: MeasureKind::Percent, canonical: "%", fullname: "процент",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Electrical: Voltage ───────────────────────────────────────────────────
    BaseUnit {
        names: &["ВОЛЬТ", "ВОЛЬТА", "ВОЛЬТОВ", "В.", "VAC", "VOLT", "VOLTS"],
        kind: MeasureKind::Voltage, canonical: "В", fullname: "вольт",
        si_prefixes: &[KILO, MEGA, MILLI, MICRO],
        si_abbr_cyr: "В.", si_abbr_lat: "V.",
        si_base_names_ru: &["ВОЛЬТ", "ВОЛЬТНЫЙ"], si_base_names_en: &["VOLT"],
    },
    // ── Power ────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ВАТТ", "ВАТТА", "ВАТТОВ", "ВТ", "W", "WATT", "WATTS"],
        kind: MeasureKind::Power, canonical: "Вт", fullname: "ватт",
        si_prefixes: &[KILO, MEGA, GIGA, MILLI],
        si_abbr_cyr: "ВТ.", si_abbr_lat: "W.",
        si_base_names_ru: &["ВАТТ", "ВАТТНЫЙ"], si_base_names_en: &["WATT", "WATTS"],
    },
    BaseUnit {
        names: &["ЛОШАДИНАЯ СИЛА", "Л.С.", "ЛС", "HP", "PS", "HORSEPOWER"],
        kind: MeasureKind::Power, canonical: "л.с.", fullname: "лошадиная сила",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Energy ───────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ДЖОУЛЬ", "ДЖ", "J", "JOULE", "JOULES"],
        kind: MeasureKind::Energy, canonical: "Дж", fullname: "джоуль",
        si_prefixes: &[KILO, MEGA, GIGA, TERA, MILLI],
        si_abbr_cyr: "ДЖ.", si_abbr_lat: "J.",
        si_base_names_ru: &["ДЖОУЛЬ"], si_base_names_en: &["JOULE"],
    },
    BaseUnit {
        names: &["КАЛОРИЯ", "КАЛОРИЙ", "ККАЛ", "КАЛ", "CAL", "KCAL", "CALORIE"],
        kind: MeasureKind::Energy, canonical: "кал", fullname: "калория",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Resistance ────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ОМ", "ОМОВ", "OHM", "OHMS", "Ω"],
        kind: MeasureKind::Resistance, canonical: "Ом", fullname: "Ом",
        si_prefixes: &[KILO, MEGA, GIGA, MICRO, MILLI],
        si_abbr_cyr: "ОМ", si_abbr_lat: "Ω",
        si_base_names_ru: &["ОМ"], si_base_names_en: &["OHM"],
    },
    // ── Current ──────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["АМПЕР", "АМПЕРА", "АМПЕР", "А", "A", "AMP", "AMPERE"],
        kind: MeasureKind::Current, canonical: "А", fullname: "ампер",
        si_prefixes: &[KILO, MEGA, GIGA, MICRO, MILLI],
        si_abbr_cyr: "А.", si_abbr_lat: "A.",
        si_base_names_ru: &["АМПЕР", "АМПЕРНЫЙ"], si_base_names_en: &["AMPERE", "AMP"],
    },
    // ── Frequency ─────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ГЕРЦ", "ГЕРЦА", "ГЦ", "HZ", "HERZ", "HERTZ"],
        kind: MeasureKind::Frequency, canonical: "Гц", fullname: "герц",
        si_prefixes: &[KILO, MEGA, GIGA, MICRO],
        si_abbr_cyr: "ГЦ.", si_abbr_lat: "HZ.",
        si_base_names_ru: &["ГЕРЦ", "ГЕРЦОВЫЙ"], si_base_names_en: &["HERZ", "HERTZ"],
    },
    // ── Pressure ─────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ПАСКАЛЬ", "ПАСКАЛЯ", "ПА", "PA", "PASCAL"],
        kind: MeasureKind::Pressure, canonical: "Па", fullname: "паскаль",
        si_prefixes: &[KILO, MEGA, GIGA, MICRO, MILLI],
        si_abbr_cyr: "ПА.", si_abbr_lat: "PA.",
        si_base_names_ru: &["ПАСКАЛЬ"], si_base_names_en: &["PASCAL"],
    },
    BaseUnit {
        names: &["БАР", "BAR", "BARS"],
        kind: MeasureKind::Pressure, canonical: "бар", fullname: "бар",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ММ.РТ.СТ.", "MM HG", "MMHG", "ТОРР", "TORR"],
        kind: MeasureKind::Pressure, canonical: "мм.рт.ст.", fullname: "миллиметр ртутного столба",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Data ─────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["БИТ", "БИТОВ", "BIT", "BITS"],
        kind: MeasureKind::Data, canonical: "бит", fullname: "бит",
        si_prefixes: &[KILO, MEGA, GIGA, TERA],
        si_abbr_cyr: "БИТ", si_abbr_lat: "BIT",
        si_base_names_ru: &["БИТ"], si_base_names_en: &["BIT"],
    },
    BaseUnit {
        names: &["БАЙТ", "БАЙТА", "БАЙТОВ", "BYTE", "BYTES"],
        kind: MeasureKind::Data, canonical: "байт", fullname: "байт",
        si_prefixes: &[KILO, MEGA, GIGA, TERA],
        si_abbr_cyr: "Б.", si_abbr_lat: "B.",
        si_base_names_ru: &["БАЙТ"], si_base_names_en: &["BYTE"],
    },
    // ── Angle ─────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["РАДИАН", "РАД", "RADIAN", "RAD"],
        kind: MeasureKind::Angle, canonical: "рад", fullname: "радиан",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Force ─────────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["НЬЮТОН", "НЬЮТОНА", "Н.", "Н", "N.", "N", "NEWTON"],
        kind: MeasureKind::Force, canonical: "Н", fullname: "ньютон",
        si_prefixes: &[KILO, MEGA, MICRO, MILLI],
        si_abbr_cyr: "Н.", si_abbr_lat: "N.",
        si_base_names_ru: &["НЬЮТОН"], si_base_names_en: &["NEWTON"],
    },
    // ── Luminous ──────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ЛЮКС", "ЛК", "LX", "LUX"],
        kind: MeasureKind::Luminous, canonical: "лк", fullname: "люкс",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    BaseUnit {
        names: &["ЛЮМЕН", "ЛМ", "LM", "LUMEN"],
        kind: MeasureKind::Luminous, canonical: "лм", fullname: "люмен",
        si_prefixes: &[], si_abbr_cyr: "", si_abbr_lat: "",
        si_base_names_ru: &[], si_base_names_en: &[],
    },
    // ── Capacity (electric) ───────────────────────────────────────────────────
    BaseUnit {
        names: &["ФАРАД", "ФА", "F", "FARAD"],
        kind: MeasureKind::Capacity, canonical: "Ф", fullname: "фарад",
        si_prefixes: &[KILO, MICRO, MILLI, NANO, PICO],
        si_abbr_cyr: "Ф.", si_abbr_lat: "F.",
        si_base_names_ru: &["ФАРАД"], si_base_names_en: &["FARAD"],
    },
    // ── Radiation ─────────────────────────────────────────────────────────────
    BaseUnit {
        names: &["ЗИВЕРТ", "ЗВ", "SV", "SIEVERT"],
        kind: MeasureKind::Radiation, canonical: "Зв", fullname: "зиверт",
        si_prefixes: &[KILO, MEGA, GIGA, MICRO, MILLI, NANO],
        si_abbr_cyr: "ЗВ.", si_abbr_lat: "SV.",
        si_base_names_ru: &["ЗИВЕРТ"], si_base_names_en: &["SIEVERT"],
    },
    BaseUnit {
        names: &["БЕККЕРЕЛЬ", "БК.", "BQ.", "BECQUEREL"],
        kind: MeasureKind::Radiation, canonical: "Бк", fullname: "беккерель",
        si_prefixes: &[KILO, MEGA, MICRO, MILLI, NANO],
        si_abbr_cyr: "БК.", si_abbr_lat: "BQ.",
        si_base_names_ru: &["БЕККЕРЕЛЬ"], si_base_names_en: &["BECQUEREL"],
    },
];

// ── Table builder ─────────────────────────────────────────────────────────────

fn build_table() -> HashMap<String, UnitInfo> {
    let mut map: HashMap<String, UnitInfo> = HashMap::new();

    for base in BASES {
        let info = UnitInfo { kind: base.kind, canonical: base.canonical, fullname: base.fullname };

        for &name in base.names {
            map.entry(name.to_string()).or_insert_with(|| info.clone());
        }

        // Generate SI-prefixed variants
        for &si_idx in base.si_prefixes {
            let p = &SI_PREFIXES[si_idx];
            // Determine canonical abbreviation for the prefixed unit
            // e.g., "К" + "М." → "КМ", but we simplify to prefixed fullname
            let pref_canonical = if base.si_abbr_cyr.is_empty() {
                format!("{}{}", p.abbr_cyr.to_lowercase(), base.canonical)
            } else {
                // Use first abbr
                let base_abbr = base.si_abbr_cyr.split(';').next().unwrap_or("");
                format!("{}{}", p.abbr_cyr.to_lowercase(), base_abbr.trim_end_matches('.').to_lowercase())
            };
            let pref_fullname_owned = format!("{}{}", p.name_cyr, base.fullname);

            // Box the strings so they're 'static — we use a leak trick since this is init-once
            let leaked_canonical = Box::leak(pref_canonical.into_boxed_str());
            let leaked_fullname = Box::leak(pref_fullname_owned.into_boxed_str());
            let pref_info = UnitInfo { kind: base.kind, canonical: leaked_canonical, fullname: leaked_fullname };

            // Insert prefixed full names (RU)
            for &base_name in base.si_base_names_ru {
                let prefixed_ru = format!("{}{}", p.prefix_ru.to_uppercase(), base_name);
                map.entry(prefixed_ru).or_insert_with(|| pref_info.clone());
                let prefixed_cyr = format!("{}{}", p.abbr_cyr, base_name);
                map.entry(prefixed_cyr).or_insert_with(|| pref_info.clone());
            }
            // Insert prefixed EN names
            for &base_name in base.si_base_names_en {
                let prefixed_en = format!("{}{}", p.name_lat.to_uppercase(), base_name);
                map.entry(prefixed_en).or_insert_with(|| pref_info.clone());
                let prefixed_lat = format!("{}{}", p.abbr_lat, base_name);
                map.entry(prefixed_lat).or_insert_with(|| pref_info.clone());
            }
            // Insert prefixed abbreviations (cyr + lat)
            for abbr in base.si_abbr_cyr.split(';') {
                let k = format!("{}{}", p.abbr_cyr, abbr.trim());
                map.entry(k).or_insert_with(|| pref_info.clone());
            }
            for abbr in base.si_abbr_lat.split(';') {
                let k = format!("{}{}", p.abbr_lat, abbr.trim());
                map.entry(k).or_insert_with(|| pref_info.clone());
            }
        }
    }

    // Extra hand-crafted common lookups for abbreviations that appear in text
    let extras: &[(&str, MeasureKind, &'static str, &'static str)] = &[
        ("КМ",  MeasureKind::Length, "км",  "километр"),
        ("CM",  MeasureKind::Length, "см",  "сантиметр"),
        ("MM",  MeasureKind::Length, "мм",  "миллиметр"),
        ("КГ",  MeasureKind::Weight, "кг",  "килограмм"),
        ("MG",  MeasureKind::Weight, "мг",  "миллиграмм"),
        ("МЛ",  MeasureKind::Volume, "мл",  "миллилитр"),
        ("ML",  MeasureKind::Volume, "мл",  "миллилитр"),
        ("МГЦ", MeasureKind::Frequency, "МГц", "мегагерц"),
        ("ГГЦ", MeasureKind::Frequency, "ГГц", "гигагерц"),
        ("КВТ", MeasureKind::Power, "кВт", "киловатт"),
        ("МВТ", MeasureKind::Power, "МВт", "мегаватт"),
        ("КВТ.", MeasureKind::Power, "кВт", "киловатт"),
        ("КВ",   MeasureKind::Voltage, "кВ",  "киловольт"),
        ("МКМ",  MeasureKind::Length, "мкм", "микрометр"),
        ("НМ",   MeasureKind::Length, "нм",  "нанометр"),
    ];
    for &(name, kind, canonical, fullname) in extras {
        map.entry(name.to_string()).or_insert(UnitInfo { kind, canonical, fullname });
    }

    map
}
