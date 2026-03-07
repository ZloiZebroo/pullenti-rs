/// NamedEntityReferent — "type" + "name" entity (planets, rivers, monuments, buildings, art).
use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME: &str = "NAMEDENTITY";
pub const ATTR_NAME: &str = "NAME";
pub const ATTR_KIND: &str = "KIND";
pub const ATTR_TYPE: &str = "TYPE";

pub fn new_named_referent() -> Referent {
    Referent {
        type_name: OBJ_TYPENAME.to_string(),
        slots: Vec::new(),
        occurrence: Vec::new(),
        data: Box::new(()),
    }
}

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter().find(|s| s.type_name == attr)
        .and_then(|s| if let Some(SlotValue::Str(v)) = &s.value { Some(v.clone()) } else { None })
}

pub fn add_name(r: &mut Referent, name: &str) {
    r.slots.push(Slot {
        type_name: ATTR_NAME.to_string(),
        value: Some(SlotValue::Str(name.to_string())),
        count: 1,
        occurrence: Vec::new(),
    });
}

pub fn set_kind(r: &mut Referent, kind: &str) {
    if let Some(s) = r.slots.iter_mut().find(|s| s.type_name == ATTR_KIND) {
        s.value = Some(SlotValue::Str(kind.to_string()));
    } else {
        r.slots.push(Slot {
            type_name: ATTR_KIND.to_string(),
            value: Some(SlotValue::Str(kind.to_string())),
            count: 1,
            occurrence: Vec::new(),
        });
    }
}

pub fn set_type(r: &mut Referent, typ: &str) {
    if let Some(s) = r.slots.iter_mut().find(|s| s.type_name == ATTR_TYPE) {
        s.value = Some(SlotValue::Str(typ.to_string()));
    } else {
        r.slots.push(Slot {
            type_name: ATTR_TYPE.to_string(),
            value: Some(SlotValue::Str(typ.to_string())),
            count: 1,
            occurrence: Vec::new(),
        });
    }
}

pub fn get_name(r: &Referent) -> Option<String> { get_str(r, ATTR_NAME) }
pub fn get_kind(r: &Referent) -> Option<String> { get_str(r, ATTR_KIND) }
pub fn get_type(r: &Referent) -> Option<String> { get_str(r, ATTR_TYPE) }

pub fn get_names(r: &Referent) -> Vec<String> {
    r.slots.iter()
        .filter(|s| s.type_name == ATTR_NAME)
        .filter_map(|s| if let Some(SlotValue::Str(v)) = &s.value { Some(v.clone()) } else { None })
        .collect()
}
