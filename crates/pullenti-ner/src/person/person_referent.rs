/// PersonReferent — a person entity.
/// Mirrors Pullenti C# `PersonReferent`.

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME:   &str = "PERSON";
pub const ATTR_FIRSTNAME: &str = "FIRSTNAME";
pub const ATTR_MIDDLENAME:&str = "MIDDLENAME";  // patronymic
pub const ATTR_LASTNAME:  &str = "LASTNAME";
pub const ATTR_SEX:       &str = "SEX";
pub const ATTR_NICKNAME:  &str = "NICKNAME";
pub const ATTR_UNDEFNAME: &str = "UNDEFNAME";
pub const ATTR_IDENTITY:  &str = "IDENTITY";
pub const ATTR_ATTR:      &str = "ATTRIBUTE";  // PersonPropertyReferent
pub const ATTR_IDDOC:     &str = "IDDOC";      // PersonIdentityReferent link

// Sex values
pub const SEX_MALE:   &str = "Male";
pub const SEX_FEMALE: &str = "Female";

pub fn new_person_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

fn set_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.retain(|s| s.type_name != attr);
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

// ── Setters ───────────────────────────────────────────────────────────────────

pub fn set_firstname(r: &mut Referent, v: &str)  { set_str(r, ATTR_FIRSTNAME,  v); }
pub fn set_middlename(r: &mut Referent, v: &str) { set_str(r, ATTR_MIDDLENAME, v); }
pub fn set_lastname(r: &mut Referent, v: &str)   { set_str(r, ATTR_LASTNAME,   v); }
pub fn set_sex(r: &mut Referent, v: &str)         { set_str(r, ATTR_SEX,        v); }
pub fn set_undefname(r: &mut Referent, v: &str)   { set_str(r, ATTR_UNDEFNAME,  v); }

// ── Getters ───────────────────────────────────────────────────────────────────

pub fn get_firstname(r: &Referent)  -> Option<String> { get_str(r, ATTR_FIRSTNAME) }
pub fn get_middlename(r: &Referent) -> Option<String> { get_str(r, ATTR_MIDDLENAME) }
pub fn get_lastname(r: &Referent)   -> Option<String> { get_str(r, ATTR_LASTNAME) }
pub fn get_sex(r: &Referent)        -> Option<String> { get_str(r, ATTR_SEX) }

/// Returns the short display form: "Иванов И.И." or "Иван Иванович" etc.
pub fn to_string_short(r: &Referent) -> String {
    let last  = get_lastname(r);
    let first = get_firstname(r);
    let mid   = get_middlename(r);

    let mut parts: Vec<String> = Vec::new();
    if let Some(l) = last  { parts.push(capitalize(&l)); }
    if let Some(f) = first { parts.push(capitalize(&f)); }
    if let Some(m) = mid   { parts.push(capitalize(&m)); }
    if parts.is_empty() {
        if let Some(u) = get_str(r, ATTR_UNDEFNAME) { parts.push(capitalize(&u)); }
    }
    parts.join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}
