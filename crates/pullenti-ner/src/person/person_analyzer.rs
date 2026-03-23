/// PersonAnalyzer — simplified port of PersonAnalyzer.cs.
///
/// Recognizes Russian personal names using morphological class flags:
///   `is_proper_surname()`, `is_proper_name()`, `is_proper_secname()` (patronymic).
///
/// Patterns handled:
///  1. Surname FirstName Patronymic  ("Иванов Иван Иванович")
///  2. Surname FirstName             ("Иванов Иван")
///  3. FirstName Patronymic          ("Иван Иванович")
///  4. Initials before/after Surname ("И.И. Иванов" or "Иванов И.И.")
///  5. Surname alone after a title keyword ("директор Иванов")

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::person::person_referent as pr;
use crate::person::person_property_referent as ppr;
use crate::person::person_attr_table as pat;
use crate::person::person_id_token;

pub struct PersonAnalyzer;

impl PersonAnalyzer {
    pub fn new() -> Self { PersonAnalyzer }
}

impl Analyzer for PersonAnalyzer {
    fn name(&self) -> &'static str { "PERSON" }
    fn caption(&self) -> &'static str { "Персоны" }

    fn process(&self, kit: &mut AnalysisKit) {
        // Eagerly init the table so its cost is paid once.
        let _ = pat::get_table();

        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }
            // Try: identity document (паспорт серия/номер) — before person patterns
            if let Some((referent, begin, end)) = person_id_token::try_attach(&t, &sofa) {
                let r_rc = Rc::new(RefCell::new(referent));
                let r_rc = kit.add_entity(r_rc);
                // Check if the immediately preceding token is a PERSON → link via IDDOC
                {
                    let prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    if let Some(prev_tok) = prev {
                        let pb = prev_tok.borrow();
                        // Check if it's a comma/colon before checking further back
                        let check = if pb.is_char(',', &sofa) || pb.is_char(':', &sofa) {
                            pb.prev.as_ref().and_then(|w| w.upgrade())
                        } else {
                            Some(prev_tok.clone())
                        };
                        drop(pb);
                        if let Some(pers_tok) = check {
                            let ptb = pers_tok.borrow();
                            if let TokenKind::Referent(ref rd) = ptb.kind {
                                if rd.referent.borrow().type_name == pr::OBJ_TYPENAME {
                                    rd.referent.borrow_mut().add_slot(
                                        pr::ATTR_IDDOC,
                                        crate::referent::SlotValue::Referent(r_rc.clone()),
                                        false,
                                    );
                                }
                            }
                        }
                    }
                }
                let tok = Rc::new(RefCell::new(Token::new_referent(begin, end, r_rc)));
                kit.embed_token(tok.clone());
                cur = tok.borrow().next.clone();
                continue;
            }
            // Try: prefix/title term → person name
            if let Some(pairs) = try_prefix_person(&t, &sofa) {
                let mut last_tok = t.clone();
                for (referent, begin, end) in pairs {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let tok = Rc::new(RefCell::new(
                        Token::new_referent(begin, end, r_rc)
                    ));
                    kit.embed_token(tok.clone());
                    last_tok = tok;
                }
                cur = last_tok.borrow().next.clone();
                continue;
            }
            match try_parse(&t, &sofa) {
                None => { cur = t.borrow().next.clone(); }
                Some((referent, end)) => {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let tok = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), end, r_rc)
                    ));
                    kit.embed_token(tok.clone());
                    cur = tok.borrow().next.clone();
                }
            }
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();
    let TokenKind::Text(_) = &tb.kind else { return None; };

    // Must start with an uppercase letter
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    if !surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return None;
    }
    drop(tb);

    // --- Pattern A: Initials (e.g. "И." or "И.И.") before/without surname ---
    // Try initials + surname first so we don't accidentally consume the initial
    // as a one-letter word.
    if let Some(r) = try_initials_then_surname(t, sofa) {
        return Some(r);
    }

    // --- Pattern EN: English / Latin-script names ---
    // Must try before Russian patterns so that ASCII names don't fall through.
    if let Some(r) = try_english_person(t, sofa) {
        return Some(r);
    }

    // --- Pattern B: Surname as start ---
    if is_proper_surname_token(t) {
        // B1: Surname FirstName Patronymic
        if let Some(r) = try_surname_name_secname(t, sofa) { return Some(r); }
        // B2: Surname FirstName
        if let Some(r) = try_surname_name(t, sofa) { return Some(r); }
        // B3: Surname Initials ("Иванов И.И.")
        if let Some(r) = try_surname_initials(t, sofa) { return Some(r); }
        // B4: Surname alone (only if after a title keyword in previous token)
        if let Some(r) = try_surname_alone(t, sofa) { return Some(r); }
    }

    // --- Pattern C: FirstName [Patronymic | Surname] ---
    if is_proper_name_token(t) {
        // C1: FirstName + Patronymic
        if let Some(r) = try_name_secname(t, sofa) { return Some(r); }
        // C2: FirstName + Surname (e.g. "Михаилом Жуковым")
        if let Some(r) = try_name_surname(t, sofa) { return Some(r); }
    }

    None
}

// ── Pattern A: Initials + Surname ("И.И. Иванов") ────────────────────────────

