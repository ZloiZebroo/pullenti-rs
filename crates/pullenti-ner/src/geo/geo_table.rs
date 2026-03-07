/// Geo lookup tables loaded from embedded t.dat (countries + regions)
/// and c.dat (cities).  Both files are gzip-compressed XML.
///
/// Lookup flow:
///   1.  `lookup_name(s)` → looks up a single uppercase name/acronym
///       and returns a reference to a `GeoEntry`.
///   2.  `lookup_adj(s)` → looks up an adjective form (e.g. "МОСКОВСКАЯ")
///       and returns the entry (regions only).
///   3.  `type_keyword(s)` → classifies a token as a territory type keyword
///       and returns its canonical lowercase form.

use std::collections::HashMap;
use std::sync::OnceLock;

use pullenti_morph::internal::morph_deserializer::MorphDeserializer;

// ── Public entry type ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GeoEntryKind {
    State,
    Region,
    City,
}

#[derive(Debug, Clone)]
pub struct GeoEntry {
    pub kind:           GeoEntryKind,
    /// Canonical type string (lowercase), e.g. "государство", "область", "город"
    pub type_str:       String,
    /// ISO 3166-1 alpha-2 code (uppercase), countries only.
    pub alpha2:         Option<String>,
    /// Shortest canonical display name (uppercase).
    pub canonical_name: String,
    /// All name forms stored in the entry (uppercase).
    pub all_names:      Vec<String>,
}

// ── Static tables ─────────────────────────────────────────────────────────────

struct Tables {
    /// name/acronym → entry index
    name_map: HashMap<String, usize>,
    /// adjective form → entry index  (regions only)
    adj_map:  HashMap<String, usize>,
    entries:  Vec<GeoEntry>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn get_tables() -> &'static Tables {
    TABLES.get_or_init(build_tables)
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn lookup_name(name: &str) -> Option<&'static GeoEntry> {
    let t = get_tables();
    t.name_map.get(&name.to_uppercase()).map(|&i| &t.entries[i])
}

pub fn lookup_adj(name: &str) -> Option<&'static GeoEntry> {
    let t = get_tables();
    t.adj_map.get(&name.to_uppercase()).map(|&i| &t.entries[i])
}

/// Returns true if the uppercase word is registered in the geo name table as a
/// standalone single-word entry.  Used to distinguish city-country patterns
/// ("Bangkok, Thailand") from person-name patterns ("Pierre Andrews").
pub fn is_likely_geo_name(word: &str) -> bool {
    lookup_name(word).is_some()
}

/// Classify a token string as a territory type keyword.
/// Returns (canonical_type_str, is_always_prefix) or None.
pub fn type_keyword(s: &str) -> Option<(&'static str, bool)> {
    TYPE_KEYWORDS.iter().find_map(|&(k, v, prefix)| {
        if k == s { Some((v, prefix)) } else { None }
    })
}

/// Returns true if `s` is a city type prefix abbreviation (e.g. "Г", "Г.", "ГОР.").
pub fn is_city_prefix(s: &str) -> bool {
    // Note: standalone "С" and "П" are intentionally excluded — they are
    // far more commonly the Russian prepositions "с" (with/from) and "п"
    // than the settlement-type abbreviations "с." (село) and "п." (посёлок).
    // Only the forms that include a period ("С.", "П.") or full words
    // ("СЕЛО", "ПОС.") are safe to treat as city prefix abbreviations.
    matches!(s, "Г" | "Г." | "ГОР." | "ГОРОД" |
        "П." | "ПОС." | "ПГТ" | "ПГТ." |
        "С." | "СЕЛ." | "СЕЛО" |
        "ДЕР." | "ДЕРЕВНЯ" | "СТ." | "СТАНИЦА")
}

// ── Territory type keywords ───────────────────────────────────────────────────
//
// (uppercase match key, canonical lowercase type, always_prefix)

