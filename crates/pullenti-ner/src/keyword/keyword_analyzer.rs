/// KeywordAnalyzer — extracts keywords (noun phrases, predicates, referent wrappers).
/// Mirrors `KeywordAnalyzer.cs`.

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef, TokenKind};
use crate::referent::{Referent, SlotValue};
use crate::denomination::DenominationAnalyzer;
use crate::core::noun_phrase::{try_parse as npt_try_parse, NounPhraseParseAttr};

use super::keyword_referent::{
    OBJ_TYPENAME, ATTR_VALUE, ATTR_NORMAL, ATTR_REF,
    KeywordType,
    new_keyword_referent, set_typ, add_value, add_normal, add_rank,
    compute_rank_delta, get_typ,
};

pub struct KeywordAnalyzer;

impl KeywordAnalyzer {
    pub fn new() -> Self { KeywordAnalyzer }
}

impl Default for KeywordAnalyzer {
    fn default() -> Self { KeywordAnalyzer }
}

// ── Deduplication helper ───────────────────────────────────────────────────

/// Simple dedup store for keyword referents within a document.
struct KeywordStore {
    items: Vec<Rc<RefCell<Referent>>>,
}

impl KeywordStore {
    fn new() -> Self { KeywordStore { items: Vec::new() } }

    /// Register the referent. If an equivalent one already exists, merge and return the
    /// existing one; otherwise store and return as-is.
    fn register(&mut self, r: Rc<RefCell<Referent>>) -> Rc<RefCell<Referent>> {
        // Find equivalent: same typ + same VALUE or NORMAL
        for existing in &self.items {
            if can_be_equals(&existing.borrow(), &r.borrow()) {
                merge_slots(&mut existing.borrow_mut(), &r.borrow());
                return existing.clone();
            }
        }
        self.items.push(r.clone());
        r
    }
}

/// Check whether two keyword referents are equivalent.
fn can_be_equals(a: &Referent, b: &Referent) -> bool {
    if a.type_name != OBJ_TYPENAME || b.type_name != OBJ_TYPENAME {
        return false;
    }
    let ta = get_typ(a);
    let tb = get_typ(b);
    if ta != tb { return false; }

    // For referent-type keywords, compare the underlying referent by pointer
    if ta == KeywordType::Referent {
        let ra = a.find_slot(ATTR_REF, None)
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_referent());
        let rb = b.find_slot(ATTR_REF, None)
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_referent());
        if let (Some(ra), Some(rb)) = (ra, rb) {
            return Rc::ptr_eq(&ra, &rb);
        }
        return false;
    }

    // For object / predicate keywords: any VALUE or NORMAL overlap
    for sa in &a.slots {
        if sa.type_name != ATTR_NORMAL && sa.type_name != ATTR_VALUE { continue; }
        let sv = match sa.value.as_ref().and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        if b.find_slot(ATTR_NORMAL, Some(sv)).is_some() { return true; }
        if b.find_slot(ATTR_VALUE,  Some(sv)).is_some() { return true; }
    }
    false
}

/// Merge slots from `src` into `dst` (for deduplication accumulation).
fn merge_slots(dst: &mut Referent, src: &Referent) {
    for slot in &src.slots {
        // Skip TYPE (already same by construction) and RANK (accumulated separately)
        let n = slot.type_name.as_str();
        if n == "TYPE" || n == "RANK" { continue; }
        if let Some(ref v) = slot.value {
            let v_str = v.to_string();
            if dst.find_slot(n, Some(&v_str)).is_none() {
                dst.add_slot(n, v.clone(), false);
            }
        }
    }
}

// ── Analyzer impl ──────────────────────────────────────────────────────────

impl Analyzer for KeywordAnalyzer {
    fn name(&self) -> &'static str { "KEYWORD" }
    fn caption(&self) -> &'static str { "Ключевые комбинации" }
    fn is_specific(&self) -> bool { true }
    fn progress_weight(&self) -> i32 { 1 }

