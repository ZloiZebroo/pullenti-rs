use pullenti_morph::{MorphBaseInfo, MorphWordForm, MorphLang, MorphClass, MorphCase, MorphGenderFlags, MorphNumber};

/// Voice of a verb form
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MorphVoice {
    #[default]
    Undefined = 0,
    Active = 1,
    Passive = 2,
    Middle = 3,
}

impl std::ops::BitOrAssign for MorphVoice {
    fn bitor_assign(&mut self, rhs: Self) {
        if *self == MorphVoice::Undefined {
            *self = rhs;
        } else if rhs != MorphVoice::Undefined && *self != rhs {
            *self = MorphVoice::Undefined; // conflicting voices
        }
    }
}

/// Aggregate morphological information for a token — a collection of MorphWordForm variants
#[derive(Debug, Clone)]
pub struct MorphCollection {
    items: Vec<MorphWordForm>,
    // Cached aggregate values (recalculated lazily)
    cached_class: MorphClass,
    cached_case: MorphCase,
    cached_gender: MorphGenderFlags,
    cached_number: MorphNumber,
    cached_lang: MorphLang,
    cached_voice: MorphVoice,
    need_recalc: bool,
}

impl Default for MorphCollection {
    fn default() -> Self {
        MorphCollection {
            items: Vec::new(),
            cached_class: MorphClass::new(),
            cached_case: MorphCase::new(),
            cached_gender: MorphGenderFlags::UNDEFINED,
            cached_number: MorphNumber::UNDEFINED,
            cached_lang: MorphLang::UNKNOWN,
            cached_voice: MorphVoice::Undefined,
            need_recalc: true,
        }
    }
}

