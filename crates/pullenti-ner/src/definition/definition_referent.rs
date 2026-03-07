/// DefinitionReferent — a thesis/definition/assertion entity.
/// Mirrors Pullenti C# `DefinitionReferent` and `DefinitionKind`.
///
/// OBJ_TYPENAME = "THESIS"
/// Represents patterns like:
///   "Предприниматель — это физическое лицо..."   → THESIS, kind=Definition
///   "Договор является соглашением сторон..."     → THESIS, kind=Assertation
///   "Стороны несут ответственность..."           → THESIS, kind=Assertation

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME:    &str = "THESIS";
pub const ATTR_TERMIN:     &str = "TERMIN";     // term being defined (left side)
pub const ATTR_TERMIN_ADD: &str = "TERMINADD";  // additional term qualifier
pub const ATTR_VALUE:      &str = "VALUE";      // definition text (right side)
pub const ATTR_MISC:       &str = "MISC";       // miscellaneous qualifier
pub const ATTR_KIND:       &str = "KIND";       // DefinitionKind as string
pub const ATTR_DECREE:     &str = "DECREE";     // reference to normative act

/// Thesis type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Undefined,
    /// Simple assertion ("является", "это", "есть")
    Assertation,
    /// Strict definition ("—" em-dash with nominative right side)
    Definition,
    /// Negation ("не является")
    Negation,
}

impl DefinitionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefinitionKind::Undefined   => "Undefined",
            DefinitionKind::Assertation => "Assertation",
            DefinitionKind::Definition  => "Definition",
            DefinitionKind::Negation    => "Negation",
        }
    }
}

// ── Constructor ───────────────────────────────────────────────────────────────

pub fn new_thesis_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Slot helpers ─────────────────────────────────────────────────────────────

pub fn add_slot_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

pub fn set_kind(r: &mut Referent, kind: &DefinitionKind) {
    r.slots.retain(|s| s.type_name != ATTR_KIND);
    r.slots.push(Slot::new(ATTR_KIND, Some(SlotValue::Str(kind.as_str().to_string()))));
}

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub fn get_termin(r: &Referent)    -> Option<String> { get_str(r, ATTR_TERMIN) }
pub fn get_termin_add(r: &Referent)-> Option<String> { get_str(r, ATTR_TERMIN_ADD) }
pub fn get_value(r: &Referent)     -> Option<String> { get_str(r, ATTR_VALUE) }
pub fn get_kind_str(r: &Referent)  -> Option<String> { get_str(r, ATTR_KIND) }