static TYPE_KEYWORDS: &[(&str, &str, bool)] = &[
    // States / countries
    ("ГОСУДАРСТВО",     "государство",     false),
    ("ДЕРЖАВА",         "государство",     false),
    ("СТРАНА",          "страна",          false),
    ("COUNTRY",         "country",         false),
    ("ИМПЕРИЯ",         "империя",         false),
    ("КОРОЛЕВСТВО",     "королевство",     false),
    ("ИМПЕРИЯ",         "империя",         false),
    ("KINGDOM",         "kingdom",         false),
    ("DUCHY",           "duchy",           false),
    ("СОЮЗ",            "союз",            false),
    ("UNION",           "union",           false),
    ("ФЕДЕРАЦИЯ",       "федерация",       false),
    ("FEDERATION",      "federation",      false),
    ("REPUBLIC",        "republic",        false),

    // Regions
    ("РЕСПУБЛИКА",      "республика",      true),
    ("REPUBLIC",        "republic",        true),
    ("ОБЛАСТЬ",         "область",         false),
    ("ОБЛ.",            "область",         false),
    ("REGION",          "region",          false),
    ("РАЙОН",           "район",           false),
    ("Р-Н",             "район",           false),
    ("DISTRICT",        "district",        false),
    ("КРАЙ",            "край",            false),
    ("KRAI",            "край",            false),
    ("ОКРУГ",           "округ",           false),
    ("ОКRUG",           "округ",           false),
    ("ШТАТ",            "штат",            true),
    ("STATE",           "state",           false),
    ("ПРОВИНЦИЯ",       "провинция",       true),
    ("PROVINCE",        "province",        true),
    ("ПРЕФЕКТУРА",      "префектура",      true),
    ("PREFECTURE",      "prefecture",      true),
    ("ГРАФСТВО",        "графство",        true),
    ("COUNTY",          "county",          false),
    ("АВТОНОМИЯ",       "автономия",       false),
    ("AUTONOMY",        "autonomy",        false),
    ("ГУБЕРНИЯ",        "губерния",        false),
    ("УЕЗД",            "уезд",            false),
    ("ВОЛОСТЬ",         "волость",         false),
    ("РЕГИОН",          "регион",          false),

    // Cities
    ("ГОРОД",           "город",           true),
    ("ГОРОД",           "город",           true),
    ("МІСТО",           "город",           true),
    ("CITY",            "city",            true),
    ("TOWN",            "town",            true),
    ("ПОСЕЛОК",         "поселок",         true),
    ("ПОСЁЛОК",         "поселок",         true),
    ("ПОС.",            "поселок",         true),
    ("СЕЛО",            "село",            true),
    ("ДЕРЕВНЯ",         "деревня",         true),
    ("СТАНИЦА",         "станица",         true),
    ("АУЛ",             "аул",             true),
    ("MUNICIPALITY",    "municipality",    false),
    ("LOCALITY",        "locality",        false),
    ("VILLAGE",         "village",         true),
];

// ── XML parser & table builder ────────────────────────────────────────────────

static T_DAT: &[u8] = include_bytes!("../../resources/t.dat");
static C_DAT: &[u8] = include_bytes!("../../resources/c.dat");

fn build_tables() -> Tables {
    let mut entries: Vec<GeoEntry> = Vec::new();
    let mut name_map: HashMap<String, usize> = HashMap::new();
    let mut adj_map: HashMap<String, usize> = HashMap::new();

    // Parse t.dat (countries + regions)
    let t_xml = MorphDeserializer::deflate_gzip(T_DAT);
    parse_t_dat(&t_xml, &mut entries, &mut name_map, &mut adj_map);

    // Parse c.dat (cities)
    let c_xml = MorphDeserializer::deflate_gzip(C_DAT);
    parse_c_dat(&c_xml, &mut entries, &mut name_map);

    Tables { name_map, adj_map, entries }
}

