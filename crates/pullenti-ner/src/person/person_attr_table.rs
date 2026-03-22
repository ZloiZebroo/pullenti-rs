/// Person attribute / position term table.
///
/// Loads attr_ru.dat (gzip XML) to build a lookup set of known position terms
/// plus hardcoded prefix terms (господин, мистер, etc.).
use std::collections::HashMap;
use std::sync::OnceLock;
use pullenti_morph::internal::morph_deserializer::MorphDeserializer;

// ── PersonAttrKind ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PersonAttrKind {
    Position,   // generic position / job title
    Prefix,     // honorific prefix (господин, мистер, доктор...)
    King,       // noble / royal / clergy title
    Kin,        // kinship term (отец, сын, брат...)
    MilitaryRank,
    Nationality,
}

// ── PersonAttrEntry ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PersonAttrEntry {
    /// Canonical lowercase display name for the position.
    pub canonic: String,
    pub kind: PersonAttrKind,
    /// 1 = can be followed by person name; 2 = strongly expects person after
    pub can_has_person_after: u8,
    pub is_boss: bool,
    pub gender: Option<bool>, // Some(true) = male, Some(false) = female
}

impl PersonAttrEntry {
    fn new(canonic: &str, kind: PersonAttrKind) -> Self {
        PersonAttrEntry {
            canonic: canonic.to_string(),
            kind,
            can_has_person_after: 0,
            is_boss: false,
            gender: None,
        }
    }
}

// ── Global table ─────────────────────────────────────────────────────────────

static TABLE: OnceLock<HashMap<String, PersonAttrEntry>> = OnceLock::new();

pub fn get_table() -> &'static HashMap<String, PersonAttrEntry> {
    TABLE.get_or_init(build_table)
}

/// Returns the entry for an uppercase term (e.g. "ДИРЕКТОР"), if known.
pub fn lookup(term: &str) -> Option<&'static PersonAttrEntry> {
    get_table().get(term)
}

fn build_table() -> HashMap<String, PersonAttrEntry> {
    let mut map: HashMap<String, PersonAttrEntry> = HashMap::new();

    // ── Hardcoded prefix terms ────────────────────────────────────────────────
    let prefixes_m: &[(&str, &str)] = &[
        ("ГОСПОДИН",    "господин"),
        ("Г-Н",         "господин"),
        ("ГРАЖДАНИН",   "гражданин"),
        ("ТОВАРИЩ",     "товарищ"),
        ("ТОВ.",        "товарищ"),
        ("МИСТЕР",      "мистер"),
        ("МР.",         "мистер"),
        ("СЭР",         "сэр"),
        ("ДОН",         "дон"),
        ("МАЭСТРО",     "маэстро"),
        ("МЭТР",        "мэтр"),
        ("СЕНЬОР",      "сеньор"),
        ("СИНЬОР",      "синьор"),
        ("МОН",         "мон"),
        ("ГРАФ",        "граф"),
        ("КНЯЗЬ",       "князь"),
        ("БАРОН",       "барон"),
        ("ГЕРЦОГ",      "герцог"),
        ("МАРКИЗ",      "маркиз"),
    ];
    for (term, canonic) in prefixes_m {
        let mut e = PersonAttrEntry::new(canonic, PersonAttrKind::Prefix);
        e.can_has_person_after = 1;
        e.gender = Some(true);
        map.insert(term.to_string(), e);
    }
    let prefixes_f: &[(&str, &str)] = &[
        ("ГОСПОЖА",         "госпожа"),
        ("Г-ЖА",            "госпожа"),
        ("ГРАЖДАНКА",       "гражданка"),
        ("МАДАМ",           "мадам"),
        ("МАДЕМУАЗЕЛЬ",     "мадемуазель"),
        ("МИСС",            "мисс"),
        ("МИССИС",          "миссис"),
        ("ФРАУ",            "фрау"),
        ("ЛЕДИ",            "леди"),
        ("СЕНЬОРА",         "сеньора"),
        ("ДОННА",           "донна"),
    ];
    for (term, canonic) in prefixes_f {
        let mut e = PersonAttrEntry::new(canonic, PersonAttrKind::Prefix);
        e.can_has_person_after = 1;
        e.gender = Some(false);
        map.insert(term.to_string(), e);
    }
    // English prefixes
    let prefixes_en: &[(&str, &str, Option<bool>)] = &[
        ("MR",      "mr.",   Some(true)),
        ("MR.",     "mr.",   Some(true)),
        ("MRS",     "mrs.",  Some(false)),
        ("MRS.",    "mrs.",  Some(false)),
        ("MS",      "ms.",   Some(false)),
        ("MS.",     "ms.",   Some(false)),
        ("MISS",    "miss",  Some(false)),
        ("DR",      "dr.",   None),
        ("DR.",     "dr.",   None),
        ("PROF.",   "prof.", None),
        ("SIR",     "sir",   Some(true)),
        ("LORD",    "lord",  Some(true)),
        ("LADY",    "lady",  Some(false)),
    ];
    for (term, canonic, gender) in prefixes_en {
        let mut e = PersonAttrEntry::new(canonic, PersonAttrKind::Prefix);
        e.can_has_person_after = 1;
        e.gender = *gender;
        map.insert(term.to_string(), e);
    }

    // ── Load attr_ru.dat ─────────────────────────────────────────────────────
    let bytes = include_bytes!("../../resources/attr_ru.dat");
    let raw = MorphDeserializer::deflate_gzip(bytes);
    if let Ok(xml) = std::str::from_utf8(&raw) {
        parse_attrs_xml(xml, &mut map);
    }

    // ── Load attr_en.dat ─────────────────────────────────────────────────────
    let bytes_en = include_bytes!("../../resources/attr_en.dat");
    let raw_en = MorphDeserializer::deflate_gzip(bytes_en);
    if let Ok(xml_en) = std::str::from_utf8(&raw_en) {
        parse_attrs_xml(xml_en, &mut map);
    }

    map
}

