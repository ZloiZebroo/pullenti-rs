/// ConjunctionToken + ConjunctionHelper
///
/// Mirrors `ConjunctionToken.cs` / `ConjunctionHelper.cs`.

use std::sync::{Arc, OnceLock};
use crate::token::{Token, TokenRef, TokenKind};
use crate::core::termin::{Termin, TerminCollection, TerminToken};
use crate::source_of_analysis::SourceOfAnalysis;

// ── ConjunctionType ────────────────────────────────────────────────────────

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConjunctionType {
    Undefined = 0,
    Comma     = 1,
    And       = 2,
    Or        = 3,
    Not       = 4,
    But       = 5,
    If        = 6,
    Then      = 7,
    Else      = 8,
    When      = 9,
    Because   = 10,
    Include   = 11,
    Except    = 12,
}

// ── ConjunctionToken ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ConjunctionToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    pub normal:      String,
    pub typ:         ConjunctionType,
    pub is_simple:   bool,
}

impl ConjunctionToken {
    pub fn new(begin: TokenRef, end: TokenRef) -> Self {
        ConjunctionToken {
            begin_token: begin,
            end_token:   end,
            normal:      String::new(),
            typ:         ConjunctionType::Undefined,
            is_simple:   false,
        }
    }
}

// ── Ontology ───────────────────────────────────────────────────────────────

static ONTOLOGY: OnceLock<TerminCollection> = OnceLock::new();

fn tag_i32(v: i32) -> Arc<dyn std::any::Any + Send + Sync> {
    Arc::new(v)
}

fn ontology() -> &'static TerminCollection {
    ONTOLOGY.get_or_init(|| {
        let mut tc = TerminCollection::new();

        // ConjunctionType::And = 2
        let mut te = Termin::new("ТАКЖЕ");
        te.tag = Some(tag_i32(ConjunctionType::And as i32));
        te.add_variant("А ТАКЖЕ");
        te.add_variant("КАК И");
        te.add_variant("ТАК И");
        te.add_variant("А РАВНО");
        te.add_variant("А РАВНО И");
        tc.add(te);

        // ConjunctionType::If = 6
        let mut te = Termin::new("ЕСЛИ");
        te.tag = Some(tag_i32(ConjunctionType::If as i32));
        tc.add(te);

        // ConjunctionType::Then = 7
        let mut te = Termin::new("ТО");
        te.tag = Some(tag_i32(ConjunctionType::Then as i32));
        tc.add(te);

        // ConjunctionType::Else = 8
        let mut te = Termin::new("ИНАЧЕ");
        te.tag = Some(tag_i32(ConjunctionType::Else as i32));
        tc.add(te);

        // ConjunctionType::Except = 12, tag2 = true (signals verb-ending check)
        let mut te = Termin::new("ИНАЧЕ КАК");
        te.tag  = Some(tag_i32(ConjunctionType::Except as i32));
        te.tag2 = Some(tag_i32(1i32)); // signals verb-ending check
        te.add_variant("ИНАЧЕ, КАК");
        te.add_variant("ЗА ИСКЛЮЧЕНИЕМ");
        te.add_variant("ИСКЛЮЧАЯ");
        te.add_abridge("КРОМЕ");
        te.add_abridge("КРОМЕ КАК");
        te.add_abridge("КРОМЕ, КАК");
        tc.add(te);

        // ConjunctionType::Include = 11, tag2 = true
        let mut te = Termin::new("ВКЛЮЧАЯ");
        te.tag  = Some(tag_i32(ConjunctionType::Include as i32));
        te.tag2 = Some(tag_i32(1i32));
        te.add_variant("В ТОМ ЧИСЛЕ");
        tc.add(te);

        tc
    })
}

