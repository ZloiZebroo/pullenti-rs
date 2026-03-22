/// PersonNormalData + PersonNormalHelper::Analyze — mirrors `PersonNormalData.cs` and
/// both analysis paths in `PersonNormalHelper.cs`:
///
///  1. **EmptyProcessor path** (primary): morphology-only tokenization →
///     `person_item_token::try_attach_list()` → `person_normal_node::score_and_build()`.
///  2. **StandardProcessor fallback**: full NER pipeline (PersonAnalyzer).

use std::collections::HashMap;
use std::sync::OnceLock;

use pullenti_morph::MorphologyService;
use crate::processor_service::ProcessorService;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::{TokenKind, build_token_chain};
use crate::person::person_referent::{get_firstname, get_middlename, get_lastname, get_sex};
use crate::person::person_normal_result::PersonNormalResult;
use crate::person::person_item_token::try_attach_list;
use crate::person::person_normal_node::score_and_build;

// ── PersonNormalData ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct PersonNormalData {
    /// Фамилия
    pub lastname:     Option<String>,
    /// Фамилия до замужества (alternative)
    pub lastname_alt: Option<String>,
    /// Имя
    pub firstname:    Option<String>,
    /// Имя альтернативное (исходное уменьшительное)
    pub firstname_alt: Option<String>,
    /// Отчество
    pub middlename:   Option<String>,
    /// Пол: 1 = мужчина, 2 = женщина, 0 = неизвестно
    pub gender:       i32,
    /// Тип результата
    pub res_typ:      PersonNormalResult,
    /// Коэффициент качества (0–100)
    pub coef:         i32,
    /// Сообщение об ошибке (если есть)
    pub error_message: Option<String>,
    /// Откорректированные слова: исходное → коррекция
    pub corr_words:   HashMap<String, String>,
}

impl PersonNormalData {
    pub fn new() -> Self {
        PersonNormalData { res_typ: PersonNormalResult::Undefined, ..Default::default() }
    }
}

impl std::fmt::Display for PersonNormalData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:?}): {} {} {}",
            self.coef, self.res_typ,
            self.lastname.as_deref().unwrap_or(""),
            self.firstname.as_deref().unwrap_or(""),
            self.middlename.as_deref().unwrap_or(""))
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Analyze a short text expected to contain a Russian person name (ФИО).
/// Populates and returns a `PersonNormalData`.
///
/// Requires `Sdk::initialize_all()` (or at minimum PersonAnalyzer + GeoAnalyzer) to
/// have been called before first use.
pub fn analyze(txt: &str) -> PersonNormalData {
    let mut res = PersonNormalData::new();
    res.res_typ = PersonNormalResult::NotPerson;
    res.error_message = Some("Похоже на просто текст".to_string());

    // 1. Preprocess text
    let txt = preprocess(txt);
    if txt.chars().count() > 200 {
        return res;
    }
    if txt.trim().is_empty() {
        return res;
    }

    // 2. EmptyProcessor path: morphology-only tokenisation → PersonItemToken scoring
    if let Some(empty_res) = try_empty_processor_path(&txt) {
        // Apply middlename OCR corrections
        let mut r = empty_res;
        apply_midname_corr(&mut r);
        return r;
    }

    // 3. Standard NER processor fallback (includes PersonAnalyzer)
    let sofa = SourceOfAnalysis::new(&txt);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    if ar.first_token.is_none() {
        return res;
    }

    // 3. Scan for GEO / ORG / ADDRESS referents (disqualifiers)
    {
        let mut cur = ar.first_token.clone();
        while let Some(t) = cur {
            let disq = {
                let tb = t.borrow();
                if let TokenKind::Referent(r) = &tb.kind {
                    let tn = r.referent.borrow().type_name.clone();
                    tn == "GEO" || tn == "STREET" || tn == "ADDRESS"
                } else { false }
            };
            if disq {
                res.error_message = Some("Похоже на адрес".to_string());
                return res;
            }
            let disq_org = {
                let tb = t.borrow();
                if let TokenKind::Referent(r) = &tb.kind {
                    r.referent.borrow().type_name == "ORGANIZATION"
                } else { false }
            };
            if disq_org {
                res.error_message = Some("Похоже на организацию".to_string());
                return res;
            }
            let next = t.borrow().next.clone();
            cur = next;
        }
    }

    // 4. Find PersonReferent and fill result
    let mut cur = ar.first_token.clone();
    while let Some(t) = cur {
        let person_rc = {
            let tb = t.borrow();
            if let TokenKind::Referent(r) = &tb.kind {
                if r.referent.borrow().type_name == "PERSON" {
                    Some(r.referent.clone())
                } else { None }
            } else { None }
        };

        if let Some(r) = person_rc {
            let rb = r.borrow();
            res.coef = 100;

            res.firstname = get_firstname(&rb);
            if res.firstname.is_none() {
                res.coef = (res.coef as f64 * 0.8) as i32;
            }
            res.middlename = get_middlename(&rb);
            res.lastname = get_lastname(&rb);
            if res.lastname.is_none() {
                res.coef = (res.coef as f64 * 0.5) as i32;
            }

            // Gender from SEX slot
            match get_sex(&rb).as_deref() {
                Some("Male")   => res.gender = 1,
                Some("Female") => res.gender = 2,
                _              => {}
            }
            drop(rb);

            // Deduct for tokens AFTER the person that are not punctuation-only
            {
                let next = t.borrow().next.clone();
                let mut tt = next;
                while let Some(ttt) = tt {
                    let is_punct_only = {
                        let tb = ttt.borrow();
                        matches!(&tb.kind, TokenKind::Text(_)) && !tb.chars.is_letter()
                    };
                    if !is_punct_only {
                        res.coef = (res.coef as f64 * 0.5) as i32;
                    }
                    let nx = ttt.borrow().next.clone();
                    tt = nx;
                }
            }

            // Deduct for tokens BEFORE the person that are not punctuation-only
            {
                let prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                let mut tt = prev;
                while let Some(ttt) = tt {
                    let is_punct_only = {
                        let tb = ttt.borrow();
                        matches!(&tb.kind, TokenKind::Text(_)) && !tb.chars.is_letter()
                    };
                    if !is_punct_only {
                        res.coef = (res.coef as f64 * 0.5) as i32;
                    }
                    let pv = ttt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    tt = pv;
                }
            }

            // Apply middlename OCR tail corrections
            apply_midname_corr(&mut res);

            res.res_typ = if res.coef >= 90 {
                PersonNormalResult::OK
            } else {
                PersonNormalResult::Manual
            };
            return res;
        }

        let next = t.borrow().next.clone();
        cur = next;
    }

    res
}