fn parse_attrs_xml(xml: &str, map: &mut HashMap<String, PersonAttrEntry>) {
    // Minimal XML parsing — look for <it v="..." a="..."> elements and <alt>...</alt> children.
    // We don't pull in an XML parser crate; instead scan for attribute patterns.
    let mut pos = 0;
    let bytes = xml.as_bytes();
    let len = bytes.len();

    while pos < len {
        // Find "<it " or "<it\t" or "<it>"
        let Some(tag_start) = find_str(xml, "<it ", pos) else { break };
        pos = tag_start + 4;

        // Parse attributes until ">"
        let tag_end = find_char(xml, '>', pos).unwrap_or(len);
        let tag_slice = &xml[pos..tag_end];

        let v = extract_attr(tag_slice, "v");
        let a = extract_attr(tag_slice, "a").unwrap_or_default();

        let Some(v) = v else { pos = tag_end + 1; continue; };
        let v_upper = v.to_uppercase();

        // Parse flag chars from 'a'
        let mut kind = PersonAttrKind::Position;
        let mut can_has_person_after: u8 = 0;
        let mut is_boss = false;
        let mut gender: Option<bool> = None;

        for ch in a.chars() {
            match ch {
                'p' => can_has_person_after = can_has_person_after.max(1),
                'P' => can_has_person_after = 2,
                'b' => is_boss = true,
                'r' => kind = PersonAttrKind::MilitaryRank,
                'n' => kind = PersonAttrKind::Nationality,
                'c' | 'q' => kind = PersonAttrKind::King,
                'k' => kind = PersonAttrKind::Kin,
                'm' => gender = Some(true),
                'f' => gender = Some(false),
                _ => {}
            }
        }
        if is_boss && kind == PersonAttrKind::Position {
            kind = PersonAttrKind::Position; // stays Position but IsBoss
        }

        let canonic = v.to_lowercase();
        let mut entry = PersonAttrEntry::new(&canonic, kind);
        entry.can_has_person_after = can_has_person_after;
        entry.is_boss = is_boss;
        entry.gender = gender;

        map.entry(v_upper.clone()).or_insert_with(|| entry.clone());

        // Also parse <alt> children between this <it ...> and the next </it> or <it
        // Look for self-closing vs. open tag
        let self_closing = tag_end > 0 && xml.as_bytes().get(tag_end - 1) == Some(&b'/');
        if !self_closing {
            // scan for </it>
            let close = find_str(xml, "</it>", tag_end).unwrap_or(len);
            let inner = &xml[tag_end + 1..close.min(len)];
            // collect <alt>...</alt> values
            let mut ap = 0;
            while ap < inner.len() {
                let Some(as_) = find_str(inner, "<alt>", ap) else { break };
                let ae = find_str(inner, "</alt>", as_ + 5).unwrap_or(inner.len());
                let alt_text = inner[as_ + 5..ae].trim().to_uppercase();
                if !alt_text.is_empty() {
                    map.entry(alt_text).or_insert_with(|| entry.clone());
                }
                ap = ae + 6;
            }
            pos = close + 5;
        } else {
            pos = tag_end + 1;
        }
    }
}

fn find_str(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    haystack[from..].find(needle).map(|p| p + from)
}

fn find_char(s: &str, ch: char, from: usize) -> Option<usize> {
    s[from..].find(ch).map(|p| p + from)
}

fn extract_attr<'a>(tag_slice: &'a str, attr_name: &str) -> Option<&'a str> {
    // Look for attr_name="..." or attr_name='...'
    let pattern = format!("{}=\"", attr_name);
    if let Some(p) = tag_slice.find(&pattern) {
        let start = p + pattern.len();
        let end = tag_slice[start..].find('"')? + start;
        return Some(&tag_slice[start..end]);
    }
    let pattern2 = format!("{}='", attr_name);
    if let Some(p) = tag_slice.find(&pattern2) {
        let start = p + pattern2.len();
        let end = tag_slice[start..].find('\'')? + start;
        return Some(&tag_slice[start..end]);
    }
    None
}
