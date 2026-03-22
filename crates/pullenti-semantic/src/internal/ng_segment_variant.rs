/// NGSegmentVariant — one valid assignment of NGLinks for a segment.
/// Mirrors `NGSegmentVariant.cs`.

use super::sent_item::{SentItem, SentItemType};
use super::ng_link::{NGLink, NGLinkType};
use super::ng_segment::NGSegment;

// AlgoParams defaults (mirroring AlgoParams.cs)
const LIST_BONUS: f64    = 2.0;

// ── NGSegmentVariant ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct NGSegmentVariant {
    pub coef:  f64,
    /// One entry per NGItem in the segment (None = no link for that item)
    pub links: Vec<Option<NGLink>>,
    /// The before_verb_idx of the source segment (for cross-segment validation)
    pub before_verb_sent_idx: Option<usize>,
}

impl NGSegmentVariant {
    /// Validate and score this variant.
    /// Returns Coef (negative means invalid).
    /// Mirrors `NGSegmentVariant.CalcCoef()`.
    pub fn calc_coef(&mut self, seg: &NGSegment, sent_items: &[SentItem]) -> f64 {
        // Sum raw link coefs
        self.coef = self.links.iter()
            .filter_map(|l| l.as_ref())
            .map(|l| l.coef)
            .sum();

        // Rule 1: No crossing non-reversed genitive links
        let n = self.links.len();
        for i in 0..n {
            let li1 = match &self.links[i] { Some(l) => l, None => continue };
            if li1.to_ord.is_none() || li1.reverce { continue; }
            let i0 = li1.to_ord.unwrap();
            if i0 >= i { return self.invalidate(); }
            for k in (i0 + 1)..i {
                let li = match &self.links[k] { Some(l) => l, None => continue };
                if li.to_verb_sent_idx.is_some() { return self.invalidate(); }
                let i1 = match li.to_ord { Some(v) => v, None => continue };
                if i1 < i0 || i1 > i { return self.invalidate(); }
                if li.typ == NGLinkType::List && li1.typ == NGLinkType::List && i0 == i1 {
                    return self.invalidate();
                }
            }
        }

        // Rule 2: List validity checks
        for i in 0..n {
            let list = match self.get_list(i, seg) { Some(l) => l, None => continue };
            // All non-first items must have some kind of and_before/or_before
            // (simplified from C# list validity check)
            let ors:  usize = list[1..].iter().filter(|&&idx| seg.items[idx].or_before).count();
            let ands: usize = list[1..].iter().filter(|&&idx| seg.items[idx].and_before).count();
            if ands > 0 && ors > 0 { return self.invalidate(); }
            // Check all have and_before if multiple
            if list.len() > 1 {
                let all_and = list[1..].iter().all(|&idx| seg.items[idx].and_before);
                if all_and {
                    self.coef += LIST_BONUS;
                } else {
                    let all_or = list[1..].iter().all(|&idx| seg.items[idx].and_before || seg.items[idx].or_before);
                    if !all_or { return self.invalidate(); }
                }
            }
        }

        // Rule 3: Max 1 Agent and 1 Pacient per verb (before/after)
        let mut bef_ag = 0usize;
        let mut bef_pac = 0usize;
        let mut aft_ag = 0usize;
        let mut aft_pac = 0usize;
        for li in self.links.iter().filter_map(|l| l.as_ref()) {
            if li.typ == NGLinkType::List || li.typ == NGLinkType::Actant { continue; }
            if !matches!(li.typ, NGLinkType::Agent | NGLinkType::Pacient) { continue; }
            let from_item = &seg.items[li.from_ord];
            let from_si = &sent_items[from_item.sent_item_idx];
            // Skip sub-sentence items
            if from_si.typ == SentItemType::SubSent { continue; }

            if let Some(bv) = seg.before_verb_idx {
                if li.to_verb_sent_idx == Some(bv) {
                    // Deeparticiple constraint: Agent to deeparticiple verb requires comma/and before
                    if li.typ == NGLinkType::Agent {
                        let is_deepart = sent_items[bv].verb_morph.as_ref()
                            .map_or(false, |vm| vm.is_deeparticiple);
                        if is_deepart {
                            let has_delim = (0..=li.from_ord)
                                .any(|ii| seg.items[ii].and_before || seg.items[ii].comma_before);
                            if !has_delim { return self.invalidate(); }
                        }
                        bef_ag += 1;
                    } else {
                        bef_pac += 1;
                    }
                    // Before-verb + after-verb: if both exist and before-verb is not a participle,
                    // require comma/and after the item
                    if seg.after_verb_idx.is_some() {
                        let bv_is_part = sent_items[bv].verb_morph.as_ref()
                            .map_or(false, |vm| {
                                vm.word_form.as_ref().map_or(false, |wf| {
                                    wf.base.class.is_adjective() // participles are tagged as adj in morph
                                })
                            });
                        if !bv_is_part {
                            let ind = li.from_ord;
                            let has_delim_after = (ind..seg.items.len())
                                .any(|ii| seg.items[ii].and_after || seg.items[ii].comma_after);
                            if !has_delim_after { return self.invalidate(); }
                        }
                    }
                }
            }
            if let Some(av) = seg.after_verb_idx {
                if li.to_verb_sent_idx == Some(av) {
                    if li.typ == NGLinkType::Agent { aft_ag += 1; }
                    else { aft_pac += 1; }
                    // After-verb + before-verb: require comma/and before item
                    if seg.before_verb_idx.is_some() {
                        let bv_idx = seg.before_verb_idx.unwrap();
                        let bv_is_part = sent_items[bv_idx].verb_morph.as_ref()
                            .map_or(false, |vm| {
                                vm.word_form.as_ref().map_or(false, |wf| wf.base.class.is_adjective())
                            });
                        if !bv_is_part {
                            let has_delim = (0..=li.from_ord)
                                .any(|ii| seg.items[ii].and_before || seg.items[ii].comma_before);
                            if !has_delim { return self.invalidate(); }
                        }
                    }
                }
            }
        }
        if bef_ag > 1 || bef_pac > 1 || aft_ag > 1 || aft_pac > 1 {
            return self.invalidate();
        }

        // Rule 4: Plural checks
        for i in 0..n {
            let li = match &self.links[i] { Some(l) => l, None => continue };
            if !matches!(li.typ, NGLinkType::Agent | NGLinkType::Pacient | NGLinkType::Genetive | NGLinkType::Participle) {
                continue;
            }
            if li.plural == 1 {
                // Plural required: check if this item is part of a list.
                // If not in a list, retry with noplural=true (mirrors C# CalcCoef(noplural=true)).
                // This allows inherently plural nouns (e.g. "строители") that have no list context.
                let mut ok = if li.typ == NGLinkType::Participle {
                    li.to_ord.map_or(false, |to_ord| self.get_list(to_ord, seg).is_some())
                } else {
                    self.get_list(i, seg).is_some()
                };
                if !ok {
                    let seg_items: Vec<usize> = seg.items.iter().map(|it| it.sent_item_idx).collect();
                    let mut link_copy = li.clone();
                    link_copy.calc_coef(&seg_items, sent_items, true);
                    if link_copy.coef > 0.0 { ok = true; }
                }
                if !ok { return self.invalidate(); }
            } else if li.plural == 0 {
                // Singular required: must NOT be in a list
                if self.get_list(i, seg).is_some() { return self.invalidate(); }
            }
        }

        self.coef
    }