fn try_initials_then_surname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Check: single uppercase letter at current token
    let tb = t.borrow();
    let surface0 = sofa.substring(tb.begin_char, tb.end_char);
    if surface0.chars().count() != 1 { return None; }
    let first_ch = surface0.chars().next()?;
    if !first_ch.is_uppercase() || !first_ch.is_alphabetic() { return None; }
    let next1 = tb.next.clone()?;
    drop(tb);

    // Next must be "." immediately after
    let n1b = next1.borrow();
    if n1b.whitespaces_before_count(sofa) != 0 || n1b.length_char() != 1 || sofa.char_at(n1b.begin_char) != '.' {
        return None;
    }
    let next2 = n1b.next.clone()?;
    drop(n1b);

    // Try to collect second initial: "В." pattern
    let mut end_tok = next1.clone(); // end at first dot
    let n2b = next2.borrow();
    let surface2 = sofa.substring(n2b.begin_char, n2b.end_char);
    let second_ch = surface2.chars().next();
    let has_second_initial = n2b.whitespaces_before_count(sofa) == 0
        && surface2.chars().count() == 1
        && second_ch.map(|c| c.is_uppercase() && c.is_alphabetic()).unwrap_or(false);

    if has_second_initial {
        let next3 = n2b.next.clone();
        drop(n2b);
        if let Some(n3) = next3 {
            let n3b = n3.borrow();
            if n3b.whitespaces_before_count(sofa) == 0 && n3b.length_char() == 1 && sofa.char_at(n3b.begin_char) == '.' {
                end_tok = n3.clone();
                let next4 = n3b.next.clone();
                drop(n3b);
                // After initials, look for surname
                if let Some(n4) = next4 {
                    if let Some((r, end)) = check_surname_after_initials(&n4, first_ch, second_ch, sofa) {
                        return Some((r, end));
                    }
                }
            } else {
                drop(n3b);
            }
        }
    } else {
        drop(n2b);
    }

    let _ = end_tok;
    None
}

fn check_surname_after_initials(
    t: &TokenRef,
    init1: char,
    init2: Option<char>,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();
    let TokenKind::Text(txt) = &tb.kind else { return None; };
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    if !surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) { return None; }
    let is_sur = tb.morph.items().iter().any(|wf| wf.base.class.is_proper_surname());
    if !is_sur { return None; }
    let name = normal_form_of(t);
    drop(tb);

    let mut r = pr::new_person_referent();
    pr::set_lastname(&mut r, &name);
    pr::set_firstname(&mut r, &init1.to_uppercase().to_string());
    if let Some(ch2) = init2 {
        pr::set_middlename(&mut r, &ch2.to_uppercase().to_string());
    }
    Some((r, t.clone()))
}

// ── Pattern B1: Surname FirstName Patronymic ─────────────────────────────────

fn try_surname_name_secname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let surname = normal_form_of(t);
    let n1 = t.borrow().next.clone()?;
    if n1.borrow().whitespaces_before_count(sofa) == 0 { return None; }
    if !is_proper_name_token(&n1) { return None; }
    let firstname = normal_form_of(&n1);
    let n2 = n1.borrow().next.clone()?;
    if n2.borrow().whitespaces_before_count(sofa) == 0 { return None; }
    if !is_proper_secname_token(&n2) { return None; }
    let midname = normal_form_of(&n2);
    let sex = infer_sex_from_secname(&midname);

    let mut r = pr::new_person_referent();
    pr::set_lastname(&mut r, &surname);
    pr::set_firstname(&mut r, &firstname);
    pr::set_middlename(&mut r, &midname);
    if let Some(s) = sex { pr::set_sex(&mut r, s); }
    Some((r, n2.clone()))
}

// ── Pattern B2: Surname FirstName ────────────────────────────────────────────

fn try_surname_name(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let surname = normal_form_of(t);
    let n1 = t.borrow().next.clone()?;
    if n1.borrow().whitespaces_before_count(sofa) == 0 { return None; }
    if !is_proper_name_token(&n1) { return None; }
    let firstname = normal_form_of(&n1);

    let mut r = pr::new_person_referent();
    pr::set_lastname(&mut r, &surname);
    pr::set_firstname(&mut r, &firstname);
    Some((r, n1.clone()))
}

// ── Pattern B3: Surname Initials ("Иванов И.И.") ─────────────────────────────

fn try_surname_initials(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let surname = normal_form_of(t);
    let n1 = t.borrow().next.clone()?;
    let (init1, init2, end) = collect_initials(&n1, sofa)?;

    let mut r = pr::new_person_referent();
    pr::set_lastname(&mut r, &surname);
    pr::set_firstname(&mut r, &init1.to_uppercase().to_string());
    if let Some(ch) = init2 {
        pr::set_middlename(&mut r, &ch.to_uppercase().to_string());
    }
    Some((r, end))
}