    fn process(&self, kit: &mut AnalysisKit) {
        // Ensure DenominationAnalyzer has already run (run it ourselves if not)
        {
            let denom = DenominationAnalyzer::new();
            // We always run denomination as a sub-pass (idempotent — it will simply
            // find no new denominations if they were already embedded)
            denom.process(kit);
        }

        let sofa = kit.sofa.clone();

        // Count total tokens for rank computation
        let max: i32 = {
            let mut n = 0i32;
            let mut t = kit.first_token.clone();
            while let Some(tok) = t {
                n += 1;
                t = tok.borrow().next.clone();
            }
            n
        };

        let mut store = KeywordStore::new();

        // ── Pass 1: Walk all tokens ──────────────────────────────────────────
        let mut cur = kit.first_token.clone();
        let mut position: i32 = 0;
        while let Some(t) = cur.clone() {
            let next_tok = t.borrow().next.clone();

            // Skip ignored tokens
            if t.borrow().is_ignored(&sofa) {
                cur = next_tok;
                position += 1;
                continue;
            }

            // ── Case A: ReferentToken ────────────────────────────────────────
            if let TokenKind::Referent(_) = &t.borrow().kind {
                if let Some(new_t) = process_referent_token(&t, &mut store, position, max, kit) {
                    let after = new_t.borrow().next.clone();
                    cur = after;
                    position += 1;
                    continue;
                }
                cur = next_tok;
                position += 1;
                continue;
            }

            // ── Case B: TextToken with letters ───────────────────────────────
            {
                let tb = t.borrow();
                if !matches!(tb.kind, TokenKind::Text(_)) || !tb.chars.is_letter() || tb.length_char() < 3 {
                    drop(tb);
                    cur = next_tok;
                    position += 1;
                    continue;
                }

                // Check special case "ЕСТЬ" — only treat as verb if preceded by a verb
                if let TokenKind::Text(ref txt) = tb.kind {
                    if txt.term == "ЕСТЬ" {
                        let prev_is_verb = tb.prev.as_ref()
                            .and_then(|p| p.upgrade())
                            .map(|p| p.borrow().get_morph_class_in_dictionary().is_verb())
                            .unwrap_or(false);
                        if !prev_is_verb {
                            drop(tb);
                            cur = next_tok;
                            position += 1;
                            continue;
                        }
                    }
                }
                drop(tb);
            }

            // Try noun phrase parse first
            let npt = npt_try_parse(
                &t,
                NounPhraseParseAttr::AdjectiveCanBeLast | NounPhraseParseAttr::ParsePreposition,
                0,
                &sofa,
            );

            if let Some(npt) = npt {
                // Skip if contains an internal noun (genitival construction — handled by union pass)
                if npt.internal_noun.is_some() {
                    cur = next_tok;
                    position += 1;
                    continue;
                }

                // Skip common filler NPs
                if should_skip_npt(&npt.end_token, &npt) {
                    // advance past the NP
                    // find token after end_token
                    let after_npt = npt.end_token.borrow().next.clone();
                    cur = after_npt;
                    position += 1;
                    continue;
                }

                // Single-token NP: check for preposition / adverb "ПОТОМ"
                let single_same = Rc::ptr_eq(&npt.begin_token, &npt.end_token);
                if single_same {
                    let mc = t.borrow().get_morph_class_in_dictionary();
                    if mc.is_preposition() {
                        cur = next_tok;
                        position += 1;
                        continue;
                    }
                    if mc.is_adverb() {
                        if t.borrow().is_value("ПОТОМ", None) {
                            cur = next_tok;
                            position += 1;
                            continue;
                        }
                    }
                }

                // Collect individual word keywords inside the NP
                let mut kw_list: Vec<Rc<RefCell<Referent>>> = Vec::new();
                let first_kw_tok: Option<TokenRef>;
                let npt_end_char = npt.end_token.borrow().end_char;

                {
                    let mut first_tok_opt: Option<TokenRef> = None;
                    let mut inner = Some(t.clone());
                    while let Some(tt) = inner.clone() {
                        let tt_end = tt.borrow().end_char;
                        if tt_end > npt_end_char { break; }

                        // Only process text tokens
                        let is_text_tok = matches!(tt.borrow().kind, TokenKind::Text(_));
                        if !is_text_tok {
                            inner = tt.borrow().next.clone();
                            continue;
                        }

                        // Length and letter check
                        {
                            let ttb = tt.borrow();
                            if ttb.length_char() < 3 || !ttb.chars.is_letter() {
                                drop(ttb);
                                inner = tt.borrow().next.clone();
                                continue;
                            }
                        }

                        // Skip prepositions, pronouns, conjunctions
                        let mc = tt.borrow().get_morph_class_in_dictionary();
                        if (mc.is_preposition() || mc.is_pronoun() || mc.is_conjunction())
                            && !tt.borrow().is_value("ОТНОШЕНИЕ", None)
                        {
                            inner = tt.borrow().next.clone();
                            continue;
                        }

                        // Get lemma for this token
                        let lemma = {
                            let ttb = tt.borrow();
                            if let TokenKind::Text(ref txt) = ttb.kind {
                                txt.lemma.clone().unwrap_or_else(|| txt.term.clone())
                            } else {
                                drop(ttb);
                                inner = tt.borrow().next.clone();
                                continue;
                            }
                        };

                        let mut kref = new_keyword_referent();
                        set_typ(&mut kref, KeywordType::Object);
                        add_value(&mut kref, &lemma);

                        let rank_delta = compute_rank_delta(&kref, position, max);
                        add_rank(&mut kref, rank_delta);

                        let kref_rc = store.register(Rc::new(RefCell::new(kref)));

                        // Embed as referent token around `tt`
                        let rt = Rc::new(RefCell::new(
                            Token::new_referent(tt.clone(), tt.clone(), kref_rc.clone())
                        ));
                        kit.embed_token(rt.clone());

                        if first_tok_opt.is_none() {
                            first_tok_opt = Some(rt.clone());
                        }
                        kw_list.push(kref_rc);

                        // Advance past the just-embedded referent token
                        inner = rt.borrow().next.clone();
                    }
                    first_kw_tok = first_tok_opt;
                }

                // If multiple words in NP, create a combined keyword covering the full span
                if kw_list.len() > 1 {
                    // Build value from individual lemmas
                    let mut val_parts: Vec<String> = Vec::new();
                    let mut norm_parts: Vec<String> = Vec::new();

                    for kw in &kw_list {
                        let kwb = kw.borrow();
                        let v = kwb.get_string_value(ATTR_VALUE)
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let n = kwb.get_string_value(ATTR_NORMAL)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| v.clone());
                        val_parts.push(v);
                        norm_parts.push(n);
                    }

                    let combined_val = val_parts.join(" ");
                    norm_parts.sort();
                    let combined_norm = norm_parts.join(" ");

                    let mut combined = new_keyword_referent();
                    set_typ(&mut combined, KeywordType::Object);
                    add_value(&mut combined, &combined_val);
                    if combined_norm != combined_val {
                        add_normal(&mut combined, &combined_norm);
                    }

                    // Add REF links to child word keywords
                    for kw in &kw_list {
                        combined.add_slot(ATTR_REF, SlotValue::Referent(kw.clone()), false);
                    }

                    let rank_delta = compute_rank_delta(&combined, position, max);
                    add_rank(&mut combined, rank_delta);

                    let combined_rc = store.register(Rc::new(RefCell::new(combined)));

                    // The combined token spans from the first embedded word kw to the last
                    // We need the actual first and last embedded referent tokens in the chain.
                    // After embedding all individual words, we can find them by scanning forward
                    // from first_kw_tok to npt_end_char.
                    let first_t = match first_kw_tok {
                        Some(ref ft) => ft.clone(),
                        None => {
                            cur = npt.end_token.borrow().next.clone();
                            position += 1;
                            continue;
                        }
                    };

                    // Find last embedded token that is still within the NP
                    let last_t = {
                        let mut last = first_t.clone();
                        let mut scan = Some(first_t.clone());
                        while let Some(st) = scan {
                            let st_end = st.borrow().end_char;
                            if st_end > npt_end_char { break; }
                            last = st.clone();
                            scan = st.borrow().next.clone();
                        }
                        last
                    };

                    let morph_clone = npt.morph.clone_collection();
                    let rt = Rc::new(RefCell::new(
                        Token::new_referent(first_t, last_t, combined_rc.clone())
                    ));
                    {
                        rt.borrow_mut().morph = morph_clone;
                    }
                    kit.embed_token(rt.clone());
                    cur = rt.borrow().next.clone();
                } else if kw_list.len() == 1 {
                    // Single-word NP: the token was already embedded
                    // Advance past the NP end
                    cur = npt.end_token.borrow().next.clone();
                } else {
                    cur = npt.end_token.borrow().next.clone();
                }
                position += 1;
                continue;
            }

            // ── Case C: No NP matched — check for verb predicate ─────────────
            {
                let tb = t.borrow();
                let mc = tb.get_morph_class_in_dictionary();
                if mc.is_verb() && !mc.is_preposition() {
                    if tb.is_verb_be() {
                        drop(tb);
                        cur = next_tok;
                        position += 1;
                        continue;
                    }
                    if tb.is_value("МОЧЬ", None) || tb.is_value("WOULD", None) {
                        drop(tb);
                        cur = next_tok;
                        position += 1;
                        continue;
                    }

                    // Get lemma / normal form
                    let norm = if let TokenKind::Text(ref txt) = tb.kind {
                        let mut n = txt.lemma.clone()
                            .unwrap_or_else(|| txt.term.clone());
                        // Strip reflexive suffix "СЯ" if ends with "ЬСЯ"
                        // "СЯ" is 4 bytes in UTF-8 (2 Cyrillic chars × 2 bytes each)
                        if n.ends_with("ЬСЯ") {
                            let trim_pos = n.len() - "СЯ".len();
                            n.truncate(trim_pos);
                        }
                        n
                    } else {
                        drop(tb);
                        cur = next_tok;
                        position += 1;
                        continue;
                    };
                    drop(tb);

                    let mut kref = new_keyword_referent();
                    set_typ(&mut kref, KeywordType::Predicate);
                    add_value(&mut kref, &norm);

                    let rank_delta = compute_rank_delta(&kref, position, max);
                    add_rank(&mut kref, rank_delta);

                    let kref_rc = store.register(Rc::new(RefCell::new(kref)));

                    let rt = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), t.clone(), kref_rc.clone())
                    ));
                    {
                        let morph_clone = t.borrow().morph.clone_collection();
                        rt.borrow_mut().morph = morph_clone;
                    }
                    kit.embed_token(rt.clone());
                    cur = rt.borrow().next.clone();
                    position += 1;
                    continue;
                }
            }

            cur = next_tok;
            position += 1;
        }

        // Register all deduplicated entities
        for item in &store.items {
            let _ = kit.add_entity(item.clone());
        }
    }
}

