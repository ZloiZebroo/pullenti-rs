use crate::referent::{Referent, Slot, SlotValue};
use crate::date::date_pointer_type::DatePointerType;

pub const OBJ_TYPENAME: &str = "DATE";
pub const ATTR_CENTURY:   &str = "CENTURY";
pub const ATTR_DECADE:    &str = "DECADE";
pub const ATTR_YEAR:      &str = "YEAR";
pub const ATTR_HALFYEAR:  &str = "HALFYEAR";
pub const ATTR_QUARTAL:   &str = "QUARTAL";
pub const ATTR_SEASON:    &str = "SEASON";
pub const ATTR_MONTH:     &str = "MONTH";
pub const ATTR_WEEK:      &str = "WEEK";
pub const ATTR_DAY:       &str = "DAY";
pub const ATTR_DAYOFWEEK: &str = "DAYOFWEEK";
pub const ATTR_HOUR:      &str = "HOUR";
pub const ATTR_MINUTE:    &str = "MINUTE";
pub const ATTR_SECOND:    &str = "SECOND";
pub const ATTR_HIGHER:    &str = "HIGHER";
pub const ATTR_POINTER:   &str = "POINTER";
pub const ATTR_NEWSTYLE:  &str = "NEWSTYLE";
pub const ATTR_ISRELATIVE:&str = "ISRELATIVE";

/// Create a new DATE referent
pub fn new_date_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Get integer slot value (returns default if absent or unparseable)
fn get_int(r: &Referent, attr: &str, default: i32) -> i32 {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => s.parse().ok(), _ => None })
        .unwrap_or(default)
}

/// Set integer slot value (replaces existing)
fn set_int(r: &mut Referent, attr: &str, v: i32) {
    r.slots.retain(|s| s.type_name != attr);
    if v != 0 {
        r.slots.push(Slot::new(attr, Some(SlotValue::Str(v.to_string()))));
    }
}

pub fn get_year(r: &Referent) -> i32    { get_int(r, ATTR_YEAR, 0) }
pub fn get_month(r: &Referent) -> i32   { get_int(r, ATTR_MONTH, 0) }
pub fn get_day(r: &Referent) -> i32     { get_int(r, ATTR_DAY, 0) }
pub fn get_hour(r: &Referent) -> i32    { get_int(r, ATTR_HOUR, -1) }
pub fn get_minute(r: &Referent) -> i32  { get_int(r, ATTR_MINUTE, -1) }
pub fn get_second(r: &Referent) -> i32  { get_int(r, ATTR_SECOND, -1) }
pub fn get_century(r: &Referent) -> i32 { get_int(r, ATTR_CENTURY, 0) }
pub fn get_quartal(r: &Referent) -> i32 { get_int(r, ATTR_QUARTAL, 0) }
pub fn get_halfyear(r: &Referent) -> i32{ get_int(r, ATTR_HALFYEAR, 0) }

pub fn set_year(r: &mut Referent, v: i32)    { set_int(r, ATTR_YEAR, v) }
pub fn set_month(r: &mut Referent, v: i32)   { set_int(r, ATTR_MONTH, v) }
pub fn set_day(r: &mut Referent, v: i32)     { set_int(r, ATTR_DAY, v) }
pub fn set_hour(r: &mut Referent, v: i32)    {
    r.slots.retain(|s| s.type_name != ATTR_HOUR);
    r.slots.push(Slot::new(ATTR_HOUR, Some(SlotValue::Str(v.to_string()))));
}
pub fn set_minute(r: &mut Referent, v: i32)  {
    r.slots.retain(|s| s.type_name != ATTR_MINUTE);
    r.slots.push(Slot::new(ATTR_MINUTE, Some(SlotValue::Str(v.to_string()))));
}
pub fn set_second(r: &mut Referent, v: i32)  {
    r.slots.retain(|s| s.type_name != ATTR_SECOND);
    r.slots.push(Slot::new(ATTR_SECOND, Some(SlotValue::Str(v.to_string()))));
}
pub fn set_century(r: &mut Referent, v: i32) { set_int(r, ATTR_CENTURY, v) }
pub fn set_quartal(r: &mut Referent, v: i32) { set_int(r, ATTR_QUARTAL, v) }
pub fn set_halfyear(r: &mut Referent, v: i32){ set_int(r, ATTR_HALFYEAR, v) }

pub fn get_pointer(r: &Referent) -> DatePointerType {
    match r.get_string_value(ATTR_POINTER) {
        None | Some("No") => DatePointerType::No,
        Some(s) => match s {
            "Begin"     => DatePointerType::Begin,
            "Center"    => DatePointerType::Center,
            "End"       => DatePointerType::End,
            "Today"     => DatePointerType::Today,
            "Winter"    => DatePointerType::Winter,
            "Spring"    => DatePointerType::Spring,
            "Summer"    => DatePointerType::Summer,
            "Autumn"    => DatePointerType::Autumn,
            "About"     => DatePointerType::About,
            "Undefined" => DatePointerType::Undefined,
            _ => DatePointerType::No,
        }
    }
}

pub fn set_pointer(r: &mut Referent, p: DatePointerType) {
    if p != DatePointerType::No {
        r.slots.retain(|s| s.type_name != ATTR_POINTER);
        r.slots.push(Slot::new(ATTR_POINTER, Some(SlotValue::Str(p.to_string()))));
    }
}

/// Store a nested "higher" date referent in this referent's HIGHER slot
pub fn set_higher_ref(r: &mut Referent, higher: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.retain(|s| s.type_name != ATTR_HIGHER);
    r.slots.push(Slot::new(ATTR_HIGHER, Some(SlotValue::Referent(higher))));
}

/// Get the HIGHER referent if present
pub fn get_higher_ref(r: &Referent) -> Option<std::rc::Rc<std::cell::RefCell<Referent>>> {
    r.slots.iter()
        .find(|s| s.type_name == ATTR_HIGHER)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v {
            SlotValue::Referent(r) => Some(r.clone()),
            _ => None,
        })
}
