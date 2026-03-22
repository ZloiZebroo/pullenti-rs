/// PersonIdentityReferent — identity document entity (passport, driver's license, etc.)
/// Mirrors `PersonIdentityReferent.cs`.

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME: &str = "PERSONIDENTITY";
pub const ATTR_TYPE:    &str = "TYPE";
pub const ATTR_NUMBER:  &str = "NUMBER";
pub const ATTR_DATE:    &str = "DATE";
pub const ATTR_ORG:     &str = "ORG";
pub const ATTR_STATE:   &str = "STATE";
pub const ATTR_ADDRESS: &str = "ADDRESS";

pub fn new_person_identity_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Setters ────────────────────────────────────────────────────────────────────

pub fn set_type(r: &mut Referent, v: &str) {
    r.slots.retain(|s| s.type_name != ATTR_TYPE);
    r.slots.push(Slot::new(ATTR_TYPE, Some(SlotValue::Str(v.to_string()))));
}

pub fn set_number(r: &mut Referent, v: &str) {
    r.slots.retain(|s| s.type_name != ATTR_NUMBER);
    r.slots.push(Slot::new(ATTR_NUMBER, Some(SlotValue::Str(v.to_string()))));
}

// ── Getters ────────────────────────────────────────────────────────────────────

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

pub fn get_type(r: &Referent)   -> Option<String> { get_str(r, ATTR_TYPE) }
pub fn get_number(r: &Referent) -> Option<String> { get_str(r, ATTR_NUMBER) }

pub fn to_string_short(r: &Referent) -> String {
    let typ = get_type(r).unwrap_or_else(|| "документ".to_string());
    let num = get_number(r);
    if let Some(n) = num {
        format!("{} №{}", typ, n)
    } else {
        typ
    }
}
