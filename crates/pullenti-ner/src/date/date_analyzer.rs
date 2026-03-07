use std::rc::Rc;
use std::cell::RefCell;
use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::referent::Referent;
use crate::token::{Token, TokenRef};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::date::date_item_token::{DateItemToken, DateItemType, try_parse_list};
use crate::date::date_referent as dr;
use crate::date::date_range_referent as drr;
use crate::date::date_pointer_type::DatePointerType;

const DATE_TYPENAME: &str = "DATE";
const APPROX_CUR_YEAR: i32 = 2026;

/// The Date & DateRange analyzer — ports DateAnalyzer.cs (simplified, absolute dates only).
pub struct DateAnalyzer;

impl DateAnalyzer {
    pub fn new() -> Self { DateAnalyzer }
}

impl Analyzer for DateAnalyzer {
    fn name(&self) -> &'static str { "DATE" }
    fn caption(&self) -> &'static str { "Даты" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();

        let mut cur = kit.first_token.clone();
        while let Some(t) = cur.clone() {
            // Skip ignored tokens
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            let pli = match try_parse_list(&t, 20, &sofa) {
                None => {
                    cur = t.borrow().next.clone();
                    continue;
                }
                Some(list) if list.is_empty() => {
                    cur = t.borrow().next.clone();
                    continue;
                }
                Some(list) => list,
            };

            let last_end = pli.last().unwrap().end_token.clone();
            let high = is_high_context(&t, &sofa);
            let rts = try_attach(&pli, high, &sofa);

            if rts.is_empty() {
                cur = last_end.borrow().next.clone();
                continue;
            }

            // Register and embed each result
            for (referent, begin, end) in rts {
                let r_rc = Rc::new(RefCell::new(referent));
                kit.add_entity(r_rc.clone());
                let tok = Rc::new(RefCell::new(
                    Token::new_referent(begin, end, r_rc)
                ));
                kit.embed_token(tok.clone());
                cur = tok.borrow().next.clone();
            }
            // If cur not advanced by embed (embedding may set cur already)
            // fallback: move past last_end
        }

        // Second pass: build year ranges like "2020–2024"
        apply_date_ranges(kit, &sofa);
    }
}

// ── Context helper ────────────────────────────────────────────────────────────

fn is_high_context(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let mut depth = 0;
    let mut tt = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    while let Some(tok) = tt {
        depth += 1;
        if depth > 5 { break; }
        let tok_b = tok.borrow();
        if tok_b.is_char(':', sofa) || tok_b.is_hiphen(sofa) {
            tt = tok_b.prev.as_ref().and_then(|w| w.upgrade());
            continue;
        }
        if tok_b.is_newline_after(sofa) { break; }
        if tok_b.is_value("ДАТА", None) || tok_b.is_value("DATE", None)
            || tok_b.is_value("ГОД", None)
        {
            return true;
        }
        if tok_b.get_referent().map_or(false, |r| r.borrow().type_name == DATE_TYPENAME) {
            return true;
        }
        break;
    }
    false
}

// ── TryAttach ─────────────────────────────────────────────────────────────────

