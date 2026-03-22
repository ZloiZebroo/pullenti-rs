/// Simplified port of `PersonNormalNode.cs` + `PersonNormalItem.cs`.
///
/// Scores a list of `PersonItemToken`s against two candidate FIO orderings
/// (Фамилия–Имя–Отчество and Имя–Отчество–Фамилия) and returns the best.

use super::person_item_token::PersonItemToken;
use super::person_normal_data::PersonNormalData;
use super::person_normal_result::PersonNormalResult;

// ── Slot type ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum SlotType { Last, First, Middle }

// ── PersonNormalItem ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PersonNormalItem {
    typ:    SlotType,
    ind0:   i32,  // start index in src (inclusive)
    ind1:   i32,  // end index in src (inclusive); ind1 < ind0 → slot empty (Middle only)
    rnd0:   i32,  // best-so-far start
    rnd1:   i32,  // best-so-far end
    gender: i32,  // 1=masculine, 2=feminine being tested
    render: i32,  // gender of best-so-far assignment
}

impl PersonNormalItem {
    fn new(typ: SlotType) -> Self {
        PersonNormalItem {
            typ, ind0: 0, ind1: -1,
            rnd0: -1, rnd1: -1,
            gender: 0, render: 0,
        }
    }

    fn init(&mut self) {
        self.ind0 = 0; self.ind1 = -1;
        self.rnd0 = -1; self.rnd1 = -1;
        self.gender = 0; self.render = 0;
    }

    fn fix(&mut self) {
        self.rnd0 = self.ind0;
        self.rnd1 = self.ind1;
        self.render = self.gender;
    }

    /// Calculate score for the current (ind0, ind1, gender) assignment.
    /// `src` is the list of PersonItemTokens.
    fn calc_coef(&self, src: &[PersonItemToken]) -> f64 {
        if self.ind1 < self.ind0 {
            // Empty slot: allowed only for Middle
            return if self.typ == SlotType::Middle { 1.0 } else { 0.0 };
        }

        // Gap check: if any adjacent tokens have large whitespace, reject
        for i in self.ind0..self.ind1 {
            if i + 1 < src.len() as i32 {
                if src[i as usize].whitespaces_before > 4 { return 0.0; }
            }
        }

        let mut co = 1.0f64;

        for i in self.ind0..=self.ind1 {
            let pit = &src[i as usize];

            match self.typ {
                SlotType::First => {
                    if self.ind0 == self.ind1 && pit.value.chars().count() == 1 {
                        // Initial — neutral
                    } else if let Some(fn_m) = &pit.firstname {
                        // Gender match bonus/penalty
                        if self.gender == 2 && fn_m.gender == 2 {}
                        else if self.gender == 1 && fn_m.gender == 1 {}
                        else { co *= 0.98; }
                    } else if pit.middlename.is_some() {
                        co *= 0.8;
                    } else if let Some(ln) = &pit.lastname {
                        if ln.is_in_dictionary || ln.is_lastname_has_std_tail {
                            co *= 0.8;
                        } else {
                            co *= 0.9;
                        }
                    } else if self.gender == 2 && pit.morph_gender == 2 {
                        co *= 0.98;
                    } else if self.gender == 1 && pit.morph_gender == 1 {
                        co *= 0.98;
                    } else {
                        co *= 0.7;
                    }
                }
                SlotType::Middle => {
                    if self.ind0 == self.ind1 && pit.value.chars().count() == 1 {
                        // Initial — neutral
                    } else if let Some(mn_m) = &pit.middlename {
                        if self.gender == 2 && mn_m.gender == 2 {}
                        else if self.gender == 1 && mn_m.gender == 1 {}
                        else { co *= 0.7; }
                    } else if pit.value.ends_with("ВНА") && self.gender == 2 {
                        // Feminine patronymic tail — good
                    } else if pit.value.ends_with("ЧНА") && self.gender == 2 {
                    } else if pit.value.ends_with("ИЧ") && self.gender == 1 {
                        co *= 0.99;
                    } else if pit.firstname.is_some() {
                        co *= 0.98;
                    } else if let Some(ln) = &pit.lastname {
                        if ln.is_in_dictionary { co *= 0.8; } else { co *= 0.7; }
                    } else {
                        co *= 0.7;
                    }
                }
                SlotType::Last => {
                    // Disallow patronymic-looking tokens in lastname position
                    // (unless at position 0 in the pattern)
                    if pit.value.ends_with("ВНА") || pit.value.ends_with("ЧНА") {
                        if self.gender == 2 {
                            co *= if self.ind0 == 0 { 0.7 } else { 0.2 };
                        } else {
                            co *= 0.3;
                        }
                    } else if pit.value.ends_with("ИЧ") {
                        co *= if self.ind0 == 0 || self.gender == 2 { 1.0 } else { 0.85 };
                    } else if let Some(ln) = &pit.lastname {
                        if ln.is_in_dictionary || ln.is_lastname_has_std_tail {
                            if self.gender == 2 && ln.gender == 2 {}
                            else if self.gender == 1 && ln.gender == 1 {}
                            else { co *= 0.98; }
                        } else if pit.firstname.is_some() || pit.middlename.is_some() {
                            co *= 0.8;
                        } else {
                            co *= 0.98;
                        }
                    } else if pit.firstname.is_some() || pit.middlename.is_some() {
                        co *= 0.8;
                    } else {
                        co *= 0.98;
                    }
                }
            }
        }

        co
    }

