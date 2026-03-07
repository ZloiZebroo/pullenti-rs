/// GeoAnalyzer — simplified port of GeoAnalyzer.cs.
///
/// Handles the main patterns:
///  1. Known country/city name (direct lookup via morph normal forms)
///  2. Territory type keyword + proper name  ("Московская область", "г. Москва")
///  3. Country/region adjective forms when followed by a type keyword

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef, TokenKind};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::geo::geo_referent as gr;
use crate::geo::geo_table::{self, GeoEntryKind};

pub struct GeoAnalyzer;

impl GeoAnalyzer {
    pub fn new() -> Self { GeoAnalyzer }
}

impl Analyzer for GeoAnalyzer {
    fn name(&self) -> &'static str { "GEO" }
    fn caption(&self) -> &'static str { "Страны, регионы, города" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }
            match try_parse(&t, &sofa) {
                None => { cur = t.borrow().next.clone(); }
                Some((referent, end)) => {
                    let r_rc = Rc::new(RefCell::new(referent));
                    kit.add_entity(r_rc.clone());
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

// ── Main parser ───────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Check keyword matches while borrow is active, store results as owned values
    let (is_city_prefix, type_kw): (bool, Option<&'static str>) = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(txt) => {
                // term and normal forms are already uppercase from morph engine
                let term = &txt.term;
                let mut is_cp = geo_table::is_city_prefix(term);
                let mut tkw: Option<&'static str> = geo_table::type_keyword(term).map(|(s, _)| s);

                for wf in tb.morph.items() {
                    if let Some(nc) = &wf.normal_case {
                        if !is_cp { is_cp = geo_table::is_city_prefix(nc); }
                        if tkw.is_none() {
                            tkw = geo_table::type_keyword(nc).map(|(s, _)| s);
                        }
                    }
                    if let Some(nf) = &wf.normal_full {
                        if !is_cp { is_cp = geo_table::is_city_prefix(nf); }
                        if tkw.is_none() {
                            tkw = geo_table::type_keyword(nf).map(|(s, _)| s);
                        }
                    }
                    if is_cp && tkw.is_some() { break; }
                }
                (is_cp, tkw)
            }
            _ => return None,
        }
    };

    // 1. City type prefix keyword (г., город, г, etc.)
    if is_city_prefix {
        if let Some(result) = try_city_prefix(t, sofa) {
            return Some(result);
        }
    }

    // 2. Territory type keyword (область, район, республика, etc.)
    if let Some(type_str) = type_kw {
        if let Some(result) = try_type_keyword_prefix(t, type_str, sofa) {
            return Some(result);
        }
    }

    // 3. Direct name lookup (country or city by noun name or acronym)
    if let Some(result) = try_direct_name(t, sofa) {
        return Some(result);
    }

    // 4. Adjective form of a region/country followed by type keyword
    if let Some(result) = try_adjective_plus_type(t, sofa) {
        return Some(result);
    }

    None
}

// ── Pattern 1: City prefix ────────────────────────────────────────────────────
//
// "г. Москва", "город Москва", "г Москва"

fn try_city_prefix(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let next = t.borrow().next.clone()?;
    let nb = next.borrow();
    // Skip dots after abbreviation
    if nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '.' {
        drop(nb);
        let after_dot = next.borrow().next.clone()?;
        return try_city_from_name(&after_dot, sofa, t);
    }
    // Skip whitespace is handled implicitly by the token chain
    if let TokenKind::Text(_) = &nb.kind {
        drop(nb);
        return try_city_from_name(&next, sofa, t);
    }
    None
}

