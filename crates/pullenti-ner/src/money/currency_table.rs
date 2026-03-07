/// Build a lookup map: uppercase currency name/abbreviation → ISO 4217 code.
///
/// CSV format (per Money.csv):
///   full_name;short_name;ISO_CODE;N sub-units;sub_unit_name
/// We index both `full_name` and `short_name` (and a few hard-coded abbreviations).

use std::collections::HashMap;
use std::sync::OnceLock;

static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Parse one CSV line and insert all name variants into the map.
/// For multi-word long names (adjective + noun), insert the full phrase.
/// For the short name (single word), insert only if not already present —
/// so that later CSV entries (processed in file order) do NOT override earlier ones,
/// EXCEPT we apply explicit overrides at the end for common currencies.
fn insert_line(map: &mut HashMap<String, String>, line: &str) {
    let line = line.trim();
    if line.is_empty() { return; }
    let parts: Vec<&str> = line.split(';').collect();
    if parts.len() < 3 { return; }

    // parts[0] = long name(s) comma-separated, parts[1] = short name, parts[2] = ISO code
    let iso = parts[2].trim().to_uppercase();
    if iso.is_empty() { return; }

    // Long name variants — insert the full phrase (good for two-word matching)
    for name in parts[0].split(',') {
        let k = name.trim().to_uppercase();
        if !k.is_empty() {
            // Full multi-word phrase: always overwrite (long phrase is unambiguous)
            map.insert(k, iso.clone());
        }
    }
    // Short name variants — use first-wins to avoid ambiguity
    for name in parts[1].split(',') {
        let k = name.trim().to_uppercase();
        if !k.is_empty() {
            map.entry(k).or_insert_with(|| iso.clone());
        }
    }
}

/// Return the global currency table (lazy-initialised).
pub fn currency_table() -> &'static HashMap<String, String> {
    TABLE.get_or_init(|| {
        let mut map: HashMap<String, String> = HashMap::new();

        // Embedded CSV data
        let ru  = include_str!("../../resources/Money.csv");
        let ua  = include_str!("../../resources/MoneyUA.csv");
        let en  = include_str!("../../resources/MoneyEN.csv");

        for csv in &[ru, ua, en] {
            for line in csv.lines() {
                insert_line(&mut map, line);
            }
        }

        // Override short names for common currencies — ensure the primary meaning wins.
        // "РУБЛЬ" first appears under BYR (Belarusian ruble) in the CSV; override to RUB.
        let priority_overrides: &[(&str, &str)] = &[
            ("РУБЛЬ",    "RUB"), ("РУБЛЕЙ",   "RUB"), ("РУБЛЯ",   "RUB"), ("РУБЛЯХ",  "RUB"),
            ("ДОЛЛАР",   "USD"), ("ДОЛЛАРОВ",  "USD"), ("ДОЛЛАРА", "USD"), ("ДОЛЛАРАХ","USD"),
            ("ЕВРО",     "EUR"),
            ("ГРИВНА",   "UAH"), ("ГРИВНЯ",   "UAH"), ("ГРИВЕНЬ", "UAH"),
            ("ТЕНГЕ",    "KZT"),
            ("ЮАН",      "CNY"), ("ЮАНЬ",     "CNY"),
            ("ФУНТ",     "GBP"),
            ("ЙЕНА",     "JPY"), ("ИЕНА",     "JPY"),
        ];
        for &(name, iso) in priority_overrides {
            map.insert(name.to_string(), iso.to_string());
        }

        // Hard-coded abbreviations that appear in text but not in CSVs
        let abbrevs: &[(&str, &str)] = &[
            ("РУБ",  "RUB"), ("РУБ.", "RUB"), ("РУБЛЕЙ", "RUB"), ("РУБЛЬ", "RUB"), ("РУБЛЯХ", "RUB"),
            ("КОП",  "RUB_KOP"), ("КОП.", "RUB_KOP"), ("КОПЕЕК", "RUB_KOP"), ("КОПЕЙКА", "RUB_KOP"),
            ("ГРН",  "UAH"), ("ГРН.", "UAH"), ("ГРИВЕН", "UAH"), ("ГРИВНЯ", "UAH"), ("ГРИВНА", "UAH"),
            ("ДОЛ",  "USD"), ("ДОЛ.", "USD"), ("ДОЛЛ",  "USD"), ("ДОЛЛ.", "USD"), ("ДОЛЛАР", "USD"), ("ДОЛЛАРОВ", "USD"),
            ("ЕВРО", "EUR"), ("EUR", "EUR"),
            ("CENT", "USD_CENT"), ("ЦЕНТ", "USD_CENT"), ("ЦЕНТОВ", "USD_CENT"),
        ];
        for &(abbr, iso) in abbrevs {
            map.entry(abbr.to_string()).or_insert_with(|| iso.to_string());
        }

        // Unicode currency symbols → ISO
        let symbols: &[(&str, &str)] = &[
            ("$", "USD"), ("€", "EUR"), ("£", "GBP"), ("¥", "JPY"),
            ("₽", "RUB"), ("₴", "UAH"), ("₩", "KRW"), ("₿", "BTC"),
        ];
        for &(sym, iso) in symbols {
            map.insert(sym.to_string(), iso.to_string());
        }

        map
    })
}

/// Lookup ISO code for a currency name or abbreviation (case-insensitive).
/// Returns `None` if not found or if the match is a sub-unit (kopeck/cent).
pub fn lookup(name: &str) -> Option<&'static str> {
    let k = name.to_uppercase();
    currency_table().get(&k).map(|s| s.as_str())
}

/// True if the ISO code is a fractional sub-unit pseudo-code (e.g. "RUB_KOP").
pub fn is_subunit(iso: &str) -> bool {
    iso.contains('_')
}