    /// Get the result value and optional alt value for the best assignment.
    ///
    /// Returns `(value, alt)` where:
    /// - `value` is the canonical name (full form if ShortName expansion applied)
    /// - `alt` is the short form that appeared in the text (set only for First slot
    ///   when ShortName expansion was performed)
    fn result_with_alt(&self, src: &[PersonItemToken]) -> (Option<String>, Option<String>) {
        if self.rnd0 < 0 || self.rnd1 < self.rnd0 { return (None, None); }

        // If First slot has range [i, i+1] with one being an initial → pick the full name
        if self.typ == SlotType::First && self.rnd1 == self.rnd0 + 1 {
            let v0 = &src[self.rnd0 as usize].value;
            let v1 = &src[self.rnd1 as usize].value;
            if v0.chars().count() == 1 && v1.chars().count() > 1 {
                return (Some(v1.clone()), None);
            }
            if v0.chars().count() > 1 && v1.chars().count() == 1 {
                return (Some(v0.clone()), None);
            }
        }

        // Concatenate all tokens in the slot
        let parts: Vec<&str> = (self.rnd0..=self.rnd1)
            .map(|i| src[i as usize].value.as_str())
            .collect();
        let joined = parts.join("-");

        // ShortName expansion for the First slot (single token)
        if self.typ == SlotType::First && self.rnd0 == self.rnd1 {
            let pit = &src[self.rnd0 as usize];
            if let Some(fn_m) = &pit.firstname {
                if fn_m.is_in_dictionary && !fn_m.vars.is_empty() {
                    for (full_name, g) in &fn_m.vars {
                        if *g == self.render
                            && full_name != &pit.value
                            // C# exclusions
                            && pit.value != "ВЛАД"
                            && pit.value != "АЛЕКС"
                        {
                            // Return expanded full name; alt = original short form
                            return (Some(full_name.clone()), Some(pit.value.clone()));
                        }
                    }
                }
            }
        }

        (Some(joined), None)
    }
}

// ── PersonNormalNode ──────────────────────────────────────────────────────────

pub struct PersonNormalNode {
    items: Vec<PersonNormalItem>,
}

impl PersonNormalNode {
    /// Build an FIO node (Фамилия first) or IOF node (Фамилия last).
    pub fn new(fio_last: bool) -> Self {
        let items = if fio_last {
            vec![
                PersonNormalItem::new(SlotType::First),
                PersonNormalItem::new(SlotType::Middle),
                PersonNormalItem::new(SlotType::Last),
            ]
        } else {
            vec![
                PersonNormalItem::new(SlotType::Last),
                PersonNormalItem::new(SlotType::First),
                PersonNormalItem::new(SlotType::Middle),
            ]
        };
        PersonNormalNode { items }
    }

