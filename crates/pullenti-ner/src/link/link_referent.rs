use std::rc::Rc;
use std::cell::RefCell;
use crate::referent::{Referent, SlotValue};

// ── Type name & slot names ─────────────────────────────────────────────────

pub const OBJ_TYPENAME: &str = "LINK";

pub const ATTR_TYPE:     &str = "TYPE";
pub const ATTR_PARAM:    &str = "PARAM";
pub const ATTR_OBJECT1:  &str = "OBJECT1";
pub const ATTR_OBJECT2:  &str = "OBJECT2";
pub const ATTR_DATEFROM: &str = "DATEFROM";
pub const ATTR_DATETO:   &str = "DATETO";

// ── LinkType enum ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    Undefined,
    Born,
    Family,
    Study,
    Work,
    Contact,
    Document,
    Address,
    Unit,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Undefined => "undefined",
            LinkType::Born      => "born",
            LinkType::Family    => "family",
            LinkType::Study     => "study",
            LinkType::Work      => "work",
            LinkType::Contact   => "contact",
            LinkType::Document  => "document",
            LinkType::Address   => "address",
            LinkType::Unit      => "unit",
        }
    }

    pub fn from_str(s: &str) -> LinkType {
        match s.to_lowercase().as_str() {
            "born"     => LinkType::Born,
            "family"   => LinkType::Family,
            "study"    => LinkType::Study,
            "work"     => LinkType::Work,
            "contact"  => LinkType::Contact,
            "document" => LinkType::Document,
            "address"  => LinkType::Address,
            "unit"     => LinkType::Unit,
            _          => LinkType::Undefined,
        }
    }
}

// ── Referent constructors ──────────────────────────────────────────────────

pub fn new_link_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Slot accessors ─────────────────────────────────────────────────────────

pub fn get_link_type(r: &Referent) -> LinkType {
    r.get_string_value(ATTR_TYPE)
        .map(|s| LinkType::from_str(s))
        .unwrap_or(LinkType::Undefined)
}

pub fn set_link_type(r: &mut Referent, typ: &LinkType) {
    if *typ != LinkType::Undefined {
        r.add_slot(ATTR_TYPE, SlotValue::Str(typ.as_str().to_string()), true);
    }
}

pub fn get_param(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_PARAM)
}

pub fn set_param(r: &mut Referent, param: &str) {
    r.add_slot(ATTR_PARAM, SlotValue::Str(param.to_string()), true);
}

pub fn get_object1(r: &Referent) -> Option<Rc<RefCell<Referent>>> {
    r.slots.iter().find(|s| s.type_name == ATTR_OBJECT1)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| v.as_referent())
}

pub fn set_object1(r: &mut Referent, obj: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_OBJECT1, SlotValue::Referent(obj), true);
}

pub fn get_object2(r: &Referent) -> Option<Rc<RefCell<Referent>>> {
    r.slots.iter().find(|s| s.type_name == ATTR_OBJECT2)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| v.as_referent())
}

pub fn set_object2(r: &mut Referent, obj: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_OBJECT2, SlotValue::Referent(obj), true);
}

pub fn set_datefrom(r: &mut Referent, date: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_DATEFROM, SlotValue::Referent(date), true);
}

pub fn set_dateto(r: &mut Referent, date: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_DATETO, SlotValue::Referent(date), true);
}