fn try_city_from_name(
    name_tok: &TokenRef,
    sofa: &SourceOfAnalysis,
    _begin: &TokenRef,
) -> Option<(Referent, TokenRef)> {
    // Try the token and following tokens as a multi-word city name
    let candidates = collect_candidates(name_tok, sofa);
    for c in &candidates {
        if let Some(entry) = geo_table::lookup_name(c) {
            if matches!(entry.kind, GeoEntryKind::City) {
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &entry.canonical_name);
                for n in &entry.all_names {
                    gr::add_name(&mut r, n);
                }
                gr::add_type(&mut r, &entry.type_str);
                return Some((r, name_tok.clone()));
            }
        }
    }
    // Even if not found in city table, create a generic geo with city type
    // if the token is a proper noun (starts with uppercase)
    let tb = name_tok.borrow();
    if let TokenKind::Text(txt) = &tb.kind {
        let surface = sofa.substring(tb.begin_char, tb.end_char);
        if surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !tb.chars.is_all_lower()
        {
            drop(tb);
            // Try multi-word hyphenated city (e.g. "Санкт-Петербург")
            let (full_name, end_tok) = collect_hyphenated_name(name_tok, sofa);
            let mut r = gr::new_geo_referent();
            gr::add_name(&mut r, &full_name);
            gr::add_type(&mut r, "город");
            return Some((r, end_tok));
        }
    }
    None
}

// ── Pattern 2: Type keyword as prefix ────────────────────────────────────────
//
// "Московская область", "Краснодарский край", "Республика Адыгея"
// The type keyword was already matched; now look for the name after it.

fn try_type_keyword_prefix(
    keyword_tok: &TokenRef,
    type_str: &str,
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let next = keyword_tok.borrow().next.clone()?;
    let nb = next.borrow();

    if let TokenKind::Text(txt) = &nb.kind {
        // Must start with uppercase (proper noun or adjective)
        let surface = sofa.substring(nb.begin_char, nb.end_char);
        if !surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return None;
        }
        // term is already uppercase from morph engine
        let name = txt.term.clone();
        // Compute Cyrillic flag while nb is still borrowed.
        let has_cyrillic = surface.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
        drop(nb);

        // Try looking up the name directly
        let candidates = collect_candidates(&next, sofa);
        for c in &candidates {
            if let Some(entry) = geo_table::lookup_name(c) {
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &entry.canonical_name);
                for n in &entry.all_names {
                    gr::add_name(&mut r, n);
                }
                // Use the keyword type (may be more specific than table entry)
                gr::add_type(&mut r, type_str);
                if !entry.type_str.is_empty() && entry.type_str != type_str {
                    gr::add_type(&mut r, &entry.type_str);
                }
                if let Some(ref a2) = entry.alpha2 {
                    gr::set_alpha2(&mut r, a2);
                }
                return Some((r, next.clone()));
            }
        }

        // Not in table — only create a fallback geo entity for Cyrillic names.
        // English words after a type keyword (e.g. "State Batches", "State Doc")
        // are almost certainly NOT place names; restrict to Cyrillic-script names
        // that may simply be missing from the embedded geo database.
        if has_cyrillic {
            let (full_name, end_tok) = collect_hyphenated_name(&next, sofa);
            if !full_name.is_empty() {
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &full_name);
                gr::add_name(&mut r, &name);
                gr::add_type(&mut r, type_str);
                return Some((r, end_tok));
            }
        }
    }
    None
}

// ── Pattern 3: Direct name lookup ────────────────────────────────────────────
//
// "Россия", "Москва", "США", "РФ"

