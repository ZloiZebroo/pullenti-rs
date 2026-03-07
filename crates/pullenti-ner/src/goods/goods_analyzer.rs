/// GoodsAnalyzer — detects product/goods entities with their attributes.
/// Mirrors `GoodsAnalyzer.cs` and the key patterns from `GoodAttrToken.cs`.
///
/// This is a SPECIFIC analyzer (is_specific = true): it processes each
/// newline-separated line/block independently, extracting a GOOD referent
/// that aggregates all the detected attributes on that line.

use std::rc::Rc;
use std::cell::RefCell;

use pullenti_morph::{MorphCase, MorphNumber, MorphGenderFlags};

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::referent::{Referent, SlotValue};
use crate::source_of_analysis::SourceOfAnalysis;

use super::good_referent::{
    ATTR_ATTR, ATTR_VALUE, ATTR_REF,
    GoodAttrType,
    new_good_referent, new_goodattr_referent,
    set_attr_type, add_attr_value, add_attr_altvalue, set_attr_name, set_attr_ref,
};

pub struct GoodsAnalyzer;

impl GoodsAnalyzer {
    pub fn new() -> Self { GoodsAnalyzer }
}

impl Default for GoodsAnalyzer {
    fn default() -> Self { GoodsAnalyzer }
}

// ── Internal parsed attribute ──────────────────────────────────────────────

struct GoodAttrTok {
    begin_token: TokenRef,
    end_token:   TokenRef,
    typ:         GoodAttrType,
    value:       Option<String>,
    alt_value:   Option<String>,
    name:        Option<String>,
    ref_r:       Option<Rc<RefCell<Referent>>>,
}

impl GoodAttrTok {
    fn new(begin: TokenRef, end: TokenRef, typ: GoodAttrType) -> Self {
        GoodAttrTok {
            begin_token: begin,
            end_token:   end,
            typ,
            value:       None,
            alt_value:   None,
            name:        None,
            ref_r:       None,
        }
    }

    /// Build a GoodAttributeReferent from this parsed attribute token.
    fn create_attr(&self) -> Option<Referent> {
        let mut ar = new_goodattr_referent();
        if self.typ != GoodAttrType::Undefined {
            set_attr_type(&mut ar, self.typ);
        }
        if let Some(ref n) = self.name {
            set_attr_name(&mut ar, n.as_str());
        }
        if let Some(ref ref_r) = self.ref_r {
            set_attr_ref(&mut ar, ref_r.clone());
        }
        if let Some(ref v) = self.value {
            let v1 = if self.typ == GoodAttrType::Proper {
                v.to_uppercase()
            } else {
                v.clone()
            };
            if !v1.is_empty() {
                add_attr_value(&mut ar, v1);
            }
        }
        if let Some(ref av) = self.alt_value {
            let av1 = if self.typ == GoodAttrType::Proper {
                av.to_uppercase()
            } else {
                av.clone()
            };
            if !av1.is_empty() {
                add_attr_altvalue(&mut ar, av1);
            }
        }
        // Must have at least a value or a ref
        if ar.find_slot(ATTR_VALUE, None).is_none() && ar.find_slot(ATTR_REF, None).is_none() {
            return None;
        }
        Some(ar)
    }
}

// ── Analyzer impl ──────────────────────────────────────────────────────────

