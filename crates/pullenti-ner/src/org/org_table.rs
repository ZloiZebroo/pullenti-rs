/// org_table.rs — Organization type keyword lookup tables.
///
/// Sources:
///   1. OrgTypes.dat (gzip XML) — government/state org type keywords
///   2. Hard-coded legal forms: ООО, ОАО, ЗАО, ПАО, АО, ТОО, ИП, etc.
///   3. Hard-coded common business words: корпорация, компания, холдинг, etc.
///   4. Hard-coded education/science words: университет, институт, академия, etc.
///   5. Orgs_ru.dat (gzip XML) — specific known organization names
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use pullenti_morph::internal::morph_deserializer::MorphDeserializer;

#[derive(Debug, Clone)]
pub struct OrgTypeEntry {
    pub canonical: String,   // canonical uppercase form
    pub profile: Option<String>,
    pub is_legal_form: bool, // ООО/ОАО/etc legal entity type
    pub is_prefix: bool,     // comes before the name
}

#[derive(Debug, Clone)]
pub struct KnownOrg {
    pub names: Vec<String>,  // all names/acronyms
    pub typ: Option<String>,
}

struct OrgTables {
    /// Maps uppercase normalized keyword → OrgTypeEntry
    type_map: HashMap<String, OrgTypeEntry>,
    /// Maps uppercase name → KnownOrg (from Orgs_ru.dat)
    known_map: HashMap<String, usize>,
    known_orgs: Vec<KnownOrg>,
    /// Legal form abbreviations (single uppercase token)
    legal_abbr: HashSet<String>,
}

static TABLES: OnceLock<OrgTables> = OnceLock::new();

fn tables() -> &'static OrgTables {
    TABLES.get_or_init(build_tables)
}

