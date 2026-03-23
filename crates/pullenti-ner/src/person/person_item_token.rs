/// Simplified port of `PersonItemToken.cs` (2524 lines → ~420 lines).
///
/// Covers the EmptyProcessor path used by `PersonNormalData::analyze()`:
///  - Standard surname tail detection (`ends_with_std_surname`)
///  - Morph-class-based role assignment (firstname / middlename / lastname)
///  - Short-name expansion via `ShortNameHelper` (Вася→Василий)
///  - Arab postfix recognition (ОГЛЫ, КЫЗЫ, ЗАДЕ, …)
///  - Initial (single capital letter) recognition
///  - `try_attach()` — parse one name token
///  - `try_attach_list()` — parse a chain, handles comma-separated FIO

use pullenti_morph::MorphGenderFlags;
use crate::token::{TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use super::short_name_helper::get_names_for_shortname;

// ── Item type ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    Value,
    Initial,
}

// ── Standard surname tails ────────────────────────────────────────────────────

/// `(suffix_uppercase, gender)` where gender: 1=masculine, 2=feminine, 0=neutral
const SURNAME_TAILS: &[(&str, i32)] = &[
    // Russian / standard (longer suffixes first so we match most specific)
    ("ЦКАЯ", 2), ("ЦКИЙ", 1),
    ("СКАЯ", 2), ("СКИЙ", 1),
    ("ОВА",  2), ("ОВ",   1),
    ("ЕВА",  2), ("ЕВ",   1),
    // Ukrainian
    ("ЄВА",  2), ("ЄВ",   1),
    ("ІНА",  2), ("ІН",   1),
    ("ИНА",  2), ("ИН",   1),
    // Gender-neutral / Georgian / Armenian / Uzbek / etc.
    ("ВИЛИ", 0), ("ДЗЕ",  0), ("ЯН",   0),
    ("УК",   0), ("ЮК",   0), ("КО",   0),
    ("МАН",  0), ("АНН",  0), ("ЙН",   0),
    ("УН",   0), ("СКУ",  0), ("СКИ",  0),
    ("СЬКІ", 0), ("ЕР",   0), ("РН",   0),
];

/// Check whether `s` ends with a standard Russian/Ukrainian surname tail.
/// Returns `Some(gender)` (0=neutral, 1=masculine, 2=feminine) on match.
/// NOTE: `s` is expected to be uppercase (morph terms are always uppercase).
pub fn ends_with_std_surname(s: &str) -> Option<i32> {
    for &(tail, gender) in SURNAME_TAILS {
        if s.ends_with(tail) && s.len() > tail.len() {
            return Some(gender);
        }
    }
    None
}

// ── Arab postfix lists ─────────────────────────────────────────────────────────

const ARAB_POSTFIX: &[&str] = &[
    "АГА", "АЛИ", "АР", "АС", "АШ", "БЕЙ", "БЕК", "ЗАДЕ",
    "ОГЛЫ", "ОГЛИ", "УГЛИ", "ОЛЬ", "ООЛ", "ПАША", "БАША",
    "УЛЬ", "УЛЫ", "УУЛУ", "ХАН", "ХАДЖИ", "ШАХ", "ЭД", "ЭЛЬ",
];
const ARAB_POSTFIX_FEM: &[&str] = &["АСУ", "АЗУ", "ГЫЗЫ", "ЗУЛЬ", "КЫЗЫ", "КЫС", "КЗЫ"];

fn is_arab_postfix(term: &str) -> bool {
    ARAB_POSTFIX.contains(&term) || ARAB_POSTFIX_FEM.contains(&term)
}

fn arab_postfix_gender(term: &str) -> i32 {
    if ARAB_POSTFIX_FEM.contains(&term) { 2 } else { 1 }
}

// ── MorphPersonItem ───────────────────────────────────────────────────────────

/// Morphological info for one name-part role.
#[derive(Debug, Clone, Default)]
pub struct MorphPersonItem {
    /// Best-guess gender: 0=unknown, 1=masculine, 2=feminine.
    pub gender: i32,
    /// Token is in the morph dictionary with the expected proper-name class.
    pub is_in_dictionary: bool,
    /// Ends with a standard surname tail.
    pub is_lastname_has_std_tail: bool,
    /// Expanded name variants from ShortNameHelper: (full_name, gender).
    /// Non-empty only for firstnames where the surface form is a short name.
    pub vars: Vec<(String, i32)>,
}