impl Analyzer for GoodsAnalyzer {
    fn name(&self) -> &'static str { "GOODS" }
    fn caption(&self) -> &'static str { "Товары и атрибуты" }
    fn is_specific(&self) -> bool { true }
    fn progress_weight(&self) -> i32 { 100 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();

        while let Some(t) = cur.clone() {
            // Only process tokens that start at a newline boundary
            if !t.borrow().is_newline_before(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            // If this token has no letters, try the next one
            let start_t: TokenRef = {
                let tb = t.borrow();
                if !tb.chars.is_letter() {
                    match tb.next.clone() {
                        None => {
                            drop(tb);
                            cur = None;
                            continue;
                        }
                        Some(next) => {
                            drop(tb);
                            next
                        }
                    }
                } else {
                    drop(tb);
                    t.clone()
                }
            };

            // Try to parse attributes from this line
            let attrs = try_parse_attr_list(&start_t, &sofa);
            if attrs.is_empty() {
                cur = t.borrow().next.clone();
                continue;
            }

            // Build GOODATTR referent tokens
            let mut attr_refs: Vec<(Rc<RefCell<Referent>>, TokenRef, TokenRef)> = Vec::new();
            for a in &attrs {
                if let Some(attr) = a.create_attr() {
                    let r_rc = Rc::new(RefCell::new(attr));
                    kit.add_entity(r_rc.clone());
                    attr_refs.push((r_rc, a.begin_token.clone(), a.end_token.clone()));
                }
            }

            if attr_refs.is_empty() {
                cur = t.borrow().next.clone();
                continue;
            }

            // Embed each attribute as a referent token
            let mut embedded: Vec<TokenRef> = Vec::new();
            for (r_rc, begin, end) in &attr_refs {
                let tok = Rc::new(RefCell::new(
                    Token::new_referent(begin.clone(), end.clone(), r_rc.clone())
                ));
                kit.embed_token(tok.clone());
                embedded.push(tok);
            }

            // Build the parent GOOD referent
            let mut good = new_good_referent();
            for (r_rc, _, _) in &attr_refs {
                let already = good.slots.iter().any(|s| {
                    if s.type_name != ATTR_ATTR { return false; }
                    if let Some(ref sv) = s.value {
                        if let Some(ref er) = sv.as_referent() {
                            return Rc::ptr_eq(er, r_rc);
                        }
                    }
                    false
                });
                if !already {
                    good.add_slot(ATTR_ATTR, SlotValue::Referent(r_rc.clone()), false);
                }
            }

            let good_rc = Rc::new(RefCell::new(good));
            kit.add_entity(good_rc.clone());

            let first_emb = embedded.first().unwrap().clone();
            let last_emb  = embedded.last().unwrap().clone();
            let good_tok = Rc::new(RefCell::new(
                Token::new_referent(first_emb, last_emb, good_rc)
            ));
            kit.embed_token(good_tok.clone());

            cur = good_tok.borrow().next.clone();
        }
    }
}

// ── Attribute list parser ──────────────────────────────────────────────────

fn try_parse_attr_list(start: &TokenRef, sofa: &SourceOfAnalysis) -> Vec<GoodAttrTok> {
    let mut res: Vec<GoodAttrTok> = Vec::new();
    let mut key_val: Option<String> = None; // value of the first Keyword

    let mut cur: Option<TokenRef> = Some(start.clone());

    while let Some(t) = cur.clone() {
        // Stop at newline (except the very first token)
        {
            let tb = t.borrow();
            if !Rc::ptr_eq(&t, start) && tb.is_newline_before(sofa) {
                break;
            }
        }

        let next = t.borrow().next.clone();

        // Try to parse a single attribute
        if let Some(attr) = try_parse_attr(&t, key_val.as_deref(), sofa) {
            // The first parsed attribute must be a Keyword; Numeric/Model alone are invalid
            if key_val.is_none() {
                match attr.typ {
                    GoodAttrType::Keyword => {
                        key_val = attr.value.clone();
                    }
                    GoodAttrType::Numeric | GoodAttrType::Model => {
                        // Cannot start a goods description with numeric/model
                        return Vec::new();
                    }
                    _ => {}
                }
            }
            let end_next = attr.end_token.borrow().next.clone();
            res.push(attr);
            cur = end_next;
            continue;
        }

        // Skip non-letter punctuation
        {
            let tb = t.borrow();
            if matches!(tb.kind, TokenKind::Text(_)) && !tb.chars.is_letter() {
                drop(tb);
                cur = next;
                continue;
            }
            // Skip prepositions and conjunctions
            if matches!(tb.kind, TokenKind::Text(_)) {
                let mc = tb.get_morph_class_in_dictionary();
                if mc.is_preposition() || mc.is_conjunction() {
                    drop(tb);
                    cur = next;
                    continue;
                }
            }
        }

        // Skip pure number tokens without units
        if matches!(t.borrow().kind, TokenKind::Number(_)) {
            cur = next;
            continue;
        }

        cur = next;
    }

    // Remove trailing single-token Character attrs that are adverbs
    while let Some(last) = res.last() {
        if last.typ == GoodAttrType::Character
            && Rc::ptr_eq(&last.begin_token, &last.end_token)
        {
            let mc = last.begin_token.borrow().get_morph_class_in_dictionary();
            if mc.is_adverb() {
                res.pop();
                continue;
            }
        }
        break;
    }

    res
}

// ── Single attribute parser ────────────────────────────────────────────────

fn try_parse_attr(t: &TokenRef, key_val: Option<&str>, sofa: &SourceOfAnalysis) -> Option<GoodAttrTok> {
    let tb = t.borrow();

    // ── ReferentToken ──────────────────────────────────────────────────────
    if let Some(ref_r) = tb.get_referent() {
        let type_name = ref_r.borrow().type_name.clone();
        drop(tb);
        match type_name.as_str() {
            "URI" | "DECREE" => {
                let val = {
                    let rb = ref_r.borrow();
                    rb.get_string_value("VALUE")
                        .or_else(|| rb.get_string_value("NUMBER"))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| type_name.clone())
                };
                let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Model);
                a.name  = Some("СПЕЦИФИКАЦИЯ".to_string());
                a.value = Some(val);
                return Some(a);
            }
            "GEO" | "ORGANIZATION" => {
                let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Referent);
                a.ref_r = Some(ref_r);
                return Some(a);
            }
            "GOODATTR" => {
                let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Referent);
                a.ref_r = Some(ref_r);
                return Some(a);
            }
            _ => return None,
        }
    }

    // ── NumberToken ────────────────────────────────────────────────────────
    if let TokenKind::Number(ref nd) = tb.kind {
        let num_val = nd.value.clone();
        let next = tb.next.clone();
        drop(tb);

        // Check for a short unit suffix after the number
        if let Some(ref next_t) = next {
            let nb = next_t.borrow();
            if matches!(nb.kind, TokenKind::Text(_))
                && nb.chars.is_letter()
                && nb.length_char() <= 5
                && !nb.is_whitespace_before(sofa)
            {
                // Treat as numeric with unit (e.g. "3.5%", "500мл")
                if let TokenKind::Text(ref txt) = nb.kind {
                    let unit = txt.term.to_lowercase();
                    let end_tok = next_t.clone();
                    drop(nb);
                    let mut a = GoodAttrTok::new(t.clone(), end_tok, GoodAttrType::Numeric);
                    a.value = Some(format!("{}{}", num_val, unit));
                    return Some(a);
                }
            }
        }

        // Bare number
        let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Numeric);
        a.value = Some(num_val);
        return Some(a);
    }

    // ── TextToken ──────────────────────────────────────────────────────────
    if let TokenKind::Text(ref txt) = tb.kind {
        let term         = txt.term.clone();
        let is_cyrillic  = tb.chars.is_cyrillic_letter();
        let is_latin     = tb.chars.is_latin_letter();
        let is_letter    = tb.chars.is_letter();
        let morph_class  = tb.get_morph_class_in_dictionary();
        let mc_dict      = tb.get_morph_class_in_dictionary();
        let char_len     = tb.length_char();
        let ws_before    = tb.is_whitespace_before(sofa);

        if !is_letter {
            drop(tb);
            return None;
        }

        // ── Cyrillic, no keyword yet → try Keyword ────────────────────────
        if is_cyrillic && key_val.is_none() {
            let is_noun_or_undef = morph_class.is_noun() || morph_class.is_undefined();
            // Adjectives and verbs don't start a goods description
            let not_adj_or_verb = !morph_class.is_adjective() && !morph_class.is_verb();

            if is_noun_or_undef && not_adj_or_verb {
                let norm = get_noun_normal_form(&tb);
                let all_upper = tb.chars.is_all_upper();
                drop(tb);

                // If all-upper, only accept if the whole block is all-upper
                if all_upper {
                    // Accept — all-upper block (e.g. acronyms like "МОЛОКО")
                }
                let end_tok = try_extend_hyphen_keyword(t, sofa);
                let mut a = GoodAttrTok::new(t.clone(), end_tok, GoodAttrType::Keyword);
                a.value = Some(norm);
                return Some(a);
            }
        }

        // ── Cyrillic adjective → Character ───────────────────────────────
        if is_cyrillic && (morph_class.is_adjective() || mc_dict.is_adjective()) {
            if !is_verb_form_to_skip(&term) {
                let norm = get_adj_normal_form(&tb);
                drop(tb);
                let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Character);
                a.value = Some(norm);
                return Some(a);
            }
        }

        // ── Latin with whitespace before → Proper brand name ─────────────
        if is_latin && ws_before && char_len >= 2 {
            drop(tb);
            let (end_tok, val, alt) = collect_latin_proper(t, sofa);
            if val.chars().count() >= 2 {
                let mut a = GoodAttrTok::new(t.clone(), end_tok, GoodAttrType::Proper);
                a.value     = Some(val);
                a.alt_value = alt;
                return Some(a);
            }
            return None;
        }

        // ── Short letter token + hyphen + number → Model code ─────────────
        // Pattern: "АК-47", "ПМ-5", "АК47" etc.
        if char_len <= 4 && ws_before && is_letter {
            let next_t = tb.next.clone();
            drop(tb);
            if let Some(ref next) = next_t {
                let nb = next.borrow();
                // hyphen immediately after (no whitespace)
                if nb.is_hiphen(sofa) && !nb.is_whitespace_before(sofa) {
                    let after = nb.next.clone();
                    drop(nb);
                    if let Some(ref num_t) = after {
                        let nmb = num_t.borrow();
                        if matches!(nmb.kind, TokenKind::Number(_)) && !nmb.is_whitespace_before(sofa) {
                            let num_val = nmb.number_value().map(|s| s.to_string()).unwrap_or_default();
                            drop(nmb);
                            let mut a = GoodAttrTok::new(t.clone(), num_t.clone(), GoodAttrType::Model);
                            a.value = Some(format!("{}-{}", term, num_val));
                            return Some(a);
                        }
                    }
                }
            }
            // Short term with proper casing → Proper
            if char_len > 2 {
                let is_not_lower = !term.chars().all(|c| c.is_lowercase());
                if is_not_lower {
                    let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Proper);
                    a.value = Some(term);
                    return Some(a);
                }
            }
            return None;
        }

        // ── Longer text: proper-cased → Proper ───────────────────────────
        if char_len > 2 && is_letter {
            let is_not_lower = !tb.chars.is_all_lower();
            drop(tb);
            if is_not_lower {
                let mut a = GoodAttrTok::new(t.clone(), t.clone(), GoodAttrType::Proper);
                a.value = Some(term);
                return Some(a);
            }
        } else {
            drop(tb);
        }
    } else {
        drop(tb);
    }

    None
}

