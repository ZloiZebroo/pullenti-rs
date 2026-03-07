/// NGItem + NGSegment — groups of noun phrases around a verb.
/// Mirrors `NGItem.cs` and `NGSegment.cs`.

use super::sent_item::{SentItem, SentItemType};
use super::ng_link::{NGLink, NGLinkType};

// ── NGItem ────────────────────────────────────────────────────────────────

/// One element in a noun-group segment.
/// `sent_item_idx` is the index into the *sentence* item list.
#[derive(Debug, Clone)]
pub struct NGItem {
    pub sent_item_idx: usize,
    /// position within the segment's items vec
    pub order:         usize,
    pub comma_before:  bool,
    pub and_before:    bool,
    pub or_before:     bool,
    pub comma_after:   bool,
    pub and_after:     bool,
    pub or_after:      bool,
    /// candidate links for this item (from/to within the same segment)
    pub links:         Vec<NGLink>,
    /// current link index during variant enumeration (CreateVariants)
    pub ind:           usize,
}

// ── NGSegment ────────────────────────────────────────────────────────────

/// A group of noun/adverb items flanked optionally by before/after verbs.
#[derive(Debug)]
pub struct NGSegment {
    /// index in sentence items of the verb *before* this segment (None if none)
    pub before_verb_idx: Option<usize>,
    /// items within this segment (noun phrases, adverbs, etc.)
    pub items: Vec<NGItem>,
    /// index in sentence items of the verb *after* this segment (None if none)
    pub after_verb_idx:  Option<usize>,
}

impl NGSegment {
    /// Create all NGSegments from a sentence item list.
    /// Mirrors `NGSegment.CreateSegments(Sentence s)`.
    pub fn create_segments(sent_items: &[SentItem]) -> Vec<NGSegment> {
        let mut res: Vec<NGSegment> = Vec::new();

        let mut i = 0;
        while i < sent_items.len() {
            let it = &sent_items[i];
            if it.typ == SentItemType::Verb || it.typ == SentItemType::Delim {
                i += 1;
                continue;
            }

            let mut seg = NGSegment {
                before_verb_idx: None,
                items: Vec::new(),
                after_verb_idx: None,
            };

            let mut nit = NGItem {
                sent_item_idx: i,
                order: 0,
                comma_before: false,
                and_before: false,
                or_before: false,
                comma_after: false,
                and_after: false,
                or_after: false,
                links: Vec::new(),
                ind: 0,
            };

            // Look backwards for BeforeVerb and comma/and flags
            let mut j = i as isize - 1;
            while j >= 0 {
                let prev = &sent_items[j as usize];
                if prev.typ == SentItemType::Verb {
                    seg.before_verb_idx = Some(j as usize);
                    break;
                }
                if prev.typ == SentItemType::Delim {
                    break;
                }
                if prev.can_be_comma_end() {
                    if prev.is_conj_or_type(pullenti_ner::core::conjunction::ConjunctionType::Comma) {
                        nit.comma_before = true;
                    } else {
                        nit.and_before = true;
                        if prev.is_conj_or_type(pullenti_ner::core::conjunction::ConjunctionType::Or) {
                            nit.or_before = true;
                        }
                    }
                }
                if prev.typ == SentItemType::Conj || prev.can_be_noun() {
                    break;
                }
                j -= 1;
            }

            // Collect items forward
            let mut comma = false;
            let mut and   = false;
            let mut or    = false;
            seg.items.push(nit);

            i += 1;
            while i < sent_items.len() {
                let it = &sent_items[i];
                if it.can_be_comma_end() {
                    comma = false; and = false; or = false;
                    if it.is_conj_or_type(pullenti_ner::core::conjunction::ConjunctionType::Comma) {
                        comma = true;
                    } else {
                        and = true;
                        if it.is_conj_or_type(pullenti_ner::core::conjunction::ConjunctionType::Or) {
                            or = true;
                        }
                    }
                    // Set after-flags on last item
                    if !seg.items.is_empty() {
                        let last = seg.items.last_mut().unwrap();
                        if comma { last.comma_after = true; }
                        else { last.and_after = true; if or { last.or_after = true; } }
                    }
                    i += 1;
                    continue;
                }
                if it.can_be_noun() || it.typ == SentItemType::Adverb {
                    let order = seg.items.len();
                    seg.items.push(NGItem {
                        sent_item_idx: i,
                        order,
                        comma_before: comma,
                        and_before:   and,
                        or_before:    or,
                        comma_after: false,
                        and_after: false,
                        or_after: false,
                        links: Vec::new(),
                        ind: 0,
                    });
                    comma = false; and = false; or = false;
                    i += 1;
                    continue;
                }
                if it.typ == SentItemType::Verb
                    || it.typ == SentItemType::Conj
                    || it.typ == SentItemType::Delim
                {
                    break;
                }
                i += 1;
            }

            // Set order on all items
            for (ord, ng_it) in seg.items.iter_mut().enumerate() {
                ng_it.order = ord;
            }

            // Find AfterVerb
            let mut j2 = i;
            while j2 < sent_items.len() {
                let it2 = &sent_items[j2];
                if it2.typ == SentItemType::Verb {
                    seg.after_verb_idx = Some(j2);
                    break;
                }
                if it2.typ == SentItemType::Conj
                    || it2.can_be_noun()
                    || it2.typ == SentItemType::Delim
                    || it2.typ == SentItemType::Adverb
                {
                    break;
                }
                j2 += 1;
            }

            // Create links
            seg.create_links(sent_items, false);

            if !seg.items.is_empty() {
                res.push(seg);
            }
        }

        res
    }

