/// URI Analyzer — port of UriAnalyzer.cs
///
/// Scheme tag integers (matching C# Tag values):
///   0  = generic URI scheme from CSV (file:, git:, etc.)
///   1  = document codes (ISBN, УДК, ББК, ГОСТ, …)
///   2  = WWW domain
///   3  = Skype / SWIFT / Telegram
///   4  = ICQ
///   5  = id numbers (ИНН, КПП, ОГРН, СНИЛС, …)
///   6  = bank account numbers (Р/С, Л/С, …)
///   7  = Cadastre number (КАДАСТРОВЫЙ НОМЕР)
///   10 = HTTP / HTTPS / FTP

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind, NumberSpellingType};
use crate::referent::Referent;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::{Termin, TerminCollection};

use super::uri_referent::{self as ur_ref};
use super::uri_item_token::{
    attach_domain_name, attach_uri_content, attach_url,
    attach_isbn, attach_bbk, attach_skype, attach_icq_content,
    attach_iso_content, attach_mail_users,
};

// ── Scheme term collection ────────────────────────────────────────────────────

static SCHEMES: OnceLock<TerminCollection> = OnceLock::new();

fn schemes() -> &'static TerminCollection {
    SCHEMES.get_or_init(build_schemes)
}

fn tag(i: i32) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
    Some(Arc::new(i) as Arc<dyn std::any::Any + Send + Sync>)
}

fn build_schemes() -> TerminCollection {
    let mut tc = TerminCollection::new();

    // Generic URI schemes from CSV (tag 0)
    let csv = include_str!("../../resources/UriSchemes.csv");
    for line0 in csv.split('\n') {
        let line = line0.trim();
        if line.is_empty() { continue; }
        let mut t = Termin::new_canonic(line.to_ascii_uppercase(), line.to_ascii_lowercase());
        t.tag = tag(0);
        tc.add(t);
    }

    // Document classification codes (tag 1)
    for s in &["ISBN", "УДК", "ББК", "ТНВЭД", "ОКВЭД"] {
        let mut t = Termin::new(*s);
        t.tag = tag(1);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("Общероссийский классификатор форм собственности", "ОКФС");
        t.add_variant("ОКФС");
        t.tag = tag(1);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic(
            "Общероссийский классификатор организационно правовых форм", "ОКОПФ",
        );
        t.add_variant("ОКОПФ");
        t.tag = tag(1);
        tc.add(t);
    }
    for s in &["ГОСТ", "RFC", "ISO", "ТУ", "ОКФС", "ОКОПФ"] {
        let mut t = Termin::new(*s);
        t.tag = tag(1);
        tc.add(t);
    }

    // WWW (tag 2)
    {
        let mut t = Termin::new("WWW");
        t.tag = tag(2);
        tc.add(t);
    }

    // HTTP / HTTPS / FTP / SHTTP (tag 10)
    for s in &["HTTP", "HTTPS", "SHTTP", "FTP"] {
        let mut t = Termin::new_canonic(*s, s.to_ascii_lowercase());
        t.tag = tag(10);
        tc.add(t);
    }

    // Skype / SWIFT / Telegram (tag 3)
    {
        let mut t = Termin::new("SKYPE");
        t.add_variant("СКАЙП");
        t.add_variant("SKYPEID");
        t.add_variant("SKYPE ID");
        t.tag = tag(3);
        tc.add(t);
    }
    {
        let mut t = Termin::new("SWIFT");
        t.add_variant("СВИФТ");
        t.tag = tag(3);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("TELEGRAM", "telegram");
        t.add_variant("ТЕЛЕГРАМ");
        t.add_variant("T.ME");
        t.tag = tag(3);
        tc.add(t);
    }

    // ICQ (tag 4)
    {
        let mut t = Termin::new("ICQ");
        t.tag = tag(4);
        tc.add(t);
    }

    // ID number schemes (tag 5)
    {
        let mut t = Termin::new("ИМЕЙ"); // IMEI
        t.add_variant("IMEI");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("основной государственный регистрационный номер", "ОГРН");
        t.add_variant("ОГРН");
        t.add_variant("ОГРН ИП");
        t.add_variant("ОГРНИП");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("Индивидуальный номер налогоплательщика", "ИНН");
        t.add_variant("ИНН");
        t.add_variant("Идентификационный номер налогоплательщика");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("Код причины постановки на учет", "КПП");
        t.add_variant("КПП");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("Банковский идентификационный код", "БИК");
        t.add_variant("БИК");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic(
            "Страховой номер индивидуального лицевого счёта", "СНИЛС",
        );
        t.add_variant("СНИЛС");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic(
            "Общероссийский классификатор предприятий и организаций", "ОКПО",
        );
        t.add_variant("ОКПО");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic(
            "Общероссийский классификатор объектов административно-территориального деления", "ОКАТО",
        );
        t.add_variant("ОКАТО");
        t.tag = tag(5);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic(
            "Общероссийский классификатор территорий муниципальных образований", "ОКТМО",
        );
        t.add_variant("ОКТМО");
        t.tag = tag(5);
        tc.add(t);
    }

    // Bank account numbers (tag 6)
    {
        let mut t = Termin::new_canonic("РАСЧЕТНЫЙ СЧЕТ", "Р/С");
        t.add_variant("СЧЕТ ПОЛУЧАТЕЛЯ");
        t.add_variant("СЧЕТ ОТПРАВИТЕЛЯ");
        t.add_variant("СЧЕТ");
        t.add_abridge("Р.С.");
        t.add_abridge("Р.СЧ.");
        t.add_abridge("РАСЧ.СЧЕТ");
        t.add_abridge("РАС.СЧЕТ");
        t.tag = tag(6);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("ЛИЦЕВОЙ СЧЕТ", "Л/С");
        t.add_abridge("Л.С.");
        t.add_abridge("Л.СЧ.");
        t.add_abridge("ЛИЦ.СЧЕТ");
        t.tag = tag(6);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("КОРРЕСПОНДЕНТСКИЙ СЧЕТ", "К/С");
        t.add_abridge("К.С.");
        t.add_abridge("К.СЧ.");
        t.add_abridge("КОР.СЧЕТ");
        t.tag = tag(6);
        tc.add(t);
    }
    {
        let mut t = Termin::new_canonic("КОД БЮДЖЕТНОЙ КЛАССИФИКАЦИИ", "КБК");
        t.add_variant("КБК");
        t.tag = tag(6);
        tc.add(t);
    }

    // Cadastre number (tag 7)
    {
        let mut t = Termin::new("КАДАСТРОВЫЙ НОМЕР");
        t.add_variant("КАДАСТРОВЫЙ НОМ.");
        t.tag = tag(7);
        tc.add(t);
    }

    tc
}

