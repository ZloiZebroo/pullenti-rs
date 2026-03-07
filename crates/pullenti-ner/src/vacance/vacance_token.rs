/// VacanceToken — internal token type for vacancy parsing.
/// Mirrors `VacanceToken.cs` and `VacanceTokenType.cs`.

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

use crate::token::{TokenRef, TokenKind};
use crate::referent::Referent;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::termin::{Termin, TerminCollection, TerminToken};
use crate::core::misc_helper::can_be_start_of_sentence;
use crate::core::noun_phrase::{NounPhraseParseAttr, try_parse as npt_try_parse};

// ── VacanceTokenType ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VacanceTokenType {
    #[default]
    Undefined,
    Dummy,
    Stop,
    Expired,
    Name,
    Date,
    Skill,
    Plus,
    Experience,
    Education,
    Money,
    Language,
    Moral,
    Driving,
    License,
}

impl VacanceTokenType {
    fn is_skill(self) -> bool {
        matches!(
            self,
            VacanceTokenType::Experience
            | VacanceTokenType::Education
            | VacanceTokenType::Skill
            | VacanceTokenType::Language
            | VacanceTokenType::Plus
            | VacanceTokenType::Moral
            | VacanceTokenType::License
            | VacanceTokenType::Driving
        )
    }
}

// ── Term data ─────────────────────────────────────────────────────────────

struct VacanceData {
    termins: TerminCollection,
}

static VACANCE_DATA: OnceLock<VacanceData> = OnceLock::new();

