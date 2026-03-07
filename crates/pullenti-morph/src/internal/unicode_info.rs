use std::sync::OnceLock;

static ALL_CHARS: OnceLock<Vec<UnicodeInfo>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct UnicodeInfo {
    pub uni_char: char,
    pub code: i32,
    value: i16,
}

impl UnicodeInfo {
    fn new(code: i16) -> Self {
        UnicodeInfo {
            uni_char: char::from_u32(code as u32).unwrap_or('?'),
            code: code as i32,
            value: 0,
        }
    }

    fn set_value(&mut self, i: u32, val: bool) {
        if val {
            self.value |= 1 << i;
        } else {
            self.value &= !(1 << i);
        }
    }

    pub fn is_whitespace(&self) -> bool { (self.value & 0x1) != 0 }
    pub fn is_digit(&self) -> bool { (self.value & 0x2) != 0 }
    pub fn is_letter(&self) -> bool { (self.value & 0x4) != 0 }
    pub fn is_upper(&self) -> bool { (self.value & 0x8) != 0 }
    pub fn is_lower(&self) -> bool { (self.value & 0x10) != 0 }
    pub fn is_latin(&self) -> bool { (self.value & 0x20) != 0 }
    pub fn is_cyrillic(&self) -> bool { (self.value & 0x40) != 0 }
    pub fn is_hiphen(&self) -> bool { (self.value & 0x80) != 0 }
    pub fn is_vowel(&self) -> bool { (self.value & 0x100) != 0 }
    pub fn is_quot(&self) -> bool { (self.value & 0x200) != 0 }
    pub fn is_apos(&self) -> bool { (self.value & 0x400) != 0 }
    pub fn is_udaren(&self) -> bool { (self.value & 0x800) != 0 }

    pub fn get_char(ch: char) -> UnicodeInfo {
        let chars = ALL_CHARS.get_or_init(Self::initialize);
        let ii = ch as usize;
        if ii >= 0x10000 {
            chars['?' as usize]
        } else {
            chars[ii]
        }
    }

    pub fn initialize() -> Vec<UnicodeInfo> {
        let cyrvowel = "АЕЁИОУЮЯЫЭЄІЇЎӘӨҰҮІаеёиоуюяыэєіїўәөұүі";
        let mut all_chars = Vec::with_capacity(0x10000);

        for i in 0..0x10000u32 {
            let ch = char::from_u32(i).unwrap_or('\0');
            let mut ui = UnicodeInfo::new(i as i16);

            if ch.is_whitespace() {
                ui.set_value(0, true); // whitespace
            } else if ch.is_ascii_digit() || (ch >= '0' && ch <= '9') || ch.is_numeric() {
                // More precise: match C# Char.IsDigit
                if ch.is_ascii_digit() {
                    ui.set_value(1, true); // digit
                } else {
                    // Check if it's a Unicode digit category
                    if ch.is_numeric() && !ch.is_alphabetic() {
                        ui.set_value(1, true);
                    }
                }
            } else if ch == 'º' || ch == '°' {
                // skip
            } else if ch.is_alphabetic() {
                ui.set_value(2, true); // letter
                if i >= 0x400 && i < 0x500 {
                    ui.set_value(6, true); // cyrillic
                    if cyrvowel.contains(ch) {
                        ui.set_value(8, true); // vowel
                    }
                } else if i < 0x200 {
                    ui.set_value(5, true); // latin
                    if "AEIOUYaeiouy".contains(ch) {
                        ui.set_value(8, true); // vowel
                    }
                }
                if ch.is_uppercase() {
                    ui.set_value(3, true); // upper
                }
                if ch.is_lowercase() {
                    ui.set_value(4, true); // lower
                }
            } else {
                // Check for hyphens
                if ch == '-' || ch == '–' || ch == '¬' || ch == '\u{00AD}'
                    || ch == '\u{2011}' || ch == '—' || ch == '−'
                {
                    ui.set_value(7, true); // hiphen
                }
                // Check for quotes
                if "\"'`\u{201C}\u{201D}\u{2018}\u{2019}".contains(ch) {
                    ui.set_value(9, true); // quot
                }
                // Check for apostrophes
                if "'`\u{2019}".contains(ch) {
                    ui.set_value(10, true); // apos
                    ui.set_value(9, true); // quot
                }
            }
            if i >= 0x300 && i < 0x370 {
                ui.set_value(11, true); // udaren
            }
            all_chars.push(ui);
        }
        all_chars
    }
}
