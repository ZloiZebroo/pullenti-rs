/// MailLine — a single line of an e-mail message with its detected type.
///
/// Simplified port of `Pullenti.Ner.Mail.Internal.MailLine`.
/// We skip the NounPhraseHelper BestRegards deep analysis and
/// PersonItemToken hello-name analysis; we rely solely on termin matching.

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::OnceLock;

use crate::referent::Referent;
use crate::token::{TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::core::termin::{Termin, TerminCollection};

// ── MailLineType ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MailLineType {
    #[default]
    Undefined = 0,
    Hello = 1,
    BestRegards = 2,
    From = 3,
}

// ── MailLine ──────────────────────────────────────────────────────────────

pub struct MailLine {
    pub begin_token: TokenRef,
    pub end_token: TokenRef,
    pub typ: MailLineType,
    /// Entity references found inside this line
    pub refs: Vec<Rc<RefCell<Referent>>>,
    /// Quote nesting level (number of leading `>` or `|` chars)
    pub lev: i32,
    /// Whether this line signals the start of a forwarded message
    pub must_be_first_line: bool,
}

impl MailLine {
    pub fn begin_char(&self) -> i32 {
        self.begin_token.borrow().begin_char
    }

    pub fn end_char(&self) -> i32 {
        self.end_token.borrow().end_char
    }

    /// Count "words" in the line: text tokens with length > 2 that are letters,
    /// excluding tokens that have `tag` set (they were consumed by termin matching).
    pub fn words(&self, _sofa: &SourceOfAnalysis) -> i32 {
        let end_char = self.end_char();
        let mut t = Some(self.begin_token.clone());
        let mut cou = 0i32;
        while let Some(tok) = t {
            {
                let tb = tok.borrow();
                if tb.end_char > end_char { break; }
                if matches!(&tb.kind, TokenKind::Text(_))
                    && tb.chars.is_letter()
                    && tb.length_char() > 2
                    && tb.tag.is_none()
                {
                    cou += 1;
                }
            }
            t = tok.borrow().next.clone();
        }
        cou
    }

    /// Check whether the first text token has term "FROM" or "ОТ".
    pub fn is_real_from(&self) -> bool {
        let tb = self.begin_token.borrow();
        match tb.term() {
            Some("FROM") | Some("ОТ") => true,
            _ => false,
        }
    }

    /// Return a URI referent with scheme "mailto" if one exists on this line, else None.
    pub fn mail_addr(&self) -> Option<Rc<RefCell<Referent>>> {
        let end_char = self.end_char();
        let mut t = Some(self.begin_token.clone());
        while let Some(tok) = t {
            {
                let tb = tok.borrow();
                if tb.end_char > end_char { break; }
                if let Some(r) = tb.get_referent() {
                    let rb = r.borrow();
                    if rb.type_name == "URI" {
                        if rb.get_string_value("SCHEME") == Some("mailto") {
                            return Some(r.clone());
                        }
                    }
                }
            }
            t = tok.borrow().next.clone();
        }
        None
    }
}

// ── Static termin collections (initialized once) ──────────────────────────

struct MailTermins {
    regard_words: TerminCollection,
    from_words:   TerminCollection,
    hello_words:  TerminCollection,
}

static MAIL_TERMINS: OnceLock<MailTermins> = OnceLock::new();

