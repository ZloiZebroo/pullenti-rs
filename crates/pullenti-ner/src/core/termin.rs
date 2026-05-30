use crate::token::TokenRef;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
        if self.text == s {
            return true;
        }
        if self.abridges.iter().any(|a| a == s) {
            return true;
        }
        if self.variants.iter().any(|v| v == s) {
            return true;
        }
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
        TerminCollection {
            termins: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn add(&mut self, t: Termin) {
        let idx = self.termins.len();
        let t = Arc::new(t);
        let mut indexed = HashSet::new();

        // Index by primary first word
        let first = first_word(&t.text);
        if indexed.insert(first.to_string()) {
            self.index.entry(first.to_string()).or_default().push(idx);
        }

        // Index by abbreviations
        for a in &t.abridges {
            let fw = first_word(a);
            if indexed.insert(fw.to_string()) {
                self.index.entry(fw.to_string()).or_default().push(idx);
            }
        }

        // Index by variants
        for v in &t.variants {
            let fw = first_word(v);
            if indexed.insert(fw.to_string()) {
                self.index.entry(fw.to_string()).or_default().push(idx);
            }
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

        let mut best: Option<(Arc<Termin>, TokenRef, usize)> = None;
        for &idx in indices {
            let t = &self.termins[idx];

            // Case 1: abridge (single token, optionally consume trailing '.')
            if t.abridges.iter().any(|a| a == &term0) {
                let next = t0.borrow().next.clone();
                let (end, len) = if let Some(n) = next {
                    let is_dot = n.borrow().term().map_or(false, |s| s == ".");
                    if is_dot {
                        (n, 2)
                    } else {
                        (t0.clone(), 1)
                    }
                } else {
                    (t0.clone(), 1)
                };
                update_best(&mut best, t, end, len);
            }

            if let Some((end, len)) = try_match_phrase(t0, &t.text) {
                update_best(&mut best, t, end, len);
            }

            for v in &t.variants {
                if let Some((end, len)) = try_match_phrase(t0, v) {
                    update_best(&mut best, t, end, len);
                }
            }
        }
        best.map(|(termin, end_token, _)| TerminToken { termin, end_token })
    }
}

fn update_best(
    best: &mut Option<(Arc<Termin>, TokenRef, usize)>,
    termin: &Arc<Termin>,
    end: TokenRef,
    len: usize,
) {
    if best.as_ref().map_or(true, |(_, _, best_len)| len > *best_len) {
        *best = Some((termin.clone(), end, len));
    }
}

/// Try to match a complete phrase from t0.
/// Returns the last matched token and matched token count.
fn try_match_phrase(t0: &TokenRef, phrase: &str) -> Option<(TokenRef, usize)> {
    let mut words = phrase.split_whitespace();
    let first = words.next()?;
    let first_term = t0.borrow().term().map(|s| s.to_string())?;
    if first_term != first {
        return None;
    }

    let mut cur = t0.clone();
    let mut len = 1usize;
    for word in words {
        let next = cur.borrow().next.clone()?;
        let n_term = next.borrow().term().map(|s| s.to_string())?;
        if n_term != word {
            return None;
        }
        cur = next;
        len += 1;
    }
    Some((cur, len))
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_of_analysis::SourceOfAnalysis;
    use crate::token::build_token_chain;
    use pullenti_morph::{MorphLang, MorphologyService};

    fn first_token(text: &str) -> (SourceOfAnalysis, TokenRef) {
        MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
        let sofa = SourceOfAnalysis::new(text);
        let morph = MorphologyService::process(&sofa.text, Some(MorphLang::RU | MorphLang::EN))
            .unwrap_or_default();
        let first = build_token_chain(morph, &sofa).expect("token chain");
        (sofa, first)
    }

    #[test]
    fn try_parse_prefers_longest_candidate() {
        let mut tc = TerminCollection::new();
        tc.add(Termin::new_canonic("БАНК", "SHORT"));
        tc.add(Termin::new_canonic("БАНК РОССИИ", "LONG"));

        let (_sofa, t) = first_token("Банк России опубликовал документ");
        let tok = tc.try_parse(&t).expect("longest term");

        assert_eq!(tok.termin.canonic_text, "LONG");
        assert_eq!(tok.end_token.borrow().term(), Some("РОССИИ"));
    }

    #[test]
    fn add_deduplicates_index_keys_for_same_termin() {
        let mut t = Termin::new("PHONE");
        t.add_variant("PHONE NUMBER");
        t.add_abridge("PHONE.");

        let mut tc = TerminCollection::new();
        tc.add(t);

        let bucket = tc.index.get("PHONE").expect("PHONE bucket");
        assert_eq!(bucket.len(), 1);
    }

    #[test]
    fn try_parse_uses_canonical_variant() {
        let mut t = Termin::new_canonic("ОБЩЕРОССИЙСКИЙ КЛАССИФИКАТОР", "ОК");
        t.add_variant("ОК");
        let mut tc = TerminCollection::new();
        tc.add(t);

        let (_sofa, first) = first_token("ОК 12");
        let tok = tc.try_parse(&first).expect("variant");

        assert_eq!(tok.termin.canonic_text, "ОК");
        assert_eq!(tok.end_token.borrow().term(), Some("ОК"));
    }
}
