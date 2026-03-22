use crate::referent::{Referent, Slot, SlotValue};
use std::rc::Rc;
use std::cell::RefCell;

pub const OBJ_TYPENAME: &str = "PERSONPROPERTY";
pub const ATTR_NAME:    &str = "NAME";
pub const ATTR_ATTR:    &str = "ATTR";
pub const ATTR_REF:     &str = "REF";
pub const ATTR_HIGHER:  &str = "HIGHER";

pub fn new_person_property_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_name(r: &Referent) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == ATTR_NAME)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

pub fn set_name(r: &mut Referent, name: &str) {
    r.slots.retain(|s| s.type_name != ATTR_NAME);
    r.slots.push(Slot::new(ATTR_NAME, Some(SlotValue::Str(name.to_string()))));
}

pub fn add_attr(r: &mut Referent, attr: &str) {
    r.slots.push(Slot::new(ATTR_ATTR, Some(SlotValue::Str(attr.to_string()))));
}

pub fn set_ref(r: &mut Referent, referent: Rc<RefCell<Referent>>) {
    r.slots.push(Slot::new(ATTR_REF, Some(SlotValue::Referent(referent))));
}

pub fn get_ref(r: &Referent) -> Option<Rc<RefCell<Referent>>> {
    r.slots.iter()
        .find(|s| s.type_name == ATTR_REF)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Referent(r) => Some(r.clone()), _ => None })
}