fn data() -> &'static VacanceData {
    VACANCE_DATA.get_or_init(|| {
        let mut tc = TerminCollection::new();

        fn add(tc: &mut TerminCollection, text: &str, typ: VacanceTokenType, tag2: bool) {
            let mut t = Termin::new(text);
            t.tag = Some(Arc::new(typ));
            if tag2 {
                t.tag2 = Some(Arc::new(true as bool));
            }
            tc.add(t);
        }

        fn add_variants(tc: &mut TerminCollection, primary: &str, variants: &[&str], typ: VacanceTokenType, tag2: bool) {
            let mut t = Termin::new(primary);
            t.tag = Some(Arc::new(typ));
            if tag2 {
                t.tag2 = Some(Arc::new(true as bool));
            }
            for v in variants {
                t.add_variant(v);
            }
            tc.add(t);
        }

        // Salary
        {
            let mut t = Termin::new("ЗАРАБОТНАЯ ПЛАТА");
            t.tag = Some(Arc::new(VacanceTokenType::Money));
            t.add_abridge("З/П");
            tc.add(t);
        }

        // Experience
        add_variants(&mut tc, "ОПЫТ РАБОТЫ",
            &["СТАЖ РАБОТЫ", "РАБОЧИЙ СТАЖ"],
            VacanceTokenType::Experience, false);

        // Education
        add(&mut tc, "ОБРАЗОВАНИЕ", VacanceTokenType::Education, false);

        // Languages
        for s in &["АНГЛИЙСКИЙ", "НЕМЕЦКИЙ", "ФРАНЦУЗСКИЙ", "ИТАЛЬЯНСКИЙ", "ИСПАНСКИЙ", "КИТАЙСКИЙ"] {
            add(&mut tc, s, VacanceTokenType::Language, false);
        }

        // Driving
        for s in &["ВОДИТЕЛЬСКИЕ ПРАВА", "ПРАВА КАТЕГОРИИ", "ВОДИТЕЛЬСКОЕ УДОСТОВЕРЕНИЕ",
                   "УДОСТОВЕРЕНИЕ ВОДИТЕЛЯ", "ПРАВА ВОДИТЕЛЯ"] {
            add(&mut tc, s, VacanceTokenType::Driving, false);
        }

        // License / certificates
        for s in &["УДОСТОВЕРЕНИЕ", "ВОДИТЕЛЬСКАЯ МЕДСПРАВКА", "ВОДИТЕЛЬСКАЯ МЕД.СПРАВКА",
                   "ВОЕННЫЙ БИЛЕТ", "МЕДИЦИНСКАЯ КНИЖКА", "МЕДКНИЖКА", "МЕД.КНИЖКА",
                   "АТТЕСТАТ", "АТТЕСТАЦИЯ", "СЕРТИФИКАТ", "ДОПУСК", "ГРУППА ДОПУСКА"] {
            add(&mut tc, s, VacanceTokenType::License, false);
        }

        // Moral qualities
        let moral_entries: &[(&str, &[&str])] = &[
            ("ЖЕЛАНИЕ", &["ЖЕЛАТЬ"]),
            ("ЖЕЛАНИЕ И СПОСОБНОСТЬ", &[]),
            ("ГОТОВНОСТЬ К", &["ГОТОВЫЙ К"]),
            ("ДОБРОСОВЕСТНОСТЬ", &["ДОБРОСОВЕСТНЫЙ"]),
            ("ГИБКОСТЬ", &[]),
            ("РАБОТА В КОМАНДЕ", &["УМЕНИЕ РАБОТАТЬ В КОМАНДЕ"]),
            ("ОБЩИТЕЛЬНОСТЬ", &["ОБЩИТЕЛЬНЫЙ", "УМЕНИЕ ОБЩАТЬСЯ С ЛЮДЬМИ", "УМЕНИЕ ОБЩАТЬСЯ", "КОНТАКТ С ЛЮДЬМИ"]),
            ("ОТВЕТСТВЕННОСТЬ", &["ОТВЕТСТВЕННЫЙ"]),
            ("АКТИВНАЯ ЖИЗНЕННАЯ ПОЗИЦИЯ", &[]),
            ("КОММУНИКАБЕЛЬНОСТЬ", &["КОММУНИКАБЕЛЬНЫЙ"]),
            ("ЛОЯЛЬНОСТЬ", &["ЛОЯЛЬНЫЙ"]),
            ("ИСПОЛНИТЕЛЬНОСТЬ", &["ИСПОЛНИТЕЛЬНЫЙ"]),
            ("РЕЗУЛЬТАТИВНОСТЬ", &["РЕЗУЛЬТАТИВНЫЙ"]),
            ("ПУНКТУАЛЬНОСТЬ", &["ПУНКТУАЛЬНЫЙ"]),
            ("ДИСЦИПЛИНИРОВАННОСТЬ", &["ДИСЦИПЛИНИРОВАННЫЙ"]),
            ("ТРУДОЛЮБИЕ", &["ТРУДОЛЮБИВЫЙ"]),
            ("ЦЕЛЕУСТРЕМЛЕННОСТЬ", &["ЦЕЛЕУСТРЕМЛЕННЫЙ"]),
            ("РАБОТОСПОСОБНОСТЬ", &["РАБОТОСПОСОБНЫЙ"]),
            ("ОПРЯТНОСТЬ", &["ОПРЯТНЫЙ"]),
            ("ВЕЖЛИВОСТЬ", &["ВЕЖЛИВЫЙ"]),
            ("ВЫНОСЛИВОСТЬ", &["ВЫНОСЛИВЫЙ"]),
            ("АКТИВНОСТЬ", &["АКТИВНЫЙ"]),
            ("УСИДЧИВОСТЬ", &["УСИДЧИВЫЙ"]),
            ("НАХОДЧИВОСТЬ", &["НАХОДЧИВЫЙ"]),
            ("ОПТИМИЗМ", &["ОПТИМИСТИЧНЫЙ"]),
            ("СТРЕМЛЕНИЕ ПОЗНАТЬ НОВОЕ", &["СТРЕМИТЬСЯ ПОЗНАТЬ НОВОЕ", "СТРЕМЛЕНИЕ УЗНАТЬ НОВОЕ"]),
            ("ОБУЧАЕМОСТЬ", &["ОБУЧАЕМЫЙ", "СПОСОБНОСТЬ К ОБУЧЕНИЮ", "ЛЕГКО ОБУЧАЕМЫЙ", "ЛЕГКООБУЧАЕМЫЙ", "БЫСТРО ОБУЧАТЬСЯ"]),
            ("ОБРАЗОВАННОСТЬ", &[]),
            ("СТРЕССОУСТОЙЧИВОСТЬ", &["СТРЕССОУСТОЙЧИВЫЙ"]),
            ("ОТЛИЧНОЕ НАСТРОЕНИЕ", &[]),
            ("ХОРОШЕЕ НАСТРОЕНИЕ", &[]),
            ("ГРАМОТНАЯ РЕЧЬ", &[]),
            ("ГРАМОТНОЕ ПИСЬМО", &[]),
            ("ГРАМОТНОЕ ПИСЬМО И РЕЧЬ", &[]),
            ("НАЦЕЛЕННОСТЬ НА РЕЗУЛЬТАТ", &["НАЦЕЛЕННЫЙ НА РЕЗУЛЬТАТ"]),
            ("ПРИВЕТЛИВОСТЬ", &["ПРИВЕТЛИВЫЙ"]),
            ("ЖЕЛАНИЕ РАБОТАТЬ", &["ЖЕЛАТЬ РАБОТАТЬ"]),
            ("ЖЕЛАНИЕ ЗАРАБАТЫВАТЬ", &["ЖЕЛАТЬ ЗАРАБАТЫВАТЬ"]),
            ("ОБЯЗАТЕЛЬНОСТЬ", &[]),
            ("ГРАМОТНОСТЬ", &[]),
            ("ИНИЦИАТИВНОСТЬ", &["ИНИЦИАТИВНЫЙ"]),
            ("ОРГАНИЗОВАННОСТЬ", &[]),
            ("АККУРАТНОСТЬ", &["АККУРАТНЫЙ"]),
            ("ВНИМАТЕЛЬНОСТЬ", &["ВНИМАТЕЛЬНЫЙ"]),
            ("БЕЗ ВРЕДНЫХ ПРИВЫЧЕК", &["ОТСУТСТВИЕ ВРЕДНЫХ ПРИВЫЧЕК", "ВРЕДНЫЕ ПРИВЫЧКИ ОТСУТСТВУЮТ"]),
        ];
        for (primary, variants) in moral_entries {
            add_variants(&mut tc, primary, variants, VacanceTokenType::Moral, false);
        }

        // Skill keywords
        for s in &["ОПЫТ", "ЗНАНИЕ", "ВЛАДЕНИЕ", "НАВЫК", "УМЕНИЕ", "ПОНИМАНИЕ",
                   "ОРГАНИЗАТОРСКИЕ НАВЫКИ", "ОРГАНИЗАТОРСКИЕ СПОСОБНОСТИ", "ПОЛЬЗОВАТЕЛЬ ПК"] {
            add(&mut tc, s, VacanceTokenType::Skill, false);
        }

        // Required-marker keywords (tag2 = true)
        for s in &["НУЖНО", "НЕОБХОДИМО", "ТРЕБОВАТЬСЯ", "НАЛИЧИЕ",
                   "ДЛЯ РАБОТЫ ТРЕБУЕТСЯ", "ОБЯЗАТЕЛЬНО", "ОБЯЗАТЕЛЕН"] {
            add(&mut tc, s, VacanceTokenType::Skill, true);
        }

        // "Nice to have" keywords
        for s in &["ЖЕЛАТЕЛЬНО", "ПРИВЕТСТВОВАТЬСЯ", "ЯВЛЯТЬСЯ ПРЕИМУЩЕСТВОМ",
                   "КАК ПЛЮС", "БУДЕТ ПРЕИМУЩЕСТВОМ", "БУДЕТ ЯВЛЯТЬСЯ ПРЕИМУЩЕСТВОМ", "МЫ ЦЕНИМ"] {
            add(&mut tc, s, VacanceTokenType::Plus, true);
        }

        // Dummy / noise keywords
        for s in &["НЕЗАМЕНИМЫЙ ОПЫТ", "ОСТАВИТЬ ОТЗЫВ", "КЛЮЧЕВЫЕ НАВЫКИ",
                   "ПОЛНАЯ ЗАНЯТОСТЬ", "КОРПОРАТИВНЫЕ ЗАНЯТИЯ", "КОМПЕНСАЦИЯ",
                   "ОПЛАТА БОЛЬНИЧНЫХ", "ПРЕМИЯ", "ВОЗМОЖНОСТЬ ПОЛУЧИТЬ",
                   "УСЛОВИЯ ДЛЯ", "СПЕЦИАЛЬНЫЕ НАВЫКИ И ЗНАНИЯ", "ПРОГРАММА ЛОЯЛЬНОСТИ",
                   "СИСТЕМА ЛОЯЛЬНОСТИ", "КОРПОРАТИВНЫЙ", "ИНТЕРЕСНАЯ РАБОТА",
                   "НА ПОСТОЯННУЮ РАБОТУ", "ПРОФСОЮЗ"] {
            add(&mut tc, s, VacanceTokenType::Dummy, false);
        }

        // Expired vacancy markers
        for s in &["ВАКАНСИЯ В АРХИВЕ", "В АРХИВЕ С"] {
            add(&mut tc, s, VacanceTokenType::Expired, false);
        }

        VacanceData { termins: tc }
    })
}