/// Returns `Vec<(Referent, begin_token, end_token)>` for each date found.
pub fn try_attach(
    dts: &[DateItemToken],
    high: bool,
    sofa: &SourceOfAnalysis,
) -> Vec<(Referent, TokenRef, TokenRef)> {
    if dts.is_empty() { return vec![]; }

    // Special: single "Today" pointer
    if dts.len() == 1 && dts[0].typ == DateItemType::Pointer
        && dts[0].ptr == DatePointerType::Today
    {
        let mut r = dr::new_date_referent();
        dr::set_pointer(&mut r, DatePointerType::Today);
        return vec![(r, dts[0].begin_token.clone(), dts[0].end_token.clone())];
    }

    // Special: Century alone
    if dts[0].typ == DateItemType::Century {
        let mut r = dr::new_date_referent();
        dr::set_century(&mut r, dts[0].int_value);
        return vec![(r, dts[0].begin_token.clone(), dts[0].end_token.clone())];
    }

    // Try formal DD.MM.YYYY
    if let Some(res) = apply_rule_formal(dts, high, sofa) {
        return vec![(res.referent, res.begin, res.end)];
    }
    // Try "DD месяц [YYYY]"
    if let Some(res) = apply_rule_with_month(dts, sofa) {
        return vec![(res.referent, res.begin, res.end)];
    }
    // Try year-only
    if let Some(res) = apply_rule_year_only(dts, sofa) {
        return vec![(res.referent, res.begin, res.end)];
    }

    // Standalone hour:minute
    if dts.len() >= 2 && dts[0].typ == DateItemType::Hour && dts[1].typ == DateItemType::Minute {
        let mut r = dr::new_date_referent();
        dr::set_hour(&mut r, dts[0].int_value);
        dr::set_minute(&mut r, dts[1].int_value);
        let end = if dts.len() >= 3 && dts[2].typ == DateItemType::Second {
            dr::set_second(&mut r, dts[2].int_value);
            dts[2].end_token.clone()
        } else {
            dts[1].end_token.clone()
        };
        return vec![(r, dts[0].begin_token.clone(), end)];
    }

    // Single year token.
    // 4-digit years (≥ 1000): accept always.
    // 2-digit years (50–99): only in high date context; they're too ambiguous otherwise
    // (e.g. "99 копеек" must NOT become year 1999).
    if dts.len() == 1 && dts[0].can_be_year() {
        let v = dts[0].int_value;
        let accept = v >= 1000 || high || {
            // Also accept 2-digit year if explicitly preceded by year preposition
            let prev = dts[0].begin_token.borrow().prev.as_ref().and_then(|w| w.upgrade());
            prev.map_or(false, |p| {
                p.borrow().is_value("В", None) || p.borrow().is_value("IN", None) || p.borrow().is_value("У", None)
            })
        };
        if accept {
            let mut r = dr::new_date_referent();
            dr::set_year(&mut r, dts[0].year());
            return vec![(r, dts[0].begin_token.clone(), dts[0].end_token.clone())];
        }
    }

    vec![]
}

// ── Rule result ───────────────────────────────────────────────────────────────

struct RuleResult {
    referent: Referent,
    begin:    TokenRef,
    end:      TokenRef,
}

// ── ApplyRuleFormal ───────────────────────────────────────────────────────────

