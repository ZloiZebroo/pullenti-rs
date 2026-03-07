use std::fmt;

/// Character info for a token (Информация о символах токена)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CharsInfo {
    pub value: i16,
}

impl CharsInfo {
    pub const fn new() -> Self {
        CharsInfo { value: 0 }
    }

    pub const fn from_value(value: i16) -> Self {
        CharsInfo { value }
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

    pub fn is_all_upper(&self) -> bool { self.get_value(0) }
    pub fn set_all_upper(&mut self, v: bool) { self.set_value(0, v); }

    pub fn is_all_lower(&self) -> bool { self.get_value(1) }
    pub fn set_all_lower(&mut self, v: bool) { self.set_value(1, v); }

    pub fn is_capital_upper(&self) -> bool { self.get_value(2) }
    pub fn set_capital_upper(&mut self, v: bool) { self.set_value(2, v); }

    pub fn is_last_lower(&self) -> bool { self.get_value(3) }
    pub fn set_last_lower(&mut self, v: bool) { self.set_value(3, v); }

    pub fn is_letter(&self) -> bool { self.get_value(4) }
    pub fn set_letter(&mut self, v: bool) { self.set_value(4, v); }

    pub fn is_latin_letter(&self) -> bool { self.get_value(5) }
    pub fn set_latin_letter(&mut self, v: bool) { self.set_value(5, v); }

    pub fn is_cyrillic_letter(&self) -> bool { self.get_value(6) }
    pub fn set_cyrillic_letter(&mut self, v: bool) { self.set_value(6, v); }

    pub fn convert_word(&self, word: &str) -> String {
        if word.is_empty() {
            return word.to_string();
        }
        if self.is_all_lower() {
            return word.to_lowercase();
        }
        if self.is_all_upper() {
            return word.to_uppercase();
        }
        if self.is_capital_upper() {
            let mut chars: Vec<char> = word.chars().collect();
            for i in 0..chars.len() {
                if i == 0 {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                } else if i > 0 && (chars[i - 1] == '-' || chars[i - 1] == ' ') {
                    chars[i] = chars[i].to_uppercase().next().unwrap_or(chars[i]);
                } else {
                    chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
                }
            }
            return chars.into_iter().collect();
        }
        word.to_string()
    }
}

impl fmt::Display for CharsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_letter() {
            return write!(f, "Nonletter");
        }
        let case = if self.is_all_upper() {
            "AllUpper"
        } else if self.is_all_lower() {
            "AllLower"
        } else if self.is_capital_upper() {
            "CapitalUpper"
        } else if self.is_last_lower() {
            "LastLower"
        } else {
            "Nonstandard"
        };
        let script = if self.is_latin_letter() {
            " Latin"
        } else if self.is_cyrillic_letter() {
            " Cyrillic"
        } else if self.is_letter() {
            " Letter"
        } else {
            ""
        };
        write!(f, "{}{}", case, script)
    }
}

impl fmt::Debug for CharsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CharsInfo({})", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let mut ci = CharsInfo::new();
        ci.set_letter(true);
        ci.set_all_upper(true);
        ci.set_cyrillic_letter(true);
        assert_eq!(ci.to_string(), "AllUpper Cyrillic");
    }

    #[test]
    fn test_convert_word() {
        let mut ci = CharsInfo::new();
        ci.set_all_lower(true);
        assert_eq!(ci.convert_word("HELLO"), "hello");
    }
}
