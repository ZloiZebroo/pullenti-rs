/// TitlePageReferent — entity describing information from title pages of articles,
/// books, dissertations, etc.
/// Mirrors `TitlePageReferent.cs`.

use crate::referent::{Referent, SlotValue};

// ── Constants ──────────────────────────────────────────────────────────────

pub const OBJ_TYPENAME:   &str = "TITLEPAGE";
pub const ATTR_NAME:       &str = "NAME";
pub const ATTR_TYPE:       &str = "TYPE";
pub const ATTR_AUTHOR:     &str = "AUTHOR";
pub const ATTR_SUPERVISOR: &str = "SUPERVISOR";
pub const ATTR_EDITOR:     &str = "EDITOR";
pub const ATTR_CONSULTANT: &str = "CONSULTANT";
pub const ATTR_OPPONENT:   &str = "OPPONENT";
pub const ATTR_TRANSLATOR: &str = "TRANSLATOR";
pub const ATTR_AFFIRMANT:  &str = "AFFIRMANT";
pub const ATTR_ORG:        &str = "ORGANIZATION";
pub const ATTR_STUDENTYEAR:&str = "STUDENTYEAR";
pub const ATTR_DATE:       &str = "DATE";
pub const ATTR_CITY:       &str = "CITY";
pub const ATTR_SPECIALITY: &str = "SPECIALITY";
pub const ATTR_ATTR:       &str = "ATTR";

// ── Constructor ────────────────────────────────────────────────────────────

pub fn new_titlepage_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Accessors ──────────────────────────────────────────────────────────────

pub fn get_name(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_NAME)
}

pub fn get_title_type(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_TYPE)
}

pub fn get_speciality(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_SPECIALITY)
}

pub fn add_name(r: &mut Referent, value: &str) {
    if !value.is_empty() {
        r.add_slot(ATTR_NAME, SlotValue::Str(value.to_string()), false);
    }
}

pub fn add_title_type(r: &mut Referent, value: &str) {
    if !value.is_empty() {
        r.add_slot(ATTR_TYPE, SlotValue::Str(value.to_lowercase()), false);
    }
}

pub fn set_speciality(r: &mut Referent, value: &str) {
    if !value.is_empty() {
        r.add_slot(ATTR_SPECIALITY, SlotValue::Str(value.to_string()), true);
    }
}
