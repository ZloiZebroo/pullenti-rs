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
use crate::address::street_table;

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
            {
                let tb = t.borrow();
                // Skip ignored, non-text, and single-char non-letter tokens early
                if tb.is_ignored(&sofa) || !matches!(tb.kind, TokenKind::Text(_)) {
                    cur = tb.next.clone();
                    continue;
                }
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

// ── Main parser ───────────────────────────────────────────────────────────────

fn try_parse(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // Check keyword matches while borrow is active, store results as owned values
    let (is_city_prefix, is_city_prefix_abbrev, type_kw): (bool, bool, Option<&'static str>) = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(txt) => {
                // term and normal forms are already uppercase from morph engine
                let term = &txt.term;
                let mut is_cp = geo_table::is_city_prefix(term);
                let mut is_cpa = geo_table::is_city_prefix_abbrev(term);
                let mut tkw: Option<&'static str> = geo_table::type_keyword(term).map(|(s, _)| s);

                for wf in tb.morph.items() {
                    // Skip word forms with verb class when looking for city/territory
                    // keywords — avoids matching verb forms like "сел" (past tense of
                    // "сесть") via their noun normal form "СЕЛО".
                    let is_verb_form = wf.base.class.is_verb();
                    if let Some(nc) = &wf.normal_case {
                        if !is_verb_form {
                            if !is_cp { is_cp = geo_table::is_city_prefix(nc); }
                            if !is_cpa { is_cpa = geo_table::is_city_prefix_abbrev(nc); }
                            if tkw.is_none() {
                                tkw = geo_table::type_keyword(nc).map(|(s, _)| s);
                            }
                        }
                    }
                    if let Some(nf) = &wf.normal_full {
                        if !is_verb_form {
                            if !is_cp { is_cp = geo_table::is_city_prefix(nf); }
                            if !is_cpa { is_cpa = geo_table::is_city_prefix_abbrev(nf); }
                            if tkw.is_none() {
                                tkw = geo_table::type_keyword(nf).map(|(s, _)| s);
                            }
                        }
                    }
                    if is_cp && tkw.is_some() { break; }
                }
                (is_cp, is_cpa, tkw)
            }
            _ => return None,
        }
    };

    // 1. City type prefix keyword (г., город, г, etc.)
    if is_city_prefix {
        if let Some(result) = try_city_prefix(t, is_city_prefix_abbrev, sofa) {
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

fn try_city_prefix(t: &TokenRef, is_abbrev: bool, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    let next = t.borrow().next.clone()?;
    let nb = next.borrow();
    // Skip dots ONLY for abbreviation-style prefixes (г., дер., сел., etc.).
    // Full-word prefixes (город, деревня, село) must NOT skip a trailing "." —
    // it is sentence-ending punctuation, not an abbreviation separator.
    if is_abbrev && nb.length_char() == 1 && sofa.char_at(nb.begin_char) == '.' {
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
    // Reject common Russian pronouns/particles that can never be city names
    // (they appear after sentence-ending "." as new sentences begin with "Я", "А", etc.)
    {
        let tb = name_tok.borrow();
        if let TokenKind::Text(txt) = &tb.kind {
            if matches!(txt.term.as_str(),
                "Я" | "МЫ" | "ТЫ" | "ВЫ" | "ОН" | "ОНА" | "ОНО" | "ОНИ" |
                "НО" | "А" | "И" | "ИЛИ" | "ЧТО" | "КТО" | "КАК" |
                "НЕ" | "НИ" | "ДА" | "НЕТ" | "ЕСЛИ" | "ТО" | "АЛЕ"
            ) {
                return None;
            }
        }
    }
    // Try the token and following tokens as a multi-word city name
    let candidates = collect_candidates(name_tok, sofa);

    // Try two-word city name first (e.g. "Нижний Новгород") before accepting a
    // single-word match — prevents "г. Нижний" when "г. Нижний Новгород" is intended.
    if let Some(result) = try_two_word_name(name_tok, &candidates, sofa) {
        return Some(result);
    }

    // Try hyphenated table lookup before single-word (e.g. "Ростов-на-Дону").
    // A definitive table match takes priority over the is_proper_surname morph tag —
    // "Ростов" is tagged as a surname but "Ростов-на-Дону" is unambiguously a city.
    if let Some(result) = try_hyphenated_name(name_tok, &candidates, sofa) {
        return Some(result);
    }

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
    // if the token is a proper noun (starts with uppercase).
    // But do NOT create fallback entities for tokens tagged as proper person names
    // or surnames — e.g. "город Максим" where "Максим" is a given name, not a city.
    let tb = name_tok.borrow();
    if let TokenKind::Text(_) = &tb.kind {
        let surface = sofa.substring(tb.begin_char, tb.end_char);
        if surface.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !tb.chars.is_all_lower()
        {
            // Reject proper person names / surnames used as "city" fallback
            let is_person_name = tb.morph.items().iter().any(|wf|
                wf.base.class.is_proper_name() || wf.base.class.is_proper_surname()
            );
            if !is_person_name {
                drop(tb);
                // Try multi-word hyphenated city (e.g. "Санкт-Петербург")
                let (full_name, end_tok) = collect_hyphenated_name(name_tok, sofa);
                let mut r = gr::new_geo_referent();
                gr::add_name(&mut r, &full_name);
                gr::add_type(&mut r, "город");
                return Some((r, end_tok));
            }
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
        // Also skip names tagged as proper person names or surnames.
        if has_cyrillic {
            let is_person_name = {
                let nb2 = next.borrow();
                nb2.morph.items().iter().any(|wf|
                    wf.base.class.is_proper_name() || wf.base.class.is_proper_surname()
                )
            };
            if !is_person_name {
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
    }
    None
}

// ── Pattern 3: Direct name lookup ────────────────────────────────────────────
//
// "Россия", "Москва", "США", "РФ"

fn try_direct_name(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<(Referent, TokenRef)> {
    // ── Pre-lookup guards ──────────────────────────────────────────────────────
    // Compute all scalar properties in one borrow block — no sofa.substring() allocation,
    // O(1) length via length_char(), O(1) first char via sofa.char_at().
    // is_all_ascii is computed once from txt.term bytes (txt.term IS uppercase surface),
    // avoiding 3× repeated surface.chars() scans in the candidates loop.
    let (term, char_count, first_char, is_all_ascii, is_adj, is_all_upper): (String, i32, char, bool, bool, bool) = {
        let tb = t.borrow();
        match &tb.kind {
            TokenKind::Text(txt) => {
                let fc = sofa.char_at(tb.begin_char);
                let all_ascii = txt.term.bytes().all(|b| b.is_ascii_alphabetic());
                let adj = tb.morph.items().iter().any(|wf| wf.base.class.is_adjective());
                (txt.term.clone(), tb.length_char(), fc, all_ascii, adj, tb.chars.is_all_upper())
            }
            _ => return None,
        }
    };

    // Guard 1 — Geo proper nouns always start with an uppercase letter.
    //   Rejects common English words mid-sentence: "early", "long", "ness", …
    if first_char.is_alphabetic() && first_char.is_lowercase() {
        return None;
    }

    // Guard 1b — Russian function words (conjunctions, particles) that begin a
    //   sentence with an uppercase letter but are never place names.
    if matches!(term.as_str(),
        "ЕСЛИ" | "КОГДА" | "ХОТЯ" | "ПОКА" | "ПУСТЬ" | "ПОТОМУ" |
        "ПОЭТОМУ" | "ОДНАКО" | "ЗАТО" | "ЛИБО" | "ТОЖЕ" | "ТАКЖЕ" |
        "КОТОРЫЙ" | "КОТОРАЯ" | "КОТОРОЕ" | "КОТОРЫЕ" |
        "ЧТОБЫ" | "ПРИЧЁМ" | "ПРИТОМ"
    ) {
        return None;
    }

    // Guard 2 — Very short tokens (≤ 2 chars).
    if char_count <= 2 {
        // 2a. Must be all-uppercase (CharsInfo flag — O(1), no chars() scan).
        //   Rejects "At", "In", "Li", "re", "by", … (title-case/lowercase).
        if !is_all_upper {
            return None;
        }
        // 2b. All-ASCII-Latin 2-char tokens are too ambiguous.
        //   "IR"=Information Retrieval vs Iran, "ID"=Identifier vs Indonesia,
        //   Cyrillic abbreviations like "РФ", "ЕС" are NOT affected.
        if is_all_ascii {
            return None;
        }
        // 2c. All-uppercase 2-char token immediately followed by apostrophe →
        //   possessive (e.g. "AI's"), not a standalone geo entity.
        //   Use sofa.char_at() instead of substring() — no allocation.
        let tb = t.borrow();
        if let Some(next) = tb.next.clone() {
            let nb = next.borrow();
            if nb.whitespaces_before_count(sofa) == 0 {
                let nc = sofa.char_at(nb.begin_char);
                if nc == '\'' || nc == '\u{2019}' {
                    return None;
                }
            }
        }
    }

    // ── Single-token table lookup ──────────────────────────────────────────────
    // is_adj and is_all_ascii already computed above — no extra borrow needed here.
    let candidates = collect_candidates(t, sofa);

    for c in &candidates {
        if let Some(entry) = geo_table::lookup_name(c) {
            // Skip adjective matches for regions (handled by pattern 4).
            if is_adj && matches!(entry.kind, GeoEntryKind::Region) {
                continue;
            }

            // Guard 3 — Selective filter for pure-ASCII-Latin surfaces.
            // Uses pre-computed is_all_ascii — no repeated chars() scan.
            if is_all_ascii {
                let should_block = match entry.kind {
                    // City entries: block only very short canonical names (≤ 4 chars).
                    GeoEntryKind::City => entry.canonical_name.chars().count() <= 4,
                    // Region entries: block sub-national county / district units.
                    GeoEntryKind::Region => matches!(
                        entry.type_str.as_str(),
                        "county" | "графство" | "район" | "уезд" | "волость" | "district"
                    ),
                    // State (sovereign country): never blocked.
                    GeoEntryKind::State => false,
                };
                if should_block { continue; }
            }

            // ── Guard: ASCII city/region in person-name context.
            if matches!(entry.kind, GeoEntryKind::City | GeoEntryKind::Region) && is_all_ascii {
                if in_person_name_context(t, sofa) { continue; }
            }

            // ── Guard: city/region preceded by a street type keyword → part of address
            if matches!(entry.kind, GeoEntryKind::City | GeoEntryKind::Region) {
                if preceded_by_street_type(t, sofa) { continue; }
            }

            // ── Guard: Cyrillic city/region with a proper-surname morph class
            //    OR preceded by a Russian patronymic → likely a surname.
            if matches!(entry.kind, GeoEntryKind::City | GeoEntryKind::Region) && !is_all_ascii {
                let is_surname = {
                    let tb = t.borrow();
                    tb.morph.items().iter().any(|wf| wf.base.class.is_proper_surname())
                };
                if is_surname {
                    // Check if preceded by patronymic OR first name → person context
                    let prev_is_name_context = t.borrow().prev.as_ref()
                        .and_then(|w| w.upgrade())
                        .map(|prev| {
                            let pb = prev.borrow();
                            if let TokenKind::Text(_) = &pb.kind {
                                pb.morph.items().iter().any(|wf|
                                    wf.base.class.is_proper_secname() || wf.base.class.is_proper_name()
                                )
                            } else { false }
                        })
                        .unwrap_or(false);
                    if prev_is_name_context { continue; }

                    // Check if FOLLOWED by initials pattern (e.g. "К.Л.") → person context
                    if followed_by_initials(t, sofa) { continue; }

                    // Check if PRECEDED by initials pattern (e.g. "И.И. Ивановым")
                    if preceded_by_initials(t, sofa) { continue; }

                    // Check if preceded by a person title/prefix (г-жа, господин, etc.)
                    if preceded_by_person_title(t, sofa) { continue; }
                }
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

/// Returns true if the next token is a single uppercase letter followed by a period
/// (e.g. "Николаев К.Л." — "К" + "."), indicating a person-name initials pattern.
/// Returns true when the token is preceded by an initials pattern like "И.И."
/// (walking backwards: dot, letter, dot, letter — with no whitespace between each).
fn preceded_by_initials(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let tb = t.borrow();
    if tb.whitespaces_before_count(sofa) == 0 { return false; }
    // Walk backwards: expect "." "X" "." "X" (reverse of "X.X.")
    let dot1 = match tb.prev.as_ref().and_then(|w| w.upgrade()) {
        Some(d) => d,
        None => return false,
    };
    drop(tb);
    {
        let db = dot1.borrow();
        if db.length_char() != 1 || sofa.char_at(db.begin_char) != '.' { return false; }
        if db.whitespaces_before_count(sofa) != 0 { return false; }
    }
    let letter1 = match dot1.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
        Some(l) => l,
        None => return false,
    };
    {
        let lb = letter1.borrow();
        let surface = sofa.substring(lb.begin_char, lb.end_char);
        if surface.chars().count() != 1 { return false; }
        let ch = match surface.chars().next() {
            Some(c) if c.is_uppercase() && c.is_alphabetic() => c,
            _ => return false,
        };
        let _ = ch;
    }
    true
}

/// Returns true if the token is preceded by a person-title word or abbreviation
/// like "г-жа", "г-н", "господин", "директор", etc.
fn preceded_by_person_title(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let prev = match t.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
        Some(p) => p,
        None => return false,
    };
    let pb = prev.borrow();
    if let TokenKind::Text(ref txt) = pb.kind {
        // Direct title match
        if is_geo_person_title(&txt.term) { return true; }
        // Check morph lemma for declined forms
        let morph_match = pb.morph.items().iter().any(|wf| {
            wf.normal_case.as_ref().map(|nc| is_geo_person_title(nc)).unwrap_or(false)
        });
        if morph_match { return true; }
    }
    drop(pb);

    // Check for "г-жа" / "г-н" pattern: prev = "жа"/"н", prev-1 = "-", prev-2 = "г"
    let prev_term = {
        let pb = prev.borrow();
        match &pb.kind {
            TokenKind::Text(ref txt) => Some(txt.term.clone()),
            _ => None,
        }
    };
    if let Some(pt) = prev_term {
        if pt == "ЖА" || pt == "Н" {
            let hyp = prev.borrow().prev.as_ref().and_then(|w| w.upgrade());
            if let Some(h) = hyp {
                let hb = h.borrow();
                if hb.whitespaces_before_count(sofa) == 0
                    && hb.length_char() == 1
                    && sofa.char_at(hb.begin_char) == '-'
                {
                    let g = hb.prev.as_ref().and_then(|w| w.upgrade());
                    drop(hb);
                    if let Some(g) = g {
                        let gb = g.borrow();
                        if let TokenKind::Text(ref txt) = gb.kind {
                            if txt.term == "Г" { return true; }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Subset of person-title terms used by GEO guard to detect person context.
fn is_geo_person_title(term: &str) -> bool {
    matches!(term,
        "ГОСПОДИН" | "ГОСПОЖА" | "ДИРЕКТОР" | "ПРЕЗИДЕНТ" | "МИНИСТР" |
        "ПРОФЕССОР" | "ДОКТОР" | "ГРАЖДАНИН" | "ГРАЖДАНКА" |
        "КЛИЕНТ" | "КЛИЕНТКА" | "ЮРИСТ" | "АДВОКАТ" | "ВРАЧ" |
        "СУДЬЯ" | "ДЕПУТАТ" | "ГУБЕРНАТОР" | "ГЕНЕРАЛ" |
        "АКАДЕМИК" | "РЕКТОР" | "ДЕКАН" | "НАЧАЛЬНИК" |
        "РУКОВОДИТЕЛЬ" | "ЗАМЕСТИТЕЛЬ" | "СЕКРЕТАРЬ" |
        "ТОВАРИЩ" | "MR" | "MRS" | "MS" | "DR"
    )
}

/// Returns true if the token is preceded by a street type keyword
/// (проспект, шоссе, бульвар, etc.) — meaning this word is a street name, not a GEO entity.
fn preceded_by_street_type(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let prev = match t.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
        Some(p) => p,
        None => return false,
    };
    let pb = prev.borrow();
    if let TokenKind::Text(ref txt) = pb.kind {
        // Check surface term
        if street_table::lookup_street_type(&txt.term).is_some() { return true; }
        // Check morph normal forms (for declined forms like "проспекту")
        if pb.morph.items().iter().any(|wf| {
            wf.normal_case.as_deref().map_or(false, |s| street_table::lookup_street_type(s).is_some())
            || wf.normal_full.as_deref().map_or(false, |s| street_table::lookup_street_type(s).is_some())
        }) {
            return true;
        }
    }
    // Also check if prev is a dot preceded by a street abbreviation (e.g. "ул." → [ул][.])
    if let TokenKind::Text(_) = pb.kind {} else {
        drop(pb);
        let pb2 = prev.borrow();
        if pb2.length_char() == 1 && sofa.char_at(pb2.begin_char) == '.' {
            if let Some(before_dot) = pb2.prev.as_ref().and_then(|w| w.upgrade()) {
                let bb = before_dot.borrow();
                if let TokenKind::Text(ref txt) = bb.kind {
                    if street_table::lookup_street_type(&txt.term).is_some() { return true; }
                }
            }
        }
    }
    false
}

fn followed_by_initials(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let next = match t.borrow().next.clone() {
        Some(n) => n,
        None => return false,
    };
    let nb = next.borrow();
    if nb.whitespaces_before_count(sofa) == 0 { return false; }
    let surface = sofa.substring(nb.begin_char, nb.end_char);
    if surface.chars().count() != 1 { return false; }
    let ch = match surface.chars().next() {
        Some(c) if c.is_uppercase() && c.is_alphabetic() => c,
        _ => return false,
    };
    let _ = ch;
    let dot = match nb.next.clone() {
        Some(d) => d,
        None => return false,
    };
    drop(nb);
    let db = dot.borrow();
    db.whitespaces_before_count(sofa) == 0 && db.length_char() == 1 && sofa.char_at(db.begin_char) == '.'
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
    // Check previous token — use whitespace before t (between prev and t), not before prev.
    let ws_before_t = t.borrow().whitespaces_before_count(sofa);
    if ws_before_t <= 1 {
        if let Some(prev_weak) = t.borrow().prev.clone() {
            if let Some(prev) = prev_weak.upgrade() {
                if is_ascii_name_candidate(&prev) {
                    return true;
                }
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
        // txt.term is already the uppercase surface — no sofa.substring() needed
        out.push(txt.term.clone());
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
    let _ = sofa; // suppress unused warning
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