// ── Helper: get nominative singular normal form for a noun ─────────────────

fn get_noun_normal_form(tb: &std::cell::Ref<crate::token::Token>) -> String {
    let wf = tb.morph.find_item(
        MorphCase::NOMINATIVE,
        MorphNumber::SINGULAR,
        MorphGenderFlags::UNDEFINED,
    );
    if let Some(wf) = wf {
        if let Some(ref nc) = wf.normal_case {
            return nc.clone();
        }
    }
    if let TokenKind::Text(ref txt) = tb.kind {
        txt.lemma.clone().unwrap_or_else(|| txt.term.clone())
    } else {
        String::new()
    }
}

// ── Helper: get masculine singular nominative form for an adjective ─────────

fn get_adj_normal_form(tb: &std::cell::Ref<crate::token::Token>) -> String {
    let wf = tb.morph.find_item(
        MorphCase::NOMINATIVE,
        MorphNumber::SINGULAR,
        MorphGenderFlags::MASCULINE,
    );
    if let Some(wf) = wf {
        if let Some(ref nc) = wf.normal_case {
            return nc.clone();
        }
    }
    if let TokenKind::Text(ref txt) = tb.kind {
        txt.lemma.clone().unwrap_or_else(|| txt.term.clone())
    } else {
        String::new()
    }
}