fn term_typ(tok: &TerminToken) -> VacanceTokenType {
    tok.termin.tag.as_ref()
        .and_then(|t| t.downcast_ref::<VacanceTokenType>())
        .copied()
        .unwrap_or(VacanceTokenType::Undefined)
}

fn term_has_tag2(tok: &TerminToken) -> bool {
    tok.termin.tag2.as_ref()
        .and_then(|t| t.downcast_ref::<bool>())
        .copied()
        .unwrap_or(false)
}

// ── VacanceToken ──────────────────────────────────────────────────────────

pub struct VacanceToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub typ:  VacanceTokenType,
    /// Referent objects collected in this span
    pub refs: Vec<Rc<RefCell<Referent>>>,
    pub value:  Option<String>,
}

impl VacanceToken {
    fn new(begin: TokenRef, end: TokenRef) -> Self {
        VacanceToken {
            begin_token: begin,
            end_token:   end,
            typ:  VacanceTokenType::Undefined,
            refs: Vec::new(),
            value: None,
        }
    }

    fn length_char(&self) -> i32 {
        self.end_token.borrow().end_char - self.begin_token.borrow().begin_char
    }

    fn is_skill(&self) -> bool {
        self.typ.is_skill()
    }

    // ── TryParseList ─────────────────────────────────────────────────────

