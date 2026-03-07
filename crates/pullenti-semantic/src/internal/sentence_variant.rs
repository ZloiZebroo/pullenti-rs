/// SentenceVariant — combination of NGSegmentVariants for all segments in a sentence.
/// Validates cross-segment consistency (no duplicate Agent/Pacient).
/// Mirrors `SentenceVariant.cs`.

use super::ng_link::NGLinkType;
use super::ng_segment_variant::NGSegmentVariant;

// ── SentenceVariant ───────────────────────────────────────────────────────

pub struct SentenceVariant {
    pub coef: f64,
    pub segs: Vec<Option<NGSegmentVariant>>,
}

impl SentenceVariant {
    pub fn calc_coef(&mut self) -> f64 {
        self.coef = self.segs.iter()
            .filter_map(|s| s.as_ref())
            .map(|s| s.coef)
            .sum();

        let n = self.segs.len();
        for i in 0..(n.saturating_sub(1)) {
            let before_verb = match &self.segs[i + 1] {
                Some(s) => s.before_verb_sent_idx,
                None => continue,
            };

            let mut has_agent   = false;
            let mut has_pacient = false;

            if let Some(seg0) = &self.segs[i] {
                for li in seg0.links.iter().filter_map(|l| l.as_ref()) {
                    if li.to_verb_sent_idx == before_verb {
                        if li.typ == NGLinkType::Agent   { has_agent   = true; }
                        else if li.typ == NGLinkType::Pacient { has_pacient = true; }
                    }
                }
            }

            if let Some(seg1) = &self.segs[i + 1] {
                for li in seg1.links.iter().filter_map(|l| l.as_ref()) {
                    if li.to_verb_sent_idx == before_verb {
                        if li.typ == NGLinkType::Agent   && has_agent   { self.coef = -1.0; return -1.0; }
                        if li.typ == NGLinkType::Pacient && has_pacient { self.coef = -1.0; return -1.0; }
                    }
                }
            }
        }
        self.coef
    }
}

// ── pick_best_sentence_variant ────────────────────────────────────────────

/// Enumerate cross-segment variant combinations and return the best valid one.
/// Mirrors the enumeration loop in `Sentence.CalcCoef()`.
pub fn pick_best_sentence_variant(
    seg_variants: &[Vec<NGSegmentVariant>],
) -> Vec<Option<NGSegmentVariant>> {
    let n = seg_variants.len();
    if n == 0 { return vec![]; }

    // Single segment: just return the best variant
    if n == 1 {
        return vec![seg_variants[0].first().cloned()];
    }

    let mut inds = vec![0usize; n];
    let mut best_coef = -1.0f64;
    let mut best_segs: Vec<Option<NGSegmentVariant>> = vec![None; n];

    for _ in 0..1000 {
        let segs: Vec<Option<NGSegmentVariant>> = inds.iter().enumerate()
            .map(|(i, &idx)| seg_variants[i].get(idx).cloned())
            .collect();

        let mut svar = SentenceVariant { coef: 0.0, segs };
        let coef = svar.calc_coef();
        if coef > best_coef {
            best_coef = coef;
            best_segs = svar.segs;
        }

        // Odometer advance from the right
        let mut j = n as isize - 1;
        loop {
            if j < 0 { return best_segs; }
            inds[j as usize] += 1;
            let seg_len = seg_variants[j as usize].len().max(1);
            if inds[j as usize] >= seg_len {
                inds[j as usize] = 0;
                j -= 1;
            } else {
                break;
            }
        }
    }

    best_segs
}