// ── Deduplication store ───────────────────────────────────────────────────────

struct UriDedup {
    items: Vec<Rc<RefCell<Referent>>>,
}

impl UriDedup {
    fn new() -> Self { UriDedup { items: Vec::new() } }

    fn register(&mut self, r: Rc<RefCell<Referent>>) -> Rc<RefCell<Referent>> {
        for existing in &self.items {
            if ur_ref::can_be_equals(&existing.borrow(), &r.borrow()) {
                return existing.clone();
            }
        }
        self.items.push(r.clone());
        r
    }
}

// ── Helper: embed a URI referent token ───────────────────────────────────────

fn embed_uri(
    kit: &mut AnalysisKit,
    dedup: &mut UriDedup,
    scheme: &str,
    value: &str,
    begin: TokenRef,
    end: TokenRef,
) -> TokenRef {
    let new_ref = ur_ref::new_uri(scheme, value);
    let registered = dedup.register(new_ref.clone());
    if Rc::ptr_eq(&registered, &new_ref) {
        kit.add_entity(registered.clone());
    }
    let tok = Rc::new(RefCell::new(Token::new_referent(begin, end, registered)));
    kit.embed_token(tok.clone());
    tok
}

// ── Bank account X/С prefix detector ─────────────────────────────────────────

/// If `t` is "Р", "К", or "Л" immediately followed (no space) by "/" and "С",
/// return `(scheme_str, end_of_prefix_token)`.
fn try_bank_account_prefix<'a>(
    t: &TokenRef,
    sofa: &SourceOfAnalysis,
) -> Option<(&'a str, TokenRef)> {
    let term = {
        let tb = t.borrow();
        tb.term().map(|s| s.to_string())?
    };
    // len() is byte count; Cyrillic chars are 2 bytes each
    if term.chars().count() != 1 { return None; }
    let ch = term.chars().next()?;
    // Next must be "/" with no whitespace
    let slash = t.borrow().next.clone()?;
    if slash.borrow().whitespaces_before_count(sofa) > 0 { return None; }
    if !slash.borrow().is_char('/', sofa) { return None; }
    // Then "С" or "с" with no whitespace
    let s_tok = slash.borrow().next.clone()?;
    if s_tok.borrow().whitespaces_before_count(sofa) > 0 { return None; }
    let s_term = s_tok.borrow().term().map(|s| s.to_string())?;
    if s_term != "С" { return None; }

    match ch {
        'Р' => Some(("Р/С", s_tok)),
        'К' => Some(("К/С", s_tok)),
        'Л' => Some(("Л/С", s_tok)),
        _ => None,
    }
}

// ── Site-before helper ────────────────────────────────────────────────────────

/// Walk backwards from `t` looking for "САЙТ", "WEB", etc. prefix tokens.
/// Returns the first such token or None.
fn site_before(t: Option<TokenRef>, sofa: &SourceOfAnalysis) -> Option<TokenRef> {
    let t = t?;
    let t = if t.borrow().is_char(':', sofa) {
        t.borrow().prev.as_ref().and_then(|w| w.upgrade())?
    } else {
        t
    };

    let (val_website, val_web, val_www) = {
        let tb = t.borrow();
        (
            tb.is_value("ВЕБСАЙТ", None) || tb.is_value("WEBSITE", None),
            tb.is_value("WEB", None) || tb.is_value("WWW", None),
            tb.is_value("WEB", None),
        )
    };
    let _ = val_www;

    if val_website || val_web { return Some(t); }

    let mut t0: Option<TokenRef> = None;
    let is_site = t.borrow().is_value("САЙТ", None) || t.borrow().is_value("SITE", None);
    let prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());

    if is_site {
        t0 = Some(t.clone());
        let mut tt = prev;
        // Extend back past hyphen — can't assign to tt inside if-let that borrows it,
        // so compute the new value first then assign outside.
        let hiphen_prev: Option<Option<TokenRef>> = if let Some(ref p) = tt {
            if p.borrow().is_hiphen(sofa) {
                Some(p.borrow().prev.as_ref().and_then(|w| w.upgrade()))
            } else { None }
        } else { None };
        if let Some(v) = hiphen_prev { tt = v; }
        if let Some(ref p) = tt {
            if p.borrow().is_value("WEB", None) || p.borrow().is_value("ВЕБ", None) {
                t0 = Some(p.clone());
            }
        }
        t0
    } else {
        None
    }
}

// ── Telegram-before helper ────────────────────────────────────────────────────

