use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::any::Any;

use pullenti_morph::MorphLang;
use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::{TokenRef, TokenKind, build_token_chain};
use crate::referent::Referent;

/// Per-analyzer state stored in AnalysisKit
pub struct AnalyzerData {
    pub items: Vec<Box<dyn Any>>,
}

impl AnalyzerData {
    pub fn new() -> Self { AnalyzerData { items: Vec::new() } }
}

/// Central working context during NER analysis
pub struct AnalysisKit {
    /// Input text (shared via Arc to avoid deep cloning per analyzer)
    pub sofa: Arc<SourceOfAnalysis>,
    /// First token in the chain
    pub first_token: Option<TokenRef>,
    /// Detected base language
    pub base_language: MorphLang,
    /// Accumulated entities
    pub entities: Vec<Rc<RefCell<Referent>>>,
    /// Per-analyzer scratch data (keyed by analyzer name)
    pub analyzer_data: HashMap<String, AnalyzerData>,
    /// Dedup index: (type_name, first_slot_value) → indices into entities Vec
    entity_dedup: HashMap<(String, String), Vec<usize>>,
}

impl AnalysisKit {
    pub fn new(sofa: Arc<SourceOfAnalysis>) -> Self {
        AnalysisKit {
            sofa,
            first_token: None,
            base_language: MorphLang::UNKNOWN,
            entities: Vec::new(),
            analyzer_data: HashMap::new(),
            entity_dedup: HashMap::new(),
        }
    }

    /// Build the token chain from morphological analysis results
    pub fn build_tokens(&mut self, morph_tokens: Vec<pullenti_morph::MorphToken>) {
        self.first_token = build_token_chain(morph_tokens, &self.sofa);
    }

    /// Get character at position in source text
    pub fn get_text_character(&self, pos: i32) -> char {
        self.sofa.char_at(pos)
    }

    /// Find the token at the given character position
    pub fn find_token_by_pos(&self, pos: i32) -> Option<TokenRef> {
        let mut t = self.first_token.clone();
        while let Some(tok) = t {
            let (bc, ec) = {
                let b = tok.borrow();
                (b.begin_char, b.end_char)
            };
            if bc <= pos && pos <= ec {
                return Some(tok);
            }
            if bc > pos { break; }
            t = tok.borrow().next.clone();
        }
        None
    }

    /// Register an entity in the kit, deduplicating by slot equality.
    /// Returns the canonical entity (existing one if dedup matched, otherwise the new one).
    pub fn add_entity(&mut self, r: Rc<RefCell<Referent>>) -> Rc<RefCell<Referent>> {
        // Collect non-internal string slots for comparison — single borrow block
        let (type_name, slots): (String, Vec<(String, String)>) = {
            let rb = r.borrow();
            let tn = rb.type_name.clone();
            let sl = rb.slots.iter()
                .filter(|s| !s.is_internal())
                .filter_map(|s| {
                    s.value.as_ref()
                        .and_then(|v| v.as_str())
                        .map(|sv| (s.type_name.clone(), sv.to_string()))
                })
                .collect();
            (tn, sl)
        };

        if !slots.is_empty() {
            // Build dedup key from first slot
            let dedup_key = (type_name.clone(), slots[0].1.clone());

            // Only check candidates with matching dedup key (instead of ALL entities)
            if let Some(candidates) = self.entity_dedup.get(&dedup_key) {
                for &idx in candidates {
                    let existing = &self.entities[idx];
                    let existing_b = existing.borrow();
                    // Check bidirectional slot equality (string slots only)
                    let a_in_b = slots.iter().all(|(name, val)| {
                        existing_b.find_slot(name, Some(val)).is_some()
                    });
                    if !a_in_b { continue; }
                    let b_in_a = existing_b.slots.iter()
                        .filter(|s| !s.is_internal())
                        .filter_map(|s| s.value.as_ref().and_then(|v| v.as_str())
                            .map(|sv| (s.type_name.as_str(), sv.to_string())))
                        .all(|(name, val)| {
                            slots.iter().any(|(n, v)| n == name && v == &val)
                        });
                    if b_in_a {
                        // Merge occurrences from new into existing
                        drop(existing_b);
                        let new_occ: Vec<(i32, i32)> = r.borrow().occurrence.iter()
                            .map(|o| (o.begin_char, o.end_char))
                            .collect();
                        for (bc, ec) in new_occ {
                            existing.borrow_mut().add_occurrence(bc, ec);
                        }
                        return existing.clone();
                    }
                }
            }

            // No match found — register in dedup index
            let idx = self.entities.len();
            self.entity_dedup.entry(dedup_key).or_default().push(idx);
        }

        self.entities.push(r.clone());
        r
    }