    fn invalidate(&mut self) -> f64 {
        self.coef = -1.0;
        -1.0
    }

    /// Return the list of item indices (root + members) when `ord` is the list ROOT.
    /// Returns None if `ord` is a list MEMBER (has a noun→noun List link) or has no list.
    ///
    /// Mirrors C#'s `GetList(int ord)`:
    ///   - If Links[ord] is a noun→noun List link: the item is a MEMBER → return None.
    ///   - Otherwise, scan forward for items that have a List link pointing to `ord`.
    ///     If any found, `ord` is the ROOT → return [root, member1, member2, ...].
    pub fn get_list(&self, ord: usize, _seg: &NGSegment) -> Option<Vec<usize>> {
        if ord >= self.links.len() { return None; }
        let li = self.links[ord].as_ref()?;

        // If this item has a noun→noun List link, it is a MEMBER (not the root) → None.
        // (Matches C#: if (li.Typ==List && li.ToVerb==null) return null;)
        if li.typ == NGLinkType::List && li.to_verb_sent_idx.is_none() {
            return None;
        }

        // Scan forward: find items j > ord that have a noun→noun List link pointing to ord.
        // If found, ord is the ROOT of a list.
        let mut res: Option<Vec<usize>> = None;
        let mut ngit_ord = ord;
        for i in (ord + 1)..self.links.len() {
            if let Some(ref ll) = self.links[i] {
                if ll.typ == NGLinkType::List && ll.to_verb_sent_idx.is_none()
                    && ll.to_ord == Some(ngit_ord)
                {
                    if res.is_none() {
                        res = Some(vec![ord]);
                    }
                    res.as_mut().unwrap().push(i);
                    ngit_ord = i;
                }
            }
        }

        res.filter(|v| v.len() > 1)
    }
}

// ── create_variants ───────────────────────────────────────────────────────

/// Create all valid NGSegmentVariants for a segment (up to `max_count` best).
/// Mirrors `NGSegment.CreateVariants()`.
pub fn create_variants(seg: &mut NGSegment, sent_items: &[SentItem], max_count: usize) -> Vec<NGSegmentVariant> {
    let mut variants: Vec<NGSegmentVariant> = Vec::new();

    // Sort links descending by coef so the odometer explores high-coef variants first.
    // This is critical for large segments where the 1000-iteration cap would otherwise
    // cut off before reaching the best combinations.
    for item in &mut seg.items {
        item.links.sort_by(|a, b| b.coef.partial_cmp(&a.coef).unwrap_or(std::cmp::Ordering::Equal));
        item.ind = 0;
    }

    let n = seg.items.len();
    if n == 0 { return variants; }

    for _ in 0..1000 {
        // Build current variant
        let links: Vec<Option<NGLink>> = seg.items.iter().map(|it| {
            if it.ind < it.links.len() {
                Some(it.links[it.ind].clone())
            } else {
                None
            }
        }).collect();

        let mut var = NGSegmentVariant { coef: 0.0, links, before_verb_sent_idx: seg.before_verb_idx };
        let coef = var.calc_coef(seg, sent_items);
        if coef >= 0.0 {
            variants.push(var);
            if variants.len() > max_count * 5 {
                variants.sort_by(|a, b| b.coef.partial_cmp(&a.coef).unwrap_or(std::cmp::Ordering::Equal));
                variants.truncate(max_count);
            }
        }

        // Advance to next combination (odometer-style from the end)
        let mut j = n as isize - 1;
        loop {
            if j < 0 { break; }
            let it = &mut seg.items[j as usize];
            it.ind += 1;
            if it.ind >= it.links.len() + 1 {
                // +1 because we also allow "no link" (ind = links.len())
                it.ind = 0;
                j -= 1;
            } else {
                break;
            }
        }
        if j < 0 { break; }
    }

    variants.sort_by(|a, b| b.coef.partial_cmp(&a.coef).unwrap_or(std::cmp::Ordering::Equal));
    variants.truncate(max_count);
    variants
}