fn try_direct_name(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // ── Pre-lookup guards ──────────────────────────────────────────────────────
    // Compute surface text and term once; all guards below use them.
    let (surface, term): (String, String) = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(txt) => (
                sofa.substring(tb.begin_char, tb.end_char).to_string(),
                txt.term.clone(), // already uppercase from morph engine
            ),
            _ => return None,
        }
    };
    let char_count = surface.chars().count();

    // Guard 1 — Geo proper nouns always start with an uppercase letter.
    //   Rejects common English words mid-sentence: "early", "long", "ness", …
    if surface.chars().next().map(|c| c.is_alphabetic() && c.is_lowercase()).unwrap_or(false) {
        return None;
    }

    // Guard 1b — Russian function words (conjunctions, particles) that begin a
    //   sentence with an uppercase letter but are never place names.
    //   Use term (already uppercase) to avoid allocating an uppercase copy of surface.
    {
        if matches!(term.as_str(),
            "ЕСЛИ" | "КОГДА" | "ХОТЯ" | "ПОКА" | "ПУСТЬ" | "ПОТОМУ" |
            "ПОЭТОМУ" | "ОДНАКО" | "ЗАТО" | "ЛИБО" | "ТОЖЕ" | "ТАКЖЕ" |
            "КОТОРЫЙ" | "КОТОРАЯ" | "КОТОРОЕ" | "КОТОРЫЕ" |
            "ЧТОБЫ" | "ПРИЧЁМ" | "ПРИТОМ"
        ) {
            return None;
        }
    }

    // Guard 2 — Very short tokens (≤ 2 chars).
    if char_count <= 2 {
        // 2a. Must be all-uppercase.
        //   Rejects "At", "In", "Li", "re", "by", … (title-case/lowercase).
        let all_upper = surface.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
        if !all_upper {
            return None;
        }
        // 2b. All-ASCII-Latin 2-char tokens are too ambiguous.
        //   "IR"=Information Retrieval vs Iran, "ID"=Identifier vs Indonesia,
        //   "LR"=Learning Rate vs Liberia, "AI"=artificial intelligence vs Anguilla.
        //   Cyrillic abbreviations like "РФ", "ЕС" are NOT affected by this check.
        if surface.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        // 2c. All-uppercase 2-char token immediately followed by apostrophe →
        //   possessive (e.g. "AI's"), not a standalone geo entity.
        let tb = t.borrow();
        if let Some(next) = tb.next.clone() {
            let nb = next.borrow();
            if nb.whitespaces_before_count(sofa) == 0 {
                let ns = sofa.substring(nb.begin_char, nb.end_char);
                if ns.starts_with('\'') || ns.starts_with('\u{2019}') {
                    return None;
                }
            }
        }
    }

    // ── Single-token table lookup ──────────────────────────────────────────────
    let candidates = collect_candidates(t, sofa);
    let is_adj = {
        let tb = t.borrow();
        tb.morph.items().iter().any(|wf| wf.base.class.is_adjective())
    };

    for c in &candidates {
        if let Some(entry) = geo_table::lookup_name(c) {
            // Skip adjective matches for regions (handled by pattern 4).
            if is_adj && matches!(entry.kind, GeoEntryKind::Region) {
                continue;
            }

            // Guard 3 — Selective filter for pure-ASCII-Latin surfaces.
            if surface.chars().all(|c| c.is_ascii_alphabetic()) {
                let should_block = match entry.kind {
                    // City entries: block only very short canonical names (≤ 4 chars).
                    // Words like "Long" (4), "Yolo" (4), "Ness" (4), "Bath" (4) appear
                    // in worldwide city databases but are clearly common English words
                    // in context.  Well-known cities — Miami (5), Bangkok (7),
                    // Singapore (9) — still pass.
                    GeoEntryKind::City => entry.canonical_name.chars().count() <= 4,

                    // Region entries: block sub-national county / district units.
                    // US county names (Howard County, Lewis County, Long County, …)
                    // are identical to common English surnames and cause PERSON tokens
                    // to be wrongly labelled GEO.  Major administrative divisions
                    // (states / provinces / oblasts) are legitimate and kept.
                    GeoEntryKind::Region => matches!(
                        entry.type_str.as_str(),
                        "county" | "графство" | "район" | "уезд" | "волость" | "district"
                    ),

                    // State (sovereign country): never blocked.
                    GeoEntryKind::State => false,
                };
                if should_block { continue; }
            }

            // ── Guard: ASCII city/region preceded or followed by an uppercase
            //    Latin word that is NOT itself a geo name → likely a person name
            //    context ("Pierre Andrews", "Kaifeng Chen", "Matthew Wallingford").
            if matches!(entry.kind, GeoEntryKind::City | GeoEntryKind::Region)
                && surface.chars().all(|c| c.is_ascii_alphabetic())
            {
                if in_person_name_context(t, sofa) { continue; }
            }

            let mut r = gr::new_geo_referent();
            gr::add_name(&mut r, &entry.canonical_name);
            for n in &entry.all_names {
                gr::add_name(&mut r, n);
            }
            gr::add_type(&mut r, &entry.type_str);
            if let Some(ref a2) = entry.alpha2 {
                gr::set_alpha2(&mut r, a2);
            }
            return Some((r, t.clone()));
        }
    }

    // ── Hyphenated compound lookup ─────────────────────────────────────────────
    // Handles "Санкт-Петербург", "Нью-Йорк", "Ростов-на-Дону", etc.
    if let Some(result) = try_hyphenated_name(t, &candidates, sofa) {
        return Some(result);
    }

    // ── Line-break hyphen join ─────────────────────────────────────────────────
    // Handles "Thai-\nland" → "THAILAND", "Bang-\nkok" → "BANGKOK", etc.
    // PDF/text extraction often hyphenates words at line ends.
    if let Some(result) = try_linebreak_join(t, &candidates, sofa) {
        return Some(result);
    }

    // ── 3-word compound lookup ─────────────────────────────────────────────────
    // Handles "United Arab Emirates", "Papua New Guinea", etc.
    if let Some(result) = try_three_word_name(t, &candidates, sofa) {
        return Some(result);
    }

    // ── 2-word compound lookup ─────────────────────────────────────────────────
    // Handles "New York", "Abu Dhabi", "San Francisco", "Hong Kong", etc.
    if let Some(result) = try_two_word_name(t, &candidates, sofa) {
        return Some(result);
    }

    None
}