fn typ_from_tag(tok: &TerminToken) -> ConjunctionType {
    tok.termin.tag.as_ref()
        .and_then(|a| a.downcast_ref::<i32>())
        .copied()
        .map(|v| match v {
            1  => ConjunctionType::Comma,
            2  => ConjunctionType::And,
            3  => ConjunctionType::Or,
            4  => ConjunctionType::Not,
            5  => ConjunctionType::But,
            6  => ConjunctionType::If,
            7  => ConjunctionType::Then,
            8  => ConjunctionType::Else,
            9  => ConjunctionType::When,
            10 => ConjunctionType::Because,
            11 => ConjunctionType::Include,
            12 => ConjunctionType::Except,
            _  => ConjunctionType::Undefined,
        })
        .unwrap_or(ConjunctionType::Undefined)
}

// ── try_parse ──────────────────────────────────────────────────────────────

pub fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<ConjunctionToken> {
    let tb = t.borrow();
    let TokenKind::Text(ref txt) = tb.kind else { return None; };
    let term = txt.term.clone();
    drop(tb);

    // Comma — potentially followed by conjunction
    if term == "," {
        // Return comma token; caller may merge with next
        let mut c = ConjunctionToken::new(t.clone(), t.clone());
        c.typ = ConjunctionType::Comma;
        c.normal = ",".to_string();
        c.is_simple = true;
        return Some(c);
    }

    // Try multi-word conjunctions from ontology
    if let Some(tok) = ontology().try_parse(t) {
        // tag2 check: if tag2 is set, the end token must end with "АЯ" (verb form)
        if tok.termin.tag2.is_some() {
            let end_tb = tok.end_token.borrow();
            let TokenKind::Text(ref et) = end_tb.kind else { return None; };
            if et.term.ends_with("АЯ") {
                // OK
            } else if end_tb.get_morph_class_in_dictionary().is_verb() {
                // Verb: only accept if ends with АЯ
                if !et.term.ends_with("АЯ") { return None; }
            }
            drop(end_tb);
        }
        let typ = typ_from_tag(&tok);
        let mut c = ConjunctionToken::new(t.clone(), tok.end_token.clone());
        c.normal = tok.termin.canonic_text.clone();
        c.typ = typ;
        return Some(c);
    }

    // Check morph dictionary
    let mc = t.borrow().get_morph_class_in_dictionary();
    if !mc.is_conjunction() { return None; }

    if t.borrow().is_and(sofa) || t.borrow().is_or(sofa) {
        let is_or = t.borrow().is_or(sofa);
        let mut c = ConjunctionToken::new(t.clone(), t.clone());
        c.normal = term.clone();
        c.is_simple = true;
        c.typ = if is_or { ConjunctionType::Or } else { ConjunctionType::And };

        // Check for "(или)", "/ или" variant extensions
        let next_opt = t.borrow().next.clone();
        if let Some(next) = next_opt {
            let nb = next.borrow();
            if let TokenKind::Text(ref nt) = nb.kind {
                // t.Next = "(" or "/" then "ИЛИ" or "OR"
                if nb.is_char('(', sofa) {
                    if let Some(nn) = nb.next.clone() {
                        let nnb = nn.borrow();
                        if nnb.is_or(sofa) {
                            if let Some(nnn) = nnb.next.clone() {
                                let nnnb = nnn.borrow();
                                if nnnb.is_char(')', sofa) {
                                    drop(nnnb);
                                    c.end_token = nnn.clone();
                                }
                            }
                        }
                    }
                } else if nb.is_char_of("\\/", sofa) {
                    if let Some(nn) = nb.next.clone() {
                        let nnb = nn.borrow();
                        if nnb.is_or(sofa) {
                            drop(nnb);
                            c.end_token = nn.clone();
                        }
                    }
                }
            }
            drop(nb);
        }
        return Some(c);
    }

    match term.as_str() {
        "НИ" => {
            let mut c = ConjunctionToken::new(t.clone(), t.clone());
            c.normal = term;
            c.typ = ConjunctionType::Not;
            Some(c)
        }
        "А" | "НО" | "ЗАТО" | "ОДНАКО" => {
            let mut c = ConjunctionToken::new(t.clone(), t.clone());
            c.normal = term;
            c.typ = ConjunctionType::But;
            Some(c)
        }
        _ => None,
    }
}
