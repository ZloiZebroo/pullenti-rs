use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME: &str = "MONEY";
pub const ATTR_CURRENCY: &str = "CURRENCY";
pub const ATTR_VALUE:    &str = "VALUE";
pub const ATTR_REST:     &str = "REST";
pub const ATTR_ALTVALUE: &str = "ALTVALUE";
pub const ATTR_ALTREST:  &str = "ALTREST";

pub fn new_money_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

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

pub fn get_currency(r: &Referent) -> Option<String> { get_str(r, ATTR_CURRENCY) }
pub fn get_value(r: &Referent) -> Option<String>    { get_str(r, ATTR_VALUE) }
pub fn get_rest(r: &Referent) -> i32 {
    get_str(r, ATTR_REST).and_then(|s| s.parse().ok()).unwrap_or(0)
}

pub fn set_currency(r: &mut Referent, v: &str) { set_str(r, ATTR_CURRENCY, v); }
pub fn set_value(r: &mut Referent, v: &str)    { set_str(r, ATTR_VALUE, v); }
pub fn set_rest(r: &mut Referent, v: i32) {
    if v != 0 { set_str(r, ATTR_REST, &v.to_string()); }
}

/// Numeric value of the integer part
pub fn value_f64(r: &Referent) -> f64 {
    get_str(r, ATTR_VALUE).and_then(|s| s.parse().ok()).unwrap_or(0.0)
}
/// Numeric value including kopecks
pub fn real_value(r: &Referent) -> f64 {
    value_f64(r) + (get_rest(r) as f64) / 100.0
}