/// Try to read "X." or "X.Y." where X and Y are uppercase single letters.
/// Returns (first_initial, Option<second_initial>, end_token).
fn collect_initials(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(char, Option<char>, TokenRef)> {
    let tb = t.borrow();
    if tb.whitespaces_before_count(sofa) == 0 { return None; }
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    if surface.chars().count() != 1 { return None; }
    let c1 = surface.chars().next()?;
    if !c1.is_uppercase() || !c1.is_alphabetic() { return None; }
    let next1 = tb.next.clone()?;
    drop(tb);

    let n1b = next1.borrow();
    if n1b.whitespaces_before_count(sofa) != 0 || n1b.length_char() != 1 || sofa.char_at(n1b.begin_char) != '.' {
        return None;
    }
    let next2 = n1b.next.clone();
    let dot1 = next1.clone();
    drop(n1b);

    // Try second initial
    if let Some(n2) = next2 {
        let n2b = n2.borrow();
        let surf2 = sofa.substring(n2b.begin_char, n2b.end_char);
        if n2b.whitespaces_before_count(sofa) == 0 && surf2.chars().count() == 1 {
            let c2 = surf2.chars().next().unwrap_or('\0');
            if c2.is_uppercase() && c2.is_alphabetic() {
                let next3 = n2b.next.clone();
                drop(n2b);
                if let Some(n3) = next3 {
                    let n3b = n3.borrow();
                    if n3b.whitespaces_before_count(sofa) == 0 && n3b.length_char() == 1 && sofa.char_at(n3b.begin_char) == '.' {
                        drop(n3b);
                        return Some((c1, Some(c2), n3.clone()));
                    }
                }
            }
        }
    }

    Some((c1, None, dot1))
}

// ── Pattern B4: Surname alone after title ─────────────────────────────────────

fn try_surname_alone(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Only extract surname alone if preceded by a person-title keyword.
    // Check if the previous token is a known title.
    let prev = t.borrow().prev.as_ref()?.upgrade()?;
    let pb = prev.borrow();
    let is_title = match &pb.kind {
        TokenKind::Text(txt) => is_person_title(&txt.term),
        _ => false,
    };
    drop(pb);
    if !is_title { return None; }

    let surname = normal_form_of(t);
    let mut r = pr::new_person_referent();
    pr::set_lastname(&mut r, &surname);
    Some((r, t.clone()))
}

// ── Pattern C: FirstName Patronymic [Surname] ─────────────────────────────────

fn try_name_secname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let firstname = normal_form_of(t);
    let n1 = t.borrow().next.clone()?;
    if n1.borrow().whitespaces_before_count(sofa) == 0 { return None; }
    if !is_proper_secname_token(&n1) { return None; }
    let midname = normal_form_of(&n1);
    let sex = infer_sex_from_secname(&midname);

    // C3 extension: FirstName + Patronymic + Surname ("Мария Петровна Иванова")
    if let Some(n2) = n1.borrow().next.clone() {
        if n2.borrow().whitespaces_before_count(sofa) <= 1
            && is_proper_surname_token_ctx(&n2, sofa)
        {
            let surname = normal_form_of(&n2);
            let mut r = pr::new_person_referent();
            pr::set_firstname(&mut r, &firstname);
            pr::set_middlename(&mut r, &midname);
            pr::set_lastname(&mut r, &surname);
            if let Some(s) = sex { pr::set_sex(&mut r, s); }
            return Some((r, n2.clone()));
        }
    }

    let mut r = pr::new_person_referent();
    pr::set_firstname(&mut r, &firstname);
    pr::set_middlename(&mut r, &midname);
    if let Some(s) = sex { pr::set_sex(&mut r, s); }
    Some((r, n1.clone()))
}

// ── Pattern C2: FirstName Surname ("Михаилом Жуковым") ───────────────────────