// ── EmptyProcessor path ───────────────────────────────────────────────────────

/// Morphology-only path: no NER analyzers, just PersonItemToken scoring.
/// Returns Some(PersonNormalData) if confidence is high enough, else None.
fn try_empty_processor_path(txt: &str) -> Option<PersonNormalData> {
    let sofa = SourceOfAnalysis::new(txt);

    // Build morph tokens (language auto-detected)
    let morph_tokens = MorphologyService::process(txt, None)?;
    if morph_tokens.is_empty() { return None; }

    let first_token = build_token_chain(morph_tokens, &sofa)?;

    // Try to parse a name-part list
    let pits = try_attach_list(&first_token, &sofa, 10)?;

    // Score with PersonNormalNode (threshold 0.35 — below this we fall through)
    let (mut nd, coef) = score_and_build(&pits, 0.35)?;

    // Penalise for tokens AFTER the matched span
    let last_end = pits.last().unwrap().end_token.clone();
    let mut after = last_end.borrow().next.clone();
    while let Some(tt) = after {
        let is_punct_only = {
            let tb = tt.borrow();
            matches!(&tb.kind, TokenKind::Text(_)) && !tb.chars.is_letter()
        };
        if !is_punct_only {
            nd.coef = (nd.coef as f64 * 0.5) as i32;
        }
        let nx = tt.borrow().next.clone();
        after = nx;
    }

    // Penalise for tokens BEFORE
    let first_begin = pits.first().unwrap().begin_token.clone();
    let mut before = first_begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
    while let Some(tt) = before {
        let is_punct_only = {
            let tb = tt.borrow();
            matches!(&tb.kind, TokenKind::Text(_)) && !tb.chars.is_letter()
        };
        if !is_punct_only {
            nd.coef = (nd.coef as f64 * 0.5) as i32;
        }
        let pv = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
        before = pv;
    }

    nd.res_typ = if nd.coef >= 90 {
        PersonNormalResult::OK
    } else if nd.coef >= 35 {
        PersonNormalResult::Manual
    } else {
        return None;
    };
    nd.error_message = None;

    // Gender
    match nd.gender {
        1 => { nd.gender = 1; }
        2 => { nd.gender = 2; }
        _ => {}
    }

    let _ = coef; // already encoded in nd.coef
    Some(nd)
}

// ── Text preprocessing ────────────────────────────────────────────────────────

