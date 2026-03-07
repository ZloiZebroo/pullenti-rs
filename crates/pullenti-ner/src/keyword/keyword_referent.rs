/// KeywordReferent — a keyword/key-combination entity.
/// Mirrors `KeywordReferent.cs` and `KeywordType.cs`.

use crate::referent::{Referent, SlotValue};

// ── Constants ─────────────────────────────────────────────────────────────

pub const OBJ_TYPENAME: &str = "KEYWORD";
pub const ATTR_TYPE:    &str = "TYPE";
pub const ATTR_VALUE:   &str = "VALUE";
pub const ATTR_NORMAL:  &str = "NORMAL";
pub const ATTR_REF:     &str = "REF";

// ── KeywordType ────────────────────────────────────────────────────────────

/// Type of a keyword combination (mirrors `KeywordType` enum in C#)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeywordType {
    /// Undefined / not set
    #[default]
    Undefined,
    /// Noun-phrase object
    Object,
    /// Named entity reference
    Referent,
    /// Verbal predicate
    Predicate,
    /// Auto-annotation sentence summary
    Annotation,
}

impl KeywordType {
    pub fn as_str(self) -> &'static str {
        match self {
            KeywordType::Undefined   => "Undefined",
            KeywordType::Object      => "Object",
            KeywordType::Referent    => "Referent",
            KeywordType::Predicate   => "Predicate",
            KeywordType::Annotation  => "Annotation",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Object"     => KeywordType::Object,
            "Referent"   => KeywordType::Referent,
            "Predicate"  => KeywordType::Predicate,
            "Annotation" => KeywordType::Annotation,
            _            => KeywordType::Undefined,
        }
    }
}

// ── Accessors ──────────────────────────────────────────────────────────────

pub fn new_keyword_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Get the KeywordType stored in the TYPE slot.
pub fn get_typ(r: &Referent) -> KeywordType {
    r.get_string_value(ATTR_TYPE)
        .map(KeywordType::from_str)
        .unwrap_or_default()
}

/// Set the TYPE slot.
pub fn set_typ(r: &mut Referent, typ: KeywordType) {
    r.add_slot(ATTR_TYPE, SlotValue::Str(typ.as_str().to_string()), true);
}

/// Get the VALUE slot (non-normalised string value).
pub fn get_value(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_VALUE)
}

/// Add a VALUE slot (may be multi-valued — don't clear_old).
pub fn add_value(r: &mut Referent, v: impl Into<String>) {
    r.add_slot(ATTR_VALUE, SlotValue::Str(v.into()), false);
}

/// Get the NORMAL slot (normalised / lemmatised value).
pub fn get_normal(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_NORMAL)
}

/// Add a NORMAL slot (may be multi-valued).
pub fn add_normal(r: &mut Referent, v: impl Into<String>) {
    r.add_slot(ATTR_NORMAL, SlotValue::Str(v.into()), false);
}

/// Short display string for a keyword referent.
pub fn to_string_short(r: &Referent) -> String {
    if let Some(v) = get_value(r) {
        return v.to_string();
    }
    if let Some(n) = get_normal(r) {
        return n.to_string();
    }
    "?".to_string()
}

/// Get the accumulated rank stored in the RANK slot (as formatted f64 string).
pub fn get_rank(r: &Referent) -> f64 {
    r.get_string_value("RANK")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// Accumulate rank (adds to any existing RANK value).
pub fn add_rank(r: &mut Referent, delta: f64) {
    let current = get_rank(r);
    let new_val = current + delta;
    r.add_slot("RANK", SlotValue::Str(format!("{:.6}", new_val)), true);
}

/// Compute the rank contribution for a single occurrence.
/// Mirrors `_setRank` in C#.
pub fn compute_rank_delta(r: &Referent, cur: i32, max: i32) -> f64 {
    let mut rank: f64 = 1.0;
    let typ = get_typ(r);
    match typ {
        KeywordType::Predicate => {
            rank = 1.0;
        }
        KeywordType::Object => {
            let v = get_value(r).or_else(|| get_normal(r)).unwrap_or("");
            for ch in v.chars() {
                if ch == ' ' || ch == '-' {
                    rank += 1.0;
                }
            }
        }
        KeywordType::Referent => {
            rank = 3.0;
            // Boost for PERSON
            if let Some(slot) = r.find_slot(ATTR_REF, None) {
                if let Some(ref_val) = slot.value.as_ref() {
                    if let Some(ref_r) = ref_val.as_referent() {
                        if ref_r.borrow().type_name == "PERSON" {
                            rank = 4.0;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    if max > 0 {
        rank *= 1.0 - (0.5 * cur as f64 / max as f64);
    }
    rank
}
