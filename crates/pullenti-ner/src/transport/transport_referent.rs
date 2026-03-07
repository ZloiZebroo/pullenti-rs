/// TransportReferent — a vehicle/transport entity.
/// Mirrors Pullenti C# `TransportReferent`.

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME:      &str = "TRANSPORT";
pub const ATTR_TYPE:         &str = "TYPE";    // vehicle type string (автомобиль, самолет...)
pub const ATTR_BRAND:        &str = "BRAND";   // manufacturer brand
pub const ATTR_MODEL:        &str = "MODEL";   // model designation
pub const ATTR_KIND:         &str = "KIND";    // TransportKind as string
pub const ATTR_NAME:         &str = "NAME";    // proper name (ships)
pub const ATTR_NUMBER:       &str = "NUMBER";  // registration number

/// Category of transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportKind {
    Undefined,
    Auto,
    Train,
    Ship,
    Fly,
    Space,
}

impl TransportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportKind::Undefined => "Undefined",
            TransportKind::Auto     => "Auto",
            TransportKind::Train    => "Train",
            TransportKind::Ship     => "Ship",
            TransportKind::Fly      => "Fly",
            TransportKind::Space    => "Space",
        }
    }
}

pub fn new_transport_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

pub fn add_slot_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

pub fn set_kind(r: &mut Referent, kind: &TransportKind) {
    r.slots.retain(|s| s.type_name != ATTR_KIND);
    r.slots.push(Slot::new(ATTR_KIND, Some(SlotValue::Str(kind.as_str().to_string()))));
}

pub fn get_type(r: &Referent)   -> Option<String> { get_str(r, ATTR_TYPE) }
pub fn get_brand(r: &Referent)  -> Option<String> { get_str(r, ATTR_BRAND) }
pub fn get_model(r: &Referent)  -> Option<String> { get_str(r, ATTR_MODEL) }
pub fn get_name(r: &Referent)   -> Option<String> { get_str(r, ATTR_NAME) }
pub fn get_kind(r: &Referent)   -> Option<String> { get_str(r, ATTR_KIND) }
pub fn get_number(r: &Referent) -> Option<String> { get_str(r, ATTR_NUMBER) }