    /// Embed a meta token into the chain, replacing the span from begin to end.
    /// Uses begin_token/end_token from MetaTokenData for O(1) neighbor lookup
    /// instead of scanning the entire token chain.
    pub fn embed_token(&mut self, meta: TokenRef) {
        // Try fast path: extract begin/end tokens from MetaTokenData
        let (prev, after) = {
            let m = meta.borrow();
            match &m.kind {
                TokenKind::Referent(rd) => {
                    let prev = rd.meta.begin_token.as_ref()
                        .and_then(|bt| bt.borrow().prev.as_ref().and_then(|w| w.upgrade()));
                    let after = rd.meta.end_token.as_ref()
                        .and_then(|et| et.borrow().next.clone());
                    (prev, after)
                }
                _ => {
                    // Fallback: scan for position (non-referent tokens)
                    let meta_begin = m.begin_char;
                    let meta_end = m.end_char;
                    drop(m);
                    return self.embed_token_scan(meta, meta_begin, meta_end);
                }
            }
        };

        self.wire_token(meta, prev, after);
    }

    /// Fallback: scan token chain to find neighbors (for non-referent tokens)
    fn embed_token_scan(&mut self, meta: TokenRef, meta_begin: i32, meta_end: i32) {
        let mut prev: Option<TokenRef> = None;
        let mut t = self.first_token.clone();
        while let Some(tok) = t.clone() {
            if tok.borrow().begin_char >= meta_begin { break; }
            prev = Some(tok.clone());
            t = tok.borrow().next.clone();
        }

        let mut after: Option<TokenRef> = None;
        let mut t2 = t;
        while let Some(tok) = t2 {
            let tok_end = tok.borrow().end_char;
            t2 = tok.borrow().next.clone();
            if tok_end >= meta_end {
                after = t2;
                break;
            }
        }

        self.wire_token(meta, prev, after);
    }

    /// Wire a meta token between prev and after in the chain
    fn wire_token(&mut self, meta: TokenRef, prev: Option<TokenRef>, after: Option<TokenRef>) {
        {
            let mut m = meta.borrow_mut();
            m.prev = prev.as_ref().map(|p| Rc::downgrade(p));
            m.next = after.clone();
            m.invalidate_attrs();
        }

        if let Some(ref p) = prev {
            p.borrow_mut().next = Some(meta.clone());
            p.borrow().invalidate_attrs();
        } else {
            self.first_token = Some(meta.clone());
        }

        if let Some(ref a) = after {
            a.borrow_mut().prev = Some(Rc::downgrade(&meta));
            a.borrow().invalidate_attrs();
        }
    }

    /// Remove a token from the chain
    pub fn debed_token(&mut self, token: &TokenRef) {
        let prev_weak = token.borrow().prev.clone();
        let next = token.borrow().next.clone();

        if let Some(ref pw) = prev_weak {
            if let Some(prev) = pw.upgrade() {
                prev.borrow_mut().next = next.clone();
                prev.borrow().invalidate_attrs();
            }
        } else {
            // Was first token
            self.first_token = next.clone();
        }

        if let Some(ref n) = next {
            n.borrow_mut().prev = prev_weak;
            n.borrow().invalidate_attrs();
        }
    }

    /// Get or create per-analyzer data
    pub fn get_analyzer_data(&mut self, name: &str) -> &mut AnalyzerData {
        self.analyzer_data.entry(name.to_string()).or_insert_with(AnalyzerData::new)
    }

    /// Iteratively drop the token chain, preventing recursive-drop stack overflow
    pub fn drain_token_chain(&mut self) {
        let mut cur = self.first_token.take();
        while let Some(tok) = cur {
            cur = tok.borrow_mut().next.take();
        }
    }

    /// Determine the base language from the token chain
    pub fn define_base_language(&mut self) {
        let mut ru = 0i32;
        let mut ua = 0i32;
        let mut en = 0i32;

        let mut t = self.first_token.clone();
        while let Some(tok) = t {
            let lang = tok.borrow().morph.clone_collection().language();
            if lang.is_ru() { ru += 1; }
            if lang.is_ua() { ua += 1; }
            if lang.is_en() { en += 1; }
            t = tok.borrow().next.clone();
        }

        if ru >= ua && ru >= en { self.base_language = MorphLang::RU; }
        else if ua > ru && ua >= en { self.base_language = MorphLang::UA; }
        else if en > 0 { self.base_language = MorphLang::EN; }
        else { self.base_language = MorphLang::RU; }
    }
}

