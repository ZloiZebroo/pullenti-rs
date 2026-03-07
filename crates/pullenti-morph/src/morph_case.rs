use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

/// Grammatical case (Падеж)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MorphCase {
    pub value: i16,
}

impl MorphCase {
    pub const fn new() -> Self {
        MorphCase { value: 0 }
    }

    pub const fn from_value(value: i16) -> Self {
        MorphCase { value }
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

    pub fn count(&self) -> i32 {
        if self.value == 0 { return 0; }
        let mut cou = 0;
        for i in 0..12 {
            if (self.value & (1 << i)) != 0 {
                cou += 1;
            }
        }
        cou
    }

    pub fn is_nominative(&self) -> bool { self.get_value(0) }
    pub fn set_nominative(&mut self, v: bool) { self.set_value(0, v); }

    pub fn is_genitive(&self) -> bool { self.get_value(1) }
    pub fn set_genitive(&mut self, v: bool) { self.set_value(1, v); }

    pub fn is_dative(&self) -> bool { self.get_value(2) }
    pub fn set_dative(&mut self, v: bool) { self.set_value(2, v); }

    pub fn is_accusative(&self) -> bool { self.get_value(3) }
    pub fn set_accusative(&mut self, v: bool) { self.set_value(3, v); }

    pub fn is_instrumental(&self) -> bool { self.get_value(4) }
    pub fn set_instrumental(&mut self, v: bool) { self.set_value(4, v); }

    pub fn is_prepositional(&self) -> bool { self.get_value(5) }
    pub fn set_prepositional(&mut self, v: bool) { self.set_value(5, v); }

    pub fn is_vocative(&self) -> bool { self.get_value(6) }
    pub fn set_vocative(&mut self, v: bool) { self.set_value(6, v); }

    pub fn is_partial(&self) -> bool { self.get_value(7) }
    pub fn set_partial(&mut self, v: bool) { self.set_value(7, v); }

    pub fn is_common(&self) -> bool { self.get_value(8) }
    pub fn set_common(&mut self, v: bool) { self.set_value(8, v); }

    pub fn is_possessive(&self) -> bool { self.get_value(9) }
    pub fn set_possessive(&mut self, v: bool) { self.set_value(9, v); }

    // Constants
    pub const UNDEFINED: MorphCase = MorphCase { value: 0 };
    pub const NOMINATIVE: MorphCase = MorphCase { value: 1 };
    pub const GENITIVE: MorphCase = MorphCase { value: 2 };
    pub const DATIVE: MorphCase = MorphCase { value: 4 };
    pub const ACCUSATIVE: MorphCase = MorphCase { value: 8 };
    pub const INSTRUMENTAL: MorphCase = MorphCase { value: 0x10 };
    pub const PREPOSITIONAL: MorphCase = MorphCase { value: 0x20 };
    pub const VOCATIVE: MorphCase = MorphCase { value: 0x40 };
    pub const PARTIAL: MorphCase = MorphCase { value: 0x80 };
    pub const COMMON: MorphCase = MorphCase { value: 0x100 };
    pub const POSSESSIVE: MorphCase = MorphCase { value: 0x200 };
    pub const ALL_CASES: MorphCase = MorphCase { value: 0x3FF };

    const NAMES: [&'static str; 10] = [
        "именит.", "родит.", "дател.", "винит.", "творит.",
        "предлож.", "зват.", "частич.", "общ.", "притяж.",
    ];

    pub fn parse(s: &str) -> MorphCase {
        let mut res = MorphCase::new();
        if s.is_empty() { return res; }
        for part in s.split('|') {
            for (i, name) in Self::NAMES.iter().enumerate() {
                if part == *name {
                    res.set_value(i as u32, true);
                    break;
                }
            }
        }
        res
    }
}

impl fmt::Display for MorphCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.is_nominative() { parts.push("именит."); }
        if self.is_genitive() { parts.push("родит."); }
        if self.is_dative() { parts.push("дател."); }
        if self.is_accusative() { parts.push("винит."); }
        if self.is_instrumental() { parts.push("творит."); }
        if self.is_prepositional() { parts.push("предлож."); }
        if self.is_vocative() { parts.push("зват."); }
        if self.is_partial() { parts.push("частич."); }
        if self.is_common() { parts.push("общ."); }
        if self.is_possessive() { parts.push("притяж."); }
        write!(f, "{}", parts.join("|"))
    }
}

impl fmt::Debug for MorphCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MorphCase({})", self)
    }
}

impl BitAnd for MorphCase {
    type Output = MorphCase;
    fn bitand(self, rhs: Self) -> Self::Output {
        MorphCase { value: self.value & rhs.value }
    }
}

impl BitAndAssign for MorphCase {
    fn bitand_assign(&mut self, rhs: Self) {
        self.value &= rhs.value;
    }
}

impl BitOr for MorphCase {
    type Output = MorphCase;
    fn bitor(self, rhs: Self) -> Self::Output {
        MorphCase { value: self.value | rhs.value }
    }
}

impl BitOrAssign for MorphCase {
    fn bitor_assign(&mut self, rhs: Self) {
        self.value |= rhs.value;
    }
}

impl BitXor for MorphCase {
    type Output = MorphCase;
    fn bitxor(self, rhs: Self) -> Self::Output {
        MorphCase { value: self.value ^ rhs.value }
    }
}

impl BitXorAssign for MorphCase {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.value ^= rhs.value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(MorphCase::NOMINATIVE.is_nominative());
        assert!(!MorphCase::NOMINATIVE.is_genitive());
        assert_eq!(MorphCase::NOMINATIVE.count(), 1);
        assert_eq!(MorphCase::ALL_CASES.count(), 10);
    }

    #[test]
    fn test_parse() {
        let c = MorphCase::parse("именит.|родит.");
        assert!(c.is_nominative());
        assert!(c.is_genitive());
        assert!(!c.is_dative());
    }
}