/// Try to match a hyphenated geo name (e.g. "Санкт-Петербург", "Нью-Йорк",
/// "Ростов-на-Дону") where the tokenizer splits [Word]["-"][Word...].
/// Tries up to 3 hyphen-joined segments (e.g. "Ростов-на-Дону").
fn try_hyphenated_name(
    t: &TokenRef,
    first_candidates: &[String],
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    // next token must be an adjacent hyphen
    let hyp = t.borrow().next.clone()?;
    {
        let hb = hyp.borrow();
        if hb.whitespaces_before_count(sofa) != 0 { return None; }
        if hb.length_char() != 1 { return None; }
        let ch = sofa.char_at(hb.begin_char);
        if ch != '-' && ch != '\u{2013}' && ch != '\u{2014}' { return None; }
    }

    // Collect segments: start with token after hyphen, keep going while
    // the pattern continues as "-Word".
    let mut segments: Vec<TokenRef> = Vec::new();  // the Word tokens after each hyphen
    let mut hyphens: Vec<TokenRef> = Vec::new();   // the hyphen tokens
    {
        let mut cur_hyp = hyp.clone();
        loop {
            let word = cur_hyp.borrow().next.clone()?;
            {
                let wb = word.borrow();
                if wb.whitespaces_before_count(sofa) != 0 { break; }
                if !matches!(wb.kind, TokenKind::Text(_)) { break; }
            }
            hyphens.push(cur_hyp.clone());
            segments.push(word.clone());

            // Check if there's another hyphen immediately after this word
            let maybe_next_hyp = word.borrow().next.clone();
            match maybe_next_hyp {
                None => break,
                Some(nh) => {
                    let nhb = nh.borrow();
                    if nhb.whitespaces_before_count(sofa) != 0 { break; }
                    if nhb.length_char() != 1 { break; }
                    let ch = sofa.char_at(nhb.begin_char);
                    if ch != '-' && ch != '\u{2013}' && ch != '\u{2014}' { break; }
                    drop(nhb);
                    cur_hyp = nh;
                }
            }
            if segments.len() >= 3 { break; } // max 3 segments (e.g. "Ростов-на-Дону")
        }
    }

    if segments.is_empty() { return None; }

    // Build candidate strings for each segment
    let seg_candidates: Vec<Vec<String>> = segments.iter()
        .map(|s| collect_candidates(s, sofa))
        .collect();

    // Try all combinations from longest to shortest.
    // For each segment we try all its morph candidates (to handle inflected forms
    // like "Петербурге" → "ПЕТЕРБУРГ").
    for num_segs in (1..=segments.len()).rev() {
        let end_tok = &segments[num_segs - 1];
        for c1 in first_candidates.iter().take(3) {
            // Try each candidate for the first segment after the hyphen
            let first_seg_cands = &seg_candidates[0];
            for c2 in first_seg_cands.iter().take(4) {
                if num_segs == 1 {
                    let key = format!("{}-{}", c1, c2);
                    if let Some(entry) = geo_table::lookup_name(&key) {
                        let mut r = gr::new_geo_referent();
                        gr::add_name(&mut r, &entry.canonical_name);
                        for n in &entry.all_names {
                            gr::add_name(&mut r, n);
                        }
                        gr::add_type(&mut r, &entry.type_str);
                        if let Some(ref a2) = entry.alpha2 {
                            gr::set_alpha2(&mut r, a2);
                        }
                        return Some((r, end_tok.clone()));
                    }
                } else {
                    // For multi-segment, build rest as fixed (use first candidate per segment)
                    let mut key = format!("{}-{}", c1, c2);
                    let mut valid = true;
                    for i in 1..num_segs {
                        let part_cands = &seg_candidates[i];
                        if part_cands.is_empty() { valid = false; break; }
                        key.push('-');
                        key.push_str(&part_cands[0]);
                    }
                    if !valid { continue; }
                    if let Some(entry) = geo_table::lookup_name(&key) {
                        let mut r = gr::new_geo_referent();
                        gr::add_name(&mut r, &entry.canonical_name);
                        for n in &entry.all_names {
                            gr::add_name(&mut r, n);
                        }
                        gr::add_type(&mut r, &entry.type_str);
                        if let Some(ref a2) = entry.alpha2 {
                            gr::set_alpha2(&mut r, a2);
                        }
                        return Some((r, end_tok.clone()));
                    }
                }
            }
        }
    }

    None
}