/// Simple line-by-line XML parser for t.dat format.
fn parse_t_dat(
    xml: &[u8],
    entries: &mut Vec<GeoEntry>,
    name_map: &mut HashMap<String, usize>,
    adj_map: &mut HashMap<String, usize>,
) {
    let text = std::str::from_utf8(xml).unwrap_or("");

    #[derive(PartialEq)]
    enum State { Outside, InState, InReg }

    let mut state = State::Outside;
    let mut cur_entry: Option<GeoEntry> = None;
    let mut cur_adjs: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<state") {
            state = State::InState;
            cur_entry = Some(GeoEntry {
                kind: GeoEntryKind::State,
                type_str: "государство".into(),
                alpha2: None,
                canonical_name: String::new(),
                all_names: Vec::new(),
            });
            cur_adjs.clear();
        } else if trimmed.starts_with("<reg") {
            state = State::InReg;
            cur_entry = Some(GeoEntry {
                kind: GeoEntryKind::Region,
                type_str: String::new(),
                alpha2: None,
                canonical_name: String::new(),
                all_names: Vec::new(),
            });
            cur_adjs.clear();
        } else if trimmed == "</state>" || trimmed == "</reg>" {
            if let Some(mut entry) = cur_entry.take() {
                // Set canonical name = shortest of all_names
                if let Some(shortest) = entry.all_names.iter().min_by_key(|n| n.chars().count()) {
                    entry.canonical_name = shortest.clone();
                }
                let idx = entries.len();
                // Register all noun names
                for n in &entry.all_names {
                    name_map.entry(n.clone()).or_insert(idx);
                }
                // Register adjective forms (region only)
                if matches!(state, State::InReg) {
                    for a in &cur_adjs {
                        adj_map.entry(a.clone()).or_insert(idx);
                        // Also register the masculine form so that morph normal forms match.
                        // Russian adjective lemmas are masculine nominative (e.g. МОСКОВСКИЙ),
                        // but the XML stores the feminine form (e.g. МОСКОВСКАЯ).
                        if let Some(masc) = adj_feminine_to_masculine(a) {
                            adj_map.entry(masc).or_insert(idx);
                        }
                    }
                }
                // Also register alpha2 / acronym for states
                if let Some(ref a2) = entry.alpha2 {
                    name_map.entry(a2.clone()).or_insert(idx);
                }
                entries.push(entry);
            }
            state = State::Outside;
            cur_adjs.clear();
        } else if state != State::Outside {
            if let Some(ref mut entry) = cur_entry {
                if let Some(val) = extract_tag(trimmed, "n") {
                    let up = val.to_uppercase();
                    if !entry.all_names.contains(&up) {
                        entry.all_names.push(up);
                    }
                } else if let Some(val) = extract_tag(trimmed, "acr") {
                    let up = val.to_uppercase();
                    if !entry.all_names.contains(&up) {
                        entry.all_names.push(up.clone());
                    }
                    // acr also goes in name_map (done above at commit time)
                } else if let Some(val) = extract_tag(trimmed, "a2") {
                    entry.alpha2 = Some(val.to_uppercase());
                } else if let Some(val) = extract_tag(trimmed, "t") {
                    // Type for regions
                    if entry.type_str.is_empty() {
                        entry.type_str = canonical_type(&val.to_uppercase());
                    }
                } else if let Some(val) = extract_tag(trimmed, "a") {
                    cur_adjs.push(val.to_uppercase());
                    // adjective-form variants: also add as names for regions
                    // so "МОСКОВСКАЯ" can be found by name too when combined with type
                    if matches!(state, State::InReg) {
                        let up = val.to_uppercase();
                        if !entry.all_names.contains(&up) {
                            entry.all_names.push(up);
                        }
                    }
                }
            }
        }
    }
}

/// Simple line-by-line XML parser for c.dat format.
fn parse_c_dat(
    xml: &[u8],
    entries: &mut Vec<GeoEntry>,
    name_map: &mut HashMap<String, usize>,
) {
    let text = std::str::from_utf8(xml).unwrap_or("");

    let mut in_city = false;
    let mut cur_entry: Option<GeoEntry> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<bigcity") || trimmed.starts_with("<city") {
            in_city = true;
            cur_entry = Some(GeoEntry {
                kind: GeoEntryKind::City,
                type_str: "город".into(),
                alpha2: None,
                canonical_name: String::new(),
                all_names: Vec::new(),
            });
        } else if trimmed.starts_with("</bigcity>") || trimmed.starts_with("</city>") {
            if let Some(mut entry) = cur_entry.take() {
                if let Some(shortest) = entry.all_names.iter().min_by_key(|n| n.chars().count()) {
                    entry.canonical_name = shortest.clone();
                }
                let idx = entries.len();
                for n in &entry.all_names {
                    // Cities override pre-existing county/district region entries
                    // (e.g. "MIAMI" is both a US county in t.dat and a city in c.dat).
                    let should_override = name_map.get(n)
                        .map(|&i| {
                            matches!(entries[i].kind, GeoEntryKind::Region)
                                && matches!(
                                    entries[i].type_str.as_str(),
                                    "county" | "district" | "графство" | "округ" | "borough"
                                )
                        })
                        .unwrap_or(false);
                    if should_override {
                        name_map.insert(n.clone(), idx);
                    } else {
                        name_map.entry(n.clone()).or_insert(idx);
                    }
                }
                entries.push(entry);
            }
            in_city = false;
        } else if in_city {
            if let Some(ref mut entry) = cur_entry {
                if let Some(val) = extract_tag(trimmed, "n") {
                    let up = val.to_uppercase();
                    if !entry.all_names.contains(&up) {
                        entry.all_names.push(up);
                    }
                }
                // <a> (adjective) for cities not added to name_map
            }
        }
    }
}