    /// Score this node against `src`.  Returns best combined coefficient [0..1].
    pub fn process(&mut self, src: &[PersonItemToken]) -> f64 {
        for item in &mut self.items { item.init(); }

        let n = src.len() as i32;
        if n == 0 { return 0.0; }

        let mut best = 0.0f64;

        // Try both masculine (1) and feminine (2) genders
        for gender in [1i32, 2] {
            // 3-slot partition: items[0]=[0..i0], items[1]=[i0+1..i1], items[2]=[i1+1..i2]
            // The third slot may be "empty" if it's the Middle slot and i2 == n-1.
            for i0 in 0..=(n - 1) {
                self.items[0].ind0 = 0;
                self.items[0].ind1 = i0;
                self.items[0].gender = gender;
                let c0 = self.items[0].calc_coef(src);
                if c0 <= 0.0 { continue; }

                // items[1] and items[2] share the remaining tokens
                for i1 in (i0 + 1)..n {
                    self.items[1].ind0 = i0 + 1;
                    self.items[1].ind1 = i1;
                    self.items[1].gender = gender;
                    let c1 = self.items[1].calc_coef(src);
                    if c0 * c1 <= best { continue; }

                    // items[2] gets the rest; if empty and it's Middle → OK
                    {
                        self.items[2].ind0 = i1 + 1;
                        self.items[2].ind1 = n - 1;
                        self.items[2].gender = gender;
                        // Middle can also be empty (ind1 < ind0)
                        if self.items[2].ind0 > self.items[2].ind1 {
                            if self.items[2].typ != SlotType::Middle { continue; }
                            self.items[2].ind1 = self.items[2].ind0 - 1; // flag: empty
                        }
                        let c2 = self.items[2].calc_coef(src);
                        let total = c0 * c1 * c2;
                        if total > best {
                            best = total;
                            for item in &mut self.items { item.fix(); }
                        }
                    }
                }

                // 2-token fallback: items[0]=[0..i0], items[1 or 2]=[i0+1..n-1],
                // remaining Middle slot empty
                if n == 2 || (n == 1 && i0 == 0) {
                    let k = if self.items[1].typ == SlotType::Middle { 2 } else { 1 };
                    if k < self.items.len() {
                        self.items[k].ind0 = i0 + 1;
                        self.items[k].ind1 = n - 1;
                        self.items[k].gender = gender;
                        // Middle is empty
                        let mid_k = if k == 1 { 2 } else { 1 };
                        self.items[mid_k].ind0 = n;
                        self.items[mid_k].ind1 = n - 1; // empty
                        self.items[mid_k].gender = gender;
                        let ck = self.items[k].calc_coef(src);
                        let total = c0 * ck;
                        if total > best {
                            best = total;
                            for item in &mut self.items { item.fix(); }
                        }
                    }
                }
            }
        }

        best
    }

    /// Write the best assignment into `res`.
    pub fn create_result(&self, src: &[PersonItemToken], res: &mut PersonNormalData) {
        // Gender comes from the first slot's render
        res.gender = self.items[0].render;

        for item in &self.items {
            let (val, alt) = item.result_with_alt(src);
            match item.typ {
                SlotType::First  => { res.firstname = val; res.firstname_alt = alt; }
                SlotType::Middle => { res.middlename = val; }
                SlotType::Last   => { res.lastname = val; }
            }
        }
    }
}

// ── Public entry ──────────────────────────────────────────────────────────────

/// Score a list of PersonItemTokens with both FIO orderings and return the
/// best `PersonNormalData`.  Returns `None` if score < threshold.
pub fn score_and_build(
    pits: &[PersonItemToken],
    threshold: f64,
) -> Option<(PersonNormalData, f64)> {
    if pits.is_empty() { return None; }

    let mut node_fio = PersonNormalNode::new(false); // Last First Middle
    let mut node_iof = PersonNormalNode::new(true);  // First Middle Last

    let coef_fio = node_fio.process(pits);
    let coef_iof = node_iof.process(pits);

    let (best_coef, best_node) = if coef_fio >= coef_iof {
        (coef_fio, &mut node_fio as *mut PersonNormalNode)
    } else {
        (coef_iof, &mut node_iof as *mut PersonNormalNode)
    };

    if best_coef < threshold { return None; }

    let mut res = PersonNormalData::new();
    // SAFETY: we don't alias the two nodes simultaneously
    unsafe { (*best_node).create_result(pits, &mut res); }

    // Set coef as 0–100 percentage
    res.coef = (best_coef * 100.0) as i32;

    // Set result type
    res.res_typ = if res.coef >= 90 {
        PersonNormalResult::OK
    } else {
        PersonNormalResult::Manual
    };

    // Gender
    // (already set by create_result)

    Some((res, best_coef))
}