    /// Build candidate NGLinks for each item.
    /// Mirrors `NGSegment.CreateLinks(afterPart)`.
    pub fn create_links(&mut self, sent_items: &[SentItem], after_part: bool) {
        // Build the seg_items index array (sent_item_idx for each position)
        let seg_items: Vec<usize> = self.items.iter().map(|ng| ng.sent_item_idx).collect();

        // Clear existing links
        for ng in &mut self.items {
            ng.links.clear();
        }

        // For each item, build candidate links
        for i in 0..self.items.len() {
            let it = &sent_items[seg_items[i]];

            // Adverbs don't get genitive/name/be links
            if it.typ == SentItemType::Adverb {
                // Adverb → before/after verb
                self.try_add_adverb_verb_links(i, &seg_items, sent_items);
                continue;
            }

            let comma_or_and = self.items[i].comma_before || self.items[i].and_before;

            if comma_or_and {
                // List links to previous items
                for j in (0..i).rev() {
                    let mut li = NGLink {
                        typ: NGLinkType::List,
                        from_ord: i, to_ord: Some(j), to_verb_sent_idx: None,
                        ..NGLink::default()
                    };
                    li.calc_coef(&seg_items, sent_items, false);
                    if li.coef >= 0.0 {
                        self.items[i].links.push(li);
                    }
                    // Participle link for PartBefore with comma
                    if (it.typ == SentItemType::PartBefore || it.typ == SentItemType::SubSent || it.typ == SentItemType::Deepart)
                        && self.items[i].comma_before
                    {
                        let mut li2 = NGLink {
                            typ: NGLinkType::Participle,
                            from_ord: i, to_ord: Some(j), to_verb_sent_idx: None,
                            ..NGLink::default()
                        };
                        li2.calc_coef(&seg_items, sent_items, false);
                        if li2.coef >= 0.0 {
                            self.items[i].links.push(li2);
                        }
                    }
                }
            } else {
                // Genitive/Name/Be links to previous items
                for j in (0..i).rev() {
                    let jit = &sent_items[seg_items[j]];
                    if jit.typ == SentItemType::SubSent { continue; }

                    let mut li_gen = NGLink {
                        typ: NGLinkType::Genetive,
                        from_ord: i, to_ord: Some(j), to_verb_sent_idx: None,
                        ..NGLink::default()
                    };
                    li_gen.calc_coef(&seg_items, sent_items, false);
                    if li_gen.coef >= 0.0 { self.items[i].links.push(li_gen); }

                    let mut li_name = NGLink {
                        typ: NGLinkType::Name,
                        from_ord: i, to_ord: Some(j), to_verb_sent_idx: None,
                        ..NGLink::default()
                    };
                    li_name.calc_coef(&seg_items, sent_items, false);
                    if li_name.coef >= 0.0 { self.items[i].links.push(li_name); }

                    // Be link only if no comma/and between j..i
                    let no_delim = !(self.items[j+1..=i].iter().any(|ng| ng.comma_before || ng.and_before));
                    if no_delim {
                        let mut li_be = NGLink {
                            typ: NGLinkType::Be,
                            from_ord: i, to_ord: Some(j), to_verb_sent_idx: None,
                            ..NGLink::default()
                        };
                        li_be.calc_coef(&seg_items, sent_items, false);
                        if li_be.coef >= 0.0 { self.items[i].links.push(li_be); }
                    }
                }

                // Agent/Pacient/Actant links to BeforeVerb
                let before_verb_idx = self.before_verb_idx;
                let after_verb_idx  = self.after_verb_idx;

                if let Some(bv_idx) = before_verb_idx {
                    if it.typ != SentItemType::Deepart {
                        let mut li_ag = NGLink {
                            typ: NGLinkType::Agent,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(bv_idx),
                            ..NGLink::default()
                        };
                        li_ag.calc_coef(&seg_items, sent_items, false);
                        if li_ag.coef >= 0.0 { self.items[i].links.push(li_ag); }

                        let mut li_pac = NGLink {
                            typ: NGLinkType::Pacient,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(bv_idx),
                            ..NGLink::default()
                        };
                        li_pac.calc_coef(&seg_items, sent_items, false);
                        if li_pac.coef >= 0.0 { self.items[i].links.push(li_pac); }

                        let mut li_act = NGLink {
                            typ: NGLinkType::Actant,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(bv_idx),
                            ..NGLink::default()
                        };
                        li_act.calc_coef(&seg_items, sent_items, false);
                        if li_act.coef >= 0.0 { self.items[i].links.push(li_act); }
                    }
                }

                // Agent/Pacient/Actant links to AfterVerb
                if let Some(av_idx) = after_verb_idx {
                    if it.typ != SentItemType::Deepart {
                        let mut li_ag = NGLink {
                            typ: NGLinkType::Agent,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(av_idx),
                            ..NGLink::default()
                        };
                        li_ag.calc_coef(&seg_items, sent_items, false);
                        if li_ag.coef >= 0.0 { self.items[i].links.push(li_ag); }

                        let mut li_pac = NGLink {
                            typ: NGLinkType::Pacient,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(av_idx),
                            ..NGLink::default()
                        };
                        li_pac.calc_coef(&seg_items, sent_items, false);
                        if li_pac.coef >= 0.0 { self.items[i].links.push(li_pac); }

                        let mut li_act = NGLink {
                            typ: NGLinkType::Actant,
                            from_ord: i, to_ord: None, to_verb_sent_idx: Some(av_idx),
                            ..NGLink::default()
                        };
                        li_act.calc_coef(&seg_items, sent_items, false);
                        if li_act.coef >= 0.0 { self.items[i].links.push(li_act); }
                    }
                }
            }
        }

        // Reverse genitive: if two adjacent nouns and the first has no links
        let n = self.items.len();
        for i in 1..n {
            let it0_typ = sent_items[seg_items[i-1]].typ;
            let it1_typ = sent_items[seg_items[i]].typ;
            if it0_typ != SentItemType::Noun || it1_typ != SentItemType::Noun { continue; }
            if !self.items[i-1].links.is_empty() { continue; }
            let mut li = NGLink {
                typ: NGLinkType::Genetive,
                from_ord: i-1, to_ord: Some(i), to_verb_sent_idx: None,
                reverce: true,
                ..NGLink::default()
            };
            li.calc_coef(&seg_items, sent_items, true);
            if li.coef > 0.0 { self.items[i-1].links.push(li); }
        }
    }

