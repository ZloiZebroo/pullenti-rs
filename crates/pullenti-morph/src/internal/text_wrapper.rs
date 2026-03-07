use super::unicode_info::UnicodeInfo;

pub struct TextWrapper {
    chars: Vec<UnicodeInfo>,
    pub length: usize,
}

impl TextWrapper {
    pub fn new(text: &str, _good_text: bool) -> Self {
        let chars: Vec<UnicodeInfo> = text.chars().map(|ch| UnicodeInfo::get_char(ch)).collect();
        let length = chars.len();
        TextWrapper { chars, length }
    }

    pub fn get_char(&self, idx: usize) -> UnicodeInfo {
        if idx < self.chars.len() {
            self.chars[idx]
        } else {
            UnicodeInfo::get_char('?')
        }
    }

    pub fn chars_count(&self) -> usize {
        self.chars.len()
    }
}
