/// address_referent.rs — Street and Address referents.
use crate::referent::{Referent, Slot, SlotValue};

// ── StreetReferent ────────────────────────────────────────────────────────────

pub const STREET_TYPENAME: &str = "STREET";
pub const STREET_ATTR_TYPE: &str = "TYP";
pub const STREET_ATTR_NAME: &str = "NAME";
pub const STREET_ATTR_NUMBER: &str = "NUMBER";

pub fn new_street_referent() -> Referent {
    Referent {
        type_name: STREET_TYPENAME.to_string(),
        slots: Vec::new(),
        occurrence: Vec::new(),
        data: Box::new(()),
    }
}

// ── AddressReferent ───────────────────────────────────────────────────────────

pub const ADDRESS_TYPENAME: &str = "ADDRESS";
pub const ADDRESS_ATTR_STREET: &str = "STREET";
pub const ADDRESS_ATTR_HOUSE:  &str = "HOUSE";
pub const ADDRESS_ATTR_FLAT:   &str = "FLAT";
pub const ADDRESS_ATTR_CORPUS: &str = "CORPUS";
pub const ADDRESS_ATTR_FLOOR:  &str = "FLOOR";
pub const ADDRESS_ATTR_OFFICE: &str = "OFFICE";
pub const ADDRESS_ATTR_POST:   &str = "ZIP";     // ZIP/postal code

pub fn new_address_referent() -> Referent {
    Referent {
        type_name: ADDRESS_TYPENAME.to_string(),
        slots: Vec::new(),
        occurrence: Vec::new(),
        data: Box::new(()),
    }
}

// ── Generic helpers ────────────────────────────────────────────────────────────

pub fn add_slot_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.push(Slot {
        type_name: attr.to_string(),
        value: Some(SlotValue::Str(val.to_string())),
        count: 1,
        occurrence: Vec::new(),
    });
}

pub fn set_slot_str(r: &mut Referent, attr: &str, val: &str) {
    if let Some(s) = r.slots.iter_mut().find(|s| s.type_name == attr) {
        s.value = Some(SlotValue::Str(val.to_string()));
    } else {
        add_slot_str(r, attr, val);
    }
}

pub fn get_slot_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter().find(|s| s.type_name == attr)
        .and_then(|s| if let Some(SlotValue::Str(v)) = &s.value { Some(v.clone()) } else { None })
}

pub fn get_street_type(r: &Referent) -> Option<String> { get_slot_str(r, STREET_ATTR_TYPE) }
pub fn get_street_name(r: &Referent) -> Option<String> { get_slot_str(r, STREET_ATTR_NAME) }
pub fn get_street_number(r: &Referent) -> Option<String> { get_slot_str(r, STREET_ATTR_NUMBER) }

pub fn get_house(r: &Referent) -> Option<String>  { get_slot_str(r, ADDRESS_ATTR_HOUSE) }
pub fn get_flat(r: &Referent) -> Option<String>   { get_slot_str(r, ADDRESS_ATTR_FLAT) }
pub fn get_corpus(r: &Referent) -> Option<String> { get_slot_str(r, ADDRESS_ATTR_CORPUS) }
pub fn get_floor(r: &Referent) -> Option<String>  { get_slot_str(r, ADDRESS_ATTR_FLOOR) }
pub fn get_office(r: &Referent) -> Option<String> { get_slot_str(r, ADDRESS_ATTR_OFFICE) }
pub fn get_post(r: &Referent) -> Option<String>   { get_slot_str(r, ADDRESS_ATTR_POST) }