    fn try_add_adverb_verb_links(&mut self, i: usize, seg_items: &[usize], sent_items: &[SentItem]) {
        if let Some(bv_idx) = self.before_verb_idx {
            let mut li = NGLink {
                typ: NGLinkType::Adverb,
                from_ord: i, to_ord: None, to_verb_sent_idx: Some(bv_idx),
                ..NGLink::default()
            };
            li.coef = 1.0;
            self.items[i].links.push(li);
        }
        if let Some(av_idx) = self.after_verb_idx {
            let mut li = NGLink {
                typ: NGLinkType::Adverb,
                from_ord: i, to_ord: None, to_verb_sent_idx: Some(av_idx),
                ..NGLink::default()
            };
            li.coef = 1.0;
            self.items[i].links.push(li);
        }
    }

    /// Returns the best single-variant (greedy: pick highest-coef link per item).
    /// Returns vec of (item_idx → chosen NGLink or None).
    pub fn best_links(&self, sent_items: &[SentItem]) -> Vec<Option<NGLink>> {
        let mut result = vec![None; self.items.len()];
        for (i, ng_it) in self.items.iter().enumerate() {
            let best = ng_it.links.iter()
                .max_by(|a, b| a.coef.partial_cmp(&b.coef).unwrap_or(std::cmp::Ordering::Equal));
            result[i] = best.cloned();
        }
        result
    }
}
