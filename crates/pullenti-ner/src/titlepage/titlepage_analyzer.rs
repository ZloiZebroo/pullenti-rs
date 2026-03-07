/// TitlePageAnalyzer — detects title-page information (title, authors, type, etc.)
/// from the beginning of a document.
/// Mirrors `TitlePageAnalyzer.cs` (simplified port).
///
/// This is a *specific* analyzer (is_specific = true); it must be opted in via
/// `Sdk::initialize_with` or included explicitly in the analyzer set.
///
/// ## Recognition strategy (simplified)
///
/// The analyzer scans up to ~30 lines from the start of the document looking for
/// a block that looks like a title page:
///  - Lines with uppercase / title-case text (candidate title/type)
///  - Person referents (authors, supervisors, etc.)
///  - Org referents, Geo (city) referents, Date referents
///  - Role keywords (АВТОР, НАУЧНЫЙ РУКОВОДИТЕЛЬ, РЕДАКТОР, КОНСУЛЬТАНТ, …)
///  - Type keywords (РЕФЕРАТ, ДИССЕРТАЦИЯ, ДИПЛОМ, АВТОРЕФЕРАТ, …)
///
/// The result is a TITLEPAGE referent with NAME, TYPE, AUTHOR, SUPERVISOR, etc.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::referent::{Referent, SlotValue};
use crate::source_of_analysis::SourceOfAnalysis;

use super::titlepage_referent::{
    self as tr,
    ATTR_TYPE, ATTR_AUTHOR, ATTR_SUPERVISOR, ATTR_EDITOR,
    ATTR_CONSULTANT, ATTR_OPPONENT, ATTR_TRANSLATOR, ATTR_AFFIRMANT,
    ATTR_ORG, ATTR_DATE, ATTR_CITY,
};

// ── Role types matching TitleItemToken.Types ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleType {
    Undefined,
    Worker,    // author / исполнитель
    Boss,      // scientific supervisor
    Editor,    // editor / reviewer
    Consultant,
    Opponent,
    Translate, // translator
    Adopt,     // affirmant / утверждающий
}

impl RoleType {
    fn attr_name(self) -> Option<&'static str> {
        match self {
            RoleType::Worker     => Some(ATTR_AUTHOR),
            RoleType::Boss       => Some(ATTR_SUPERVISOR),
            RoleType::Editor     => Some(ATTR_EDITOR),
            RoleType::Consultant => Some(ATTR_CONSULTANT),
            RoleType::Opponent   => Some(ATTR_OPPONENT),
            RoleType::Translate  => Some(ATTR_TRANSLATOR),
            RoleType::Adopt      => Some(ATTR_AFFIRMANT),
            RoleType::Undefined  => None,
        }
    }
}

// ── Person relation accumulator ────────────────────────────────────────────

struct PersonRel {
    person: Rc<RefCell<Referent>>,
    coefs: Vec<(RoleType, f32)>,
}

impl PersonRel {
    fn new(person: Rc<RefCell<Referent>>) -> Self {
        PersonRel { person, coefs: Vec::new() }
    }

    fn add(&mut self, typ: RoleType, coef: f32) {
        for (t, c) in &mut self.coefs {
            if *t == typ {
                *c += coef;
                return;
            }
        }
        self.coefs.push((typ, coef));
    }

    fn best(&self) -> RoleType {
        let mut best = RoleType::Undefined;
        let mut max = 0.0f32;
        for (t, c) in &self.coefs {
            if *c > max {
                best = *t;
                max = *c;
            } else if (*c - max).abs() < f32::EPSILON && *t != best {
                best = RoleType::Undefined;
            }
        }
        best
    }
}

// ── Role keyword detection ─────────────────────────────────────────────────