/// Try to match a 2-word geo name (e.g. "New York", "Abu Dhabi") by combining
/// the current token's candidates with the next text token's candidates.
fn try_two_word_name(
    t: &TokenRef,
    first_candidates: &[String],
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let next = t.borrow().next.clone()?;
    {
        let nb = next.borrow();
        // Only try if the next token is a text token with at most 1 space before it.
        if nb.whitespaces_before_count(sofa) > 1 { return None; }
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }

    let second_candidates = collect_candidates(&next, sofa);

    for c1 in first_candidates {
        // Skip nationality-mapped candidates for the first word (e.g. don't form
        // "КИТАЙ YORK" from "Chinese York").  Only use the first 3 candidates
        // (term + surface + first morph form) to keep the cross-product small.
        for c2 in second_candidates.iter().take(3) {
            let two_word = format!("{} {}", c1, c2);
            if let Some(entry) = geo_table::lookup_name(&two_word) {
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &entry.canonical_name);
                for n in &entry.all_names {
                    gr::add_name(&mut r, n);
                }
                gr::add_type(&mut r, &entry.type_str);
                if let Some(ref a2) = entry.alpha2 {
                    gr::set_alpha2(&mut r, a2);
                }
                return Some((r, next.clone()));
            }
        }
    }
    None
}

/// Try to match a 3-word geo name (e.g. "United Arab Emirates", "Papua New Guinea").
fn try_three_word_name(
    t: &TokenRef,
    first_candidates: &[String],
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let second = t.borrow().next.clone()?;
    {
        let nb = second.borrow();
        if nb.whitespaces_before_count(sofa) > 1 { return None; }
        if !matches!(nb.kind, TokenKind::Text(_)) { return None; }
    }
    let third = second.borrow().next.clone()?;
    {
        let tb = third.borrow();
        if tb.whitespaces_before_count(sofa) > 1 { return None; }
        if !matches!(tb.kind, TokenKind::Text(_)) { return None; }
    }
    let second_candidates = collect_candidates(&second, sofa);
    let third_candidates  = collect_candidates(&third, sofa);

    for c1 in first_candidates.iter().take(3) {
        for c2 in second_candidates.iter().take(3) {
            for c3 in third_candidates.iter().take(3) {
                let key = format!("{} {} {}", c1, c2, c3);
                if let Some(entry) = geo_table::lookup_name(&key) {
                    let mut r = gr::new_geo_referent();
                    gr::add_name(&mut r, &entry.canonical_name);
                    for n in &entry.all_names { gr::add_name(&mut r, n); }
                    gr::add_type(&mut r, &entry.type_str);
                    if let Some(ref a2) = entry.alpha2 { gr::set_alpha2(&mut r, a2); }
                    return Some((r, third.clone()));
                }
            }
        }
    }
    None
}

