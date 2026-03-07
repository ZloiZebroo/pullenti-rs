use std::collections::HashMap;

/// Input text wrapper for NER analysis (SourceOfAnalysis)
#[derive(Debug, Clone)]
pub struct SourceOfAnalysis {
    /// Source text
    pub text: String,
    /// Mapping from char index → byte offset in `text` (precomputed for O(1) access)
    char_to_byte: Vec<usize>,
    /// Per-character style bits (bold, italic, etc.)
    pub styles: Option<Vec<u8>>,
    /// Typo correction dictionary
    pub correction_dict: Option<HashMap<String, String>>,
    /// Whether to auto-create number tokens
    pub create_number_tokens: bool,
    /// Whether to do word merging based on morphology
    pub do_words_merging_by_morph: bool,
    /// Whether to do word correction based on morphology
    pub do_word_correction_by_morph: bool,
    /// Start of range to ignore during processing (0 = disabled)
    pub ignored_begin_char: i32,
    /// End of range to ignore during processing (0 = disabled)
    pub ignored_end_char: i32,
    /// Number of CRLF corrections made during processing
    pub crlf_corrected_count: i32,
}

impl SourceOfAnalysis {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let char_to_byte: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        SourceOfAnalysis {
            text,
            char_to_byte,
            styles: None,
            correction_dict: None,
            create_number_tokens: true,
            do_words_merging_by_morph: false,
            do_word_correction_by_morph: false,
            ignored_begin_char: 0,
            ignored_end_char: 0,
            crlf_corrected_count: 0,
        }
    }

    /// Convert a char-based position to a byte offset.
    /// Returns `text.len()` if pos is past the end.
    pub fn char_to_byte_offset(&self, char_pos: usize) -> usize {
        self.char_to_byte.get(char_pos).copied().unwrap_or(self.text.len())
    }

    /// Total number of characters (Unicode scalar values)
    pub fn char_len(&self) -> usize {
        self.char_to_byte.len()
    }

    /// Get a substring using char-based begin/end positions (inclusive)
    pub fn substring(&self, begin: i32, end: i32) -> &str {
        let b = begin.max(0) as usize;
        let e = (end + 1) as usize;
        let byte_b = self.char_to_byte_offset(b);
        let byte_e = self.char_to_byte_offset(e);
        if byte_b >= byte_e { return ""; }
        &self.text[byte_b..byte_e]
    }

    /// Get character at char-based position, or null char if out of bounds
    pub fn char_at(&self, pos: i32) -> char {
        let p = pos as usize;
        if p >= self.char_to_byte.len() { return '\0'; }
        let byte_pos = self.char_to_byte[p];
        self.text[byte_pos..].chars().next().unwrap_or('\0')
    }
}

impl std::fmt::Display for SourceOfAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let preview = if self.char_to_byte.len() > 100 {
            let byte_end = self.char_to_byte_offset(100);
            format!("{}...", &self.text[..byte_end])
        } else {
            self.text.clone()
        };
        write!(f, "{}", preview)
    }
}
