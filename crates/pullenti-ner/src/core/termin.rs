use std::collections::HashMap;
use std::sync::Arc;
use crate::token::TokenRef;

/// A keyword/term with variants and abbreviations (Термин)
pub struct Termin {
    /// Primary matching text (uppercase)
    pub text: String,
    /// Canonical text — used as the scheme/type name; may differ in case from `text`
    pub canonic_text: String,
    /// Additional variants (uppercase)
    pub variants: Vec<String>,
    /// Abbreviations without trailing dot (uppercase)
    pub abridges: Vec<String>,
    /// Arbitrary tag (e.g. used to mark "additional number" keywords)
    pub tag: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Secondary tag (e.g. PhoneKind for phone-type keywords)
    pub tag2: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl Termin {
    pub fn new(text: impl Into<String>) -> Self {
        let upper = text.into().to_uppercase();
        Termin {
            canonic_text: upper.clone(),
            text: upper,
            variants: Vec::new(),
            abridges: Vec::new(),
            tag: None,
            tag2: None,
        }
    }

    /// Create a termin whose canonical text differs from the matching text
    /// (e.g. text="TELEGRAM", canonic_text="telegram")
    pub fn new_canonic(text: impl Into<String>, canonic: impl Into<String>) -> Self {
        Termin {
            text: text.into().to_uppercase(),
            canonic_text: canonic.into(),
            variants: Vec::new(),
            abridges: Vec::new(),
            tag: None,
            tag2: None,
        }
    }

    /// Add an abbreviation like "ТЕЛ." → stored as "ТЕЛ" (strip trailing dot)
    pub fn add_abridge(&mut self, abridge: &str) {
        let s = abridge.trim_end_matches('.').to_uppercase();
        if !self.abridges.contains(&s) {
            self.abridges.push(s);
        }
    }

    /// Add a variant spelling
    pub fn add_variant(&mut self, variant: &str) {
        let s = variant.to_uppercase();
        if !self.variants.contains(&s) {
            self.variants.push(s);
        }
    }

    /// Check if a given uppercase string matches this term
    pub fn matches(&self, s: &str) -> bool {
        if self.text == s { return true; }
        if self.abridges.iter().any(|a| a == s) { return true; }
        if self.variants.iter().any(|v| v == s) { return true; }
        false
    }
}

/// Result of a successful termin match
pub struct TerminToken {
    pub termin: Arc<Termin>,
    /// Last token consumed by the match
    pub end_token: TokenRef,
}

/// Collection of Termins with fast lookup (TerminCollection)
pub struct TerminCollection {
    termins: Vec<Arc<Termin>>,
    /// index: first word → list of termins whose primary text / variants start with that word
    index: HashMap<String, Vec<usize>>,
}

impl TerminCollection {
    pub fn new() -> Self {
        TerminCollection { termins: Vec::new(), index: HashMap::new() }
    }

    pub fn add(&mut self, t: Termin) {
        let idx = self.termins.len();
        let t = Arc::new(t);

        // Index by primary first word
        let first = first_word(&t.text);
        self.index.entry(first.to_string()).or_default().push(idx);

        // Index by abbreviations
        for a in &t.abridges {
            let fw = first_word(a);
            self.index.entry(fw.to_string()).or_default().push(idx);
        }

        // Index by variants
        for v in &t.variants {
            let fw = first_word(v);
            self.index.entry(fw.to_string()).or_default().push(idx);
        }

        self.termins.push(t);
    }

    /// Try to match one or more tokens starting at `t0`.
    /// Returns the matched TerminToken, or None.
    /// Prefers longer matches (multi-word primary text beats single-word).
    pub fn try_parse(&self, t0: &TokenRef) -> Option<TerminToken> {
        let term0 = {
            let b = t0.borrow();
            b.term().map(|s| s.to_string())?
        };

        let indices = self.index.get(&term0)?;

        for &idx in indices {
            let t = &self.termins[idx];
            let words: Vec<&str> = t.text.split_whitespace().collect();

            // Determine if this termin is reachable by term0
            let is_abridge = t.abridges.iter().any(|a| a == &term0);
            let primary_first_matches = words.first().map_or(false, |w| *w == term0.as_str());
            let variant_first_matches = t.variants.iter().any(|v| {
                v.split_whitespace().next().map_or(false, |w| w == term0.as_str())
            });

            if !is_abridge && !primary_first_matches && !variant_first_matches {
                continue;
            }

            // Case 1: abridge (single token, optionally consume trailing '.')
            if is_abridge {
                let next = t0.borrow().next.clone();
                let end = if let Some(n) = next {
                    let is_dot = n.borrow().term().map_or(false, |s| s == ".");
                    if is_dot { n } else { t0.clone() }
                } else { t0.clone() };
                return Some(TerminToken { termin: t.clone(), end_token: end });
            }

            // Case 2: multi-word primary text (try before single-word fallback)
            if words.len() > 1 && primary_first_matches {
                if let Some(end) = try_match_words_seq(t0, &words[1..]) {
                    return Some(TerminToken { termin: t.clone(), end_token: end });
                }
                // Multi-word didn't match — fall through to single-word variants
            }

            // Case 3: single-word primary text
            if words.len() == 1 && primary_first_matches {
                return Some(TerminToken { termin: t.clone(), end_token: t0.clone() });
            }

            // Case 4: multi-word variant
            for v in &t.variants {
                let vwords: Vec<&str> = v.split_whitespace().collect();
                if vwords.len() > 1 && vwords.first().map_or(false, |w| *w == term0.as_str()) {
                    if let Some(end) = try_match_words_seq(t0, &vwords[1..]) {
                        return Some(TerminToken { termin: t.clone(), end_token: end });
                    }
                }
            }

            // Case 5: single-word variant
            if t.variants.iter().any(|v| v == &term0) {
                return Some(TerminToken { termin: t.clone(), end_token: t0.clone() });
            }
        }
        None
    }
}

/// Try to match remaining words sequentially from the token AFTER t0.
/// Returns the last matched token, or None if any word doesn't match.
fn try_match_words_seq(t0: &TokenRef, remaining: &[&str]) -> Option<TokenRef> {
    let mut cur = t0.clone();
    for &word in remaining {
        let next = cur.borrow().next.clone()?;
        let n_term = next.borrow().term().map(|s| s.to_string())?;
        if n_term != word { return None; }
        cur = next;
    }
    Some(cur)
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or(s)
}