// ── Helper: should_skip_npt ────────────────────────────────────────────────

/// Returns true if the NP should be skipped (common filler phrases).
fn should_skip_npt(end_tok: &TokenRef, npt: &crate::core::noun_phrase::NounPhraseToken) -> bool {
    let has_preposition = npt.preposition.is_some();

    if end_tok.borrow().is_value("ЦЕЛОМ", None) || end_tok.borrow().is_value("ЧАСТНОСТИ", None) {
        if has_preposition { return true; }
    }
    if end_tok.borrow().is_value("СТОРОНЫ", None) && has_preposition {
        if let Some(ref prep) = npt.preposition {
            if prep.normal == "С" { return true; }
        }
    }
    false
}

// ── Helper: process_referent_token ────────────────────────────────────────

/// Wrap an existing referent token in a keyword referent.
/// Returns the new outer referent token, or None if the referent type should be skipped.
fn process_referent_token(
    t: &TokenRef,
    store: &mut KeywordStore,
    cur: i32,
    max: i32,
    kit: &mut AnalysisKit,
) -> Option<TokenRef> {
    let r_rc = t.borrow().get_referent()?;
    let type_name = r_rc.borrow().type_name.clone();

    // Skip phone/uri/bank entirely
    if type_name == "PHONE" || type_name == "URI" || type_name == "BANKDATA" {
        return None;
    }

    // Skip date ranges
    if type_name == "DATE" || type_name == "DATERANGE" || type_name == "BOOKLINKREF" {
        return None;
    }

    // Denomination → Referent keyword with NORMAL = denomination value
    if type_name == "DENOMINATION" {
        let normals: Vec<String> = {
            let rb = r_rc.borrow();
            rb.get_all_string_values("VALUE")
                .iter()
                .map(|s| s.to_string())
                .collect()
        };

        let mut kref = new_keyword_referent();
        set_typ(&mut kref, KeywordType::Referent);
        for n in &normals {
            add_normal(&mut kref, n);
        }
        kref.add_slot(ATTR_REF, SlotValue::Referent(r_rc.clone()), false);

        let rank_delta = compute_rank_delta(&kref, cur, max);
        add_rank(&mut kref, rank_delta);

        let kref_rc = store.register(Rc::new(RefCell::new(kref)));
        let rt = Rc::new(RefCell::new(
            Token::new_referent(t.clone(), t.clone(), kref_rc)
        ));
        kit.embed_token(rt.clone());
        return Some(rt);
    }

    // Money → Object keyword with NORMAL = currency code
    if type_name == "MONEY" {
        let currency = r_rc.borrow().get_string_value("CURRENCY")
            .map(|s| s.to_string());
        if let Some(cur_str) = currency {
            let mut kref = new_keyword_referent();
            set_typ(&mut kref, KeywordType::Object);
            add_normal(&mut kref, &cur_str);

            let rank_delta = compute_rank_delta(&kref, cur, max);
            add_rank(&mut kref, rank_delta);

            let kref_rc = store.register(Rc::new(RefCell::new(kref)));
            let rt = Rc::new(RefCell::new(
                Token::new_referent(t.clone(), t.clone(), kref_rc)
            ));
            kit.embed_token(rt.clone());
            return Some(rt);
        }
        return None;
    }

    // General referent — build a Referent keyword
    let mut kref = new_keyword_referent();
    set_typ(&mut kref, KeywordType::Referent);

    // Get the display string
    let norm = if type_name == "GEO" {
        r_rc.borrow().get_string_value("ALPHA2").map(|s| s.to_string())
            .unwrap_or_else(|| referent_to_string(&r_rc.borrow()))
    } else {
        referent_to_string(&r_rc.borrow())
    };

    if !norm.is_empty() {
        add_normal(&mut kref, &norm.to_uppercase());
    }
    kref.add_slot(ATTR_REF, SlotValue::Referent(r_rc.clone()), false);

    let rank_delta = compute_rank_delta(&kref, cur, max);
    add_rank(&mut kref, rank_delta);

    let kref_rc = store.register(Rc::new(RefCell::new(kref)));
    let rt = Rc::new(RefCell::new(
        Token::new_referent(t.clone(), t.clone(), kref_rc)
    ));
    kit.embed_token(rt.clone());
    Some(rt)
}

// ── Helper: referent_to_string ────────────────────────────────────────────

/// Produce a short display string for an arbitrary referent.
/// Mirrors `Referent.ToStringEx(shortVariant=true)`.
fn referent_to_string(r: &Referent) -> String {
    // Try common display attributes
    for attr in &["NAME", "VALUE", "NUMBER", "TYPE"] {
        if let Some(v) = r.get_string_value(attr) {
            if !v.is_empty() { return v.to_string(); }
        }
    }
    // Fallback: first non-internal slot string value
    for slot in &r.slots {
        if slot.is_internal() { continue; }
        if let Some(v) = slot.value.as_ref().and_then(|v| v.as_str()) {
            if !v.is_empty() { return v.to_string(); }
        }
    }
    r.type_name.clone()
}
