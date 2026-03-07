use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

/// Language (Язык)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MorphLang {
    pub value: i16,
}

impl MorphLang {
    pub const fn new() -> Self {
        MorphLang { value: 0 }
    }

    pub const fn from_value(value: i16) -> Self {
        MorphLang { value }
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

    pub fn is_undefined(&self) -> bool {
        self.value == 0
    }

    pub fn is_ru(&self) -> bool { self.get_value(0) }
    pub fn set_ru(&mut self, v: bool) { self.set_value(0, v); }

    pub fn is_ua(&self) -> bool { self.get_value(1) }
    pub fn set_ua(&mut self, v: bool) { self.set_value(1, v); }

    pub fn is_by(&self) -> bool { self.get_value(2) }
    pub fn set_by(&mut self, v: bool) { self.set_value(2, v); }

    pub fn is_en(&self) -> bool { self.get_value(3) }
    pub fn set_en(&mut self, v: bool) { self.set_value(3, v); }

    pub fn is_it(&self) -> bool { self.get_value(4) }
    pub fn set_it(&mut self, v: bool) { self.set_value(4, v); }

    pub fn is_kz(&self) -> bool { self.get_value(5) }
    pub fn set_kz(&mut self, v: bool) { self.set_value(5, v); }

    pub fn is_cyrillic(&self) -> bool {
        self.is_ru() || self.is_ua() || self.is_by() || self.is_kz()
    }

    /// Well-known language constants
    pub const UNKNOWN: MorphLang = MorphLang { value: 0 };
    pub const RU: MorphLang = MorphLang { value: 1 };
    pub const UA: MorphLang = MorphLang { value: 2 };
    pub const BY: MorphLang = MorphLang { value: 4 };
    pub const EN: MorphLang = MorphLang { value: 8 };
    pub const IT: MorphLang = MorphLang { value: 16 };
    pub const KZ: MorphLang = MorphLang { value: 32 };

    const NAMES: [&'static str; 6] = ["RU", "UA", "BY", "EN", "IT", "KZ"];

    pub fn try_parse(s: &str) -> Option<MorphLang> {
        let mut lang = MorphLang::new();
        let mut remaining = s;
        while !remaining.is_empty() {
            let mut found = false;
            for (i, name) in Self::NAMES.iter().enumerate() {
                if remaining.to_uppercase().starts_with(name) {
                    lang.value |= 1 << i;
                    // Skip past the name and any non-letter chars
                    let after_name = &remaining[name.len()..];
                    match after_name.find(|c: char| c.is_alphabetic()) {
                        Some(pos) => remaining = &after_name[pos..],
                        None => remaining = "",
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        if lang.is_undefined() {
            None
        } else {
            Some(lang)
        }
    }
}

impl fmt::Display for MorphLang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.is_ru() { parts.push("RU"); }
        if self.is_ua() { parts.push("UA"); }
        if self.is_by() { parts.push("BY"); }
        if self.is_en() { parts.push("EN"); }
        if self.is_it() { parts.push("IT"); }
        if self.is_kz() { parts.push("KZ"); }
        write!(f, "{}", parts.join(";"))
    }
}

impl fmt::Debug for MorphLang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MorphLang({})", self)
    }
}

impl BitAnd for MorphLang {
    type Output = MorphLang;
    fn bitand(self, rhs: Self) -> Self::Output {
        MorphLang { value: self.value & rhs.value }
    }
}

impl BitAndAssign for MorphLang {
    fn bitand_assign(&mut self, rhs: Self) {
        self.value &= rhs.value;
    }
}

impl BitOr for MorphLang {
    type Output = MorphLang;
    fn bitor(self, rhs: Self) -> Self::Output {
        MorphLang { value: self.value | rhs.value }
    }
}

impl BitOrAssign for MorphLang {
    fn bitor_assign(&mut self, rhs: Self) {
        self.value |= rhs.value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(MorphLang::RU.is_ru());
        assert!(!MorphLang::RU.is_en());
        assert!(MorphLang::EN.is_en());
        assert!(MorphLang::UNKNOWN.is_undefined());
    }

    #[test]
    fn test_bitwise_ops() {
        let ru_en = MorphLang::RU | MorphLang::EN;
        assert!(ru_en.is_ru());
        assert!(ru_en.is_en());
        assert!(!ru_en.is_ua());

        let anded = ru_en & MorphLang::RU;
        assert!(anded.is_ru());
        assert!(!anded.is_en());
    }

    #[test]
    fn test_cyrillic() {
        assert!(MorphLang::RU.is_cyrillic());
        assert!(MorphLang::UA.is_cyrillic());
        assert!(!MorphLang::EN.is_cyrillic());
    }

    #[test]
    fn test_display() {
        assert_eq!(MorphLang::RU.to_string(), "RU");
        assert_eq!((MorphLang::RU | MorphLang::EN).to_string(), "RU;EN");
    }

    #[test]
    fn test_try_parse() {
        let lang = MorphLang::try_parse("RU").unwrap();
        assert!(lang.is_ru());
        let lang = MorphLang::try_parse("RU;EN").unwrap();
        assert!(lang.is_ru());
        assert!(lang.is_en());
        assert!(MorphLang::try_parse("XX").is_none());
    }
}