fn try_name_surname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let firstname = normal_form_of(t);
    let n1 = t.borrow().next.clone()?;
    if n1.borrow().whitespaces_before_count(sofa) == 0 { return None; }
    if !is_proper_surname_token_ctx(&n1, sofa) { return None; }
    // Note: we do NOT filter on is_proper_name_token(n1) here because many common Russian
    // surnames (Петров, Иванов, Сидоров) are flagged as both proper_surname AND proper_name
    // in the morph dictionary. The is_proper_surname_token_ctx check above (which also
    // requires uppercase) is the sufficient filter.
    let surname = normal_form_of(&n1);
    let mut r = pr::new_person_referent();
    pr::set_firstname(&mut r, &firstname);
    pr::set_lastname(&mut r, &surname);
    Some((r, n1.clone()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn starts_uppercase(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    let surface = sofa.substring(tb.begin_char, tb.end_char);
    surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

fn is_proper_surname_token(t: &TokenRef) -> bool {
    t.borrow().morph.items().iter().any(|wf| wf.base.class.is_proper_surname())
}

fn is_proper_surname_token_ctx(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    // Surnames always start with uppercase in Russian prose.
    // Rejects lowercase homophones like "оттого" (conjunction) that happen to
    // have a rare is_proper_surname morph form.
    starts_uppercase(t, sofa) && is_proper_surname_token(t)
}

fn is_proper_name_token(t: &TokenRef) -> bool {
    t.borrow().morph.items().iter().any(|wf| wf.base.class.is_proper_name())
}

fn is_proper_secname_token(t: &TokenRef) -> bool {
    t.borrow().morph.items().iter().any(|wf| wf.base.class.is_proper_secname())
}

/// Get the canonical (nominative) form of a name token.
/// Tries normal_case first; falls back to the term (surface form).
fn normal_form_of(t: &TokenRef) -> String {
    let tb = t.borrow();
    if let TokenKind::Text(txt) = &tb.kind {
        // Among proper-noun forms, prefer forms that also have `nf` set (the full
        // dictionary base form — e.g. "АНФИСА" with nf="АНФИСА" over "АНФИС" with nf=None).
        // Use indices to avoid cloning strings until we know which one we need.
        let items = tb.morph.items();
        let mut proper_idx_with_nf: Option<usize> = None;
        let mut proper_idx_no_nf:   Option<usize> = None;
        let mut first_idx:          Option<usize> = None;
        for (i, wf) in items.iter().enumerate() {
            if wf.normal_case.is_some() {
                let is_proper = wf.base.class.is_proper_surname()
                    || wf.base.class.is_proper_name()
                    || wf.base.class.is_proper_secname();
                if is_proper {
                    if wf.normal_full.is_some() {
                        if proper_idx_with_nf.is_none() { proper_idx_with_nf = Some(i); }
                    } else if proper_idx_no_nf.is_none() {
                        proper_idx_no_nf = Some(i);
                    }
                }
                if first_idx.is_none() { first_idx = Some(i); }
            }
        }
        let chosen = proper_idx_with_nf.or(proper_idx_no_nf).or(first_idx);
        if let Some(idx) = chosen {
            return items[idx].normal_case.as_ref().unwrap().clone();
        }
        return txt.term.clone();
    }
    String::new()
}

/// Infer gender from patronymic ending.
/// Russian: -вич → Male; -вна/-овна/-евна → Female
fn infer_sex_from_secname(secname: &str) -> Option<&'static str> {
    // secname comes from normal_form_of() which returns uppercase morph forms
    if secname.ends_with("ВИЧ") || secname.ends_with("ИЧ") { return Some(pr::SEX_MALE); }
    if secname.ends_with("ВНА") || secname.ends_with("НА") { return Some(pr::SEX_FEMALE); }
    None
}

/// Returns true if the uppercase term is a common person-title keyword.
fn is_person_title(term: &str) -> bool {
    matches!(term,
        "ДЕПУТАТ" | "ДИРЕКТОР" | "ПРЕЗИДЕНТ" | "МИНИСТР" | "ГУБЕРНАТОР" |
        "ГЛАВА" | "МЭРЬ" | "МЭР" | "ПРОКУРОР" | "СУДЬЯ" | "ГЕНЕРАЛ" |
        "ПОЛКОВНИК" | "МАЙОР" | "КАПИТАН" | "ЛЕЙТЕНАНТ" | "СЕРЖАНТ" |
        "АКАДЕМИК" | "ПРОФЕССОР" | "ДОЦЕНТ" | "ДОКТОР" | "КАНДИДАТ" |
        "РЕКТОР" | "ДЕКАН" | "ПРЕДСЕДАТЕЛЬ" | "СЕКРЕТАРЬ" | "РУКОВОДИТЕЛЬ" |
        "НАЧАЛЬНИК" | "ЗАМЕСТИТЕЛЬ" | "ЗАМГЛАВЫ" | "ПОМОЩНИК" | "СОВЕТНИК" |
        "СЕНАТОР" | "КОМИССАР" | "СЛЕДОВАТЕЛЬ" | "ИНСПЕКТОР" | "АУДИТОР" |
        "МЕНЕДЖЕР" | "КООРДИНАТОР" | "АНАЛИТИК" | "СПЕЦИАЛИСТ" | "ЭКСПЕРТ" |
        "MEMBER" | "DIRECTOR" | "PRESIDENT" | "MINISTER" | "GOVERNOR" |
        "SENATOR" | "PROFESSOR" | "DOCTOR" | "JUDGE" | "GENERAL" |
        "КАПИТАН-ЛЕЙТЕНАНТ" | "ГЕНЕРАЛ-МАЙОР" | "ГЕНЕРАЛ-ЛЕЙТЕНАНТ" |
        "ГОСПОДИН" | "ГРАЖДАНИН" | "МУЖЧИНА" | "ЖЕНЩИНА" |
        "ТОВАРИЩ" | "ТОВ." | "Г-Н" | "ГОСПОДА" | "ГН" | "ГЖА" |
        // Historical / literary Russian titles
        "ГРАФ" | "КНЯЗЬ" | "БАРОН" | "ГЕРЦОГ" | "МАРКИЗ" | "ВИКОНТ" |
        "ПОМЕЩИК" | "БАРИН" | "ДВОРЯНИН" | "КУПЕЦ" | "БОЯРИН" |
        "ПОРУЧИК" | "ШТАБС-КАПИТАН" | "КОЛЛЕЖСКИЙ" | "НАДВОРНЫЙ" | "СТАТСКИЙ" |
        // Clergy
        "АРХИЕПИСКОП" | "ЕПИСКОП" | "МИТРОПОЛИТ" | "ПАТРИАРХ" | "ПРОТОИЕРЕЙ" |
        "СВЯЩЕННИК" | "БАТЮШКА" | "ДЬЯКОН" |
        // More EN titles
        "COLONEL" | "MAJOR" | "CAPTAIN" | "LIEUTENANT" | "SERGEANT" |
        "MR" | "MRS" | "MS" | "MISS" | "SIR" | "LORD" | "LADY" | "DR" | "PROF"
    )
}

// ── English / Latin-script person detection ───────────────────────────────────
//
// Handles names with ASCII and extended-Latin characters (é, ü, Ç, à, etc.).
// These names do NOT have Russian morphological proper-noun class flags, so
// the Russian patterns above miss them entirely.
//
// Patterns:
//   EN-1: Firstname Lastname                  ("Jacob Devlin", "Onur Çelebi")
//   EN-2: Firstname Initial. Lastname         ("Marta R. Costa-jussà", "Keith B. Hall")
//
// Firstname: simple | linebreak-hyphen ("Ken-\nton") | inline-hyphen ("Pierre-Emmanuel")
// Lastname:  simple | inline-hyphen ("Costa-jussà") | linebreak-hyphen ("Hef-\nfernan")
//            | 2-word ("Mejia Gonzalez", "Kalim Akram")

/// Extended-Latin alphabetic: includes accented chars but not Cyrillic.
fn is_en_alpha(c: char) -> bool {
    c.is_alphabetic() && (c as u32) < 0x0400
}

/// A valid English-script name word:
/// starts uppercase, all chars extended-Latin alphabetic, not an all-caps abbreviation.
fn is_valid_en_name_word(s: &str) -> bool {
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_alphabetic() || !first.is_uppercase() { return false; }
    let second = match chars.next() {
        Some(c) => c,
        None => return false, // need at least 2 chars
    };
    if !is_en_alpha(second) { return false; }
    // Check remaining chars and count uppercase alphabetics in one pass
    let mut alpha_upper_count: usize = if first.is_uppercase() { 1 } else { 0 }
        + if second.is_alphabetic() && second.is_uppercase() { 1 } else { 0 };
    let mut total_alpha: usize = 2; // first + second are both alphabetic
    for c in chars {
        if !is_en_alpha(c) { return false; }
        if c.is_alphabetic() {
            total_alpha += 1;
            if c.is_uppercase() { alpha_upper_count += 1; }
        }
    }
    // All-caps with ≥ 3 alphabetic chars = abbreviation (NLLB, ACM, IEEE…)
    if total_alpha >= 3 && alpha_upper_count == total_alpha { return false; }
    true
}

fn get_surface(t: &TokenRef, sofa: &SourceOfAnalysis) -> String {
    let tb = t.borrow();
    sofa.substring(tb.begin_char, tb.end_char).to_string()
}

/// Try "word-\ncont" linebreak-hyphen: hyphen adjacent to t, continuation after newline.
fn try_linebreak_hyphen(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let base = get_surface(t, sofa);
    let n1 = t.borrow().next.clone()?;
    {
        let nb = n1.borrow();
        if nb.whitespaces_before_count(sofa) != 0 { return None; }
        if nb.length_char() != 1 || sofa.char_at(nb.begin_char) != '-' { return None; }
    }
    let n2 = n1.borrow().next.clone()?;
    {
        let nb = n2.borrow();
        if nb.whitespaces_before_count(sofa) < 1 { return None; } // newline after hyphen
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }
    let cont = get_surface(&n2, sofa);
    if cont.is_empty() { return None; }
    if !cont.chars().all(|c| is_en_alpha(c)) { return None; }
    Some((format!("{}-{}", base, cont), n2))
}

/// Try "word-cont" inline-hyphen: both hyphen and continuation adjacent (no whitespace).
/// `require_uppercase_cont`: for firstnames the second part must be uppercase.
fn try_inline_hyphen(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
    require_uppercase_cont: bool,
) -> Option<(String, TokenRef)> {
    let base = get_surface(t, sofa);
    let n1 = t.borrow().next.clone()?;
    {
        let nb = n1.borrow();
        if nb.whitespaces_before_count(sofa) != 0 { return None; }
        if nb.length_char() != 1 || sofa.char_at(nb.begin_char) != '-' { return None; }
    }
    let n2 = n1.borrow().next.clone()?;
    {
        let nb = n2.borrow();
        if nb.whitespaces_before_count(sofa) != 0 { return None; } // must be inline (no newline)
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }
    let cont = get_surface(&n2, sofa);
    let first_cont = match cont.chars().next() {
        Some(c) => c,
        None => return None,
    };
    if require_uppercase_cont
        && (!first_cont.is_alphabetic() || !first_cont.is_uppercase())
    {
        return None;
    }
    if !cont.chars().all(|c| is_en_alpha(c)) { return None; }
    Some((format!("{}-{}", base, cont), n2))
}

/// Collect a firstname starting at t.
/// Returns (text, last_consumed_token).
fn collect_en_firstname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let surface = get_surface(t, sofa);
    if !is_valid_en_name_word(&surface) { return None; }
    if is_en_stop_word(&surface.to_uppercase()) { return None; }
    // Linebreak hyphen takes priority: "Ken-\nton"
    if let Some((combined, last)) = try_linebreak_hyphen(t, sofa) {
        return Some((combined, last));
    }
    // Inline hyphen with uppercase second part: "Pierre-Emmanuel", "Ming-Wei"
    if let Some((combined, last)) = try_inline_hyphen(t, sofa, true) {
        return Some((combined, last));
    }
    Some((surface, t.clone()))
}

/// Collect a lastname starting at t.
/// Returns (text, last_consumed_token).
fn collect_en_lastname(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef)> {
    let surface = get_surface(t, sofa);
    if !is_valid_en_name_word(&surface) { return None; }
    if is_en_stop_word(&surface.to_uppercase()) { return None; }
    // Linebreak hyphen: "Hef-\nfernan"
    if let Some((combined, last)) = try_linebreak_hyphen(t, sofa) {
        return Some((combined, last));
    }
    // Inline hyphen (allow lowercase continuation): "Costa-jussà", "Howard-Snyder"
    if let Some((combined, last)) = try_inline_hyphen(t, sofa, false) {
        return Some((combined, last));
    }
    // 2-word lastname: "Mejia Gonzalez", "Kalim Akram"
    // Require n2 is a valid name word AND n3 after n2 is NOT (prevents over-absorption).
    {
        let n2 = t.borrow().next.clone();
        if let Some(n2) = n2 {
            let n2_ok = {
                let nb = n2.borrow();
                let ws = nb.whitespaces_before_count(sofa);
                ws >= 1 && ws <= 10
                    && matches!(nb.kind, TokenKind::Text(_))
            };
            if n2_ok {
                let n2_surf = get_surface(&n2, sofa);
                if is_valid_en_name_word(&n2_surf)
                    && !is_en_stop_word(&n2_surf.to_uppercase())
                {
                    let n3 = n2.borrow().next.clone();
                    let n3_is_name = n3
                        .map(|n3| {
                            let n3_ws;
                            let n3_is_text;
                            {
                                let nb = n3.borrow();
                                n3_ws = nb.whitespaces_before_count(sofa);
                                n3_is_text = matches!(nb.kind, TokenKind::Text(_));
                            }
                            if n3_ws < 1 || n3_ws > 10 || !n3_is_text {
                                return false;
                            }
                            let surf = get_surface(&n3, sofa);
                            is_valid_en_name_word(&surf)
                                && !is_en_stop_word(&surf.to_uppercase())
                        })
                        .unwrap_or(false);
                    if !n3_is_name {
                        let combined = format!("{} {}", surface, n2_surf);
                        return Some((combined, n2.clone()));
                    }
                }
            }
        }
    }
    Some((surface, t.clone()))
}

fn try_english_person(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let (firstname, fn_end) = collect_en_firstname(t, sofa)?;
    // EN-2: Firstname  Initial.  Lastname  ("Marta R. Costa-jussà", "Keith B. Hall")
    if let Some(r) = try_en_v2_initial_lastname(&fn_end, &firstname, sofa) {
        return Some(r);
    }
    // EN-1: Firstname  Lastname  ("Jacob Devlin", "Onur Çelebi")
    if let Some(r) = try_en_v1_lastname(&fn_end, &firstname, sofa) {
        return Some(r);
    }
    None
}

fn try_en_v1_lastname(
    fn_end: &TokenRef,
    firstname: &str,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let n1 = fn_end.borrow().next.clone()?;
    {
        let nb = n1.borrow();
        if nb.whitespaces_before_count(sofa) == 0 { return None; }
        if nb.whitespaces_before_count(sofa) > 10 { return None; }
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }
    let (lastname, last_tok) = collect_en_lastname(&n1, sofa)?;
    let mut r = pr::new_person_referent();
    pr::set_firstname(&mut r, firstname);
    pr::set_lastname(&mut r, &lastname);
    Some((r, last_tok))
}

fn try_en_v2_initial_lastname(
    fn_end: &TokenRef,
    firstname: &str,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    // Next: single uppercase (possibly extended-Latin) letter
    let n1 = fn_end.borrow().next.clone()?;
    let (initial_ch, dot_tok) = {
        let nb = n1.borrow();
        if nb.whitespaces_before_count(sofa) == 0 { return None; }
        let s = sofa.substring(nb.begin_char, nb.end_char);
        let chs: Vec<char> = s.chars().collect();
        if chs.len() != 1 { return None; }
        let c = chs[0];
        if !c.is_alphabetic() || !c.is_uppercase() { return None; }
        // Must be followed by adjacent "."
        let dot = nb.next.clone()?;
        let db = dot.borrow();
        if db.whitespaces_before_count(sofa) != 0 { return None; }
        if db.length_char() != 1 || sofa.char_at(db.begin_char) != '.' { return None; }
        (c, dot.clone())
    };
    // After dot: lastname
    let n3 = dot_tok.borrow().next.clone()?;
    {
        let nb = n3.borrow();
        if nb.whitespaces_before_count(sofa) == 0 { return None; }
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }
    let (lastname, last_tok) = collect_en_lastname(&n3, sofa)?;
    let mut r = pr::new_person_referent();
    pr::set_firstname(&mut r, firstname);
    pr::set_middlename(&mut r, &initial_ch.to_string());
    pr::set_lastname(&mut r, &lastname);
    Some((r, last_tok))
}

/// Common English words that are NOT person names.
/// Prevents false-positives like "Natural Language", "Deep Learning", etc.
fn is_en_stop_word(upper: &str) -> bool {
    matches!(upper,
        // Articles, determiners
        "THE" | "A" | "AN" | "THIS" | "THAT" | "THESE" | "THOSE" | "ANY" | "ALL" |
        "SOME" | "EACH" | "EVERY" | "NO" | "BOTH" | "EITHER" | "NEITHER" |
        // Prepositions
        "IN" | "ON" | "AT" | "TO" | "FOR" | "OF" | "WITH" | "BY" | "FROM" |
        "AS" | "INTO" | "THROUGH" | "DURING" | "BEFORE" | "AFTER" | "ABOVE" |
        "BELOW" | "BETWEEN" | "AMONG" | "UNDER" | "OVER" | "ACROSS" | "ALONG" |
        "ABOUT" | "AROUND" | "AGAINST" | "WITHOUT" | "WITHIN" | "UPON" |
        "TOWARD" | "TOWARDS" | "ONTO" | "OFF" | "OUT" | "UP" | "DOWN" |
        "NEAR" | "SINCE" | "UNTIL" | "WHILE" | "DESPITE" | "EXCEPT" | "PER" |
        "BEHIND" | "LEFT" | "RIGHT" | "FRONT" | "BACK" | "AHEAD" |
        // Conjunctions
        "AND" | "OR" | "BUT" | "NOR" | "YET" | "SO" | "BOTH" | "EITHER" |
        "NEITHER" | "WHETHER" | "ALTHOUGH" | "THOUGH" | "BECAUSE" | "SINCE" |
        "UNLESS" | "WHEN" | "WHILE" | "WHERE" | "AFTER" | "BEFORE" | "IF" |
        // Pronouns
        "HE" | "SHE" | "IT" | "THEY" | "WE" | "I" | "YOU" | "HIM" | "HER" |
        "THEM" | "US" | "ME" | "HIS" | "ITS" | "THEIR" | "OUR" | "MY" | "YOUR" |
        "WHO" | "WHOM" | "WHOSE" | "WHICH" | "WHAT" | "THAT" |
        // Common verbs
        "IS" | "ARE" | "WAS" | "WERE" | "BE" | "BEEN" | "BEING" | "HAVE" | "HAS" |
        "HAD" | "DO" | "DOES" | "DID" | "WILL" | "WOULD" | "SHALL" | "SHOULD" |
        "MAY" | "MIGHT" | "MUST" | "CAN" | "COULD" | "GET" | "MAKE" | "USE" |
        // NLP / ML / academic common words (prevent false positives in papers)
        "NATURAL" | "LANGUAGE" | "PROCESSING" | "MACHINE" | "LEARNING" | "DEEP" |
        "NEURAL" | "NETWORK" | "TRANSFORMER" | "ATTENTION" | "TRAINING" |
        "TESTING" | "EVALUATION" | "BENCHMARK" | "DATASET" | "MODEL" | "MODELS" |
        "SYSTEM" | "SYSTEMS" | "METHOD" | "METHODS" | "APPROACH" | "APPROACHES" |
        "RETRIEVAL" | "GENERATION" | "CLASSIFICATION" | "EMBEDDING" | "EMBEDDINGS" |
        "REPRESENTATION" | "REPRESENTATIONS" | "ENCODER" | "DECODER" |
        "SELF" | "MULTI" | "PRE" | "FINE" | "LARGE" |
        "SMALL" | "BASE" | "BASED" | "END" | "STATE" | "ART" | "NEW" | "OLD" |
        "HIGH" | "LOW" | "FAST" | "SLOW" | "BEST" | "GOOD" | "BETTER" | "MORE" |
        "LESS" | "MOST" | "LEAST" | "VERY" | "WELL" | "ALSO" | "ONLY" | "JUST" |
        "EVEN" | "STILL" | "AGAIN" | "THEN" | "NOW" | "ALREADY" | "ALWAYS" |
        "NEVER" | "OFTEN" | "USUALLY" | "RECENTLY" | "HOWEVER" | "THEREFORE" |
        "THUS" | "HENCE" | "FINALLY" | "FIRST" | "SECOND" | "THIRD" | "LAST" |
        "NEXT" | "PREVIOUS" | "FOLLOWING" | "ABOVE" | "BELOW" | "HERE" | "THERE" |
        // Common organizational words
        "ASSOCIATION" | "CONFERENCE" | "WORKSHOP" | "PROCEEDINGS" | "JOURNAL" |
        "INTERNATIONAL" | "NATIONAL" | "ANNUAL" | "COMPUTATIONAL" | "EMPIRICAL" |
        "LINGUISTICS" | "INTELLIGENCE" | "ARTIFICIAL" | "SCIENCE" | "RESEARCH" |
        "UNIVERSITY" | "INSTITUTE" | "LABORATORY" | "DEPARTMENT" | "CENTER" |
        "PAPER" | "REPORT" | "SURVEY" | "REVIEW" | "ANALYSIS" | "STUDY" |
        "RESULTS" | "EXPERIMENTS" | "PERFORMANCE" | "ACCURACY" | "QUALITY" |
        "HUMAN" | "USER" | "QUERY" | "DOCUMENT" | "TOKEN" | "WORD" | "SENTENCE" |
        "TEXT" | "DATA" | "INFORMATION" | "KNOWLEDGE" | "QUESTION" | "ANSWER" |
        // Common non-name nouns / adjectives
        "TOP" | "BOTTOM" | "SIDE" | "WEB" | "NET" | "ONLINE" |
        "SEARCH" | "GUN" | "SHOW" | "PLAY" | "LOAD" | "BUILD" |
        // Security / technical terms (false-positive suppressors)
        "COMPROMISED" | "BLACKLIST" | "WHITELIST" | "DOMAIN" | "DOMAINS" |
        "ALGORITHM" | "ALGORITHMS" | "BULLET" | "POINT" | "LINE" | "ARCTIC" |
        "ENGLISH" | "SPANISH" | "FRENCH" | "GERMAN" | "CHINESE" | "JAPANESE" |
        "KOREAN" | "ARABIC" | "PORTUGUESE" | "ITALIAN" | "DUTCH" | "RUSSIAN" |
        "YES" | "NO" | "TRUE" | "FALSE" | "NULL" | "NONE" | "OK" | "OKAY" |
        // Group / role nouns
        "TEAM" | "GROUP" | "STAFF" | "CREW" | "CAST" | "BOARD" | "PANEL" |
        // Ordinals
        "FOURTH" | "FIFTH" | "SIXTH" | "SEVENTH" | "EIGHTH" | "NINTH" | "TENTH" |
        "ELEVENTH" | "TWELFTH" | "THIRTEENTH" | "FOURTEENTH" | "FIFTEENTH" |
        "SIXTEENTH" | "SEVENTEENTH" | "EIGHTEENTH" | "NINETEENTH" |
        "TWENTIETH" | "THIRTIETH" | "FORTIETH" | "FIFTIETH" | "HUNDREDTH"
    )
}

// ── Prefix + Person detection ─────────────────────────────────────────────────
//
// If `t` is a known person-attribute prefix/position term (господин, директор,
// профессор, mr., …) and the following token(s) parse as a person name, we
// return a list of (referent, begin, end) pairs:
//   [PersonPropertyReferent(begin=t, end=prefix_end),
//    PersonReferent(begin=person_start, end=person_end)]
//
// Multi-word prefixes up to 3 tokens are tried: "генеральный директор", etc.

fn try_prefix_person(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<Vec<(Referent, TokenRef, TokenRef)>> {
    let table = pat::get_table();

    // Get the term of the current token
    let term0 = {
        let tb = t.borrow();
        let TokenKind::Text(txt) = &tb.kind else { return None; };
        txt.term.clone()
    };

    // Try single-word, then 2-word, then 3-word prefix
    let mut prefix_end: TokenRef = t.clone();
    let mut entry: Option<&pat::PersonAttrEntry> = None;

    if let Some(e) = table.get(&term0) {
        entry = Some(e);
    } else {
        // 2-word: term0 + term1 — use a reusable buffer instead of format!()
        let t1 = t.borrow().next.clone()?;
        if !t1.borrow().is_newline_before(sofa) {
            let term1 = {
                let tb = t1.borrow();
                let TokenKind::Text(txt) = &tb.kind else { return None; };
                txt.term.clone()
            };
            let mut buf = String::with_capacity(term0.len() + 1 + term1.len() + 1 + 20);
            buf.push_str(&term0);
            buf.push(' ');
            buf.push_str(&term1);
            if let Some(e) = table.get(&buf) {
                entry = Some(e);
                prefix_end = t1.clone();
            } else {
                // 3-word: extend buffer with term2
                let t2 = t1.borrow().next.clone()?;
                if !t2.borrow().is_newline_before(sofa) {
                    let term2 = {
                        let tb = t2.borrow();
                        let TokenKind::Text(txt) = &tb.kind else { return None; };
                        txt.term.clone()
                    };
                    buf.push(' ');
                    buf.push_str(&term2);
                    if let Some(e) = table.get(&buf) {
                        entry = Some(e);
                        prefix_end = t2.clone();
                    }
                }
            }
        }
    }

    let entry = entry?;

    // Skip nationality / kin terms — they don't directly precede names
    if entry.kind == pat::PersonAttrKind::Nationality
        || entry.kind == pat::PersonAttrKind::Kin
    {
        return None;
    }

    // The next token after the prefix must not be separated by a newline
    // Also: skip a single "." immediately after an abbreviation prefix (e.g. "Mr. Smith")
    let person_start = {
        let next = prefix_end.borrow().next.clone()?;
        if next.borrow().is_newline_before(sofa) { return None; }
        // Skip lone abbreviation dot: no whitespace before, single '.' char
        let skip_dot = {
            let nb = next.borrow();
            nb.whitespaces_before_count(sofa) == 0
                && nb.length_char() == 1
                && sofa.char_at(nb.begin_char) == '.'
        };
        if skip_dot {
            let after_dot = next.borrow().next.clone()?;
            if after_dot.borrow().is_newline_before(sofa) { return None; }
            after_dot
        } else {
            next
        }
    };

    // Try to parse a person starting at person_start.
    // For EN prefix terms (MR/MRS/MS/DR etc.) also accept a single uppercase EN
    // word as a surname, since "Mr Smith" is valid even without a firstname.
    let (mut person_ref, person_end) = if let Some(res) = try_parse(&person_start, sofa) {
        res
    } else if entry.kind == pat::PersonAttrKind::Prefix {
        // Accept a single valid EN name word as a last name
        let surface = get_surface(&person_start, sofa);
        if is_valid_en_name_word(&surface) && !is_en_stop_word(&surface.to_uppercase()) {
            let mut r = pr::new_person_referent();
            pr::set_lastname(&mut r, &surface);
            if let Some(gender_male) = entry.gender {
                pr::set_sex(&mut r, if gender_male { pr::SEX_MALE } else { pr::SEX_FEMALE });
            }
            (r, person_start.clone())
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Apply gender hint from prefix if person has no sex yet
    if pr::get_sex(&person_ref).is_none() {
        if let Some(gender_male) = entry.gender {
            pr::set_sex(&mut person_ref, if gender_male { pr::SEX_MALE } else { pr::SEX_FEMALE });
        }
    }

    // Build PersonPropertyReferent
    let mut prop = ppr::new_person_property_referent();
    ppr::set_name(&mut prop, &entry.canonic);

    Some(vec![
        (prop, t.clone(), prefix_end),
        (person_ref, person_start, person_end),
    ])
}