fn build_tables() -> OrgTables {
    let mut type_map: HashMap<String, OrgTypeEntry> = HashMap::new();

    // ── 1. Hard-coded legal form abbreviations ────────────────────────────────
    let legal_abbrs: &[(&str, &str)] = &[
        ("ООО", "ООО"), ("ОАО", "ОАО"), ("ЗАО", "ЗАО"), ("ПАО", "ПАО"),
        ("АО",  "АО"),  ("ТОО", "ТОО"), ("ОДО", "ОДО"), ("ОАТ", "ОАТ"),
        ("ЗАТ", "ЗАТ"), ("ПАТ", "ПАТ"), ("АТ",  "АТ"),
        ("ИП",  "ИП"),  ("ЧП",  "ЧП"),  ("ПП",  "ПП"),
        ("ГП",  "ГП"),  ("ДП",  "ДП"),
        ("ГУП", "ГУП"), ("МУП", "МУП"), ("КУП", "КУП"),
        ("ФГУП","ФГУП"),("ФГУ", "ФГУ"), ("ФГБУ","ФГБУ"),("ФГКУ","ФГКУ"),
        ("НПО", "НПО"), ("НПК", "НПК"),
        ("ГК",  "ГК"),  ("ЧОП", "ЧОП"), ("ЧОО", "ЧОО"),
        ("ПОК", "ПОК"), ("СНТ", "СНТ"), ("ДНТ", "ДНТ"),
        ("ЧАО", "ЧАО"), ("ЧТУП","ЧТУП"),("ЧУП", "ЧУП"),
        ("КФХ", "КФХ"), // крестьянское фермерское хозяйство
        // English equivalents
        ("LLC",  "LLC"), ("LTD",  "LTD"), ("INC",  "INC"),
        ("CORP", "CORP"),("PLC",  "PLC"), ("JSC",  "JSC"),
        ("PJSC", "PJSC"),("GmbH", "GmbH"),("AG",   "AG"),
    ];
    for (abbr, canonical) in legal_abbrs {
        let key = abbr.to_uppercase();
        type_map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
            canonical: canonical.to_string(),
            profile: None,
            is_legal_form: true,
            is_prefix: true,
        });
    }

    // ── 2. Hard-coded legal form full names ───────────────────────────────────
    let legal_full: &[(&str, &str)] = &[
        ("ОБЩЕСТВО С ОГРАНИЧЕННОЙ ОТВЕТСТВЕННОСТЬЮ", "ООО"),
        ("ОТКРЫТОЕ АКЦИОНЕРНОЕ ОБЩЕСТВО", "ОАО"),
        ("ЗАКРЫТОЕ АКЦИОНЕРНОЕ ОБЩЕСТВО", "ЗАО"),
        ("ПУБЛИЧНОЕ АКЦИОНЕРНОЕ ОБЩЕСТВО", "ПАО"),
        ("АКЦИОНЕРНОЕ ОБЩЕСТВО", "АО"),
        ("ТОВАРИЩЕСТВО С ОГРАНИЧЕННОЙ ОТВЕТСТВЕННОСТЬЮ", "ТОО"),
        ("ИНДИВИДУАЛЬНОЕ ПРЕДПРИЯТИЕ", "ИП"),
        ("ЧАСТНОЕ ПРЕДПРИЯТИЕ", "ЧП"),
        ("ГОСУДАРСТВЕННОЕ УНИТАРНОЕ ПРЕДПРИЯТИЕ", "ГУП"),
        ("МУНИЦИПАЛЬНОЕ УНИТАРНОЕ ПРЕДПРИЯТИЕ", "МУП"),
        ("ФЕДЕРАЛЬНОЕ ГОСУДАРСТВЕННОЕ УНИТАРНОЕ ПРЕДПРИЯТИЕ", "ФГУП"),
        ("ФЕДЕРАЛЬНОЕ ГОСУДАРСТВЕННОЕ БЮДЖЕТНОЕ УЧРЕЖДЕНИЕ", "ФГБУ"),
        ("ГОСУДАРСТВЕННОЕ КАЗЁННОЕ УЧРЕЖДЕНИЕ", "ГКУ"),
        ("АВТОНОМНАЯ НЕКОММЕРЧЕСКАЯ ОРГАНИЗАЦИЯ", "АНО"),
        ("НЕКОММЕРЧЕСКОЕ ПАРТНЁРСТВО", "НП"),
        ("ПОТРЕБИТЕЛЬСКИЙ КООПЕРАТИВ", "ПК"),
        ("ПРОИЗВОДСТВЕННЫЙ КООПЕРАТИВ", "ПК"),
    ];
    for (full, canonical) in legal_full {
        let key = full.to_uppercase();
        type_map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
            canonical: canonical.to_string(),
            profile: None,
            is_legal_form: true,
            is_prefix: true,
        });
    }

    // ── 3. Common business/organization words ─────────────────────────────────
    let common: &[(&str, Option<&str>)] = &[
        ("КОРПОРАЦИЯ", None), ("КОМПАНИЯ", None), ("ФИРМА", None),
        ("ХОЛДИНГ", None), ("КОНЦЕРН", None), ("КОНСОРЦИУМ", None),
        ("ТРЕСТ", None), ("СИНДИКАТ", None),
        ("ГРУППА КОМПАНИЙ", None), ("МЕДИАГРУППА", None),
        ("АГЕНТСТВО", None), ("БЮРО", None),
        ("БАНК", Some("Finance")), ("СБЕРБАНК", Some("Finance")),
        ("СТРАХОВАЯ КОМПАНИЯ", Some("Finance")),
        ("СТРАХОВОЕ ОБЩЕСТВО", Some("Finance")),
        ("АССОЦИАЦИЯ", Some("Union")), ("СОЮЗ", Some("Union")),
        ("ФЕДЕРАЦИЯ", Some("Union")), ("ЛИГА", Some("Union")),
        ("КЛУБ", Some("Sport")),
        ("ПАРТИЯ", Some("Policy")),
        ("ФОНД", None), ("БЛАГОТВОРИТЕЛЬНЫЙ ФОНД", None),
        ("ТОРГОВЫЙ ДОМ", None), ("ИЗДАТЕЛЬСТВО", None),
        ("ИЗДАТЕЛЬСКИЙ ДОМ", None),
        // English
        ("CORPORATION", None), ("COMPANY", None), ("HOLDING", None),
        ("GROUP", None), ("BANK", Some("Finance")),
        ("FUND", None), ("FOUNDATION", None),
        ("ASSOCIATION", Some("Union")), ("ALLIANCE", Some("Union")),
        ("UNION", Some("Union")), ("FEDERATION", Some("Union")),
    ];
    for (word, profile) in common {
        let key = word.to_uppercase();
        type_map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
            canonical: word.to_string(),
            profile: profile.map(|s| s.to_string()),
            is_legal_form: false,
            is_prefix: false,
        });
    }

    // ── 4. Education / Science ────────────────────────────────────────────────
    let edu: &[&str] = &[
        "УНИВЕРСИТЕТ", "ИНСТИТУТ", "АКАДЕМИЯ", "КОЛЛЕДЖ",
        "ТЕХНИКУМ", "УЧИЛИЩЕ", "ШКОЛА", "ГИМНАЗИЯ", "ЛИЦЕЙ",
        "СЕМИНАРИЯ", "КОНСЕРВАТОРИЯ",
        "UNIVERSITY", "INSTITUTE", "ACADEMY", "COLLEGE",
        "ШКОЛА-ИНТЕРНАТ", "КАДЕТСКИЙ КОРПУС",
        "ИССЛЕДОВАТЕЛЬСКИЙ ИНСТИТУТ", "НИИ",
    ];
    for word in edu {
        let key = word.to_uppercase();
        type_map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
            canonical: word.to_string(),
            profile: Some("Education".to_string()),
            is_legal_form: false,
            is_prefix: false,
        });
    }

    // ── 5. Medical ────────────────────────────────────────────────────────────
    let med: &[&str] = &[
        "БОЛЬНИЦА", "КЛИНИКА", "ГОСПИТАЛЬ", "ПОЛИКЛИНИКА",
        "САНАТОРИЙ", "ДИСПАНСЕР",
        "HOSPITAL", "CLINIC",
    ];
    for word in med {
        let key = word.to_uppercase();
        type_map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
            canonical: word.to_string(),
            profile: Some("Medicine".to_string()),
            is_legal_form: false,
            is_prefix: false,
        });
    }

    // ── 6. Load OrgTypes.dat ──────────────────────────────────────────────────
    let bytes = include_bytes!("../../resources/OrgTypes.dat");
    let raw = MorphDeserializer::deflate_gzip(bytes);
    if let Ok(xml) = std::str::from_utf8(&raw) {
        parse_org_types_xml(xml, &mut type_map);
    }

    // ── 7. Build legal_abbr set ───────────────────────────────────────────────
    let legal_abbr: HashSet<String> = type_map.iter()
        .filter(|(_, e)| e.is_legal_form && !e.canonical.contains(' '))
        .map(|(k, _)| k.clone())
        .collect();

    // ── 8. Load Orgs_ru.dat ───────────────────────────────────────────────────
    let mut known_orgs: Vec<KnownOrg> = Vec::new();
    let mut known_map: HashMap<String, usize> = HashMap::new();
    let bytes2 = include_bytes!("../../resources/Orgs_ru.dat");
    let raw2 = MorphDeserializer::deflate_gzip(bytes2);
    if let Ok(xml2) = std::str::from_utf8(&raw2) {
        parse_known_orgs_xml(xml2, &mut known_orgs, &mut known_map);
    }

    OrgTables { type_map, known_map, known_orgs, legal_abbr }
}

