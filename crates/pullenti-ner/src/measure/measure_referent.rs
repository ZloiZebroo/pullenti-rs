use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME: &str = "MEASURE";
pub const ATTR_VALUE:    &str = "VALUE";
pub const ATTR_UNIT:     &str = "UNIT";
pub const ATTR_KIND:     &str = "KIND";

pub fn new_measure_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

fn set_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.retain(|s| s.type_name != attr);
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

/// Add a VALUE slot (multiple values allowed for ranges)
pub fn add_value(r: &mut Referent, v: &str) {
    r.slots.push(Slot::new(ATTR_VALUE, Some(SlotValue::Str(v.to_string()))));
}

pub fn set_unit(r: &mut Referent, unit: &str) { set_str(r, ATTR_UNIT, unit); }
pub fn set_kind(r: &mut Referent, kind: &str) { set_str(r, ATTR_KIND, kind); }

pub fn get_value(r: &Referent) -> Option<String> { get_str(r, ATTR_VALUE) }
pub fn get_unit(r: &Referent) -> Option<String>  { get_str(r, ATTR_UNIT) }
pub fn get_kind(r: &Referent) -> Option<String>  { get_str(r, ATTR_KIND) }
