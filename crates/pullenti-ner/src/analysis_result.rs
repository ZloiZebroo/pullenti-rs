use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use pullenti_morph::MorphLang;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::TokenRef;
use crate::referent::Referent;

/// Output container from NER text processing
pub struct AnalysisResult {
    /// Input source (shared via Arc to avoid deep cloning)
    pub sofa: Arc<SourceOfAnalysis>,
    /// Extracted named entities
    pub entities: Vec<Rc<RefCell<Referent>>>,
    /// First token in the chain
    pub first_token: Option<TokenRef>,
    /// Detected base language
    pub base_language: MorphLang,
    /// Log messages from analysis
    pub log: Vec<String>,
    /// Processing errors
    pub errors: Vec<String>,
    /// Whether processing was cut short by timeout
    pub is_timeout_breaked: bool,
}

impl AnalysisResult {
    pub fn new(sofa: Arc<SourceOfAnalysis>) -> Self {
        AnalysisResult {
            sofa,
            entities: Vec::new(),
            first_token: None,
            base_language: MorphLang::UNKNOWN,
            log: Vec::new(),
            errors: Vec::new(),
            is_timeout_breaked: false,
        }
    }

    /// Total number of tokens
    pub fn tokens_count(&self) -> usize {
        let mut count = 0;
        let mut t = self.first_token.clone();
        while let Some(tok) = t {
            count += 1;
            t = tok.borrow().next.clone();
        }
        count
    }

    /// Find token at the given character position
    pub fn find_token_by_pos(&self, pos: i32) -> Option<TokenRef> {
        let mut t = self.first_token.clone();
        while let Some(tok) = t {
            let (bc, ec) = {
                let b = tok.borrow();
                (b.begin_char, b.end_char)
            };
            if bc <= pos && pos <= ec { return Some(tok); }
            if bc > pos { break; }
            t = tok.borrow().next.clone();
        }
        None
    }
}

// SAFETY: AnalysisResult contains Rc<RefCell<Referent>> and Option<TokenRef> (also Rc-based).
// These are created thread-locally inside Processor::process() and are exclusively owned
// after the function returns. No Rc is shared across threads. This impl enables returning
// AnalysisResult from parallel batch processing (rayon par_iter).
unsafe impl Send for AnalysisResult {}

impl Drop for AnalysisResult {
    fn drop(&mut self) {
        // Iteratively break the token chain to avoid recursive-drop stack overflow.
        // Without this, Rc<RefCell<Token>>.drop() recurses through every `next` pointer
        // in the linked list, blowing the stack for documents with 100K+ tokens.
        let mut cur = self.first_token.take();
        while let Some(tok) = cur {
            cur = tok.borrow_mut().next.take();
            // tok drops here; next is already None so no recursion
        }
    }
}

impl std::fmt::Debug for AnalysisResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnalysisResult({} entities, {} tokens)",
            self.entities.len(), self.tokens_count())
    }
}