impl MorphPersonItem {
    pub fn new(gender: i32, is_in_dictionary: bool, is_lastname_has_std_tail: bool) -> Self {
        MorphPersonItem {
            gender, is_in_dictionary, is_lastname_has_std_tail,
            vars: Vec::new(),
        }
    }
}

// ── PersonItemToken ───────────────────────────────────────────────────────────

/// One parsed name-part (single token or initial letter).
#[derive(Debug, Clone)]
pub struct PersonItemToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub typ:   ItemType,
    /// Uppercase term (morph term or initial letter).
    pub value: String,
    /// Morph info if this token can be a Firstname.
    pub firstname:  Option<MorphPersonItem>,
    /// Morph info if this token can be a Middlename (patronymic).
    pub middlename: Option<MorphPersonItem>,
    /// Morph info if this token can be a Lastname (surname).
    pub lastname:   Option<MorphPersonItem>,
    /// Overall token gender (from morph — may differ from role gender).
    pub morph_gender: i32,
    /// Number of whitespace characters before this token.
    pub whitespaces_before: usize,
    /// True if newline appears before this token.
    pub is_newline_before: bool,
    /// True if token text is all-lowercase.
    pub is_all_lower: bool,
}

impl PersonItemToken {
    /// Create a minimal PersonItemToken for an initial letter.
    fn initial(t: TokenRef, end: TokenRef, letter: char, ws: usize, nl: bool) -> Self {
        PersonItemToken {
            begin_token: t, end_token: end,
            typ: ItemType::Initial,
            value: letter.to_uppercase().to_string(),
            firstname: None, middlename: None, lastname: None,
            morph_gender: 0,
            whitespaces_before: ws,
            is_newline_before: nl,
            is_all_lower: false,
        }
    }
}

// ── Gender helpers ────────────────────────────────────────────────────────────

/// Convert `MorphGenderFlags` to i32: 1=masculine, 2=feminine, 0=other/unknown.
fn gender_i32(g: MorphGenderFlags) -> i32 {
    if (g & MorphGenderFlags::MASCULINE) != MorphGenderFlags::UNDEFINED { return 1; }
    if (g & MorphGenderFlags::FEMINIE)   != MorphGenderFlags::UNDEFINED { return 2; }
    0
}

// ── try_attach ────────────────────────────────────────────────────────────────