/// Try to join a word across a line-break hyphen ("Thai-\nland" → "THAILAND").
/// PDF and plain-text files often hyphenate long words at end of line.
/// Heuristic: hyphen immediately after current word, then newline whitespace,
/// then a continuation word starting with lowercase.
fn try_linebreak_join(
    t: &TokenRef,
    first_candidates: &[String],
    sofa: &SourceOfAnalysis,
) -> Option<(Referent, TokenRef)> {
    let hyp = t.borrow().next.clone()?;
    {
        let hb = hyp.borrow();
        if hb.whitespaces_before_count(sofa) != 0 { return None; }
        if hb.length_char() != 1 { return None; }
        let ch = sofa.char_at(hb.begin_char);
        if ch != '-' && ch != '\u{2013}' { return None; }
    }
    let cont = hyp.borrow().next.clone()?;
    let cont_term: String = {
        let cb = cont.borrow();
        // Must have whitespace before it (newline gap)
        if cb.whitespaces_before_count(sofa) == 0 { return None; }
        // The gap between hyphen end and cont begin must contain a newline
        let hyp_end   = hyp.borrow().end_char;
        let cont_begin = cb.begin_char;
        let mut has_nl = false;
        for pos in (hyp_end + 1)..cont_begin {
            if sofa.char_at(pos) == '\n' { has_nl = true; break; }
        }
        if !has_nl { return None; }
        // Continuation must start with a lowercase letter (line-break word wrap)
        let fc = sofa.char_at(cb.begin_char);
        if !fc.is_alphabetic() || !fc.is_lowercase() { return None; }
        match &cb.kind {
            // term is already uppercase from morph engine
            TokenKind::Text(txt) => txt.term.clone(),
            _ => return None,
        }
    };

    // Form joined word without hyphen and look up
    for c1 in first_candidates.iter().take(3) {
        let joined = format!("{}{}", c1, cont_term);
        if let Some(entry) = geo_table::lookup_name(&joined) {
            // Apply the same person-name-context guard as in single-word lookups.
            if matches!(entry.kind, GeoEntryKind::City | GeoEntryKind::Region)
                && joined.chars().all(|c| c.is_ascii_alphabetic())
            {
                let after_cont_is_name = cont.borrow().next.clone().map_or(false, |nx| {
                    let nb = nx.borrow();
                    if nb.whitespaces_before_count(sofa) != 1 { return false; }
                    if !matches!(nb.kind, TokenKind::Text(_)) { return false; }
                    let s = sofa.substring(nb.begin_char, nb.end_char);
                    let mut cs = s.chars();
                    let f = cs.next();
                    let s2 = cs.next();
                    match (f, s2) {
                        (Some(f), Some(s2)) =>
                            f.is_ascii_alphabetic() && f.is_uppercase()
                            && s2.is_ascii_alphabetic()
                            && cs.all(|c| c.is_ascii_alphabetic()),
                        _ => false,
                    }
                });
                if in_person_name_context(t, sofa) || after_cont_is_name { continue; }
            }
            let mut r = gr::new_geo_referent();
            gr::add_name(&mut r, &entry.canonical_name);
            for n in &entry.all_names { gr::add_name(&mut r, n); }
            gr::add_type(&mut r, &entry.type_str);
            if let Some(ref a2) = entry.alpha2 { gr::set_alpha2(&mut r, a2); }
            return Some((r, cont.clone()));
        }
    }
    None
}