fn parse_org_types_xml(xml: &str, map: &mut HashMap<String, OrgTypeEntry>) {
    let mut profile: Option<String> = None;
    let mut is_top = false;
    let mut in_set = false;

    for line in xml.lines() {
        let line = line.trim();
        // <set .../>  or <type .../>
        let is_set = line.starts_with("<set ");
        let is_type = line.starts_with("<type ");
        if !is_set && !is_type { continue; }

        if is_set {
            in_set = true;
            profile = extract_attr(line, "profile");
            is_top = extract_attr(line, "top").as_deref() == Some("true");
            continue;
        }

        // <type ...> — inherit profile from enclosing <set>
        let p = extract_attr(line, "profile").or_else(|| if in_set { profile.clone() } else { None });
        let top = extract_attr(line, "top").as_deref() == Some("true") || (in_set && is_top);

        let names_ru = extract_attr(line, "name");
        let names_en = extract_attr(line, "nameEn");

        for names_str in [names_ru, names_en].iter().flatten() {
            for name in names_str.split(';') {
                let name = name.trim();
                if name.is_empty() { continue; }
                let key = name.to_uppercase();
                map.entry(key.clone()).or_insert_with(|| OrgTypeEntry {
                    canonical: name.to_uppercase(),
                    profile: p.clone(),
                    is_legal_form: false,
                    is_prefix: top,
                });
            }
        }
    }
}

fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    let val = &rest[..end];
    if val.is_empty() { None } else { Some(val.to_string()) }
}

fn parse_known_orgs_xml(xml: &str, orgs: &mut Vec<KnownOrg>, map: &mut HashMap<String, usize>) {
    let mut cur_typs: Vec<String> = Vec::new();
    let mut cur_names: Vec<String> = Vec::new();
    let mut in_org = false;

    for line in xml.lines() {
        let line = line.trim();
        if line == "<org>" {
            in_org = true;
            cur_typs.clear();
            cur_names.clear();
        } else if line == "</org>" {
            if !cur_names.is_empty() {
                let idx = orgs.len();
                let org = KnownOrg {
                    names: cur_names.clone(),
                    typ: cur_typs.first().cloned(),
                };
                for name in &cur_names {
                    map.entry(name.to_uppercase()).or_insert(idx);
                }
                orgs.push(org);
            }
            in_org = false;
        } else if in_org {
            if let Some(v) = extract_simple_tag(line, "nam") {
                cur_names.push(v.to_uppercase());
            } else if let Some(v) = extract_simple_tag(line, "typ") {
                cur_typs.push(v.to_uppercase());
            }
        }
    }
}

fn extract_simple_tag<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = line.find(open.as_str())? + open.len();
    let end = line[start..].find(close.as_str())?;
    Some(&line[start..start + end])
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Look up an org type keyword (uppercase). Returns Some(entry) if found.
pub fn lookup_type(key: &str) -> Option<&'static OrgTypeEntry> {
    tables().type_map.get(key)
}

/// Returns true if the token is a standalone legal form abbreviation (ООО, ОАО, etc.)
pub fn is_legal_abbr(key: &str) -> bool {
    tables().legal_abbr.contains(key)
}

/// Look up a known organization name (uppercase). Returns Some(KnownOrg ref).
pub fn lookup_known(name: &str) -> Option<&'static KnownOrg> {
    let t = tables();
    t.known_map.get(name).and_then(|&idx| t.known_orgs.get(idx))
}