/// Try to parse a single name-part PersonItemToken starting at `t`.
/// Returns `None` if the token cannot be a name part.
pub fn try_attach(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<PersonItemToken> {
    let tb = t.borrow();

    // Must be a text token with letters
    let TokenKind::Text(txt) = &tb.kind else { return None; };
    if !tb.chars.is_letter() { return None; }

    let ws = tb.whitespaces_before_count(sofa) as usize;
    let nl = tb.is_newline_before(sofa);
    // ── Check for initial: single uppercase letter, possibly followed by "." ──
    // Note: single uppercase letters like "И" have is_all_upper=true but
    // is_capital_upper=false (capital_upper requires trailing lowercase letters).
    // So we test the char directly with is_uppercase() rather than is_capital_upper().
    // Check for single-char term without allocating a Vec<char>
    let mut chars_iter = txt.term.chars();
    let first_ch = chars_iter.next();
    let second_ch_exists = chars_iter.next().is_some();
    if let Some(ch) = first_ch {
        if !second_ch_exists && ch.is_alphabetic() && ch.is_uppercase() {
            let next_opt = tb.next.clone();
            drop(tb);
            if let Some(next) = next_opt {
                let nb = next.borrow();
                if nb.whitespaces_before_count(sofa) == 0
                    && nb.length_char() == 1
                    && sofa.char_at(nb.begin_char) == '.'
                {
                    let end = next.clone();
                    drop(nb);
                    return Some(PersonItemToken::initial(t.clone(), end, ch, ws, nl));
                }
                drop(nb);
            }
            return Some(PersonItemToken::initial(t.clone(), t.clone(), ch, ws, nl));
        }
    }

    // ── Full word ──────────────────────────────────────────────────────────────

    let term = txt.term.clone();
    drop(tb);

    {
        let tb2 = t.borrow();
        if tb2.chars.is_all_lower() { return None; }
        // ALL-CAPS abbreviations (≥3 chars) without morph proper-name flags → reject
        let is_all_upper = term.chars().all(|c| c.is_uppercase());
        if is_all_upper && term.chars().count() >= 3 {
            let has_proper = tb2.morph.items().iter().any(|wf| {
                wf.base.class.is_proper_surname()
                    || wf.base.class.is_proper_name()
                    || wf.base.class.is_proper_secname()
            });
            if !has_proper { return None; }
        }
    }

    // Collect morph information
    let (fn_item, mn_item, ln_item, morph_gender) = {
        let tb2 = t.borrow();
        let items = tb2.morph.items();

        let mut fn_dict = false;
        let mut mn_dict = false;
        let mut ln_dict = false;
        let mut fn_gender = 0i32;
        let mut mn_gender = 0i32;
        let mut ln_gender = 0i32;
        let mut overall_gender = 0i32;

        for wf in items {
            let g = gender_i32(wf.base.gender);
            if overall_gender == 0 { overall_gender = g; }
            if wf.base.class.is_proper_name()    { fn_dict = true; if fn_gender == 0 { fn_gender = g; } }
            if wf.base.class.is_proper_secname() { mn_dict = true; if mn_gender == 0 { mn_gender = g; } }
            if wf.base.class.is_proper_surname() { ln_dict = true; if ln_gender == 0 { ln_gender = g; } }
        }

        let std_tail = ends_with_std_surname(&term);

        // Build MorphPersonItem for firstname, expanding short names if possible
        let fn_item = if fn_dict {
            let mut m = MorphPersonItem::new(fn_gender, true, false);
            // ShortName expansion: Вася→Василий, Женя→Евгений/Евгения, …
            if let Some(full_names) = get_names_for_shortname(&term) {
                for (full, g) in full_names {
                    // Only add if the expanded full name differs from the surface form
                    if *full != term {
                        m.vars.push((full.clone(), *g));
                    }
                }
            }
            Some(m)
        } else { None };

        let mn_item = if mn_dict { Some(MorphPersonItem::new(mn_gender, true, false)) } else { None };
        let ln_item = if ln_dict || std_tail.is_some() {
            let has_tail = std_tail.is_some();
            let ln_g = if ln_dict { ln_gender } else { std_tail.unwrap_or(0) };
            Some(MorphPersonItem::new(ln_g, ln_dict, has_tail))
        } else { None };

        (fn_item, mn_item, ln_item, overall_gender)
    };

    // No role detected → check if capitalized unknown word could be a surname
    if fn_item.is_none() && mn_item.is_none() && ln_item.is_none() {
        let tb2 = t.borrow();
        let is_capital = tb2.chars.is_capital_upper();
        // Single pass over morph items instead of 5 separate .any() calls
        let mut is_non_name = false;
        for wf in tb2.morph.items() {
            let c = &wf.base.class;
            if c.is_verb() || c.is_adjective() || c.is_pronoun()
                || c.is_preposition() || c.is_conjunction()
            {
                is_non_name = true;
                break;
            }
        }
        drop(tb2);
        if !is_capital || is_non_name {
            return None;
        }
        // Unknown capitalized word — treat as potential lastname
        let mut pit = PersonItemToken {
            begin_token: t.clone(), end_token: t.clone(),
            typ: ItemType::Value, value: term.clone(),
            firstname: None, middlename: None,
            lastname: Some(MorphPersonItem::new(0, false, false)),
            morph_gender,
            whitespaces_before: ws,
            is_newline_before: nl,
            is_all_lower: false,
        };
        // Try Arab postfix after unknown word
        extend_with_arab_postfix(&mut pit, sofa);
        return Some(pit);
    }

    let is_all_lower = t.borrow().chars.is_all_lower();

    let mut pit = PersonItemToken {
        begin_token: t.clone(), end_token: t.clone(),
        typ: ItemType::Value, value: term,
        firstname: fn_item, middlename: mn_item, lastname: ln_item,
        morph_gender,
        whitespaces_before: ws,
        is_newline_before: nl,
        is_all_lower,
    };

    // Try Arab postfix (e.g. "МАМЕД-ОГЛЫ" or "МАМЕД ОГЛЫ" with whitespace ≤ 2)
    extend_with_arab_postfix(&mut pit, sofa);

    Some(pit)
}

/// Extend `pit.end_token` over any immediately following Arab postfix tokens.
/// Handles both hyphen-attached ("ЗАДЕ") and space-separated (≤2 spaces) postfixes.
fn extend_with_arab_postfix(pit: &mut PersonItemToken, sofa: &SourceOfAnalysis) {
    loop {
        let end_next = pit.end_token.borrow().next.clone();
        let Some(next) = end_next else { break };

        let nb = next.borrow();
        // Hyphen-attached: name-ОГЛЫ
        if nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '-' {
            let after = nb.next.clone();
            drop(nb);
            if let Some(after_t) = after {
                let ab = after_t.borrow();
                if let TokenKind::Text(txt) = &ab.kind {
                    if is_arab_postfix(&txt.term) {
                        let _g = arab_postfix_gender(&txt.term);
                        drop(ab);
                        pit.end_token = after_t.clone();
                        continue;
                    }
                }
                drop(ab);
            }
            break;
        }
        // Space-separated: name ОГЛЫ (whitespace ≤ 2)
        let ws = nb.whitespaces_before_count(sofa);
        if ws <= 2 {
            if let TokenKind::Text(txt) = &nb.kind {
                if is_arab_postfix(&txt.term) && !nb.chars.is_all_lower() {
                    let _g = arab_postfix_gender(&txt.term);
                    drop(nb);
                    pit.end_token = next.clone();
                    continue;
                }
            }
        }
        drop(nb);
        break;
    }
}

// ── try_attach_list ───────────────────────────────────────────────────────────

/// Parse a chain of name-part tokens starting at `t`.
///
/// Handles:
/// - Consecutive name parts separated by whitespace
/// - Comma-separated forms: "Иванов, И.И."
/// - Hyphen-joined tokens (hyphenated names passed through)
///
/// Stops at large whitespace gaps, double newlines, or when no further name
/// parts can be parsed.
pub fn try_attach_list(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
    max_count: usize,
) -> Option<Vec<PersonItemToken>> {
    let pit0 = try_attach(t, sofa)?;
    let mut res: Vec<PersonItemToken> = vec![pit0];

    // Whether a comma was seen (comma-inverted FIO: "Иванов, И.И.")
    let mut comma_seen = false;

    let mut cur = res[0].end_token.borrow().next.clone();

    while let Some(tt) = cur.clone() {
        if res.len() >= max_count { break; }

        let ws;
        let nl;
        let is_comma;
        let is_hyphen;
        {
            let tb = tt.borrow();
            ws = tb.whitespaces_before_count(sofa) as usize;
            nl = tb.is_newline_before(sofa);
            is_comma  = tb.length_char() == 1 && sofa.char_at(tb.begin_char) == ',';
            is_hyphen = tb.length_char() == 1 && sofa.char_at(tb.begin_char) == '-';
        }

        // Stop on large whitespace
        if ws > 15 { break; }
        // Stop on newline after the first token (except comma-mode allows one newline gap)
        if nl && !comma_seen { break; }

        // ── Comma handling: "Иванов, И.И." ───────────────────────────────────
        if is_comma && res.len() == 1 && !comma_seen {
            // Only allow comma if the first token looks like a surname
            let first = &res[0];
            let first_looks_like_surname = first.lastname.as_ref()
                .map(|ln| ln.is_in_dictionary || ln.is_lastname_has_std_tail)
                .unwrap_or(false);
            if !first_looks_like_surname {
                break;
            }
            let after_comma = tt.borrow().next.clone();
            let Some(ac) = after_comma else { break };
            // Must have a parseable name part after the comma
            if try_attach(&ac, sofa).is_none() { break; }
            comma_seen = true;
            cur = Some(ac);
            continue;
        }

        // ── Hyphen-adjacent tokens ─────────────────────────────────────────────
        let candidate_t = if is_hyphen {
            let after = tt.borrow().next.clone();
            let Some(after_t) = after else { break };
            let ab_ws = after_t.borrow().whitespaces_before_count(sofa) as usize;
            if ab_ws != 0 { break; }
            after_t
        } else {
            tt.clone()
        };

        // ── Try to parse the next name part ───────────────────────────────────
        match try_attach(&candidate_t, sofa) {
            None    => break,
            Some(p) => {
                cur = p.end_token.borrow().next.clone();
                res.push(p);
            }
        }
    }

    // If a comma was seen, validate we got a multi-token result
    if comma_seen && res.len() < 2 { return None; }

    // A single initial alone (without a surname in the list) is not enough
    if res.len() == 1 && res[0].typ == ItemType::Initial { return None; }

    Some(res)
}