// ── Helper: try to extend a keyword over a hyphenated compound ─────────────
// e.g. "молоко-продукт" → Keyword(МОЛОКО-ПРОДУКТ)

fn try_extend_hyphen_keyword(t: &TokenRef, sofa: &SourceOfAnalysis) -> TokenRef {
    let next1 = t.borrow().next.clone();
    if let Some(ref n1) = next1 {
        let n1b = n1.borrow();
        if n1b.is_hiphen(sofa) && !n1b.is_whitespace_before(sofa) && !n1b.is_whitespace_after(sofa) {
            let next2 = n1b.next.clone();
            drop(n1b);
            if let Some(ref n2) = next2 {
                let n2b = n2.borrow();
                if matches!(n2b.kind, TokenKind::Text(_)) && n2b.chars.is_letter() {
                    let suffix_lower = n2b.chars.is_all_lower();
                    drop(n2b);
                    if suffix_lower {
                        return n2.clone();
                    }
                }
            }
        }
    }
    t.clone()
}

// ── Helper: collect a Latin proper-name span ───────────────────────────────

fn collect_latin_proper(t: &TokenRef, sofa: &SourceOfAnalysis) -> (TokenRef, String, Option<String>) {
    let mut end = t.clone();
    let mut parts: Vec<String> = Vec::new();

    let mut cur: Option<TokenRef> = Some(t.clone());
    while let Some(tok) = cur.clone() {
        let tb = tok.borrow();
        // Stop if whitespace before (except the first)
        if !Rc::ptr_eq(&tok, t) && tb.is_whitespace_before(sofa) {
            break;
        }
        match tb.kind {
            TokenKind::Text(ref txt) if tb.chars.is_latin_letter() => {
                parts.push(txt.term.clone());
                end = tok.clone();
            }
            TokenKind::Text(_) if !tb.chars.is_letter() => {
                // Non-letter text (punctuation) — skip but don't break
            }
            _ => break,
        }
        let nxt = tb.next.clone();
        drop(tb);
        cur = nxt;
    }

    let val = parts.join(" ");
    let alt = if val.contains(' ') {
        Some(val.replace(' ', ""))
    } else {
        None
    };
    (end, val, alt)
}

// ── Helper: verb forms that are NOT characteristics ────────────────────────

fn is_verb_form_to_skip(term: &str) -> bool {
    matches!(
        term,
        "ПРЕДНАЗНАЧИТЬ" | "ПРЕДНАЗНАЧАТЬ" | "ИЗГОТОВИТЬ" | "ИЗГОТОВЛЯТЬ"
            | "ПРИМЕНЯТЬ" | "ИСПОЛЬЗОВАТЬ" | "ИЗГОТАВЛИВАТЬ"
    )
}