/// Check if a token or small token span matches a role keyword.
/// Returns (role_type, end_token) or None.
fn try_match_role(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(RoleType, TokenRef)> {
    let tb = t.borrow();

    // Single-token role keywords
    let term = match &tb.kind {
        TokenKind::Text(txt) => txt.term.as_str(),
        _ => return None,
    };

    // Two-word patterns first
    if let Some(next) = tb.next.clone() {
        let nterm = match &next.borrow().kind {
            TokenKind::Text(txt) => txt.term.clone(),
            _ => String::new(),
        };
        let pair = format!("{} {}", term, nterm);
        let role = match pair.as_str() {
            "НАУЧНЫЙ РУКОВОДИТЕЛЬ" | "НАУЧНЫЙ КОНСУЛЬТАНТ" |
            "НАУКОВИЙ КЕРІВНИК"    | "НАУКОВИЙ КОНСУЛЬТАНТ" => {
                if pair.contains("КОНСУЛЬТАНТ") { Some(RoleType::Consultant) }
                else { Some(RoleType::Boss) }
            }
            "ОФИЦИАЛЬНЫЙ ОППОНЕНТ" | "ОФІЦІЙНИЙ ОПОНЕНТ" => Some(RoleType::Opponent),
            "ОТВЕТСТВЕННЫЙ ИСПОЛНИТЕЛЬ" | "ВІДПОВІДАЛЬНИЙ ВИКОНАВЕЦЬ" => Some(RoleType::Worker),
            "РЕДАКТОРСКАЯ ГРУППА" | "РЕДАКТОРСЬКА ГРУПА" => Some(RoleType::Editor),
            _ => None,
        };
        if let Some(r) = role {
            drop(tb);
            return Some((r, next));
        }
    }

    // Single-token patterns
    let role = match term {
        "АВТОР" | "АВТОРИ" => Some(RoleType::Worker),
        "ИСПОЛНИТЕЛЬ" | "ВИКОНАВЕЦЬ" => Some(RoleType::Worker),
        "ДИПЛОМНИК" => Some(RoleType::Worker),
        "ВЫПОЛНИТЬ" | "ВИКОНАТИ" => Some(RoleType::Worker),
        "РУКОВОДИТЕЛЬ" | "КЕРІВНИК" => Some(RoleType::Boss),
        "РЕДАКТОР" | "РЕЦЕНЗЕНТ" => Some(RoleType::Editor),
        "КОНСУЛЬТАНТ" => Some(RoleType::Consultant),
        "ОППОНЕНТ" | "ОПОНЕНТ" => Some(RoleType::Opponent),
        "ПЕРЕВОДЧИК" | "ПЕРЕКЛАДАЧ" => Some(RoleType::Translate),
        "УТВЕРЖДАТЬ" | "СОГЛАСЕН" | "СТВЕРДЖУВАТИ" | "ЗГОДЕН" => Some(RoleType::Adopt),
        _ => None,
    };

    role.map(|r| {
        drop(tb);
        (r, t.clone())
    })
}

// ── Type keyword detection ─────────────────────────────────────────────────

/// Check if a token matches a document-type keyword.
/// Returns the canonical type string, or None.
fn try_match_type(t: &TokenRef) -> Option<String> {
    let tb = t.borrow();
    let term = match &tb.kind {
        TokenKind::Text(txt) => txt.term.as_str(),
        _ => return None,
    };
    let typ = match term {
        "РЕФЕРАТ"      => Some("реферат"),
        "АВТОРЕФЕРАТ"  => Some("автореферат"),
        "ДИССЕРТАЦИЯ"  => Some("диссертация"),
        "ДИСЕРТАЦІЯ"   => Some("дисертація"),
        "ДИПЛОМ"       => Some("диплом"),
        "РАБОТА"       => Some("работа"),
        "РОБОТА"       => Some("робота"),
        "ОТЧЕТ"        => Some("отчет"),
        "ЗВІТ"         => Some("звіт"),
        "ОБЗОР"        => Some("обзор"),
        "ОГЛЯД"        => Some("огляд"),
        "ПРОЕКТ"       => Some("проект"),
        "СПРАВКА"      => Some("справка"),
        "ДОВІДКА"      => Some("довідка"),
        "УЧЕБНИК"      => Some("учебник"),
        _ => None,
    };
    typ.map(|s| s.to_string())
}

// ── Line structure ─────────────────────────────────────────────────────────

/// A "logical line" — tokens on a single visual line.
struct Line {
    begin: TokenRef,
    end: TokenRef,
}

impl Line {
    fn chars_count(&self) -> i32 {
        let mut n = 0i32;
        let end_char = self.end.borrow().end_char;
        let mut cur = Some(self.begin.clone());
        while let Some(t) = cur {
            n += t.borrow().length_char();
            if t.borrow().end_char >= end_char { break; }
            cur = t.borrow().next.clone();
        }
        n
    }

    /// True if the line contains only Latin letters (no Cyrillic).
    fn is_pure_en(&self) -> bool {
        let end_char = self.end.borrow().end_char;
        let mut ru = 0i32;
        let mut en = 0i32;
        let mut cur = Some(self.begin.clone());
        while let Some(t) = cur {
            {
                let tb = t.borrow();
                if tb.chars.is_letter() {
                    if tb.chars.is_cyrillic_letter() { ru += 1; }
                    else if tb.chars.is_latin_letter() { en += 1; }
                }
                if tb.end_char >= end_char { break; }
            }
            cur = t.borrow().next.clone();
        }
        en > 0 && ru == 0
    }

    /// True if the line contains only Cyrillic letters (no Latin).
    fn is_pure_ru(&self) -> bool {
        let end_char = self.end.borrow().end_char;
        let mut ru = 0i32;
        let mut en = 0i32;
        let mut cur = Some(self.begin.clone());
        while let Some(t) = cur {
            {
                let tb = t.borrow();
                if tb.chars.is_letter() {
                    if tb.chars.is_cyrillic_letter() { ru += 1; }
                    else if tb.chars.is_latin_letter() { en += 1; }
                }
                if tb.end_char >= end_char { break; }
            }
            cur = t.borrow().next.clone();
        }
        ru > 0 && en == 0
    }
}

/// Parse up to max_lines logical lines from `t0`, stopping at max_chars total
/// characters or max_end_char position.
fn parse_lines(t0: &TokenRef, max_lines: usize, max_chars: i32, max_end_char: i32, sofa: &SourceOfAnalysis) -> Vec<Line> {
    let mut res: Vec<Line> = Vec::new();
    let mut total_chars = 0i32;
    let mut cur = Some(t0.clone());

    while let Some(t) = cur.clone() {
        {
            let tb = t.borrow();
            if max_end_char > 0 && tb.begin_char > max_end_char { break; }
        }

        // Find end of this visual line: advance until newline-after or end of chain
        let mut t1 = t.clone();
        loop {
            let t1_next = t1.borrow().next.clone();
            match t1_next {
                None => break,
                Some(nx) => {
                    // Stop at newline after t1
                    if t1.borrow().is_newline_after(sofa) {
                        // Check whether next token starts a new sentence
                        // (simplified: any newline after suffices)
                        break;
                    }
                    t1 = nx;
                }
            }
        }

        // Stop at "СОДЕРЖАНИЕ" / "ОГЛАВЛЕНИЕ" (start of table of contents)
        if is_toc_start(&t, sofa) { break; }

        // Stop if this line begins with "КЛЮЧЕВЫЕ СЛОВА" / "KEYWORDS"
        if is_keywords_line(&t) { break; }

        let line = Line { begin: t.clone(), end: t1.clone() };
        let lc = line.chars_count();
        res.push(line);
        total_chars += lc;

        if res.len() >= max_lines || total_chars >= max_chars { break; }

        // Advance past the line
        let after = t1.borrow().next.clone();
        cur = after;
    }
    res
}

fn is_toc_start(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    if !t.borrow().is_newline_before(sofa) { return false; }
    t.borrow().is_value("СОДЕРЖАНИЕ", Some("ЗМІСТ")) ||
    t.borrow().is_value("ОГЛАВЛЕНИЕ", None) ||
    t.borrow().is_value("СОДЕРЖИМОЕ", None)
}

fn is_keywords_line(t: &TokenRef) -> bool {
    if let TokenKind::Text(ref txt) = t.borrow().kind {
        matches!(txt.term.as_str(), "КЛЮЧЕВЫЕ" | "KEYWORDS" | "КЛЮЧОВІ")
    } else {
        false
    }
}

// ── Title scoring ──────────────────────────────────────────────────────────

/// Extract the best candidate title text from a span of tokens.
/// Returns (name_text, type_value) or None if no good candidate found.
fn try_extract_title(begin: &TokenRef, end: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, Option<String>)> {
    // Skip purely-lowercase start (not a title)
    if begin.borrow().chars.is_all_lower() { return None; }

    let mut words = 0i32;
    let mut up_words = 0i32;
    let mut notwords = 0i32;
    let mut rank: i32 = 0;
    let mut type_value: Option<String> = None;
    let mut name_start: Option<TokenRef> = None;
    let mut line_count = 0i32;

    let end_char = end.borrow().end_char;
    let mut cur = Some(begin.clone());

    // Check for leading type keyword on its own line
    if let Some(tv) = try_match_type(begin) {
        let nb = begin.borrow().next.clone();
        if nb.as_ref().map_or(false, |nx| nx.borrow().is_newline_before(sofa)) ||
           begin.borrow().is_newline_after(sofa) {
            type_value = Some(tv);
            rank += 5;
            name_start = begin.borrow().next.clone();
            cur = name_start.clone();
        }
    }

    if name_start.is_none() {
        name_start = Some(begin.clone());
    }

    while let Some(t) = cur.clone() {
        if t.borrow().end_char > end_char { break; }

        // Role keywords degrade rank
        if let Some((_, role_end)) = try_match_role(&t, sofa) {
            let re_end_char = role_end.borrow().end_char;
            rank -= 4;
            // Skip past role keyword
            let mut skip_cur = Some(t.clone());
            while let Some(sc) = skip_cur.clone() {
                let sc_end = sc.borrow().end_char;
                skip_cur = sc.borrow().next.clone();
                if sc_end >= re_end_char { break; }
            }
            cur = skip_cur;
            continue;
        }

        // Type keywords (not at start) — moderate penalty
        if let Some(tv) = try_match_type(&t) {
            if type_value.is_none() {
                type_value = Some(tv);
            }
            rank -= 2;
            cur = t.borrow().next.clone();
            continue;
        }

        let tb = t.borrow();
        let is_newline = tb.is_newline_before(sofa) && !Rc::ptr_eq(&t, begin);

        if is_newline {
            line_count += 1;
            if line_count > 4 { break; }
            // Continuation lines in lowercase are good (sentence wrapped)
            if tb.chars.is_all_lower() { rank += 10; }
        }

        // Referent tokens — penalize GEO/PERSON on their own lines
        if let TokenKind::Referent(ref rd) = tb.kind {
            let rtype = rd.referent.borrow().type_name.clone();
            if is_newline && (rtype == "GEO" || rtype == "PERSON") {
                rank -= 10;
            }
            if rtype == "PHONE" || rtype == "URI" {
                return None; // URI/phone → definitely not a title
            }
            words += 1;
            if tb.chars.is_all_upper() { up_words += 1; }
            drop(tb);
            cur = t.borrow().next.clone();
            continue;
        }

        if let TokenKind::Text(ref txt) = tb.kind {
            if txt.term == "©" { rank -= 10; }
            if tb.chars.is_letter() && tb.length_char() > 2 {
                words += 1;
                if tb.chars.is_all_upper() { up_words += 1; }
            } else if tb.is_char_of("._", sofa) {
                rank -= 5;
            } else if !tb.is_char(',', sofa) {
                notwords += 1;
            }
            // Pure verb in lowercase → stop
            let mc = tb.get_morph_class_in_dictionary();
            if mc.is_verb() && tb.chars.is_all_lower() && !mc.is_noun() {
                rank -= 30;
                break;
            }
        }

        drop(tb);
        cur = t.borrow().next.clone();
    }

    rank += words;
    rank -= notwords;

    if rank < 1 || words < 1 { return None; }

    // Build name text from name_start..end
    let ns = name_start?;
    let name_text = collect_text(&ns, end, sofa);
    if name_text.trim().is_empty() { return None; }

    Some((name_text, type_value))
}

/// Collect the surface text for a token span (begin..=end), preserving spaces.
fn collect_text(begin: &TokenRef, end: &TokenRef, sofa: &SourceOfAnalysis) -> String {
    let end_char = end.borrow().end_char;
    let mut parts: Vec<String> = Vec::new();
    let mut cur = Some(begin.clone());

    while let Some(t) = cur.clone() {
        let tb = t.borrow();
        let bc = tb.begin_char;
        let ec = tb.end_char;

        if bc > end_char { break; }

        let text = sofa.substring(bc, ec);
        if !text.is_empty() {
            parts.push(text.to_string());
        }

        if ec >= end_char { break; }
        drop(tb);
        cur = t.borrow().next.clone();
    }

    parts.join(" ").trim().to_string()
}

// ── Analyzer ───────────────────────────────────────────────────────────────

pub struct TitlePageAnalyzer;

impl TitlePageAnalyzer {
    pub fn new() -> Self { TitlePageAnalyzer }
}

impl Default for TitlePageAnalyzer {
    fn default() -> Self { TitlePageAnalyzer }
}

impl Analyzer for TitlePageAnalyzer {
    fn name(&self)        -> &'static str { "TITLEPAGE" }
    fn caption(&self)     -> &'static str { "Титульный лист" }
    fn is_specific(&self) -> bool         { true }
    fn progress_weight(&self) -> i32      { 1 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();

        let first = match kit.first_token.clone() {
            Some(t) => t,
            None => return,
        };

        // Parse up to 30 lines / 1500 chars from the document start
        let lines = parse_lines(&first, 30, 1500, 0, &sofa);
        if lines.is_empty() { return; }

        // Find candidate line span for title (stop at long prose)
        let cou = lines.len();

        // Try to find a good title span across consecutive lines
        let mut best_name: Option<String> = None;
        let mut best_type: Option<String> = None;
        let mut best_begin: Option<TokenRef> = None;
        let mut best_end: Option<TokenRef> = None;

        'outer: for i in 0..cou {
            // Check for pure language switches (EN→RU or RU→EN break span)
            for j in i..cou.min(i + 5) {
                if j > i {
                    if lines[j-1].is_pure_en() && lines[j].is_pure_ru() { break; }
                    if lines[j-1].is_pure_ru() && lines[j].is_pure_en() { break; }
                }
                let span_begin = lines[i].begin.clone();
                let span_end   = lines[j].end.clone();
                if let Some((name, tv)) = try_extract_title(&span_begin, &span_end, &sofa) {
                    // Prefer longer name (more lines)
                    let better = match &best_name {
                        None => true,
                        Some(prev) => name.len() > prev.len() || (j > i && tv.is_some()),
                    };
                    if better {
                        best_name  = Some(name);
                        best_type  = tv;
                        best_begin = Some(span_begin);
                        best_end   = Some(span_end);
                    }
                    break;
                }
            }
        }

        let end_char = if cou > 0 { lines[cou - 1].end.borrow().end_char } else { 0 };

        // Build referent
        let mut res = tr::new_titlepage_referent();

        if let (Some(name), Some(nb), Some(ne)) = (best_name.clone(), best_begin.clone(), best_end.clone()) {
            // Trim trailing period
            let name = if name.ends_with('.') && !name.ends_with("..") {
                name[..name.len()-1].trim().to_string()
            } else { name };
            tr::add_name(&mut res, &name);
            if let Some(tv) = best_type.clone() {
                tr::add_title_type(&mut res, &tv);
            }
        }

        // ── Scan for persons, orgs, dates, cities ────────────────────────────
        let mut person_rels: Vec<PersonRel> = Vec::new();
        let mut cur_role = RoleType::Undefined;
        let mut found_date = false;
        let mut found_city = false;
        let mut found_org  = false;

        // We also track the farthest end token we encounter
        let mut end_token_tracker: Option<TokenRef> = best_end.clone().or_else(|| kit.first_token.clone());

        let mut cur = Some(first.clone());
        while let Some(t) = cur.clone() {
            {
                let tb = t.borrow();
                if tb.begin_char > end_char { break; }
            }

            // Role keyword detection
            if let Some((role, role_end)) = try_match_role(&t, &sofa) {
                cur_role = role;
                // Track end
                {
                    let re_end_char = role_end.borrow().end_char;
                    if let Some(ref et) = end_token_tracker {
                        if re_end_char > et.borrow().end_char {
                            end_token_tracker = Some(role_end.clone());
                        }
                    } else {
                        end_token_tracker = Some(role_end.clone());
                    }
                }
                // Skip to after role keyword (and optional ":-")
                let after_role = role_end.borrow().next.clone();
                let skip = match after_role {
                    Some(ref s) if s.borrow().is_char_of(":-", &sofa) => {
                        s.borrow().next.clone()
                    }
                    other => other,
                };
                cur = skip;
                continue;
            }

            // Type keyword detection
            if let Some(tv) = try_match_type(&t) {
                // Only set type if not already set
                if res.get_string_value(ATTR_TYPE).is_none() {
                    tr::add_title_type(&mut res, &tv);
                }
                cur_role = RoleType::Undefined;
                cur = t.borrow().next.clone();
                continue;
            }

            // Referent tokens
            let r_opt = {
                let tb = t.borrow();
                match &tb.kind {
                    TokenKind::Referent(rd) => Some(rd.referent.clone()),
                    _ => None,
                }
            };

            if let Some(r_rc) = r_opt {
                let rtype = r_rc.borrow().type_name.clone();

                // Track end pos
                {
                    let te = t.borrow().end_char;
                    if let Some(ref et) = end_token_tracker {
                        if te > et.borrow().end_char {
                            end_token_tracker = Some(t.clone());
                        }
                    }
                }

                match rtype.as_str() {
                    "PERSON" => {
                        // Check ATTR_ATTR slots for role hints
                        let inferred_role = infer_role_from_person_attrs(&r_rc);
                        let effective_role = if inferred_role != RoleType::Undefined {
                            inferred_role
                        } else {
                            cur_role
                        };

                        // Find or create PersonRel entry
                        let ptr_match = person_rels.iter().position(|pr| Rc::ptr_eq(&pr.person, &r_rc));
                        match ptr_match {
                            Some(idx) => { person_rels[idx].add(effective_role, 1.0); }
                            None => {
                                let mut pr = PersonRel::new(r_rc.clone());
                                pr.add(effective_role, if effective_role == RoleType::Undefined { 0.5 } else { 1.0 });
                                person_rels.push(pr);
                            }
                        }
                    }
                    "DATE" | "DATERANGE" => {
                        if !found_date {
                            found_date = true;
                            res.add_slot(ATTR_DATE, SlotValue::Referent(r_rc.clone()), true);
                        }
                        cur_role = RoleType::Undefined;
                    }
                    "GEO" => {
                        // Only cities
                        let is_city = r_rc.borrow().get_string_value("TYPE")
                            .map(|v| v.to_lowercase().contains("city") || v.to_lowercase().contains("город"))
                            .unwrap_or(false);
                        // Heuristic: if GEO has no "TYPE" slot we still accept it as city candidate
                        let is_city2 = r_rc.borrow().find_slot("HIGHER", None).is_none();
                        if !found_city && (is_city || is_city2) {
                            found_city = true;
                            res.add_slot(ATTR_CITY, SlotValue::Referent(r_rc.clone()), true);
                        }
                        cur_role = RoleType::Undefined;
                    }
                    "ORGANIZATION" => {
                        if !found_org {
                            found_org = true;
                            res.add_slot(ATTR_ORG, SlotValue::Referent(r_rc.clone()), true);
                        }
                        cur_role = RoleType::Undefined;
                    }
                    _ => {
                        cur_role = RoleType::Undefined;
                    }
                }
            }

            cur = t.borrow().next.clone();
        }

        // Assign persons to their best roles
        let role_order = [
            RoleType::Worker, RoleType::Boss, RoleType::Editor,
            RoleType::Consultant, RoleType::Opponent, RoleType::Translate, RoleType::Adopt,
        ];

        for role in &role_order {
            for pr in &person_rels {
                if pr.best() == *role {
                    if let Some(attr) = role.attr_name() {
                        res.add_slot(attr, SlotValue::Referent(pr.person.clone()), false);
                    }
                }
            }
        }

        // If no explicit author assigned, pick the first Undefined-best person as author
        if res.find_slot(ATTR_AUTHOR, None).is_none() {
            for pr in &person_rels {
                if pr.best() == RoleType::Undefined {
                    res.add_slot(ATTR_AUTHOR, SlotValue::Referent(pr.person.clone()), false);
                    break;
                }
            }
        }

        // Only register if we have at least one slot
        if res.slots.is_empty() { return; }

        let r_rc = Rc::new(RefCell::new(res));
        kit.add_entity(r_rc.clone());

        // Embed as a referent token covering the best name span (if found)
        if let (Some(nb), Some(ne)) = (best_begin, best_end) {
            let tok = Rc::new(RefCell::new(
                Token::new_referent(nb, ne, r_rc.clone())
            ));
            kit.embed_token(tok);
        }
    }
}

// ── Helper: infer role from PersonReferent ATTR_ATTR slots ────────────────

fn infer_role_from_person_attrs(person: &Rc<RefCell<Referent>>) -> RoleType {
    let pb = person.borrow();
    for slot in &pb.slots {
        if slot.type_name == "ATTR" {
            if let Some(ref v) = slot.value {
                let s = v.to_string().to_lowercase();
                if s.contains("руководител") || s.contains("керівник") { return RoleType::Boss; }
                if s.contains("студент") || s.contains("слушател") || s.contains("студент") { return RoleType::Worker; }
                if s.contains("редактор") || s.contains("рецензент") { return RoleType::Editor; }
                if s.contains("консультант") { return RoleType::Consultant; }
                if s.contains("исполнитель") || s.contains("виконавець") { return RoleType::Worker; }
            }
        }
    }
    RoleType::Undefined
}