/// Handle patterns: DD.MM.YYYY, YYYY.MM.DD, DD/MM/YYYY etc.
fn apply_rule_formal(
    its: &[DateItemToken],
    high: bool,
    sofa: &SourceOfAnalysis,
) -> Option<RuleResult> {
    let n = its.len();
    if n < 5 { return None; }

    let mut i = 0;
    while i + 4 < n {
        // Pattern: [i] Delim [i+2] Delim [i+4] where delims match
        if its[i + 1].typ != DateItemType::Delim || its[i + 3].typ != DateItemType::Delim {
            i += 1; continue;
        }
        if its[i + 1].string_value != its[i + 3].string_value { i += 1; continue; }

        // All 5 tokens must be adjacent (no whitespace between them)
        let contiguous = !its[i].is_whitespace_after(sofa)
            && !its[i + 1].is_whitespace_after(sofa)
            && !its[i + 2].is_whitespace_after(sofa)
            && !its[i + 3].is_whitespace_after(sofa);
        if !contiguous && !high { i += 1; continue; }

        let delim = match its[i + 1].string_value.chars().next() {
            Some(c) => c, None => { i += 1; continue; }
        };
        if !matches!(delim, '.' | '/' | '\\' | '-') { i += 1; continue; }

        let a = &its[i];
        let b = &its[i + 2];
        let c = &its[i + 4];

        // Determine layout: DD.MM.YYYY vs YYYY.MM.DD
        let (year_item, mon_item, day_opt): (&DateItemToken, &DateItemToken, Option<&DateItemToken>) =
            if a.can_be_year() && !b.can_by_month() {
                // YYYY.DD.month or skip
                i += 1; continue;
            } else if a.can_be_year() && b.can_by_month() {
                // YYYY.MM.DD
                (a, b, Some(c))
            } else if a.can_be_day() && b.can_by_month() {
                // DD.MM.YYYY
                (c, b, Some(a))
            } else if a.can_by_month() && b.can_be_day() {
                // MM.DD.YYYY (American)
                (c, a, Some(b))
            } else {
                i += 1; continue;
            };

        if !year_item.can_be_year() {
            if !high || year_item.int_value < 1000 { i += 1; continue; }
        }

        // Month sanity: single-digit must be zero-headed or year >= 1980
        if mon_item.int_value < 10 && !mon_item.is_zero_headed(sofa) {
            if year_item.year() < 1980 { i += 1; continue; }
        }

        // For "." and "-" delimiters, year must be >= 1900
        if matches!(delim, '.' | '-') && year_item.year() < 1900 { i += 1; continue; }

        let mut r = dr::new_date_referent();
        dr::set_year(&mut r, year_item.year());
        dr::set_month(&mut r, mon_item.int_value);
        if let Some(d) = day_opt { dr::set_day(&mut r, d.int_value); }

        return Some(RuleResult {
            referent: r,
            begin: its[i].begin_token.clone(),
            end:   its[i + 4].end_token.clone(),
        });
    }

    // High mode: also accept DD.MM.YY (2-digit year)
    if high && n >= 5 {
        let a = &its[0]; let b = &its[1]; let c = &its[2]; let d = &its[3]; let e = &its[4];
        if b.typ == DateItemType::Delim && d.typ == DateItemType::Delim
            && b.string_value == "." && d.string_value == "."
            && a.can_be_day() && c.can_by_month()
            && a.length_char() == 2 && c.length_char() == 2
            && (e.length_char() == 2 || e.length_char() == 4)
            && !a.is_whitespace_after(sofa) && !b.is_whitespace_after(sofa)
            && !c.is_whitespace_after(sofa) && !d.is_whitespace_after(sofa)
        {
            let yv = e.int_value;
            let year = if yv > 80 && yv < 100 { 1900 + yv }
                else if yv <= (APPROX_CUR_YEAR - 2000) { yv + 2000 }
                else { return None; };
            let mut r = dr::new_date_referent();
            dr::set_year(&mut r, year);
            dr::set_month(&mut r, c.int_value);
            dr::set_day(&mut r, a.int_value);
            return Some(RuleResult {
                referent: r,
                begin: its[0].begin_token.clone(),
                end:   its[4].end_token.clone(),
            });
        }
    }

    None
}

// ── ApplyRuleWithMonth ────────────────────────────────────────────────────────

/// Handle patterns: "DD месяца [YYYY]", "месяц [YYYY]", "[DD] месяц YYYY".
fn apply_rule_with_month(
    its: &[DateItemToken],
    sofa: &SourceOfAnalysis,
) -> Option<RuleResult> {
    let n = its.len();

    // Find the month token
    let mi = its.iter().position(|t| t.typ == DateItemType::Month)?;
    let mon_val = its[mi].int_value;

    // Day: the Number token just before the month (possibly separated by delim)
    let day_idx = if mi > 0 && its[mi - 1].can_be_day()
        && its[mi - 1].typ != DateItemType::Delim
    {
        Some(mi - 1)
    } else if mi > 1
        && its[mi - 1].typ == DateItemType::Delim
        && its[mi - 2].can_be_day()
    {
        Some(mi - 2)
    } else {
        None
    };

    // Year: scan forward from month token
    let year_idx = {
        let mut yi = None;
        let mut j = mi + 1;
        while j < n {
            match its[j].typ {
                DateItemType::Delim => { j += 1; continue; }
                DateItemType::Year => { yi = Some(j); break; }
                DateItemType::Number if its[j].can_be_year() => { yi = Some(j); break; }
                _ => break,
            }
        }
        yi
    };

    let year_val = year_idx.map_or(0, |yi| its[yi].year());

    // Only emit if we have at least year or day context
    if year_val == 0 && day_idx.is_none() {
        // Month-only: emit only if it's a real month token (not ambiguous number)
        if its[mi].typ == DateItemType::Month {
            let mut r = dr::new_date_referent();
            dr::set_month(&mut r, mon_val);
            return Some(RuleResult {
                referent: r,
                begin: its[mi].begin_token.clone(),
                end:   its[mi].end_token.clone(),
            });
        }
        return None;
    }

    let start_i = day_idx.unwrap_or(mi);
    let end_i   = year_idx.map_or(mi, |yi| yi);

    let mut r = dr::new_date_referent();
    if year_val != 0 { dr::set_year(&mut r, year_val); }
    dr::set_month(&mut r, mon_val);
    if let Some(di) = day_idx { dr::set_day(&mut r, its[di].int_value); }

    Some(RuleResult {
        referent: r,
        begin: its[start_i].begin_token.clone(),
        end:   its[end_i].end_token.clone(),
    })
}

