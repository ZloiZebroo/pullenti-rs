use std::rc::Rc;
use std::cell::RefCell;
use crate::referent::{Referent, SlotValue};

pub const OBJ_TYPENAME: &str = "URI";
pub const ATTR_VALUE: &str = "VALUE";
pub const ATTR_DETAIL: &str = "DETAIL";
pub const ATTR_SCHEME: &str = "SCHEME";

/// Create a new URI referent with the given scheme and value
pub fn new_uri(scheme: &str, value: &str) -> Rc<RefCell<Referent>> {
    let mut r = Referent::new(OBJ_TYPENAME);
    if !scheme.is_empty() {
        r.add_slot(ATTR_SCHEME, SlotValue::Str(scheme.to_string()), true);
    }
    r.add_slot(ATTR_VALUE, SlotValue::Str(value.to_string()), true);
    Rc::new(RefCell::new(r))
}

pub fn get_value(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_VALUE)
}

pub fn get_scheme(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_SCHEME)
}

pub fn set_detail(r: &mut Referent, detail: &str) {
    r.add_slot(ATTR_DETAIL, SlotValue::Str(detail.to_string()), true);
}

pub fn can_be_equals(a: &Referent, b: &Referent) -> bool {
    if a.type_name != OBJ_TYPENAME || b.type_name != OBJ_TYPENAME { return false; }
    let av = get_value(a).unwrap_or("");
    let bv = get_value(b).unwrap_or("");
    av.eq_ignore_ascii_case(bv)
}