impl MorphCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_word_forms(forms: Vec<MorphWordForm>) -> Self {
        let mut mc = MorphCollection::new();
        mc.items = forms;
        mc.need_recalc = true;
        mc
    }

    pub fn clone_collection(&self) -> Self {
        MorphCollection {
            items: self.items.clone(),
            cached_class: self.cached_class,
            cached_case: self.cached_case,
            cached_gender: self.cached_gender,
            cached_number: self.cached_number,
            cached_lang: self.cached_lang,
            cached_voice: self.cached_voice,
            need_recalc: self.need_recalc,
        }
    }

    fn recalc(&mut self) {
        self.need_recalc = false;
        self.cached_class = MorphClass::new();
        self.cached_case = MorphCase::new();
        self.cached_gender = MorphGenderFlags::UNDEFINED;
        self.cached_number = MorphNumber::UNDEFINED;
        self.cached_lang = MorphLang::UNKNOWN;
        self.cached_voice = MorphVoice::Undefined;

        let mut verb_has_undef = false;
        for it in &self.items {
            self.cached_class.value |= it.base.class.value;
            self.cached_gender.0 |= it.base.gender.0;
            self.cached_case.value |= it.base.case.value;
            self.cached_number.0 |= it.base.number.0;
            self.cached_lang.value |= it.base.language.value;
            if it.base.class.is_verb() {
                // Get voice from misc info
                let v = it.misc.as_ref().map_or(MorphVoice::Undefined, |m| {
                    if m.attrs.contains(&"действ.з.".to_string()) {
                        MorphVoice::Active
                    } else if m.attrs.contains(&"страд.з.".to_string()) {
                        MorphVoice::Passive
                    } else {
                        MorphVoice::Undefined
                    }
                });
                if v == MorphVoice::Undefined {
                    verb_has_undef = true;
                } else {
                    self.cached_voice |= v;
                }
            }
        }
        if verb_has_undef {
            self.cached_voice = MorphVoice::Undefined;
        }
    }

    pub fn items(&self) -> &[MorphWordForm] {
        &self.items
    }

    pub fn items_count(&self) -> usize {
        self.items.len()
    }

    pub fn add_item(&mut self, item: MorphWordForm) {
        self.items.push(item);
        self.need_recalc = true;
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            self.need_recalc = true;
        }
    }

    pub fn class(&mut self) -> MorphClass {
        if self.need_recalc { self.recalc(); }
        self.cached_class
    }

    pub fn case(&mut self) -> MorphCase {
        if self.need_recalc { self.recalc(); }
        self.cached_case
    }

    pub fn gender(&mut self) -> MorphGenderFlags {
        if self.need_recalc { self.recalc(); }
        self.cached_gender
    }

    pub fn number(&mut self) -> MorphNumber {
        if self.need_recalc { self.recalc(); }
        self.cached_number
    }

    pub fn language(&mut self) -> MorphLang {
        if self.need_recalc { self.recalc(); }
        self.cached_lang
    }

    pub fn voice(&mut self) -> MorphVoice {
        if self.need_recalc { self.recalc(); }
        self.cached_voice
    }

    /// Set language directly (overrides recalc)
    pub fn set_language(&mut self, lang: MorphLang) {
        self.cached_lang = lang;
        self.need_recalc = false; // mark as explicitly set
        for it in &mut self.items {
            it.base.language = lang;
        }
    }

    /// Check if any morph variant is the given word value
    pub fn contains_term(&self, term: &str) -> bool {
        self.items.iter().any(|wf| {
            wf.normal_case.as_deref() == Some(term)
                || wf.normal_full.as_deref() == Some(term)
        })
    }

    /// Check if any word form in this collection has the given attribute string.
    pub fn contains_attr(&self, attr: &str, _tag2: Option<()>) -> bool {
        self.items.iter().any(|wf| wf.contains_attr(attr))
    }

    /// Find item matching case/number/gender constraints
    pub fn find_item(&self, case: MorphCase, number: MorphNumber, gender: MorphGenderFlags) -> Option<&MorphWordForm> {
        for it in &self.items {
            if !case.is_undefined() {
                if (it.base.case.value & case.value) == 0 { continue; }
            }
            if number != MorphNumber::UNDEFINED {
                if (it.base.number.0 & number.0) == 0 { continue; }
            }
            if gender != MorphGenderFlags::UNDEFINED {
                if (it.base.gender.0 & gender.0) == 0 { continue; }
            }
            if it.undef_coef > 0 {
                continue; // prefer non-undef
            }
            return Some(it);
        }
        // Fallback: return first undef match
        for it in &self.items {
            if !case.is_undefined() {
                if (it.base.case.value & case.value) == 0 { continue; }
            }
            if number != MorphNumber::UNDEFINED {
                if (it.base.number.0 & number.0) == 0 { continue; }
            }
            if gender != MorphGenderFlags::UNDEFINED {
                if (it.base.gender.0 & gender.0) == 0 { continue; }
            }
            return Some(it);
        }
        None
    }

    /// Remove items not matching the given case
    pub fn remove_items_by_case(&mut self, case: MorphCase) {
        self.items.retain(|it| {
            if it.base.case.is_undefined() { return true; }
            (it.base.case.value & case.value) != 0
        });
        self.need_recalc = true;
    }

    /// Remove items not matching the given class
    pub fn remove_items_by_class(&mut self, class: MorphClass) {
        self.items.retain(|it| (it.base.class.value & class.value) != 0);
        self.need_recalc = true;
    }

    /// Remove non-dictionary items if any dictionary items exist
    pub fn remove_not_in_dictionary_items(&mut self) {
        let has_in_dict = self.items.iter().any(|wf| wf.is_in_dictionary());
        if has_in_dict {
            self.items.retain(|wf| wf.is_in_dictionary());
            self.need_recalc = true;
        }
    }

    /// Check agreement with another MorphBaseInfo
    pub fn check_accord(&self, other: &MorphBaseInfo, ignore_gender: bool, ignore_number: bool) -> bool {
        for it in &self.items {
            if it.base.check_accord(other, ignore_gender, ignore_number) {
                return true;
            }
        }
        if !self.items.is_empty() { return false; }
        // Empty items: check cached values
        let base = MorphBaseInfo {
            class: self.cached_class,
            case: self.cached_case,
            gender: self.cached_gender,
            number: self.cached_number,
            language: self.cached_lang,
        };
        base.check_accord(other, ignore_gender, ignore_number)
    }
}

impl std::fmt::Display for MorphCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut mc = self.clone();
        write!(f, "class={:?} case={} gender={:?} number={:?} lang={}",
            mc.class(),
            mc.case(),
            mc.gender(),
            mc.number(),
            mc.language(),
        )
    }
}