fn get_termins() -> &'static MailTermins {
    MAIL_TERMINS.get_or_init(|| {
        let mut regard_words = TerminCollection::new();
        for s in &[
            "УВАЖЕНИЕ", "ПОЧТЕНИЕ", "С УВАЖЕНИЕМ", "ПОЖЕЛАНИE", "ДЕНЬ",
            "ХОРОШЕГО ДНЯ", "ИСКРЕННЕ ВАШ", "УДАЧА", "СПАСИБО", "ЦЕЛОВАТЬ",
            "ПОВАГА", "З ПОВАГОЮ", "ПОБАЖАННЯ", "ЩИРО ВАШ", "ДЯКУЮ", "ЦІЛУВАТИ",
            "BEST REGARDS", "REGARDS", "BEST WISHES", "KIND REGARDS",
            "GOOD BYE", "BYE", "THANKS", "THANK YOU", "MANY THANKS",
            "DAY", "VERY MUCH", "HAVE", "LUCK",
            "YOURS SINCERELY", "SINCERELY YOURS", "LOOKING FORWARD", "AR CIEŅU",
        ] {
            regard_words.add(Termin::new(s.to_uppercase()));
        }

        let mut from_words = TerminCollection::new();
        for s in &[
            "FROM", "TO", "CC", "SENT", "SUBJECT", "SENDER", "TIME",
            "ОТ КОГО", "КОМУ", "ДАТА", "ТЕМА", "КОПИЯ", "ОТ", "ОТПРАВЛЕНО",
            "WHEN", "WHERE",
        ] {
            from_words.add(Termin::new(*s));
        }

        let mut hello_words = TerminCollection::new();
        for s in &[
            "HI", "HELLO", "DEAR",
            "GOOD MORNING", "GOOD DAY", "GOOD EVENING", "GOOD NIGHT",
            "ЗДРАВСТВУЙ", "ЗДРАВСТВУЙТЕ", "ПРИВЕТСТВУЮ", "ПРИВЕТ", "ПРИВЕТИК",
            "УВАЖАЕМЫЙ", "ДОРОГОЙ", "ЛЮБЕЗНЫЙ",
            "ГЛУБОКОУВАЖАЕМЫЙ", "ГЛУБОКО УВАЖАЕМЫЙ",
            "ДОБРОЕ УТРО", "ДОБРЫЙ ДЕНЬ", "ДОБРЫЙ ВЕЧЕР", "ДОБРОЙ НОЧИ",
            "ЗДРАСТУЙ", "ЗДРАСТУЙТЕ",
            "ВІТАЮ", "ПРИВІТ", "ШАНОВНИЙ", "ДОРОГИЙ", "ЛЮБИЙ",
            "ДОБРОГО РАНКУ", "ДОБРИЙ ДЕНЬ", "ДОБРИЙ ВЕЧІР", "ДОБРОЇ НОЧІ",
        ] {
            hello_words.add(Termin::new(s.to_uppercase()));
        }

        MailTermins { regard_words, from_words, hello_words }
    })
}

// ── Parse ─────────────────────────────────────────────────────────────────