fn preprocess(txt: &str) -> String {
    let chars: Vec<char> = txt.chars().collect();
    let n = chars.len();
    let mut buf: Vec<char> = Vec::with_capacity(n);

    // Pass 1 (forward): fix hyphen + lowercase → uppercase;
    //                   hyphen + space + letter → remove space, uppercase.
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c == '-' && i + 1 < n {
            let next1 = chars[i + 1];
            if next1.is_lowercase() {
                buf.push(c);
                for uc in next1.to_uppercase() { buf.push(uc); }
                i += 2;
                continue;
            } else if next1 == ' ' && i + 2 < n && chars[i + 2].is_alphabetic() {
                buf.push(c);
                for uc in chars[i + 2].to_uppercase() { buf.push(uc); }
                i += 3;
                continue;
            }
        }
        buf.push(c);
        i += 1;
    }

    // Pass 2 (backward scan equivalent): collapse double spaces / double hyphens,
    //         replace tabs with newlines.
    let mut out: Vec<char> = Vec::with_capacity(buf.len());
    let m = buf.len();
    let mut j = 0;
    while j < m {
        let c = buf[j];
        if c == '\t' {
            out.push('\n');
        } else if (c == ' ' || is_hiphen(c)) && j + 1 < m && buf[j + 1] == c {
            // skip duplicate (keep only one — we'll emit c and skip j+1)
            out.push(c);
            j += 1; // skip the second duplicate
        } else {
            out.push(c);
        }
        j += 1;
    }

    out.iter().collect()
}

fn is_hiphen(c: char) -> bool {
    c == '-' || c == '\u{2013}' || c == '\u{2014}'
}

// ── Middlename OCR tail corrections ──────────────────────────────────────────

static CORR_TAILS: OnceLock<Vec<(String, String)>> = OnceLock::new();

/// Returns (wrong_suffix, correct_suffix) pairs sorted longest-first for greedy match.
fn corr_tails() -> &'static Vec<(String, String)> {
    CORR_TAILS.get_or_init(|| {
        const DATA: &str = "слаовна$:славовна\nславона$:славовна\nславоич$:славович\nслаович$:славович\
\nвнана$:вна\nевана$:евна\nевнва$:евна\nевнаа$:евна\nевнна$:евна\nована$:овна\nовнва$:овна\
\nовнаа$:овна\nовнна$:овна\nевена$:евна\nевсна$:евна\nевона$:евна\nовена$:овна\nовсна$:овна\
\nовона$:овна\nовоич$:ович\nевича$:евич\nевичч$:евич\nевивч$:евич\nевиич$:евич\nеваич$:евич\
\nевнич$:евич\nовича$:ович\nовичч$:ович\nовивч$:ович\nовиич$:ович\nоваич$:ович\nовнич$:ович\
\nеана$:евна\nенва$:евна\nевнв$:евна\nевне$:евна\nевну$:евна\nевны$:евна\nевеа$:евна\
\nеван$:евна\nоана$:овна\nонва$:овна\nовнв$:овна\nовне$:овна\nовну$:овна\nовны$:овна\
\nовеа$:овна\nован$:овна\nевга$:евна\nовга$:овна\nвеич$:евич\nвоич$:ович\nивоч$:ович\
\nовоч$:ович\nевия$:евич\nевмч$:евич\nеивч$:евич\nеаич$:евич\nивеч$:евич\nевоч$:евич\
\nовия$:ович\nовмч$:ович\nоивч$:ович\nовн$:овна\nона$:овна\nеич$:евич\nови$:ович\
\nоич$:ович\nевч$:евич\nеви$:евич\nовч$:ович\nевн$:евна\nена$:евна\nишна$:ична\nофич$:ович\
\nефич$:евич\nева$:евна\nова$:овна";
        let mut pairs: Vec<(String, String)> = Vec::new();
        for line in DATA.split('\n') {
            let line = line.trim().to_uppercase();
            let sep = line.find(':').or_else(|| line.find(';'));
            if let Some(i) = sep {
                let mut key = line[..i].to_string();
                if key.ends_with('$') { key.pop(); }
                let val = line[i + 1..].to_string();
                if !pairs.iter().any(|(k, _)| k == &key) {
                    pairs.push((key, val));
                }
            }
        }
        // Sort by key length descending (longest suffix first)
        pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        pairs
    })
}

fn apply_midname_corr(res: &mut PersonNormalData) {
    if let Some(ref mid) = res.middlename.clone() {
        let mid_upper = mid.to_uppercase();
        for (wrong, correct) in corr_tails() {
            if mid_upper.ends_with(wrong.as_str()) {
                let stem_len = mid_upper.len() - wrong.len();
                let new_mid = format!("{}{}", &mid_upper[..stem_len], correct);
                if new_mid != mid_upper {
                    res.corr_words.entry(mid_upper.clone()).or_insert(new_mid.clone());
                    res.middlename = Some(new_mid);
                    res.coef = (res.coef as f64 * 0.95) as i32;
                }
                break;
            }
        }
    }
}
