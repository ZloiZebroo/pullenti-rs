/// Subsent — splits a sentence at DelimToken/ConjunctionToken boundaries
/// and establishes ownership relationships (if/then/else, because, but, …).
/// Mirrors `Subsent.cs`.

use std::collections::HashSet;

use super::sent_item::{SentItem, SentItemType, SentItemSource};
use super::delim_token::DelimType;
use pullenti_ner::core::conjunction::ConjunctionType;
use crate::types::SemFraglinkType;

// ── Subsent ───────────────────────────────────────────────────────────────

pub struct Subsent {
    /// Indices into the original `sent_items` slice (non-delimiter content)
    pub item_indices: Vec<usize>,
    /// DelimType values from any `DelimToken`s that precede this group
    pub delim_types: Vec<DelimType>,
    /// True if a ConjunctionToken (И/ИЛИ/etc.) precedes this group
    pub conj_before: bool,
    /// True if the preceding conjunction is "or" type
    pub or_before: bool,
    /// Index in the subsent list of the "owner" (e.g. the IF clause is owner of THEN)
    pub owner_idx: Option<usize>,
    pub is_or: bool,
    pub question: Option<String>,
    pub typ: SemFraglinkType,
    pub is_then_else_root: bool,
}

impl Subsent {
    /// True if any of our DelimType flags intersect with `typ`.
    pub fn check(&self, typ: DelimType) -> bool {
        self.delim_types.iter().any(|&dt| (dt as u32 & typ as u32) != 0)
    }

    /// True if this group is preceded by an "and/or" conjunction (or DelimType::And).
    pub fn check_and(&self) -> bool {
        self.conj_before || self.check(DelimType::And)
    }

    /// True if the preceding conjunction is "or".
    pub fn check_or(&self) -> bool {
        self.or_before
    }

    /// True when this group has NO DelimTokens (only conjunctions or nothing).
    pub fn only_conj(&self) -> bool {
        self.delim_types.is_empty()
    }

    /// True if `self` can be followed by `next` in a list (delimiters compatible).
    pub fn can_be_next_in_list(&self, next: &Subsent) -> bool {
        if next.delim_types.is_empty() && !next.conj_before {
            return true;
        }
        for &dt in &next.delim_types {
            if !self.check(dt) {
                return false;
            }
        }
        true
    }
}

// ── create_subsents ───────────────────────────────────────────────────────

/// Walk up the owner chain and return the root index.
fn owner_root_of(res: &[Subsent], start: usize) -> Option<usize> {
    let mut cur = res[start].owner_idx?;
    for _ in 0..100 {
        match res[cur].owner_idx {
            None    => return Some(cur),
            Some(p) => cur = p,
        }
    }
    None
}

