/// named_table.rs — keyword lookup tables for NamedEntityAnalyzer.
///
/// Mirrors NamedItemToken.Initialize() from C# source.
use std::collections::HashMap;
use std::sync::OnceLock;

/// Kind of named entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKind {
    Planet,
    Location,
    Monument,
    Building,
    Art,
}

impl NamedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NamedKind::Planet => "Planet",
            NamedKind::Location => "Location",
            NamedKind::Monument => "Monument",
            NamedKind::Building => "Building",
            NamedKind::Art => "Art",
        }
    }
}

/// A type keyword entry (e.g. "ПЛАНЕТА" → Planet).
#[derive(Debug, Clone)]
pub struct TypeEntry {
    pub kind: NamedKind,
    pub canonical: String,
}

/// A well-known name entry (e.g. "МАРС" → Planet with optional sub-type label).
#[derive(Debug, Clone)]
pub struct NameEntry {
    pub kind: NamedKind,
    pub canonical: String,
    pub type_label: Option<String>, // e.g. "океан", "река", "континент"
}

struct Tables {
    types: HashMap<String, TypeEntry>,
    names: HashMap<String, NameEntry>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn tables() -> &'static Tables {
    TABLES.get_or_init(build_tables)
}

fn build_tables() -> Tables {
    let mut types: HashMap<String, TypeEntry> = HashMap::new();
    let mut names: HashMap<String, NameEntry> = HashMap::new();

    // ── Planets ──────────────────────────────────────────────────────────────
    for kw in &["ПЛАНЕТА", "ЗВЕЗДА", "КОМЕТА", "МЕТЕОРИТ", "СОЗВЕЗДИЕ", "ГАЛАКТИКА"] {
        types.insert(kw.to_string(), TypeEntry { kind: NamedKind::Planet, canonical: kw.to_string() });
    }
    for nm in &["СОЛНЦЕ", "МЕРКУРИЙ", "ВЕНЕРА", "ЗЕМЛЯ", "МАРС", "ЮПИТЕР", "САТУРН",
                "УРАН", "НЕПТУН", "ПЛУТОН", "ЛУНА", "ДЕЙМОС", "ФОБОС", "ГАНИМЕД", "КАЛЛИСТО"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Planet, canonical: nm.to_string(), type_label: None });
    }

    // ── Locations (type keywords) ─────────────────────────────────────────────
    for kw in &["РЕКА", "ОЗЕРО", "МОРЕ", "ОКЕАН", "ЗАЛИВ", "ПРОЛИВ", "ПОБЕРЕЖЬЕ",
                "КОНТИНЕНТ", "ОСТРОВ", "ПОЛУОСТРОВ", "МЫС", "ГОРА", "ГОРНЫЙ ХРЕБЕТ",
                "ПЕРЕВАЛ", "ПАДЬ", "ЛЕС", "САД", "ЗАПОВЕДНИК", "ЗАКАЗНИК", "ДОЛИНА",
                "УЩЕЛЬЕ", "РАВНИНА", "БЕРЕГ"] {
        types.insert(kw.to_string(), TypeEntry { kind: NamedKind::Location, canonical: kw.to_string() });
    }

    // ── Well-known ocean adjectives (ТИХИЙ ОКЕАН etc.) ────────────────────────
    for nm in &["ТИХИЙ", "АТЛАНТИЧЕСКИЙ", "ИНДИЙСКИЙ", "СЕВЕРО-ЛЕДОВИТЫЙ"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Location, canonical: nm.to_string(), type_label: Some("океан".to_string()) });
    }

    // ── Continents ────────────────────────────────────────────────────────────
    for nm in &["ЕВРАЗИЯ", "АФРИКА", "АМЕРИКА", "АВСТРАЛИЯ", "АНТАРКТИДА"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Location, canonical: nm.to_string(), type_label: Some("континент".to_string()) });
    }

    // ── Well-known rivers ─────────────────────────────────────────────────────
    for nm in &["ВОЛГА", "НЕВА", "АМУР", "АНГАРА", "ЛЕНА", "ИРТЫШ", "ДНЕПР", "ДОН",
                "ДНЕСТР", "РЕЙН", "ТИГР", "ЕВФРАТ", "ИОРДАН", "МИССИСИПИ", "АМАЗОНКА",
                "ТЕМЗА", "СЕНА", "НИЛ", "ЯНЦЗЫ", "ХУАНХЭ", "НИГЕР", "ЕНИСЕЙ",
                "КАМА", "ОКА", "ВИСЛА", "ДАУГАВА", "НЕМАН", "МЕЗЕНЬ", "КУБАНЬ"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Location, canonical: nm.to_string(), type_label: Some("река".to_string()) });
    }

    // ── Well-known regions / geographic areas ────────────────────────────────
    for nm in &["ЕВРОПА", "АЗИЯ", "АРКТИКА", "КАВКАЗ", "ПРИБАЛТИКА", "СИБИРЬ",
                "ЗАПОЛЯРЬЕ", "ЧУКОТКА", "БАЛКАНЫ", "СКАНДИНАВИЯ", "ОКЕАНИЯ", "АЛЯСКА",
                "УРАЛ", "ПОВОЛЖЬЕ", "ПРИМОРЬЕ", "КУРИЛЫ", "ТИБЕТ", "ГИМАЛАИ",
                "АЛЬПЫ", "САХАРА", "ГОБИ", "БАЙКОНУР", "ЧЕРНОБЫЛЬ"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Location, canonical: nm.to_string(), type_label: None });
    }

    // ── Monuments ────────────────────────────────────────────────────────────
    for kw in &["ПАМЯТНИК", "МОНУМЕНТ", "МЕМОРИАЛ", "БЮСТ", "ОБЕЛИСК", "МОГИЛА",
                "МАВЗОЛЕЙ", "ЗАХОРОНЕНИЕ"] {
        types.insert(kw.to_string(), TypeEntry { kind: NamedKind::Monument, canonical: kw.to_string() });
    }
    for nm in &["ВЕЧНЫЙ ОГОНЬ", "МЕДНЫЙ ВСАДНИК", "ПОКЛОННАЯ ГОРА"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Monument, canonical: nm.to_string(), type_label: None });
    }

    // ── Art ──────────────────────────────────────────────────────────────────
    for kw in &["ФИЛЬМ", "КИНОФИЛЬМ", "ТЕЛЕФИЛЬМ", "СЕРИАЛ", "ТЕЛЕСЕРИАЛ",
                "БЛОКБАСТЕР", "КОМЕДИЯ", "БОЕВИК", "АЛЬБОМ", "ДИСК", "ПЕСНЯ",
                "СИНГЛ", "СПЕКТАКЛЬ", "МЮЗИКЛ", "ТЕЛЕШОУ",
                "КНИГА", "РАССКАЗ", "РОМАН", "ПОЭМА", "СТИХ", "СТИХОТВОРЕНИЕ"] {
        types.insert(kw.to_string(), TypeEntry { kind: NamedKind::Art, canonical: kw.to_string() });
    }

    // ── Buildings ────────────────────────────────────────────────────────────
    for kw in &["ДВОРЕЦ", "КРЕМЛЬ", "ЗАМОК", "КРЕПОСТЬ", "УСАДЬБА",
                "ЗДАНИЕ", "ШТАБ-КВАРТИРА", "ЖЕЛЕЗНОДОРОЖНЫЙ ВОКЗАЛ", "ВОКЗАЛ",
                "АВТОВОКЗАЛ", "АЭРОВОКЗАЛ", "АЭРОПОРТ", "АЭРОДРОМ",
                "БИБЛИОТЕКА", "СОБОР", "МЕЧЕТЬ", "СИНАГОГА", "ЛАВРА", "ХРАМ", "ЦЕРКОВЬ"] {
        types.insert(kw.to_string(), TypeEntry { kind: NamedKind::Building, canonical: kw.to_string() });
    }
    for nm in &["КРЕМЛЬ", "КАПИТОЛИЙ", "БЕЛЫЙ ДОМ", "БИГ БЕН"] {
        names.insert(nm.to_string(), NameEntry { kind: NamedKind::Building, canonical: nm.to_string(), type_label: None });
    }

    Tables { types, names }
}

/// Look up a type keyword (uppercase). Returns Some(TypeEntry) if it's a recognized type word.
pub fn lookup_type(key: &str) -> Option<&'static TypeEntry> {
    tables().types.get(key)
}

/// Look up a well-known name (uppercase). Returns Some(NameEntry) if it's recognized by itself.
pub fn lookup_name(key: &str) -> Option<&'static NameEntry> {
    tables().names.get(key)
}