/// Returns true if the token appears to be in a person-name context:
/// - immediately followed by an uppercase Latin word that is NOT a known geo name, OR
/// - immediately preceded by an uppercase Latin word that is NOT a known geo name.
/// Used to suppress false-positive city detections like "Pierre Andrews" or "Kaifeng Chen".
fn in_person_name_context(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let is_ascii_name_candidate = |tok: &TokenRef| -> bool {
        let tb = tok.borrow();
        if !matches!(tb.kind, TokenKind::Text(_)) { return false; }
        let s = sofa.substring(tb.begin_char, tb.end_char);
        // Use iterator instead of Vec<char> allocation
        let mut chars = s.chars();
        let first = match chars.next() {
            Some(c) => c,
            None => return false,
        };
        if !first.is_ascii_alphabetic() || !first.is_uppercase() { return false; }
        // Need at least 2 chars total
        let second = match chars.next() {
            Some(c) => c,
            None => return false,
        };
        if !second.is_ascii_alphabetic() { return false; }
        if !chars.all(|c| c.is_ascii_alphabetic()) { return false; }
        // term is already uppercase from morph engine — use it directly
        let term = if let TokenKind::Text(txt) = &tb.kind { &txt.term } else { return true; };
        match geo_table::lookup_name(term) {
            None => true,
            Some(entry) => matches!(entry.kind, GeoEntryKind::Region)
                && matches!(entry.type_str.as_str(),
                    "county" | "графство" | "район" | "уезд" | "волость" | "district"),
        }
    };

    // Check next token (at most 1 space away); also skip an inline hyphen.
    if let Some(next) = t.borrow().next.clone() {
        let ws = next.borrow().whitespaces_before_count(sofa);
        if ws <= 1 {
            if is_ascii_name_candidate(&next) {
                return true;
            }
            // "Pierre-Emmanuel": next is a hyphen with 0 spaces → look one more token ahead.
            if ws == 0 && next.borrow().length_char() == 1 {
                let ch = sofa.char_at(next.borrow().begin_char);
                if ch == '-' || ch == '\u{2013}' {
                    if let Some(after_hyp) = next.borrow().next.clone() {
                        let ab = after_hyp.borrow();
                        if ab.whitespaces_before_count(sofa) == 0
                            && matches!(ab.kind, TokenKind::Text(_))
                        {
                            let s = sofa.substring(ab.begin_char, ab.end_char);
                            let mut ac = s.chars();
                            let f = ac.next();
                            let s2 = ac.next();
                            if let (Some(f), Some(s2)) = (f, s2) {
                                if f.is_ascii_alphabetic()
                                    && f.is_uppercase()
                                    && s2.is_ascii_alphabetic()
                                    && ac.all(|c| c.is_ascii_alphabetic())
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Check previous token
    if let Some(prev_weak) = t.borrow().prev.clone() {
        if let Some(prev) = prev_weak.upgrade() {
            if prev.borrow().whitespaces_before_count(sofa) <= 1 && is_ascii_name_candidate(&prev) {
                return true;
            }
        }
    }
    false
}

// ── Pattern 4: Adjective + type keyword ──────────────────────────────────────
//
// token = "Московская", next = "область" → "МОСКОВСКАЯ ОБЛАСТЬ" region

fn try_adjective_plus_type(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let tb = t.borrow();
    let is_adj = tb.morph.items().iter().any(|wf| wf.base.class.is_adjective());
    if !is_adj { return None; }

    // Collect adjective candidate strings
    let candidates = collect_candidates(t, sofa);
    drop(tb);

    // Check if the next token is a type keyword (use normal forms too).
    let next = t.borrow().next.clone()?;
    let next_type = token_type_keyword(&next);
    let type_str = next_type?;

    // Look up the adjective in the adj_map
    for c in &candidates {
        if let Some(entry) = geo_table::lookup_adj(c) {
            let mut r = gr::new_geo_referent();
            gr::add_name(&mut r, &entry.canonical_name);
            for n in &entry.all_names {
                gr::add_name(&mut r, n);
            }
            gr::add_type(&mut r, type_str);
            if !entry.type_str.is_empty() && entry.type_str != type_str {
                gr::add_type(&mut r, &entry.type_str);
            }
            if let Some(ref a2) = entry.alpha2 {
                gr::set_alpha2(&mut r, a2);
            }
            return Some((r, next.clone()));
        }
    }

    // Also try the noun form lookup in combination with type keyword
    // e.g. "московская область" → try lookup("МОСКОВСКАЯ") in name_map
    for c in &candidates {
        if let Some(entry) = geo_table::lookup_name(c) {
            if matches!(entry.kind, GeoEntryKind::Region) {
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &entry.canonical_name);
                for n in &entry.all_names {
                    gr::add_name(&mut r, n);
                }
                gr::add_type(&mut r, type_str);
                if !entry.type_str.is_empty() && entry.type_str != type_str {
                    gr::add_type(&mut r, &entry.type_str);
                }
                return Some((r, next.clone()));
            }
        }
    }

    None
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Check if a token matches a territory type keyword (checking all normal forms).
/// Returns the canonical type string, e.g. "область".
fn token_type_keyword(t: &TokenRef) -> Option<&'static str> {
    let tb = t.borrow();
    if let TokenKind::Text(txt) = &tb.kind {
        // term and normal forms are already uppercase from morph engine
        if let Some((s, _)) = geo_table::type_keyword(&txt.term) {
            return Some(s);
        }
        for wf in tb.morph.items() {
            if let Some(nc) = &wf.normal_case {
                if let Some((s, _)) = geo_table::type_keyword(nc) { return Some(s); }
            }
            if let Some(nf) = &wf.normal_full {
                if let Some((s, _)) = geo_table::type_keyword(nf) { return Some(s); }
            }
        }
    }
    None
}

/// Collect candidate lookup strings for a token:
/// raw surface, term (from morph), each normal_case, each normal_full.
/// Terms and normal forms are already uppercase from the morph engine.
fn collect_candidates(t: &TokenRef, sofa: &SourceOfAnalysis) -> Vec<String> {
    let tb = t.borrow();
    let mut out: Vec<String> = Vec::with_capacity(6);
    if let TokenKind::Text(txt) = &tb.kind {
        // morph term (already uppercase)
        out.push(txt.term.clone());
        // surface text (may differ from term for transliterated words)
        let surface = sofa.substring(tb.begin_char, tb.end_char);
        let surface_up = surface.to_uppercase();
        if surface_up != txt.term {
            out.push(surface_up);
        }
        // morph normal forms (already uppercase)
        for wf in tb.morph.items() {
            if let Some(nc) = &wf.normal_case {
                if !out.iter().any(|o| o == nc) { out.push(nc.clone()); }
            }
            if let Some(nf) = &wf.normal_full {
                if !out.iter().any(|o| o == nf) { out.push(nf.clone()); }
            }
        }
    }
    out
}

/// Collect a possibly hyphenated proper name (e.g. "Санкт-Петербург",
/// "Нью-Йорк").  Returns (uppercase_name, end_token).
fn collect_hyphenated_name(t: &TokenRef, sofa: &SourceOfAnalysis) -> (String, TokenRef) {
    let tb = t.borrow();
    let name = if let TokenKind::Text(txt) = &tb.kind {
        // term is already uppercase from morph engine
        &txt.term
    } else {
        return (String::new(), t.clone());
    };
    let next = tb.next.clone();
    let name_owned = name.clone();
    drop(tb);

    // Check for hyphen continuation (e.g. "-Петербург")
    if let Some(hyp) = next {
        let hb = hyp.borrow();
        if hb.whitespaces_before_count(sofa) == 0 && hb.length_char() == 1 {
            let ch = sofa.char_at(hb.begin_char);
            if ch == '-' || ch == '–' {
                let after = hb.next.clone();
                drop(hb);
                if let Some(part2) = after {
                    let p2b = part2.borrow();
                    if p2b.whitespaces_before_count(sofa) == 0 {
                        if let TokenKind::Text(txt2) = &p2b.kind {
                            // term is already uppercase
                            let full = format!("{}-{}", name_owned, txt2.term);
                            drop(p2b);
                            return (full, part2.clone());
                        }
                    }
                }
            }
        }
    }

    (name_owned, t.clone())
}
