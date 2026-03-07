use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME:   &str = "DATERANGE";
pub const ATTR_DATE_FROM: &str = "FROM";
pub const ATTR_DATE_TO:   &str = "TO";

/// Create a new DATERANGE referent
pub fn new_date_range_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Get the FROM date referent
pub fn get_date_from(r: &Referent) -> Option<std::rc::Rc<std::cell::RefCell<Referent>>> {
    r.slots.iter()
        .find(|s| s.type_name == ATTR_DATE_FROM)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v {
            SlotValue::Referent(r) => Some(r.clone()),
            _ => None,
        })
}

/// Get the TO date referent
pub fn get_date_to(r: &Referent) -> Option<std::rc::Rc<std::cell::RefCell<Referent>>> {
    r.slots.iter()
        .find(|s| s.type_name == ATTR_DATE_TO)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v {
            SlotValue::Referent(r) => Some(r.clone()),
            _ => None,
        })
}

/// Set the FROM date referent
pub fn set_date_from(r: &mut Referent, from: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.retain(|s| s.type_name != ATTR_DATE_FROM);
    r.slots.push(Slot::new(ATTR_DATE_FROM, Some(SlotValue::Referent(from))));
}

/// Set the TO date referent
pub fn set_date_to(r: &mut Referent, to: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.retain(|s| s.type_name != ATTR_DATE_TO);
    r.slots.push(Slot::new(ATTR_DATE_TO, Some(SlotValue::Referent(to))));
}