    pub fn try_parse_list(first_token: &TokenRef, sofa: &SourceOfAnalysis) -> Option<Vec<VacanceToken>> {
        let mut res: Vec<VacanceToken> = Vec::new();
        let mut cur = Some(first_token.clone());

        while let Some(t) = cur {
            let adjacent = if res.is_empty() {
                false
            } else {
                let last = &res[res.len() - 1];
                let next = last.end_token.borrow().next.clone();
                next.as_ref().map_or(false, |n| Rc::ptr_eq(n, &t))
            };
            let prev_idx = if adjacent && !res.is_empty() { Some(res.len() - 1) } else { None };

            let vv_opt = if let Some(pi) = prev_idx {
                Self::try_parse_inner(&t, Some(&res[pi]), sofa)
            } else {
                Self::try_parse_inner(&t, None, sofa)
            };

            let vv = match vv_opt { Some(v) => v, None => break };
            let next = vv.end_token.borrow().next.clone();
            if vv.length_char() > 3 {
                res.push(vv);
            }
            cur = next;
        }

        if res.is_empty() {
            return None;
        }

        // Post-process pass 1: determine Name item
        let mut i = 0;
        while i < res.len() {
            let typ = res[i].typ;
            if typ == VacanceTokenType::Date {
                res[i].typ = VacanceTokenType::Undefined;
                i += 1;
                continue;
            }
            if typ == VacanceTokenType::Dummy { i += 1; continue; }
            if typ == VacanceTokenType::Undefined && !res[i].refs.is_empty() {
                let is_uri = res[i].refs[0].borrow().type_name == "URI";
                if is_uri { i += 1; continue; }
            }
            if typ == VacanceTokenType::Skill
                && i + 1 < res.len()
                && res[i + 1].typ == VacanceTokenType::Money
            {
                res[i].typ = VacanceTokenType::Undefined;
                i += 1;
                continue;
            }
            if typ == VacanceTokenType::Expired { i += 1; continue; }
            if typ != VacanceTokenType::Undefined { break; }
            // First meaningful undefined → Name
            res[i].typ = VacanceTokenType::Name;
            if i + 2 < res.len() {
                let next_typ = res[i + 1].typ;
                if (next_typ == VacanceTokenType::Undefined || next_typ == VacanceTokenType::Skill)
                    && res[i + 2].typ == VacanceTokenType::Money
                {
                    let new_end = res[i + 1].end_token.clone();
                    res[i].end_token = new_end;
                    res.remove(i + 1);
                }
            }
            res[i].get_value(sofa);
            break;
        }

        // Post-process pass 2: propagate type for undefined items between skills
        let mut i = 1;
        while i < res.len() {
            if res[i].typ == VacanceTokenType::Undefined && i > 0 && res[i - 1].is_skill() {
                for j in (i + 1)..(i + 2).min(res.len()) {
                    if res[j].is_skill() {
                        let nt = res[j].typ;
                        res[i].typ = if nt == VacanceTokenType::Plus || nt == VacanceTokenType::Moral {
                            nt
                        } else {
                            VacanceTokenType::Skill
                        };
                        break;
                    }
                }
            }
            i += 1;
        }

        // Post-process pass 3: extract values and merge consecutive same-type skills
        let mut i = 0;
        while i < res.len() {
            if res[i].is_skill() && res[i].value.is_none() {
                res[i].get_value(sofa);
            }
            let typ = res[i].typ;
            if typ == VacanceTokenType::Skill
                || typ == VacanceTokenType::Moral
                || typ == VacanceTokenType::Plus
            {
                while i + 1 < res.len() && res[i + 1].typ == typ {
                    let new_end = res[i + 1].end_token.clone();
                    res[i].end_token = new_end;
                    res.remove(i + 1);
                }
            }
            i += 1;
        }

        Some(res)
    }

    // ── TryParse ──────────────────────────────────────────────────────────