/// Map English nationality adjectives / demonyms to the canonical uppercase
/// country name as stored in the geo table (usually the Russian form, since
/// that is guaranteed to be present in the embedded t.dat data).
///
/// Used so that tokens like "Chinese", "German", "French" (which appear in
/// academic texts as language / country shorthand) resolve to their country.
pub fn nationality_to_country(term: &str) -> Option<&'static str> {
    match term {
        "CHINESE"               => Some("КИТАЙ"),
        "JAPANESE"              => Some("ЯПОНИЯ"),
        "RUSSIAN"               => Some("РОССИЯ"),
        "FRENCH"                => Some("ФРАНЦИЯ"),
        "GERMAN"                => Some("ГЕРМАНИЯ"),
        "SPANISH"               => Some("ИСПАНИЯ"),
        "ENGLISH" | "BRITISH"   => Some("ВЕЛИКОБРИТАНИЯ"),
        "KOREAN"                => Some("КОРЕЯ"),
        "FINNISH"               => Some("ФИНЛЯНДИЯ"),
        "ITALIAN"               => Some("ИТАЛИЯ"),
        "POLISH"                => Some("ПОЛЬША"),
        "PORTUGUESE"            => Some("ПОРТУГАЛИЯ"),
        "THAI"                  => Some("ТАИЛАНД"),
        "IRANIAN" | "PERSIAN"   => Some("ИРАН"),
        "DOMINICAN"             => Some("ДОМИНИКАНА"),
        "AMERICAN"              => Some("США"),
        "UKRAINIAN"             => Some("УКРАИНА"),
        "TURKISH"               => Some("ТУРЦИЯ"),
        "ARABIC" | "ARAB"       => Some("САУДОВСКАЯ АРАВИЯ"),
        "SWEDISH"               => Some("ШВЕЦИЯ"),
        "NORWEGIAN"             => Some("НОРВЕГИЯ"),
        "DANISH"                => Some("ДАНИЯ"),
        "DUTCH"                 => Some("НИДЕРЛАНДЫ"),
        "CZECH"                 => Some("ЧЕХИЯ"),
        "HUNGARIAN"             => Some("ВЕНГРИЯ"),
        "ROMANIAN"              => Some("РУМЫНИЯ"),
        "GREEK"                 => Some("ГРЕЦИЯ"),
        "BULGARIAN"             => Some("БОЛГАРИЯ"),
        "SERBIAN"               => Some("СЕРБИЯ"),
        "CROATIAN"              => Some("ХОРВАТИЯ"),
        "SLOVAK"                => Some("СЛОВАКИЯ"),
        "SLOVENIAN"             => Some("СЛОВЕНИЯ"),
        _ => None,
    }
}

/// Extract text content of a simple XML element: `<tag>text</tag>` on one line.
fn extract_tag<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = line.find(&open) {
        let after = start + open.len();
        if let Some(end) = line[after..].find(&close) {
            return Some(&line[after..after + end]);
        }
    }
    None
}

/// Convert a Russian feminine adjective form (e.g. "МОСКОВСКАЯ") to its
/// masculine nominative form (e.g. "МОСКОВСКИЙ"), which is what the Russian
/// morphological analyser returns as the lemma/normal-form of adjectives.
///
/// Returns `None` if the word does not end in a recognised feminine ending.
fn adj_feminine_to_masculine(fem: &str) -> Option<String> {
    // Collect chars to work with Cyrillic properly
    let chars: Vec<char> = fem.chars().collect();
    let n = chars.len();
    if n < 3 { return None; }

    // Check for "-АЯ" ending (2 Cyrillic chars = 2 chars)
    if chars[n-2] == 'А' && chars[n-1] == 'Я' {
        let stem_chars = &chars[..n-2];
        let last = *stem_chars.last()?;
        // After К, Ж, Ш, Щ, Ч the adjective ending is "ИЙ"; otherwise "ЫЙ"
        let ending = if matches!(last, 'К' | 'Ж' | 'Ш' | 'Щ' | 'Ч') { "ИЙ" } else { "ЫЙ" };
        let stem: String = stem_chars.iter().collect();
        return Some(format!("{}{}", stem, ending));
    }

    // Check for "-ЯЯ" ending (soft adjectives like "СИНЯЯ")
    if chars[n-2] == 'Я' && chars[n-1] == 'Я' {
        let stem: String = chars[..n-2].iter().collect();
        return Some(format!("{}ИЙ", stem));
    }

    None
}

/// Map an uppercase type string from the XML to a canonical lowercase type.
fn canonical_type(t: &str) -> String {
    match t {
        "РЕСПУБЛИКА" | "РЕСПУБЛІКА" => "республика".into(),
        "ОБЛАСТЬ" => "область".into(),
        "РАЙОН" => "район".into(),
        "КРАЙ" => "край".into(),
        "ОКРУГ" | "АО" => "округ".into(),
        "ШТАТ" => "штат".into(),
        "ПРОВИНЦИЯ" | "ПРОВІНЦІЯ" => "провинция".into(),
        "ПРЕФЕКТУРА" => "префектура".into(),
        "ГРАФСТВО" => "графство".into(),
        "АВТОНОМИЯ" | "АВТОНОМІЯ" => "автономия".into(),
        "ГУБЕРНИЯ" => "губерния".into(),
        "УЕЗД" => "уезд".into(),
        "ВОЛОСТЬ" => "волость".into(),
        "РЕГИОН" => "регион".into(),
        "ГОСУДАРСТВО" | "ДЕРЖАВА" => "государство".into(),
        _ => t.to_lowercase(),
    }
}