/// Split `sent_items` into logical sub-sentences at Delim/Conj boundaries,
/// then assign ownership relationships.
/// `list_char_set` contains char positions covered by list NGLinks — conjunctions
/// at those positions are NOT treated as sentence delimiters.
pub fn create_subsents(
    sent_items:    &[SentItem],
    list_char_set: &HashSet<i32>,
) -> Vec<Subsent> {
    if sent_items.is_empty() {
        return vec![];
    }

    // ── Phase 1: split into raw groups ────────────────────────────────────

    let mut res: Vec<Subsent> = Vec::new();
    let mut current = Subsent {
        item_indices: Vec::new(),
        delim_types:  Vec::new(),
        conj_before:  false,
        or_before:    false,
        owner_idx:    None,
        is_or:        false,
        question:     None,
        typ:          SemFraglinkType::Undefined,
        is_then_else_root: false,
    };
    let mut has_verb = false;

    for (i, it) in sent_items.iter().enumerate() {
        // Determine if this item is a sentence delimiter
        let mut is_delim = false;

        if it.typ == SentItemType::Delim {
            is_delim = true;
        } else if it.typ == SentItemType::Conj {
            let begin = it.begin_char();
            if !list_char_set.contains(&begin) {
                is_delim = true;
                // Comma without prior verb → keep together (list, not split)
                if let SentItemSource::Conj(cnj) = &it.source {
                    if cnj.typ == ConjunctionType::Comma && !has_verb {
                        is_delim = false;
                    }
                }
            }
        }

        if !is_delim {
            if it.typ == SentItemType::Verb { has_verb = true; }
            current.item_indices.push(i);
            continue;
        }

        // Accumulate the delimiter into the NEXT subsent
        if current.item_indices.is_empty() {
            // No content yet: attach this delim to the current (future) subsent
            match &it.source {
                SentItemSource::Delim(dlm) => current.delim_types.push(dlm.typ),
                SentItemSource::Conj(cnj)  => {
                    current.conj_before = true;
                    if cnj.typ == ConjunctionType::Or { current.or_before = true; }
                }
                _ => {}
            }
            continue;
        }

        // Flush the current subsent and start a new one with this delimiter
        let mut next_ss = Subsent {
            item_indices: Vec::new(),
            delim_types:  Vec::new(),
            conj_before:  false,
            or_before:    false,
            owner_idx:    None,
            is_or:        false,
            question:     None,
            typ:          SemFraglinkType::Undefined,
            is_then_else_root: false,
        };
        match &it.source {
            SentItemSource::Delim(dlm) => next_ss.delim_types.push(dlm.typ),
            SentItemSource::Conj(cnj) => {
                next_ss.conj_before = true;
                if cnj.typ == ConjunctionType::Or { next_ss.or_before = true; }
            }
            _ => {}
        }
        res.push(std::mem::replace(&mut current, next_ss));
        has_verb = false;
    }

    if !current.item_indices.is_empty() {
        res.push(current);
    }

    // ── Phase 2: assign ownership ──────────────────────────────────────────

    let n = res.len();

    for i in 0..n {
        if res[i].check(DelimType::If) {
            let mut has_then = false;
            let mut has_else = false;
            for j in (i + 1)..n {
                if res[j].check(DelimType::Then) {
                    if has_then { break; }
                    res[j].owner_idx = Some(i);
                    res[j].question  = Some("если".to_string());
                    res[j].typ       = SemFraglinkType::IfThen;
                    has_then         = true;
                    res[i].is_then_else_root = true;
                } else if res[j].check(DelimType::Else) {
                    if has_else { break; }
                    res[j].owner_idx = Some(i);
                    res[j].question  = Some("иначе".to_string());
                    res[j].typ       = SemFraglinkType::IfElse;
                    has_else         = true;
                    res[i].is_then_else_root = true;
                } else if res[j].check(DelimType::If) {
                    if res[j].check(DelimType::And) {
                        res[j].owner_idx = Some(i);
                    } else {
                        break;
                    }
                }
            }
            if !has_then && i > 0 {
                if res[0].owner_idx.is_none() && res[0].only_conj() {
                    res[0].owner_idx = Some(i);
                    res[0].question  = Some("если".to_string());
                    res[i].is_then_else_root = true;
                    res[0].typ       = SemFraglinkType::IfThen;
                } else if res[0].owner_idx.is_some() {
                    res[i].owner_idx = Some(0);
                    res[i].question  = Some("если".to_string());
                    res[i].typ       = SemFraglinkType::IfThen;
                }
            }
            continue;
        }

        if res[i].check(DelimType::Because) {
            let mut has_then = false;
            for j in (i + 1)..n {
                if res[j].check(DelimType::Then) {
                    if has_then { break; }
                    res[j].owner_idx = Some(i);
                    res[j].question  = Some("по причине".to_string());
                    res[j].typ       = SemFraglinkType::Because;
                    has_then         = true;
                    res[i].is_then_else_root = true;
                }
            }
            if !has_then && i > 0 {
                if res[0].owner_idx.is_none() && res[0].only_conj() {
                    res[0].owner_idx = Some(i);
                    res[0].question  = Some("по причине".to_string());
                    res[i].is_then_else_root = true;
                    res[0].typ       = SemFraglinkType::Because;
                    continue;
                }
            }
            if !has_then && i + 1 < n {
                if res[i + 1].owner_idx.is_none() && res[i + 1].only_conj() {
                    res[i + 1].owner_idx = Some(i);
                    res[i + 1].question  = Some("по причине".to_string());
                    res[i].is_then_else_root = true;
                    res[i + 1].typ       = SemFraglinkType::Because;
                    continue;
                }
            }
            continue;
        }

        if res[i].check(DelimType::But) && i > 0 {
            if res[i - 1].owner_idx.is_none() && res[i - 1].only_conj() {
                res[i - 1].owner_idx = Some(i);
                res[i - 1].question  = Some("но".to_string());
                res[i].is_then_else_root = true;
                res[i - 1].typ       = SemFraglinkType::But;
                continue;
            }
        }

        if res[i].check(DelimType::What) && i > 0 {
            if res[i - 1].owner_idx.is_none() && res[i - 1].only_conj() {
                res[i - 1].owner_idx = Some(i);
                res[i - 1].question  = Some("что".to_string());
                res[i].is_then_else_root = true;
                res[i - 1].typ       = SemFraglinkType::What;
                continue;
            }
        }

        if res[i].check(DelimType::For) {
            if i + 1 < n && res[i + 1].owner_idx.is_none() && res[i + 1].only_conj() {
                res[i + 1].owner_idx = Some(i);
                res[i + 1].question  = Some("чтобы".to_string());
                res[i].is_then_else_root = true;
                res[i + 1].typ       = SemFraglinkType::For;
                continue;
            }
            if i > 0 && res[i - 1].owner_idx.is_none() && res[i - 1].only_conj() {
                res[i - 1].owner_idx = Some(i);
                res[i - 1].question  = Some("чтобы".to_string());
                res[i].is_then_else_root = true;
                res[i - 1].typ       = SemFraglinkType::For;
                continue;
            }
        }
    }

    // ── Phase 3: merge "and/or"-connected subsents ─────────────────────────

    let mut i = 1usize;
    while i < res.len() {
        let should_merge = res[i].check_and() && res[i].owner_idx.is_none();
        if !should_merge {
            i += 1;
            continue;
        }

        let mut merged = false;
        let mut j = i as isize - 1;
        while j >= 0 {
            let ji = j as usize;

            // Compute can_be_next_in_list
            let can = {
                let (rr, r) = (&res[ji], &res[i]);
                let rr_root_can = rr.owner_idx.map_or(true, |oi| {
                    owner_root_of(&res, oi)
                        .map_or(true, |root| res[root].can_be_next_in_list(r))
                });
                rr.can_be_next_in_list(r) && (rr.owner_idx.is_none() || rr_root_can)
            };

            if can {
                if res[i].check_or() { res[ji].is_or = true; }
                let extra = std::mem::take(&mut res[i].item_indices);
                res[ji].item_indices.extend(extra);
                res.remove(i);
                if i > 0 { i -= 1; }
                merged = true;
                break;
            }
            j -= 1;
        }

        if !merged { i += 1; }
    }

    res
}
