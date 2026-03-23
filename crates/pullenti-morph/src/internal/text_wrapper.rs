use super::unicode_info::UnicodeInfo;

pub struct TextWrapper {
    uni_chars: Vec<UnicodeInfo>,
    /// Original text characters (avoids separate Vec<char> allocation in callers)
    pub text_chars: Vec<char>,
    pub length: usize,
}

impl TextWrapper {
    pub fn new(text: &str, _good_text: bool) -> Self {
        let text_chars: Vec<char> = text.chars().collect();
        let uni_chars: Vec<UnicodeInfo> = text_chars.iter().map(|&ch| UnicodeInfo::get_char(ch)).collect();
        let length = uni_chars.len();
        TextWrapper { uni_chars, text_chars, length }
    }

    pub fn get_char(&self, idx: usize) -> UnicodeInfo {
        if idx < self.uni_chars.len() {
            self.uni_chars[idx]
        } else {
            UnicodeInfo::get_char('?')
        }
    }

    pub fn chars_count(&self) -> usize {
        self.uni_chars.len()
    }
}