// ── ApplyRuleYearOnly ─────────────────────────────────────────────────────────

/// Handle patterns: "YYYY год", "в YYYY году", "в YYYY".
fn apply_rule_year_only(
    its: &[DateItemToken],
    sofa: &SourceOfAnalysis,
) -> Option<RuleResult> {
    let n = its.len();
    if n == 0 { return None; }

    // Find explicit Year token
    let yi = its.iter().position(|t| t.typ == DateItemType::Year)?;
    let year_val = its[yi].year();
    if year_val <= 0 { return None; }

    // Optional preceding pointer (начало, конец, ...)
    let pointer = if yi > 0 && its[yi - 1].typ == DateItemType::Pointer {
        its[yi - 1].ptr
    } else {
        DatePointerType::No
    };

    let start_i = if pointer != DatePointerType::No { yi - 1 } else { yi };

    let mut r = dr::new_date_referent();
    dr::set_year(&mut r, year_val);
    if pointer != DatePointerType::No { dr::set_pointer(&mut r, pointer); }

    Some(RuleResult {
        referent: r,
        begin: its[start_i].begin_token.clone(),
        end:   its[yi].end_token.clone(),
    })
}

// ── Second pass: year ranges ──────────────────────────────────────────────────

fn apply_date_ranges(kit: &mut AnalysisKit, sofa: &SourceOfAnalysis) {
    let mut cur = kit.first_token.clone();
    while let Some(t) = cur.clone() {
        let is_date1 = t.borrow().get_referent()
            .map_or(false, |r| r.borrow().type_name == DATE_TYPENAME);
        if !is_date1 {
            cur = t.borrow().next.clone();
            continue;
        }

        let next1 = match t.borrow().next.clone() {
            Some(n) => n,
            None => { cur = None; continue; }
        };

        let is_connector = next1.borrow().is_hiphen(sofa)
            || next1.borrow().is_value("ПО", Some("ДО"))
            || (next1.borrow().is_value("И", None) && !next1.borrow().is_newline_before(sofa));

        if !is_connector {
            cur = t.borrow().next.clone();
            continue;
        }

        let next2 = match next1.borrow().next.clone() {
            Some(n) => n,
            None => { cur = t.borrow().next.clone(); continue; }
        };

        let is_date2 = next2.borrow().get_referent()
            .map_or(false, |r| r.borrow().type_name == DATE_TYPENAME);
        if !is_date2 {
            cur = t.borrow().next.clone();
            continue;
        }

        let year1 = t.borrow().get_referent()
            .map(|r| dr::get_year(&r.borrow())).unwrap_or(0);
        let year2 = next2.borrow().get_referent()
            .map(|r| dr::get_year(&r.borrow())).unwrap_or(0);

        if year1 == 0 || year2 == 0 || year1 >= year2 {
            cur = t.borrow().next.clone();
            continue;
        }

        let ref1 = t.borrow().get_referent().unwrap();
        let ref2 = next2.borrow().get_referent().unwrap();

        let mut range = drr::new_date_range_referent();
        drr::set_date_from(&mut range, ref1);
        drr::set_date_to(&mut range, ref2);

        let range_rc = Rc::new(RefCell::new(range));
        kit.add_entity(range_rc.clone());
        let rt = Rc::new(RefCell::new(
            Token::new_referent(t.clone(), next2.clone(), range_rc)
        ));
        kit.embed_token(rt.clone());
        cur = rt.borrow().next.clone();
    }
}
