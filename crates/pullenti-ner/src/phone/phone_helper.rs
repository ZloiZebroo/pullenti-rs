use std::collections::HashMap;
use std::sync::OnceLock;

struct PhoneNode {
    pref: String,
    children: HashMap<char, PhoneNode>,
    countries: Vec<String>,
}

impl PhoneNode {
    fn new(pref: String) -> Self {
        PhoneNode { pref, children: HashMap::new(), countries: Vec::new() }
    }
}

struct PhoneHelper {
    root: PhoneNode,
    all_country_codes: HashMap<String, String>,
}

static INSTANCE: OnceLock<PhoneHelper> = OnceLock::new();

fn get_instance() -> &'static PhoneHelper {
    INSTANCE.get_or_init(|| {
        let txt = include_str!("../../resources/CountryPhoneCodes.txt");
        let mut root = PhoneNode::new(String::new());
        let mut all_country_codes = HashMap::new();

        for line0 in txt.split('\n') {
            let line = line0.trim();
            if line.len() < 3 { continue; }
            let country = &line[..2];
            let cod = line[2..].trim();
            if cod.is_empty() { continue; }

            all_country_codes.entry(country.to_string()).or_insert_with(|| cod.to_string());

            let mut tn = &mut root;
            for (i, dig) in cod.chars().enumerate() {
                if !tn.children.contains_key(&dig) {
                    let pref = cod[..=cod.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0)].to_string();
                    tn.children.insert(dig, PhoneNode::new(pref));
                }
                tn = tn.children.get_mut(&dig).unwrap();
            }
            tn.countries.push(country.to_string());
        }

        PhoneHelper { root, all_country_codes }
    })
}

/// Get the longest country phone code prefix matching the start of `full_number`.
/// Returns `None` if no country code matches.
pub fn get_country_prefix(full_number: &str) -> Option<String> {
    let helper = get_instance();
    let mut nod = &helper.root;
    let mut max_idx: i32 = -1;
    let mut char_count = 0usize;

    for dig in full_number.chars() {
        match nod.children.get(&dig) {
            None => break,
            Some(nn) => {
                if !nn.countries.is_empty() {
                    max_idx = char_count as i32;
                }
                nod = nn;
            }
        }
        char_count += 1;
    }

    if max_idx < 0 {
        None
    } else {
        Some(full_number[..full_number.char_indices().nth((max_idx + 1) as usize).map(|(b, _)| b).unwrap_or(full_number.len())].to_string())
    }
}

/// Return the full country code mapping.
pub fn get_all_country_codes() -> &'static HashMap<String, String> {
    &get_instance().all_country_codes
}