/// Parse one line starting at `t0` (stops at the next newline).
///
/// Returns None if `t0` is None or is not at the start of a newline
/// (i.e. the caller should only call this when `t0` is the first token
/// or is preceded by a newline).
pub fn parse(t0: &TokenRef, sofa: &SourceOfAnalysis) -> Option<MailLine> {
    let tc = get_termins();

    let mut ml = MailLine {
        begin_token: t0.clone(),
        end_token: t0.clone(),
        typ: MailLineType::Undefined,
        refs: Vec::new(),
        lev: 0,
        must_be_first_line: false,
    };

    // ── Walk to end of line ───────────────────────────────────────────
    let mut pr = true; // still in prefix (leading `>` / `|` chars)
    let mut t_cur = Some(t0.clone());

    while let Some(tok) = t_cur.clone() {
        // Stop at next newline (but not the very first token)
        {
            let tb = tok.borrow();
            let is_first = Rc::ptr_eq(&tok, t0);
            if !is_first && tb.is_newline_before(sofa) {
                break;
            }
        }

        // Update end_token
        ml.end_token = tok.clone();

        {
            let tb = tok.borrow();

            if tb.is_table_control_char(sofa) || tb.is_hiphen(sofa) {
                t_cur = tb.next.clone();
                continue;
            }

            if pr {
                // Count quote-level markers
                if matches!(&tb.kind, TokenKind::Text(_))
                    && tb.is_char_of(">|", sofa)
                {
                    ml.lev += 1;
                } else {
                    pr = false;
                    // Check for "From: " / "To: " pattern
                    drop(tb);
                    if let Some(tt) = tc.from_words.try_parse(&tok) {
                        // The next token after the match must be ':'
                        let next_is_colon = tt.end_token.borrow().next.as_ref().map_or(false, |n| {
                            n.borrow().is_char(':', sofa)
                        });
                        if next_is_colon {
                            ml.typ = MailLineType::From;
                            // Advance past "From:"
                            let colon = tt.end_token.borrow().next.clone().unwrap();
                            t_cur = colon.borrow().next.clone();
                            continue;
                        }
                    }
                    t_cur = tok.borrow().next.clone();
                    continue;
                }
                t_cur = tok.borrow().next.clone();
                continue;
            }

            // Collect entity references
            if let Some(r) = tb.get_referent() {
                let rtype = r.borrow().type_name.clone();
                if matches!(rtype.as_str(),
                    "PERSON" | "GEO" | "ADDRESS" | "PHONE" | "URI" | "ORGANIZATION"
                ) {
                    // avoid double-borrow: drop tb first
                    drop(tb);
                    ml.refs.push(r);
                    t_cur = tok.borrow().next.clone();
                    continue;
                }
            }
        }

        t_cur = tok.borrow().next.clone();
    }

    // ── Detect Hello ──────────────────────────────────────────────────
    if ml.typ == MailLineType::Undefined {
        let end_char = ml.end_char();
        let mut t = Some(t0.clone());

        // Skip leading non-letter tokens
        while let Some(tok) = t.clone() {
            let tb = tok.borrow();
            if tb.end_char >= end_char { break; }
            if !tb.is_hiphen(sofa) && tb.chars.is_letter() { break; }
            drop(tb);
            t = tok.borrow().next.clone();
        }

        let mut ok = 0i32;
        let mut oth = 0i32;
        let mut last_comma: Option<TokenRef> = None;

        while let Some(tok) = t.clone() {
            let tb = tok.borrow();
            if tb.end_char >= end_char { break; }

            // Skip person referents
            if let Some(r) = tb.get_referent() {
                if r.borrow().type_name == "PERSON" {
                    drop(tb);
                    t = tok.borrow().next.clone();
                    continue;
                }
            }

            if matches!(&tb.kind, TokenKind::Text(_)) {
                if !tb.chars.is_letter() {
                    last_comma = Some(tok.clone());
                    drop(tb);
                    t = tok.borrow().next.clone();
                    continue;
                }

                // Check for hello termin
                drop(tb);
                if let Some(tt) = tc.hello_words.try_parse(&tok) {
                    // Special-case ДОРОГОЙ: only accept if term is exactly
                    // ДОРОГОЙ / ДОРОГАЯ / ДОРОГИЕ
                    let term = tok.borrow().term().unwrap_or("").to_string();
                    let is_dorogoy = term == "ДОРОГОЙ" || term == "ДОРОГАЯ" || term == "ДОРОГИЕ";
                    let tok_is_dorogoy = {
                        let tb2 = tok.borrow();
                        tb2.is_value("ДОРОГОЙ", None)
                    };
                    let valid = if tok_is_dorogoy { is_dorogoy } else { true };

                    if valid {
                        ok += 1;
                        let end_tok = tt.end_token.clone();
                        t = end_tok.borrow().next.clone();
                        continue;
                    }
                }

                // Check for "ВСЕ" / "ALL" / "TEAM"
                let term_str = tok.borrow().term().unwrap_or("").to_string();
                if term_str == "ВСЕ" || term_str == "ALL" || term_str == "TEAM" {
                    t = tok.borrow().next.clone();
                    continue;
                }

                // Other word — counts as "other"
                oth += 1;
                if oth > 3 {
                    if ok > 0 {
                        if let Some(ref lc) = last_comma {
                            ml.end_token = lc.clone();
                            oth = 0;
                        }
                    }
                    break;
                }
                t = tok.borrow().next.clone();
                continue;
            }

            oth += 1;
            if oth > 3 { break; }
            drop(tb);
            t = tok.borrow().next.clone();
        }

        if oth < 3 && ok > 0 {
            ml.typ = MailLineType::Hello;
        }
    }

    // ── Detect BestRegards ────────────────────────────────────────────
    if ml.typ == MailLineType::Undefined {
        let end_char = ml.end_char();
        let mut ok_words = 0i32;
        let mut t: Option<TokenRef> = Some(t0.clone());

        while let Some(tok) = t.clone() {
            let tb = tok.borrow();
            if tb.end_char > end_char { break; }

            if !matches!(&tb.kind, TokenKind::Text(_)) {
                drop(tb);
                t = tok.borrow().next.clone();
                continue;
            }

            if tb.is_char('<', sofa) {
                // Skip angle-bracket spans (e.g. <email@example.com>)
                drop(tb);
                // Simple skip: advance until '>'
                let mut ti = tok.borrow().next.clone();
                let mut found_close = false;
                while let Some(inner) = ti.clone() {
                    let ib = inner.borrow();
                    if ib.is_char('>', sofa) {
                        found_close = true;
                        drop(ib);
                        t = inner.borrow().next.clone();
                        break;
                    }
                    if ib.end_char > end_char { break; }
                    drop(ib);
                    ti = inner.borrow().next.clone();
                }
                if !found_close {
                    t = ti;
                }
                continue;
            }

            if !tb.chars.is_letter() || tb.is_table_control_char(sofa) {
                drop(tb);
                t = tok.borrow().next.clone();
                continue;
            }

            drop(tb);
            // Try regard_words termin
            if let Some(tt) = tc.regard_words.try_parse(&tok) {
                ok_words += 1;
                let end_tok = tt.end_token.clone();
                t = end_tok.borrow().next.clone();
                continue;
            }

            // Try prepositions / conjunctions / misc
            {
                let tb2 = tok.borrow();
                if tb2.morph.items().iter().any(|wf| {
                    wf.base.class.is_preposition()
                        || wf.base.class.is_conjunction()
                        || wf.base.class.is_misc()
                }) || tb2.is_value("C", None)
                {
                    drop(tb2);
                    t = tok.borrow().next.clone();
                    continue;
                }
            }

            // Otherwise reset
            if ok_words > 0 {
                // check if preceding comma + uppercase → trim
                let prev_is_comma = tok.borrow().prev.as_ref()
                    .and_then(|w| w.upgrade())
                    .map_or(false, |p| p.borrow().is_char(',', sofa));
                let prev_after_start = tok.borrow().prev.as_ref()
                    .and_then(|w| w.upgrade())
                    .map_or(false, |p| p.borrow().begin_char > t0.borrow().begin_char);
                let not_all_lower = {
                    let tb2 = tok.borrow();
                    !tb2.chars.is_all_lower()
                };
                if prev_is_comma && prev_after_start && not_all_lower {
                    // trim line to previous comma
                    if let Some(p) = tok.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
                        ml.end_token = p.clone();
                    }
                    break;
                }
            }

            ok_words = 0;
            t = tok.borrow().next.clone();
        }

        if ok_words > 0 {
            ml.typ = MailLineType::BestRegards;
        }
    }

    // ── Detect forwarded message headers ─────────────────────────────
    if ml.typ == MailLineType::Undefined {
        let end_char = ml.end_char();
        let mut t: Option<TokenRef> = Some(t0.clone());

        // Skip non-letter prefix
        while let Some(tok) = t.clone() {
            let tb = tok.borrow();
            if tb.end_char >= end_char { break; }
            if !tb.is_hiphen(sofa) && tb.chars.is_letter() { break; }
            drop(tb);
            t = tok.borrow().next.clone();
        }

        if let Some(tok) = t {
            let tb = tok.borrow();
            if (tb.is_value("ПЕРЕСЫЛАЕМОЕ", None) || tb.is_value("ПЕРЕАДРЕСОВАННОЕ", None))
                && tb.next.as_ref().map_or(false, |n| n.borrow().is_value("СООБЩЕНИЕ", None))
            {
                ml.typ = MailLineType::From;
                ml.must_be_first_line = true;
            } else if tb.is_value("НАЧАЛО", None) {
                let fwd = tb.next.as_ref().map_or(false, |n| {
                    let nb = n.borrow();
                    nb.is_value("ПЕРЕСЫЛАЕМОЕ", None) || nb.is_value("ПЕРЕАДРЕСОВАННОЕ", None)
                });
                let msg = tb.next.as_ref()
                    .and_then(|n| n.borrow().next.clone())
                    .map_or(false, |nn| nn.borrow().is_value("СООБЩЕНИЕ", None));
                if fwd && msg {
                    ml.typ = MailLineType::From;
                    ml.must_be_first_line = true;
                }
            } else if tb.is_value("ORIGINAL", None) {
                let next_ok = tb.next.as_ref().map_or(false, |n| {
                    let nb = n.borrow();
                    nb.is_value("MESSAGE", None) || nb.is_value("APPOINTMENT", None)
                });
                if next_ok {
                    ml.typ = MailLineType::From;
                    ml.must_be_first_line = true;
                }
            } else if tb.is_value("ПЕРЕСЛАНО", None)
                && tb.next.as_ref().map_or(false, |n| n.borrow().is_value("ПОЛЬЗОВАТЕЛЕМ", None))
            {
                ml.typ = MailLineType::From;
                ml.must_be_first_line = true;
            }
        }
    }

    Some(ml)
}
