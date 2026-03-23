/// Simplified port of `ShortNameHelper.cs`.
///
/// Parses `ShortNames.txt` (one line per full name):
///   `{m|f} FullName short1 short2 …`
///
/// Provides:
/// - `get_names_for_shortname(short)` → full-name variants with gender
/// - `get_shortnames_for_name(full)`  → short-name list (reverse lookup)

use std::collections::HashMap;
use std::sync::OnceLock;

static MAP: OnceLock<HashMap<String, Vec<(String, i32)>>> = OnceLock::new();

static NAMES_MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

const SHORT_NAMES_TXT: &str = include_str!("../../resources/ShortNames.txt");

fn build_map() -> HashMap<String, Vec<(String, i32)>> {
    let mut map: HashMap<String, Vec<(String, i32)>> = HashMap::new();
    for line in SHORT_NAMES_TXT.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let mut parts = line.split_whitespace();
        let gender_str = match parts.next() { Some(s) => s, None => continue };
        let gender: i32 = if gender_str.eq_ignore_ascii_case("f") { 2 } else { 1 };
        let full_name = match parts.next() { Some(s) => s.to_uppercase(), None => continue };
        for short in parts {
            let short = short.to_uppercase();
            let entry = map.entry(short).or_default();
            // Avoid duplicate (full_name, gender) pairs
            if !entry.iter().any(|(n, g)| n == &full_name && *g == gender) {
                entry.push((full_name.clone(), gender));
            }
        }
    }
    map
}

fn build_names_map() -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in SHORT_NAMES_TXT.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let mut parts = line.split_whitespace();
        let _gender = parts.next();
        let full_name = match parts.next() { Some(s) => s.to_uppercase(), None => continue };
        for short in parts {
            let short = short.to_uppercase();
            map.entry(full_name.clone()).or_default().push(short);
        }
    }
    map
}

fn get_map() -> &'static HashMap<String, Vec<(String, i32)>> {
    MAP.get_or_init(build_map)
}

fn get_names_map() -> &'static HashMap<String, Vec<String>> {
    NAMES_MAP.get_or_init(build_names_map)
}

/// Returns `(full_name, gender)` pairs for a given short name.
/// Input should be uppercase (morph terms are always uppercase).
/// E.g. "САША" → [("АЛЕКСАНДР", 1), ("АЛЕКСАНДРА", 2)]
pub fn get_names_for_shortname(shortname: &str) -> Option<&'static Vec<(String, i32)>> {
    get_map().get(shortname)
}

/// Returns short-name variants for a given full name.
/// Input should be uppercase (morph terms are always uppercase).
/// E.g. "АЛЕКСАНДР" → ["САША", "ШУРА", "АЛЕКС", …]
pub fn get_shortnames_for_name(name: &str) -> Option<&'static Vec<String>> {
    get_names_map().get(name)
}