    fn try_parse_inner(t: &TokenRef, prev: Option<&VacanceToken>, sofa: &SourceOfAnalysis) -> Option<VacanceToken> {
        let d = data();
        let mut res = VacanceToken::new(t.clone(), t.clone());
        let begin_char = t.borrow().begin_char;
        let mut skills = 0i32;
        let mut dummy  = 0i32;
        let mut lang   = 0i32;
        let mut edu    = 0i32;
        let mut moral  = 0i32;
        let mut lic    = 0i32;
        let mut plus   = 0i32;

        let mut cur = Some(t.clone());
        while let Some(tt) = cur.take() {
            let tt_begin = tt.borrow().begin_char;

            // Check newline boundary (only for tokens after the first)
            if tt_begin != begin_char && tt.borrow().is_newline_before(sofa) {
                if can_be_start_of_sentence(&tt, sofa) { break; }
                if tt.borrow().is_hiphen(sofa) { break; }
                // Check if previous token continues a noun phrase
                let prev_tok_opt: Option<TokenRef> = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                let continues_np = if let Some(ref pt) = prev_tok_opt {
                    let npt = npt_try_parse(pt, NounPhraseParseAttr::No, 0, sofa);
                    npt.as_ref().map_or(false, |n| n.end_token.borrow().end_char >= tt_begin)
                } else { false };
                if !continues_np {
                    let prev_is_noun = prev_tok_opt.as_ref()
                        .map_or(false, |p| p.borrow().get_morph_class_in_dictionary().is_noun());
                    let tt_is_lower = tt.borrow().chars.is_all_lower();
                    if prev_is_noun && tt_is_lower {
                        let mut npt2 = npt_try_parse(&tt, NounPhraseParseAttr::No, 0, sofa);
                        let is_gen = npt2.as_mut().map_or(false, |n| {
                            n.morph.case().is_genitive() && !n.morph.case().is_nominative()
                        });
                        if !is_gen { break; }
                    } else {
                        break;
                    }
                }
            }

            // Semicolon ends the span
            if tt.borrow().is_char(';', sofa) { break; }

            res.end_token = tt.clone();

            // Try termin match
            if let Some(tok) = d.termins.try_parse(&tt) {
                let ty      = term_typ(&tok);
                let has_t2  = term_has_tag2(&tok);
                let tok_end = tok.end_token.clone();

                if ty == VacanceTokenType::Stop && Rc::ptr_eq(&tt, t) {
                    return None;
                }
                res.end_token = tok_end.clone();
                let cur_next = tok_end.borrow().next.clone();

                match ty {
                    VacanceTokenType::Expired => {
                        res.typ = VacanceTokenType::Expired;
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Dummy => {
                        dummy += 1;
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Education => {
                        edu += 1;
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Language => {
                        lang += 1;
                        // Discount if this looks like a teacher context
                        let mut scan: Option<TokenRef> = tt.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        while let Some(p) = scan.take() {
                            if p.borrow().begin_char < begin_char { break; }
                            if p.borrow().is_value("ПЕДАГОГ", None)
                                || p.borrow().is_value("УЧИТЕЛЬ", None)
                                || p.borrow().is_value("РЕПЕТИТОР", None)
                                || p.borrow().is_value("ПРЕПОДАВАТЕЛЬ", None)
                            {
                                lang -= 1;
                                break;
                            }
                            scan = p.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        }
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Moral => {
                        moral += 1;
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Plus => {
                        plus += 1;
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::License => {
                        lic += 1;
                        // Discount if preceded by "оформить/оформление"
                        let prev_prev: Option<TokenRef> = tok.end_token.borrow().prev
                            .as_ref().and_then(|w| w.upgrade())
                            .and_then(|p| p.borrow().prev.as_ref().and_then(|w| w.upgrade()));
                        if let Some(pp) = prev_prev {
                            if pp.borrow().is_value("ОФОРМЛЯТЬ", None)
                                || pp.borrow().is_value("ОФОРМИТЬ", None)
                                || pp.borrow().is_value("ОФОРМЛЕНИЕ", None)
                            {
                                lic -= 1;
                            }
                        }
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Skill => {
                        if has_t2 && (tt_begin - begin_char) > 3 {
                            cur = cur_next;
                            continue;
                        }
                        skills += 1;
                        if tt.borrow().is_value("ОПЫТ", None) || tt.borrow().is_value("СТАЖ", None) {
                            if res.try_parse_exp(sofa) {
                                let new_end = res.end_token.borrow().next.clone();
                                cur = new_end;
                                continue;
                            } else if prev.map_or(false, |p| p.typ == VacanceTokenType::Plus) {
                                skills -= 1;
                                plus += 1;
                            }
                        }
                        cur = cur_next;
                        continue;
                    }
                    VacanceTokenType::Experience => {
                        if res.try_parse_exp(sofa) {
                            let new_end = res.end_token.borrow().next.clone();
                            cur = new_end;
                        } else {
                            skills += 1;
                            cur = cur_next;
                        }
                        continue;
                    }
                    VacanceTokenType::Money => {
                        res.try_parse_money();
                        let new_end = res.end_token.borrow().next.clone();
                        cur = new_end;
                        continue;
                    }
                    VacanceTokenType::Driving => {
                        if res.try_parse_driving(sofa) {
                            break;
                        } else {
                            lic += 1;
                            cur = cur_next;
                            continue;
                        }
                    }
                    _ => {
                        cur = cur_next;
                        continue;
                    }
                }
            }

            // Check for referent tokens
            let ref_opt = tt.borrow().get_referent();
            if let Some(r) = ref_opt {
                let type_name = r.borrow().type_name.clone();
                if type_name == "DATE" {
                    let year  = r.borrow().get_string_value("YEAR").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    let month = r.borrow().get_string_value("MONTH").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    let day   = r.borrow().get_string_value("DAY").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    if year > 0 && month > 0 && day > 0 {
                        res.refs.push(r);
                    }
                } else if type_name == "URI" {
                    dummy += 1;
                } else if !res.refs.iter().any(|x| Rc::ptr_eq(x, &r)) {
                    if type_name == "MONEY" && (tt_begin - begin_char) < 10 {
                        if res.try_parse_money() {
                            let new_end = res.end_token.borrow().next.clone();
                            cur = new_end;
                            continue;
                        }
                    }
                    res.refs.push(r);
                }
            }

            cur = tt.borrow().next.clone();
        }

        // Determine type if still Undefined
        if res.typ == VacanceTokenType::Undefined {
            if dummy > 0 {
                res.typ = VacanceTokenType::Dummy;
            } else if lang > 0 {
                res.typ = VacanceTokenType::Language;
            } else if edu > 0 {
                res.typ = VacanceTokenType::Education;
                res.try_parse_education();
            } else if !res.refs.is_empty() {
                let first_type = res.refs[0].borrow().type_name.clone();
                if first_type == "DATE" {
                    res.typ = VacanceTokenType::Date;
                } else if moral > 0 {
                    res.typ = VacanceTokenType::Moral;
                } else if lic > 0 {
                    res.typ = VacanceTokenType::License;
                } else if plus > 0 {
                    res.typ = VacanceTokenType::Plus;
                } else if skills > 0 {
                    res.typ = VacanceTokenType::Skill;
                }
            } else if moral > 0 {
                res.typ = VacanceTokenType::Moral;
            } else if lic > 0 {
                res.typ = VacanceTokenType::License;
            } else if plus > 0 {
                res.typ = VacanceTokenType::Plus;
            } else if skills > 0 {
                res.typ = VacanceTokenType::Skill;
            }
        }

        Some(res)
    }

    // ── Helper: extract experience duration ───────────────────────────────

    fn try_parse_exp(&mut self, sofa: &SourceOfAnalysis) -> bool {
        let mut t = self.end_token.borrow().next.clone();
        // Skip "работа", hyphen, ":"
        loop {
            let tt = match t { Some(ref x) => x.clone(), None => return false };
            if tt.borrow().is_value("РАБОТА", None)
                || tt.borrow().is_hiphen(sofa)
                || tt.borrow().is_char(':', sofa)
            {
                t = tt.borrow().next.clone();
                continue;
            }
            t = Some(tt);
            break;
        }
        let t = match t { Some(x) => x, None => return false };

        // "не требоваться"
        if t.borrow().is_value2("НЕ", "ТРЕБОВАТЬСЯ") {
            if let Some(next2) = t.borrow().next.clone() {
                self.end_token = next2;
            }
            self.typ  = VacanceTokenType::Experience;
            self.value = Some("0".to_string());
            return true;
        }

        // Try Number + time word
        let kind_is_num = matches!(t.borrow().kind, TokenKind::Number(_));
        if kind_is_num {
            let num_val = if let TokenKind::Number(ref nd) = t.borrow().kind {
                nd.value.clone()
            } else { return false };
            let next = t.borrow().next.clone();
            if let Some(nt) = next {
                let is_time = nt.borrow().is_value("ГОД", None)
                    || nt.borrow().is_value("ЛЕТ", None)
                    || nt.borrow().is_value("МЕСЯЦ", None)
                    || nt.borrow().is_value("НЕДЕЛЯ", None);
                if is_time {
                    self.end_token = nt.clone();
                    // Check for "-до N лет" range
                    if let Some(nt2) = nt.borrow().next.clone() {
                        let is_range = nt2.borrow().is_value("ДО", None) || nt2.borrow().is_hiphen(sofa);
                        if is_range {
                            if let Some(nt3) = nt2.borrow().next.clone() {
                                if let Some(nt4) = nt3.borrow().next.clone() {
                                    let is_time2 = nt4.borrow().is_value("ГОД", None)
                                        || nt4.borrow().is_value("ЛЕТ", None)
                                        || nt4.borrow().is_value("МЕСЯЦ", None);
                                    if is_time2 {
                                        if let TokenKind::Number(ref nd2) = nt3.borrow().kind {
                                            self.value = Some(format!("{}-{}", num_val, nd2.value));
                                            self.end_token = nt4;
                                            self.typ = VacanceTokenType::Experience;
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    self.value = Some(num_val);
                    self.typ   = VacanceTokenType::Experience;
                    return true;
                }
            }
        } else if t.borrow().is_value("ОТ", None) {
            // "от N лет"
            if let Some(nt) = t.borrow().next.clone() {
                if let TokenKind::Number(ref nd) = nt.borrow().kind {
                    let num_val = nd.value.clone();
                    drop({});
                    if let Some(nt2) = nt.borrow().next.clone() {
                        let is_time = nt2.borrow().is_value("ГОД", None)
                            || nt2.borrow().is_value("ЛЕТ", None)
                            || nt2.borrow().is_value("МЕСЯЦ", None);
                        if is_time {
                            self.end_token = nt2;
                            self.value = Some(num_val);
                            self.typ   = VacanceTokenType::Experience;
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    // ── Helper: extract money ─────────────────────────────────────────────

    fn try_parse_money(&mut self) -> bool {
        let end_char   = self.end_token.borrow().end_char;
        let begin_char = self.begin_token.borrow().begin_char;
        let mut cur = Some(self.begin_token.clone());
        while let Some(tt) = cur.take() {
            if tt.borrow().begin_char > end_char { break; }
            if tt.borrow().begin_char - begin_char > 20 { break; }
            let ref_opt = tt.borrow().get_referent();
            if let Some(r) = ref_opt {
                if r.borrow().type_name == "MONEY" {
                    if tt.borrow().end_char > self.end_token.borrow().end_char {
                        self.end_token = tt.clone();
                    }
                    if !self.refs.iter().any(|x| Rc::ptr_eq(x, &r)) {
                        self.refs.push(r.clone());
                    }
                    self.typ = VacanceTokenType::Money;
                    // Check range: "- до N"
                    if let Some(next) = tt.borrow().next.clone() {
                        let is_range_marker = {
                            let nb = next.borrow();
                            matches!(nb.kind, TokenKind::Text(ref td) if matches!(td.term.as_str(), "-" | "–" | "—"))
                                || nb.is_value("ДО", None)
                        };
                        if is_range_marker {
                            if let Some(next2) = next.borrow().next.clone() {
                                let r2 = next2.borrow().get_referent();
                                if let Some(r2) = r2 {
                                    if r2.borrow().type_name == "MONEY" {
                                        if next2.borrow().end_char > self.end_token.borrow().end_char {
                                            self.end_token = next2.clone();
                                        }
                                        if !self.refs.iter().any(|x| Rc::ptr_eq(x, &r2)) {
                                            self.refs.push(r2);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return true;
                }
            }
            cur = tt.borrow().next.clone();
        }
        false
    }

    // ── Helper: parse driving license category ────────────────────────────

    fn try_parse_driving(&mut self, sofa: &SourceOfAnalysis) -> bool {
        let mut cur = self.end_token.borrow().next.clone();
        loop {
            let tt = match cur { Some(x) => x, None => return false };
            if tt.borrow().is_hiphen(sofa)
                || tt.borrow().is_char_of(".:,", sofa)
                || tt.borrow().is_value("КАТЕГОРИЯ", None)
                || tt.borrow().is_value("КАТ", None)
            {
                cur = tt.borrow().next.clone();
                continue;
            }
            // 1-3 letter token
            if tt.borrow().length_char() <= 3 && tt.borrow().chars.is_letter() {
                self.typ = VacanceTokenType::Driving;
                let mut val = {
                    let tb = tt.borrow();
                    if let TokenKind::Text(ref td) = tb.kind { td.term.clone() } else { String::new() }
                };
                self.end_token = tt.clone();
                let mut scan = tt.borrow().next.clone();
                loop {
                    let st = match scan { Some(x) => x, None => break };
                    if st.borrow().is_char('.', sofa) || st.borrow().is_comma_and(sofa) {
                        scan = st.borrow().next.clone();
                        continue;
                    }
                    if st.borrow().length_char() == 1
                        && st.borrow().chars.is_all_upper()
                        && st.borrow().chars.is_letter()
                    {
                        let ch = {
                            let sb = st.borrow();
                            if let TokenKind::Text(ref td) = sb.kind { td.term.clone() } else { String::new() }
                        };
                        val.push_str(&ch);
                        self.end_token = st.clone();
                        scan = self.end_token.borrow().next.clone();
                        continue;
                    }
                    break;
                }
                // Transliterate Cyrillic lookalikes to Latin
                val = val.replace('А', "A").replace('В', "B").replace('С', "C");
                self.value = Some(val);
                return true;
            }
            return false;
        }
    }

    // ── Helper: determine education level ─────────────────────────────────

    fn try_parse_education(&mut self) {
        let end_char = self.end_token.borrow().end_char;
        let mut hi   = false;
        let mut middl = false;
        let mut prof = false;
        let mut spec = false;
        let mut tech = false;

        let mut cur = Some(self.begin_token.clone());
        while let Some(tt) = cur.take() {
            if tt.borrow().end_char > end_char { break; }
            let v = |s| tt.borrow().is_value(s, None);
            if v("СРЕДНИЙ") || v("СРЕДНЕ") || v("СРЕДН") { middl = true; }
            else if v("ВЫСШИЙ") || v("ВЫСШ")              { hi    = true; }
            else if v("ПРОФЕССИОНАЛЬНЫЙ") || v("ПРОФ") || v("ПРОФИЛЬНЫЙ") { prof = true; }
            else if v("СПЕЦИАЛЬНЫЙ") || v("СПЕЦ")         { spec  = true; }
            else if v("ТЕХНИЧЕСКИЙ") || v("ТЕХ") || v("ТЕХНИЧ") { tech = true; }
            cur = tt.borrow().next.clone();
        }

        if !hi && !middl && (spec || prof || tech) {
            middl = true;
        }
        if hi || middl {
            let mut val = if hi { "ВО".to_string() } else { "СО".to_string() };
            if spec { val.push_str(",спец"); }
            if prof { val.push_str(",проф"); }
            if tech { val.push_str(",тех"); }
            self.value = Some(val);
        }
    }

    // ── Helper: extract text value ────────────────────────────────────────

    fn get_value(&mut self, sofa: &SourceOfAnalysis) {
        let d = data();
        let mut t0 = self.begin_token.clone();
        let end_char = self.end_token.borrow().end_char;

        // Skip leading punctuation / filler words
        let mut cont = true;
        while cont {
            cont = false;
            let tb = t0.borrow();
            if tb.end_char >= end_char { break; }
            let is_punct = tb.length_char() == 1 && !tb.chars.is_letter();
            if is_punct {
                if let Some(next) = tb.next.clone() {
                    drop(tb);
                    t0 = next;
                    cont = true;
                    continue;
                }
            }
            if tb.is_value("ИМЕТЬ", None) || tb.is_value("ВЛАДЕТЬ", None) || tb.is_value("ЕСТЬ", None) {
                if let Some(next) = tb.next.clone() {
                    drop(tb);
                    t0 = next;
                    cont = true;
                    continue;
                }
            }
            drop(tb);
            if let Some(tok) = d.termins.try_parse(&t0) {
                if term_has_tag2(&tok) {
                    if let Some(next) = tok.end_token.borrow().next.clone() {
                        t0 = next;
                        cont = true;
                        continue;
                    }
                }
            }
        }

        let mut t1 = self.end_token.clone();
        // Trim trailing punct
        {
            let tb = t1.borrow();
            let is_trail = tb.is_char('.', sofa)
                || tb.is_char(';', sofa)
                || tb.is_char(':', sofa)
                || tb.is_char(',', sofa)
                || tb.is_hiphen(sofa);
            if is_trail {
                if let Some(prev) = tb.prev.as_ref().and_then(|w| w.upgrade()) {
                    drop(tb);
                    t1 = prev;
                }
            }
        }

        let begin = t0.borrow().begin_char;
        let end   = t1.borrow().end_char;
        if begin <= end {
            let text_raw = sofa.substring(begin, end).to_string();
            let is_all_upper = t0.borrow().chars.is_all_upper();
            let text = if !is_all_upper && text_raw.starts_with(|c: char| c.is_lowercase()) {
                let mut chars = text_raw.chars();
                if let Some(first) = chars.next() {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                } else {
                    text_raw
                }
            } else {
                text_raw
            };
            self.value = Some(text);
        }
    }
}
