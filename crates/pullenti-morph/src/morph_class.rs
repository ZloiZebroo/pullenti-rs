use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

/// Part of speech (Часть речи)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MorphClass {
    pub value: i16,
}

impl MorphClass {
    pub const fn new() -> Self {
        MorphClass { value: 0 }
    }

    pub const fn from_value(value: i16) -> Self {
        MorphClass { value }
    }

    fn get_value(&self, i: u32) -> bool {
        ((self.value >> i) & 1) != 0
    }

    fn set_value(&mut self, i: u32, val: bool) {
        if val {
            self.value |= 1 << i;
        } else {
            self.value &= !(1 << i);
        }
    }

    pub fn is_undefined(&self) -> bool { self.value == 0 }

    pub fn is_noun(&self) -> bool { self.get_value(0) }
    pub fn set_noun(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(0, v);
    }

    pub fn is_adjective(&self) -> bool { self.get_value(1) }
    pub fn set_adjective(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(1, v);
    }

    pub fn is_verb(&self) -> bool { self.get_value(2) }
    pub fn set_verb(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(2, v);
    }

    pub fn is_adverb(&self) -> bool { self.get_value(3) }
    pub fn set_adverb(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(3, v);
    }

    pub fn is_pronoun(&self) -> bool { self.get_value(4) }
    pub fn set_pronoun(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(4, v);
    }

    pub fn is_misc(&self) -> bool { self.get_value(5) }
    pub fn set_misc(&mut self, v: bool) {
        if v { self.value = 0; }
        self.set_value(5, v);
    }

    pub fn is_preposition(&self) -> bool { self.get_value(6) }
    pub fn set_preposition(&mut self, v: bool) { self.set_value(6, v); }

    pub fn is_conjunction(&self) -> bool { self.get_value(7) }
    pub fn set_conjunction(&mut self, v: bool) { self.set_value(7, v); }

    pub fn is_proper(&self) -> bool { self.get_value(8) }
    pub fn set_proper(&mut self, v: bool) { self.set_value(8, v); }

    pub fn is_proper_surname(&self) -> bool { self.get_value(9) }
    pub fn set_proper_surname(&mut self, v: bool) {
        if v { self.set_proper(true); }
        self.set_value(9, v);
    }

    pub fn is_proper_name(&self) -> bool { self.get_value(10) }
    pub fn set_proper_name(&mut self, v: bool) {
        if v { self.set_proper(true); }
        self.set_value(10, v);
    }

    pub fn is_proper_secname(&self) -> bool { self.get_value(11) }
    pub fn set_proper_secname(&mut self, v: bool) {
        if v { self.set_proper(true); }
        self.set_value(11, v);
    }

    pub fn is_proper_geo(&self) -> bool { self.get_value(12) }
    pub fn set_proper_geo(&mut self, v: bool) {
        if v { self.set_proper(true); }
        self.set_value(12, v);
    }

    pub fn is_personal_pronoun(&self) -> bool { self.get_value(13) }
    pub fn set_personal_pronoun(&mut self, v: bool) { self.set_value(13, v); }

    // Constants
    pub const UNDEFINED: MorphClass = MorphClass { value: 0 };
    pub const NOUN: MorphClass = MorphClass { value: 1 };
    pub const ADJECTIVE: MorphClass = MorphClass { value: 2 };
    pub const VERB: MorphClass = MorphClass { value: 4 };
    pub const ADVERB: MorphClass = MorphClass { value: 8 };
    pub const PRONOUN: MorphClass = MorphClass { value: 16 };
    pub const PREPOSITION: MorphClass = MorphClass { value: 64 };
    pub const CONJUNCTION: MorphClass = MorphClass { value: 128 };
    pub const PERSONAL_PRONOUN: MorphClass = MorphClass { value: 1 << 13 };
}

impl fmt::Display for MorphClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.is_noun() { parts.push("существ."); }
        if self.is_adjective() { parts.push("прилаг."); }
        if self.is_verb() { parts.push("глагол"); }
        if self.is_adverb() { parts.push("наречие"); }
        if self.is_pronoun() { parts.push("местоим."); }
        if self.is_misc() {
            if !(self.is_conjunction() || self.is_preposition() || self.is_proper()) {
                parts.push("разное");
            }
        }
        if self.is_preposition() { parts.push("предлог"); }
        if self.is_conjunction() { parts.push("союз"); }
        if self.is_proper() { parts.push("собств."); }
        if self.is_proper_surname() { parts.push("фамилия"); }
        if self.is_proper_name() { parts.push("имя"); }
        if self.is_proper_secname() { parts.push("отч."); }
        if self.is_proper_geo() { parts.push("геогр."); }
        if self.is_personal_pronoun() { parts.push("личн.местоим."); }
        write!(f, "{}", parts.join("|"))
    }
}

impl fmt::Debug for MorphClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MorphClass({})", self)
    }
}

impl BitAnd for MorphClass {
    type Output = MorphClass;
    fn bitand(self, rhs: Self) -> Self::Output {
        MorphClass { value: self.value & rhs.value }
    }
}

impl BitAndAssign for MorphClass {
    fn bitand_assign(&mut self, rhs: Self) {
        self.value &= rhs.value;
    }
}

impl BitOr for MorphClass {
    type Output = MorphClass;
    fn bitor(self, rhs: Self) -> Self::Output {
        MorphClass { value: self.value | rhs.value }
    }
}

impl BitOrAssign for MorphClass {
    fn bitor_assign(&mut self, rhs: Self) {
        self.value |= rhs.value;
    }
}

impl BitXor for MorphClass {
    type Output = MorphClass;
    fn bitxor(self, rhs: Self) -> Self::Output {
        MorphClass { value: self.value ^ rhs.value }
    }
}

impl BitXorAssign for MorphClass {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.value ^= rhs.value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(MorphClass::NOUN.is_noun());
        assert!(!MorphClass::NOUN.is_verb());
        assert!(MorphClass::UNDEFINED.is_undefined());
    }

    #[test]
    fn test_proper_sets_proper_flag() {
        let mut c = MorphClass::new();
        c.set_proper_surname(true);
        assert!(c.is_proper());
        assert!(c.is_proper_surname());
    }

    #[test]
    fn test_bitwise() {
        let combined = MorphClass::NOUN | MorphClass::ADJECTIVE;
        assert!(combined.is_noun());
        assert!(combined.is_adjective());
        let anded = combined & MorphClass::NOUN;
        assert!(anded.is_noun());
        assert!(!anded.is_adjective());
    }
}