fn telegram_before(rt_begin: &mut TokenRef, rt_end: &mut TokenRef, val: &str, sofa: &SourceOfAnalysis) {
    // Extend to surrounding parens
    {
        let prev_is_open = rt_begin.borrow().prev.as_ref()
            .and_then(|w| w.upgrade())
            .map_or(false, |p| p.borrow().is_char('(', sofa));
        let next_is_close = rt_end.borrow().next.clone()
            .map_or(false, |n| n.borrow().is_char(')', sofa));
        if prev_is_open && next_is_close {
            let nb = rt_begin.borrow().prev.as_ref().and_then(|w| w.upgrade()).unwrap();
            *rt_begin = nb;
            let ne = rt_end.borrow().next.clone().unwrap();
            *rt_end = ne;
        }
    }

    // Look backwards for the username mentioned earlier
    let te = val.to_ascii_uppercase();
    let mut cou = 10i32;
    let mut cur = rt_begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
    while let Some(t) = cur.take() {
        cou -= 1;
        if cou <= 0 { break; }
        if let Some(term) = t.borrow().term() {
            if term == te {
                let end_char = t.borrow().end_char;
                let begin_char_rt = rt_begin.borrow().begin_char;
                if begin_char_rt - end_char < 10 {
                    *rt_begin = t.clone();
                    if rt_begin.borrow().prev.as_ref()
                        .and_then(|w| w.upgrade())
                        .map_or(false, |p| p.borrow().is_char('@', sofa))
                    {
                        let nb = rt_begin.borrow().prev.as_ref()
                            .and_then(|w| w.upgrade())
                            .unwrap_or_else(|| rt_begin.clone());
                        *rt_begin = nb;
                    }
                    break;
                }
            }
        }
        cur = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    }
}

// ── Kadastr helper ────────────────────────────────────────────────────────────

fn try_attach_kadastr(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef, TokenRef)> {
    // Must start with a short digit number (1-2 chars)
    {
        let tb = t0.borrow();
        if !matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit) {
            return None;
        }
        if tb.length_char() > 2 { return None; }
        // Must have whitespace before or previous comma
        let ws = tb.is_whitespace_before(sofa);
        let prev_comma = tb.prev.as_ref()
            .and_then(|w| w.upgrade())
            .map_or(false, |p| p.borrow().is_char(',', sofa));
        if !ws && !prev_comma { return None; }
    }

    let mut vals: Vec<String> = Vec::new();
    let mut rt_end = t0.clone();
    let mut t = t0.clone();

    loop {
        let (is_num, num_val, is_colon, next_is_num, next) = {
            let tb = t.borrow();
            let is_num = matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit);
            let num_val = if is_num { Some(tb.number_value().unwrap_or("").to_string()) } else { None };
            let next = tb.next.clone();
            let (is_colon, next_is_num) = match &next {
                None => (false, false),
                Some(n) => {
                    let n_is_colon = n.borrow().is_char(':', sofa);
                    let nn_is_num = if n_is_colon {
                        n.borrow().next.clone()
                            .map_or(false, |nn| matches!(&nn.borrow().kind, TokenKind::Number(_)))
                    } else { false };
                    (n_is_colon, nn_is_num)
                }
            };
            (is_num, num_val, is_colon, next_is_num, next)
        };

        if !is_num { break; }
        let val = num_val.unwrap();
        vals.push(val);
        rt_end = t.clone();

        if is_colon && next_is_num && !t.borrow().is_whitespace_after(sofa) {
            let colon_tok = next.clone().unwrap();
            if !colon_tok.borrow().is_whitespace_after(sofa) {
                let after_colon = colon_tok.borrow().next.clone();
                t = match after_colon { None => break, Some(n) => n };
                continue;
            }
        }
        break;
    }

    if vals.len() != 4 { return None; }

    let value = format!("{}:{}:{}:{}", vals[0], vals[1], vals[2], vals[3]);

    // Optionally extend begin backwards for "КН", "КАД...", "АКТУАЛ..."
    let mut rt_begin = t0.clone();
    let mut prev = rt_begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
    loop {
        let p = match prev.take() { None => break, Some(x) => x };
        let (is_hp, is_char_dot, term) = {
            let pb = p.borrow();
            let is_hp = pb.is_hiphen(sofa);
            let is_dot = pb.is_char_of(":.", sofa);
            let term = pb.term().map(|s| s.to_string());
            (is_hp, is_dot, term)
        };
        if is_hp || is_char_dot {
            prev = p.borrow().prev.as_ref().and_then(|w| w.upgrade());
            continue;
        }
        match term {
            None => break,
            Some(t) => {
                if t == "КН" || t.starts_with("КАД") || t.starts_with("АКТУАЛ") {
                    rt_begin = p.clone();
                    prev = p.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    continue;
                }
            }
        }
        break;
    }

    Some((value, rt_begin, rt_end))
}

// ── UriAnalyzer ───────────────────────────────────────────────────────────────

pub struct UriAnalyzer;

impl UriAnalyzer {
    pub fn new() -> Self { UriAnalyzer }
}

impl Analyzer for UriAnalyzer {
    fn name(&self) -> &'static str { "URI" }
    fn caption(&self) -> &'static str { "URI" }
    fn progress_weight(&self) -> i32 { 2 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut dedup = UriDedup::new();

        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            cur = t.borrow().next.clone();

            if t.borrow().is_ignored(&sofa) { continue; }

            // ── Bank account X/С multi-token prefix (Р/С, К/С, Л/С) ─────────
            // These abbreviations are three tokens: Letter + "/" + "С".
            if let Some((sch, prefix_end)) = try_bank_account_prefix(&t, &sofa) {
                let mut t0 = prefix_end.borrow().next.clone();
                while let Some(ref n) = t0.clone() {
                    let (skip, next) = {
                        let nb = n.borrow();
                        let skip = nb.is_char_of(".:№", &sofa)
                            || nb.is_hiphen(&sofa)
                            || nb.is_table_control_char(&sofa);
                        let next = nb.next.clone();
                        (skip, next)
                    };
                    if skip { t0 = next; } else { break; }
                }
                if let Some(ref t0) = t0 {
                    if matches!(&t0.borrow().kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit) {
                        let val = t0.borrow().get_source_text(&sofa).to_string();
                        if val.len() >= 5 {
                            let rt = embed_uri(kit, &mut dedup, sch, &val, t.clone(), t0.clone());
                            cur = rt.borrow().next.clone();
                            continue;
                        }
                    }
                }
            }

            // ── Scheme keyword match ─────────────────────────────────────────
            if let Some(tok) = schemes().try_parse(&t) {
                let scheme_tag: i32 = tok.termin.tag.as_ref()
                    .and_then(|a| a.downcast_ref::<i32>())
                    .copied()
                    .unwrap_or(0);

                let scheme_canonic = tok.termin.canonic_text.clone();
                let mut tt = tok.end_token.clone();

                // Handle "SCHEME(SCHEME)" pattern
                {
                    let next_is_open = tt.borrow().next.clone()
                        .map_or(false, |n| n.borrow().is_char('(', &sofa));
                    if next_is_open {
                        let inner = tt.borrow().next.clone()
                            .and_then(|n| n.borrow().next.clone());
                        if let Some(inner_t) = inner {
                            if let Some(tok1) = schemes().try_parse(&inner_t) {
                                if tok1.termin.canonic_text == tok.termin.canonic_text {
                                    let close = tok1.end_token.borrow().next.clone();
                                    if close.as_ref().map_or(false, |c| c.borrow().is_char(')', &sofa)) {
                                        tt = close.unwrap();
                                    }
                                }
                            }
                        }
                    }
                }

                match scheme_tag {
                    // ── i=0: generic scheme (scheme:value) ──────────────────
                    0 => {
                        let next = tt.borrow().next.clone();
                        let ok = next.as_ref().map_or(false, |n| {
                            let nb = n.borrow();
                            (nb.is_char_of(":|", &sofa) || nb.is_table_control_char(&sofa))
                                && !nb.is_whitespace_before(&sofa)
                                && nb.whitespaces_before_count(&sofa) <= 2
                        });
                        if !ok { continue; }

                        let sep = next.unwrap();
                        let mut t1 = sep.borrow().next.clone();
                        while let Some(ref n) = t1.clone() {
                            if n.borrow().is_char_of("/\\", &sofa) {
                                t1 = n.borrow().next.clone();
                            } else { break; }
                        }
                        let t1 = match t1 { None => continue, Some(x) => x };
                        if t1.borrow().whitespaces_before_count(&sofa) > 2 { continue; }

                        let ut = match attach_uri_content(&t1, &sofa, false) { None => continue, Some(x) => x };
                        let scheme_lc = scheme_canonic.to_ascii_lowercase();
                        let mut begin = t.clone();
                        let mut end = ut.end_token.clone();

                        if let Some(sb) = site_before(t.borrow().prev.as_ref().and_then(|w| w.upgrade()), &sofa) {
                            begin = sb;
                        }
                        if end.borrow().next.clone().map_or(false, |n| n.borrow().is_char_of("/\\", &sofa)) {
                            let ne = end.borrow().next.clone().unwrap();
                            end = ne;
                        }

                        let rt = embed_uri(kit, &mut dedup, &scheme_lc, &ut.value, begin, end);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=10: HTTP/HTTPS/FTP ─────────────────────────────────
                    10 => {
                        let next = tt.borrow().next.clone();
                        let ok = next.as_ref().map_or(false, |n| n.borrow().is_char(':', &sofa));
                        if !ok { continue; }
                        let mut t1 = next.unwrap().borrow().next.clone();
                        while let Some(ref n) = t1.clone() {
                            if n.borrow().is_char_of("/\\", &sofa) { t1 = n.borrow().next.clone(); }
                            else { break; }
                        }
                        let t1 = match t1 { None => continue, Some(x) => x };
                        if t1.borrow().is_newline_before(&sofa) { continue; }

                        // Optional: skip "www."
                        let mut t1 = t1;
                        if t1.borrow().is_value("WWW", None) {
                            // Eagerly extract next so the Ref<Token> is dropped before
                            // we potentially reassign t1 inside the if-let body.
                            let t1_next = t1.borrow().next.clone();
                            if let Some(dot) = t1_next {
                                if dot.borrow().is_char('.', &sofa) {
                                    t1 = match dot.borrow().next.clone() { None => continue, Some(x) => x };
                                }
                            }
                        }
                        if t1.borrow().is_newline_before(&sofa) { continue; }

                        let ut = match attach_uri_content(&t1, &sofa, true) { None => continue, Some(x) => x };
                        if ut.value.len() < 4 { continue; }

                        // Telegram t.me/... detection
                        let (scheme_use, value_use) = if ut.value.to_ascii_lowercase().starts_with("t.me/") {
                            ("telegram".to_string(), ut.value[5..].to_string())
                        } else {
                            (scheme_canonic.to_ascii_lowercase(), ut.value.clone())
                        };

                        let mut begin = t.clone();
                        let mut end = ut.end_token.clone();

                        if scheme_use == "telegram" {
                            telegram_before(&mut begin, &mut end, &value_use, &sofa);
                        } else if let Some(sb) = site_before(t.borrow().prev.as_ref().and_then(|w| w.upgrade()), &sofa) {
                            begin = sb;
                        }
                        if end.borrow().next.clone().map_or(false, |n| n.borrow().is_char_of("/\\", &sofa)) {
                            let ne = end.borrow().next.clone().unwrap();
                            end = ne;
                        }

                        let rt = embed_uri(kit, &mut dedup, &scheme_use, &value_use, begin, end);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=2: WWW domain ──────────────────────────────────────
                    2 => {
                        let dot = tt.borrow().next.clone();
                        let ok = dot.as_ref().map_or(false, |n| {
                            n.borrow().is_char('.', &sofa) && !n.borrow().is_whitespace_before(&sofa)
                        });
                        if !ok { continue; }
                        let dot = dot.unwrap();
                        let dot_ws_after = dot.borrow().is_whitespace_after(&sofa);
                        if dot_ws_after && scheme_canonic != "WWW" { continue; }

                        let after_dot = match dot.borrow().next.clone() { None => continue, Some(x) => x };
                        let ut = match attach_uri_content(&after_dot, &sofa, true) { None => continue, Some(x) => x };

                        let mut begin = t.clone();
                        let mut end = ut.end_token.clone();
                        if let Some(sb) = site_before(t.borrow().prev.as_ref().and_then(|w| w.upgrade()), &sofa) {
                            begin = sb;
                        }
                        if end.borrow().next.clone().map_or(false, |n| n.borrow().is_char_of("/\\", &sofa)) {
                            let ne = end.borrow().next.clone().unwrap();
                            end = ne;
                        }

                        let rt = embed_uri(kit, &mut dedup, "http", &ut.value, begin, end);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=1: ISBN / УДК / ББК / ГОСТ / ТУ / RFC / ISO ───────
                    1 => {
                        let sch = scheme_canonic.as_str();
                        let after_kw = tt.borrow().next.clone();

                        let ut_opt = match sch {
                            "ISBN" => after_kw.as_ref().and_then(|a| attach_isbn(a, &sofa)),
                            "RFC" | "ISO" | "ОКФС" | "ОКОПФ" =>
                                after_kw.as_ref().and_then(|a| attach_iso_content(a, &sofa, ":")),
                            "ГОСТ" =>
                                after_kw.as_ref().and_then(|a| attach_iso_content(a, &sofa, "-.")),
                            "ТУ" => {
                                if t.borrow().chars.is_all_upper() {
                                    after_kw.as_ref().and_then(|a| {
                                        attach_iso_content(a, &sofa, "-.").filter(|u| u.value.len() >= 10)
                                    })
                                } else { None }
                            }
                            _ => after_kw.as_ref().and_then(|a| attach_bbk(a, &sofa)),
                        };

                        let ut = match ut_opt { None => continue, Some(x) => x };

                        let (begin, end) = if ut.begin_char() < t.borrow().begin_char {
                            let b = ut.begin_token.clone();
                            let e = if t.borrow().next.clone()
                                .map_or(false, |n| n.borrow().is_char(')', &sofa))
                            {
                                t.borrow().next.clone().unwrap()
                            } else { t.clone() };
                            (b, e)
                        } else {
                            (t.clone(), ut.end_token.clone())
                        };

                        // Extend begin to "КОД" if present
                        let begin = t.borrow().prev.clone()
                            .and_then(|_| t.borrow().prev.as_ref().and_then(|w| w.upgrade()))
                            .filter(|p| p.borrow().is_value("КОД", None))
                            .unwrap_or(begin);

                        let rt = embed_uri(kit, &mut dedup, sch, &ut.value, begin, end);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=3: Skype / SWIFT / Telegram ───────────────────────
                    3 => {
                        let is_tg = scheme_canonic.eq_ignore_ascii_case("telegram")
                            || scheme_canonic.eq_ignore_ascii_case("t.me");

                        let mut t0 = tt.borrow().next.clone();
                        while let Some(ref n) = t0.clone() {
                            if n.borrow().is_char_of(":|\\/", &sofa)
                                || n.borrow().is_table_control_char(&sofa)
                                || n.borrow().is_hiphen(&sofa)
                            {
                                t0 = n.borrow().next.clone();
                            } else { break; }
                        }
                        let t0 = match t0 { None => continue, Some(x) => x };

                        let ut = match attach_skype(&t0, &sofa, is_tg) { None => continue, Some(x) => x };

                        let scheme_use = if scheme_canonic.to_ascii_uppercase() == "SKYPE" {
                            "skype".to_string()
                        } else {
                            scheme_canonic.clone()
                        };

                        let rt = embed_uri(kit, &mut dedup, &scheme_use, &ut.value.to_ascii_lowercase(), t.clone(), ut.end_token.clone());
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=4: ICQ ─────────────────────────────────────────────
                    4 => {
                        let mut t0 = tt.borrow().next.clone();
                        if let Some(ref n) = t0.clone() {
                            if n.borrow().is_char(':', &sofa) || n.borrow().is_hiphen(&sofa) {
                                t0 = n.borrow().next.clone();
                            }
                        }
                        let t0 = match t0 { None => continue, Some(x) => x };
                        let ut = match attach_icq_content(&t0, &sofa) { None => continue, Some(x) => x };
                        let rt = embed_uri(kit, &mut dedup, "ICQ", &ut.value, t.clone(), t0);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=5: id numbers (ИНН, КПП, ОГРН, СНИЛС, …) ─────────
                    5 => {
                        let sch = scheme_canonic.as_str();
                        let mut t0 = tt.borrow().next.clone();

                        // Skip prepositions / punctuation
                        while let Some(ref n) = t0.clone() {
                            let (skip, next) = {
                                let nb = n.borrow();
                                let skip = nb.is_char_of(".:№N", &sofa)
                                    || nb.is_hiphen(&sofa)
                                    || nb.is_table_control_char(&sofa);
                                let next = nb.next.clone();
                                (skip, next)
                            };
                            if skip { t0 = next; } else { break; }
                        }
                        let t0 = match t0 { None => continue, Some(x) => x };

                        // Must start with a digit
                        if !matches!(&t0.borrow().kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit) {
                            continue;
                        }

                        // Collect digit tokens (may span adjacent tokens)
                        let mut val = String::new();
                        let mut t_end = t0.clone();
                        {
                            let mut tc = t0.clone();
                            loop {
                                let (is_num, src, next) = {
                                    let tb = tc.borrow();
                                    let is_num = matches!(&tb.kind, TokenKind::Number(n)
                                        if n.spelling_type == NumberSpellingType::Digit);
                                    let src = tb.get_source_text(&sofa).to_string();
                                    let next = tb.next.clone();
                                    (is_num, src, next)
                                };
                                if !is_num { break; }
                                val.push_str(&src);
                                t_end = tc.clone();
                                let (is_hyp, nxt_is_num, next2) = match &next {
                                    None => (false, false, None),
                                    Some(n) => {
                                        let is_h = n.borrow().is_hiphen(&sofa)
                                            || n.borrow().is_char('.', &sofa);
                                        let n2 = n.borrow().next.clone();
                                        let nxt_num = n2.as_ref().map_or(false, |nn| {
                                            matches!(&nn.borrow().kind, TokenKind::Number(_))
                                        });
                                        (is_h, nxt_num, n2)
                                    }
                                };
                                if is_hyp && nxt_is_num {
                                    if !tc.borrow().is_whitespace_after(&sofa) {
                                        tc = match next2 { None => break, Some(n) => n };
                                        continue;
                                    }
                                }
                                tc = match next { None => break, Some(n) => n };
                                if tc.borrow().whitespaces_before_count(&sofa) > 1 { break; }
                            }
                        }

                        // Validate length for known schemes
                        let valid = match sch {
                            "БИК" => val.len() == 9,
                            "ИНН" => val.len() == 10 || val.len() == 12,
                            "СНИЛС" | "ОГРН" => val.len() >= 11,
                            _ => val.len() >= 9,
                        };
                        if !valid || val.len() < 5 { continue; }

                        // Extend begin to "НОМЕР" or "КОД" if present
                        let mut begin = t.clone();
                        let mut prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                        loop {
                            let p = match prev.take() { None => break, Some(x) => x };
                            if p.borrow().is_table_control_char(&sofa) { break; }
                            if p.borrow().morph.items().iter()
                                .any(|wf| !wf.base.class.is_undefined()) { break; }
                            let is_num_or_kod = p.borrow().is_value("НОМЕР", None)
                                || p.borrow().is_value("КОД", None);
                            if is_num_or_kod { begin = p.clone(); }
                            break;
                        }

                        let rt = embed_uri(kit, &mut dedup, sch, &val, begin, t_end);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=6: account numbers (simplified) ───────────────────
                    6 => {
                        let sch = scheme_canonic.as_str();
                        let mut t0 = tt.borrow().next.clone();
                        while let Some(ref n) = t0.clone() {
                            let (skip, next) = {
                                let nb = n.borrow();
                                let skip = nb.is_char_of(".:№", &sofa)
                                    || nb.is_hiphen(&sofa)
                                    || nb.is_table_control_char(&sofa);
                                let next = nb.next.clone();
                                (skip, next)
                            };
                            if skip { t0 = next; } else { break; }
                        }
                        let t0 = match t0 { None => continue, Some(x) => x };
                        if !matches!(&t0.borrow().kind, TokenKind::Number(n)
                            if n.spelling_type == NumberSpellingType::Digit) { continue; }

                        let val = t0.borrow().get_source_text(&sofa).to_string();
                        if val.len() < 5 { continue; }

                        let rt = embed_uri(kit, &mut dedup, sch, &val, t.clone(), t0);
                        cur = rt.borrow().next.clone();
                    }

                    // ── i=7: Cadastre number ─────────────────────────────────
                    7 => {
                        let mut t0 = tt.borrow().next.clone();
                        while let Some(ref n) = t0.clone() {
                            let (skip, next) = {
                                let nb = n.borrow();
                                let skip = nb.is_char_of(":|", &sofa)
                                    || nb.is_table_control_char(&sofa)
                                    || nb.is_hiphen(&sofa);
                                let next = nb.next.clone();
                                (skip, next)
                            };
                            if skip { t0 = next; } else { break; }
                        }
                        let t0 = match t0 { None => continue, Some(x) => x };
                        if let Some((val, _begin, end)) = try_attach_kadastr(&t0, &sofa) {
                            let rt = embed_uri(kit, &mut dedup, "КАДАСТР", &val, t.clone(), end);
                            cur = rt.borrow().next.clone();
                        }
                    }

                    _ => {}
                }
                continue;
            }

            // ── Email '@' detection ──────────────────────────────────────────
            if t.borrow().is_char('@', &sofa) {
                let prev = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
                let u1s = match prev.as_ref().and_then(|p| attach_mail_users(p, &sofa)) {
                    None => continue,
                    Some(u) => u,
                };
                let after_at = t.borrow().next.clone();
                let u2 = match after_at.as_ref().and_then(|a| attach_domain_name(a, &sofa, false, true)) {
                    None => continue,
                    Some(u) => u,
                };

                for (ii, u1) in u1s.iter().enumerate().rev() {
                    let email_val = format!("{}@{}", u1.value, u2.value).to_ascii_lowercase();

                    // Extend begin backwards for "E-MAIL", "ПОЧТА", etc.
                    let mut b = u1.begin_token.clone();
                    let prev_b = b.borrow().prev.as_ref().and_then(|w| w.upgrade());
                    if let Some(pb) = prev_b {
                        if pb.borrow().is_char(':', &sofa) {
                            if let Some(ppb) = pb.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
                                if ppb.borrow().is_value("EMAIL", None)
                                    || ppb.borrow().is_value("MAILTO", None)
                                    || ppb.borrow().is_value("MAIL", None)
                                    || ppb.borrow().is_value("ПОЧТА", None)
                                    || ppb.borrow().is_value("АДРЕС", None)
                                {
                                    b = ppb;
                                }
                            }
                        }
                    }

                    let end_tok = if ii == u1s.len() - 1 {
                        u2.end_token.clone()
                    } else {
                        u1.end_token.clone()
                    };

                    let rt = embed_uri(kit, &mut dedup, "mailto", &email_val, b, end_tok);
                    cur = rt.borrow().next.clone();
                }
                continue;
            }

            // ── Plain domain URL detection ───────────────────────────────────
            if !t.borrow().chars.is_cyrillic_letter() {
                let (is_ws, prev_is_comma_open) = {
                    let tb = t.borrow();
                    let ws = tb.is_whitespace_before(&sofa);
                    let prev_co = tb.prev.as_ref()
                        .and_then(|w| w.upgrade())
                        .map_or(false, |p| p.borrow().is_char_of(",(", &sofa));
                    (ws, prev_co)
                };
                if is_ws || prev_is_comma_open {
                    if let Some(u1) = attach_url(&t, &sofa) {
                        let not_mail = u1.end_token.borrow().next.clone()
                            .map_or(true, |n| !n.borrow().is_char('@', &sofa));
                        if not_mail {
                            let (mut begin, mut end) = (u1.begin_token.clone(), u1.end_token.clone());
                            if end.borrow().next.clone().map_or(false, |n| n.borrow().is_char_of("/\\", &sofa)) {
                                // check if next slash leads to more content
                                let slash_after = end.borrow().next.clone().unwrap();
                                if let Some(more) = attach_uri_content_inner_pub(&t, &sofa) {
                                    end = more.end_token.clone();
                                    let _ = more;
                                } else {
                                    end = slash_after;
                                }
                            }
                            let prev_begin = begin.borrow().prev.as_ref().and_then(|w| w.upgrade());
                            if let Some(sb) = site_before(prev_begin, &sofa) {
                                begin = sb;
                            }
                            let rt = embed_uri(kit, &mut dedup, "http", &u1.value, begin, end);
                            cur = rt.borrow().next.clone();
                            continue;
                        }
                    }
                }
            }

            // ── Text token followed immediately by URL content ───────────────
            {
                let (is_text, not_ws_after, len_ok) = {
                    let tb = t.borrow();
                    let is_text = matches!(&tb.kind, TokenKind::Text(_));
                    let not_ws = !tb.is_whitespace_after(&sofa);
                    let len = tb.length_char() > 2;
                    (is_text, not_ws, len)
                };
                if is_text && not_ws_after && len_ok {
                    let sb = site_before(t.borrow().prev.as_ref().and_then(|w| w.upgrade()), &sofa);
                    if sb.is_some() {
                        if let Some(ut) = attach_uri_content(&t, &sofa, true) {
                            let has_dot = ut.value.contains('.');
                            let has_at = ut.value.contains('@');
                            if has_dot && !has_at {
                                let mut end = ut.end_token.clone();
                                if end.borrow().next.clone().map_or(false, |n| n.borrow().is_char_of("/\\", &sofa)) {
                                    let ne = end.borrow().next.clone().unwrap();
                                    end = ne;
                                }
                                let rt = embed_uri(kit, &mut dedup, "http", &ut.value, sb.unwrap(), end);
                                cur = rt.borrow().next.clone();
                                continue;
                            }
                        }
                    }
                }
            }

            // ── Latin uppercase text followed by slash (Lotus notes path) ────
            {
                let (is_latin, is_all_upper, has_next, next_is_slash) = {
                    let tb = t.borrow();
                    let is_lat = tb.chars.is_latin_letter() && !tb.chars.is_all_lower();
                    let is_up = tb.chars.is_all_upper();
                    let has_next = tb.next.is_some();
                    let slash = tb.next.as_ref()
                        .map_or(false, |n| n.borrow().is_char('/', &sofa));
                    (is_lat, is_up, has_next, slash)
                };
                if is_latin && is_all_upper && has_next && next_is_slash {
                    if let Some((val, begin, end)) = try_attach_lotus(&t, &sofa) {
                        let rt = embed_uri(kit, &mut dedup, "lotus", &val, begin, end);
                        cur = rt.borrow().next.clone();
                        continue;
                    }
                }
            }

            // ── Short digit token followed by ':' (kadastr pattern) ──────────
            {
                let is_kadastr_start = {
                    let tb = t.borrow();
                    matches!(&tb.kind, TokenKind::Number(n) if n.spelling_type == NumberSpellingType::Digit)
                        && t.borrow().length_char() < 3
                        && t.borrow().next.clone().map_or(false, |n| n.borrow().is_char(':', &sofa))
                        && !t.borrow().is_whitespace_after(&sofa)
                        && t.borrow().next.clone().map_or(false, |n| {
                            !n.borrow().is_whitespace_after(&sofa)
                                && n.borrow().next.clone().map_or(false, |nn| {
                                    matches!(&nn.borrow().kind, TokenKind::Number(_))
                                })
                        })
                };
                if is_kadastr_start {
                    if let Some((val, begin, end)) = try_attach_kadastr(&t, &sofa) {
                        let rt = embed_uri(kit, &mut dedup, "КАДАСТР", &val, begin, end);
                        cur = rt.borrow().next.clone();
                        continue;
                    }
                }
            }

            // ── КАДАСТРОВЫЙ keyword ──────────────────────────────────────────
            if t.borrow().is_value("КАДАСТРОВЫЙ", None) {
                let mut tt = t.borrow().next.clone();
                // Skip "(КАДАСТРОВЫЙ)" — can't assign to tt inside if-let that borrows it
                let skip_paren: Option<Option<TokenRef>> = if let Some(ref n) = tt {
                    if n.borrow().is_char('(', &sofa) {
                        let inner_opt = n.borrow().next.clone();
                        if let Some(inner) = inner_opt {
                            let close_opt = inner.borrow().next.clone();
                            if let Some(close) = close_opt {
                                if close.borrow().is_char(')', &sofa) {
                                    Some(close.borrow().next.clone())
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None };
                if let Some(v) = skip_paren { tt = v; }
                // Skip "НОМЕР" / "НОМ." prefix
                let skip_nomor: Option<Option<TokenRef>> = if let Some(ref n) = tt {
                    if n.borrow().is_value("НОМЕР", None) || n.borrow().is_value("НОМ", None) {
                        let next_tt = n.borrow().next.clone();
                        // skip trailing dot
                        let next_tt = if next_tt.as_ref().map_or(false, |nd| nd.borrow().is_char('.', &sofa)) {
                            next_tt.as_ref().and_then(|nd| nd.borrow().next.clone())
                        } else { next_tt };
                        Some(next_tt)
                    } else { None }
                } else { None };
                if let Some(v) = skip_nomor { tt = v; }
                // Skip hyphen or table char
                let skip_hiphen: Option<Option<TokenRef>> = if let Some(ref n) = tt {
                    if n.borrow().is_hiphen(&sofa) || n.borrow().is_table_control_char(&sofa) {
                        Some(n.borrow().next.clone())
                    } else { None }
                } else { None };
                if let Some(v) = skip_hiphen { tt = v; }
                if let Some(ref t1) = tt {
                    if let Some((val, _, end)) = try_attach_kadastr(t1, &sofa) {
                        let rt = embed_uri(kit, &mut dedup, "КАДАСТР", &val, t.clone(), end);
                        cur = rt.borrow().next.clone();
                    }
                }
            }
        }
    }
}

/// Public wrapper for the inner content parser (used by the analyzer's plain-URL path)
fn attach_uri_content_inner_pub(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<super::uri_item_token::UriItemToken> {
    attach_uri_content(t0, sofa, false)
}

/// Try to parse a Lotus-Notes-style path (e.g. "ABC DEF/GHI/JKL")
fn try_attach_lotus(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(String, TokenRef, TokenRef)> {
    let next = t.borrow().next.clone()?; // the '/' token
    let after_slash = next.borrow().next.clone()?;

    // Collect tail segments (uppercase non-all-lower tokens separated by '/')
    let mut tails: Vec<String> = Vec::new();
    let mut t1 = after_slash.clone();
    {
        let mut cur = after_slash.clone();
        loop {
            let (is_ws, is_nl, is_letter, is_all_lower, term, is_text, next_tok) = {
                let tb = cur.borrow();
                let ws = tb.is_whitespace_before(sofa);
                let nl = tb.is_newline_before(sofa);
                let is_let = tb.is_letters();
                let is_low = tb.chars.is_all_lower();
                let term = tb.term().map(|s| s.to_string());
                let is_text = matches!(&tb.kind, TokenKind::Text(_));
                let next = tb.next.clone();
                (ws, nl, is_let, is_low, term, is_text, next)
            };

            if is_ws {
                if !is_nl && !tails.is_empty() {
                    // stop scanning but we already have enough
                }
                break;
            }
            if !is_text || !is_letter || is_all_lower { return None; }
            let term = match term { None => return None, Some(s) => s };
            tails.push(term);
            t1 = cur.clone();

            let ws_after = cur.borrow().is_whitespace_after(sofa);
            if ws_after || next_tok.is_none() { break; }
            let slash_tok = next_tok.unwrap();
            if !slash_tok.borrow().is_char('/', sofa) { break; }
            let after = slash_tok.borrow().next.clone();
            cur = match after { None => break, Some(n) => n };
        }
    }
    if tails.len() < 3 { return None; }

    // Collect head segments (uppercase tokens before the first '/')
    let mut heads: Vec<String> = Vec::new();
    let t_term = t.borrow().term().map(|s| s.to_string())?;
    heads.push(t_term.clone());
    let mut t0 = t.clone();

    for k in 0..2i32 {
        let prev = t0.borrow().prev.as_ref().and_then(|w| w.upgrade());
        let p = match prev { None => break, Some(x) => x };
        let (is_text, same_chars, is_single_upper, ws_count, next_is_slash) = {
            let pb = p.borrow();
            let is_text = matches!(&pb.kind, TokenKind::Text(_));
            let same = pb.chars.is_all_upper() == t.borrow().chars.is_all_upper();
            let single_upper = pb.chars.is_latin_letter() && pb.chars.is_all_upper() && pb.length_char() == 1;
            let ws = t0.borrow().whitespaces_before_count(sofa);
            let slash = !t0.borrow().is_whitespace_before(sofa)
                && t0.borrow().prev.as_ref().and_then(|w| w.upgrade())
                    .map_or(false, |pp| pp.borrow().is_char('/', sofa));
            (is_text, same, single_upper, ws, slash)
        };
        if !is_text { break; }
        if next_is_slash { break; }
        if same_chars && (ws_count == 1 || (ws_count == 10 && k == 0)) {
            let pt = p.borrow().term().map(|s| s.to_string()).unwrap_or_default();
            heads.insert(0, pt);
            t0 = p;
        } else if is_single_upper && k == 0 {
            let pt = p.borrow().term().map(|s| s.to_string()).unwrap_or_default();
            heads.insert(0, pt);
            t0 = p;
        } else { break; }
    }

    let mut val = String::new();
    for (i, h) in heads.iter().enumerate() {
        if i > 0 { val.push(' '); }
        // Capitalize first char
        let mut c = h.chars();
        if let Some(f) = c.next() {
            val.push(f.to_ascii_uppercase());
            val.push_str(&c.as_str().to_ascii_lowercase());
        }
    }
    for tail in &tails {
        val.push('/');
        val.push_str(tail);
    }

    Some((val, t0, t1))
}
