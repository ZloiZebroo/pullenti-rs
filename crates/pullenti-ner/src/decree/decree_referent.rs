/// DecreeReferent — a normative act (law, order, standard, etc.).
/// Mirrors Pullenti C# `DecreeReferent`.

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME:   &str = "DECREE";
pub const ATTR_TYPE:      &str = "TYPE";      // type string (закон, приказ, ГОСТ...)
pub const ATTR_NAME:      &str = "NAME";      // name/title of the document
pub const ATTR_NUMBER:    &str = "NUMBER";    // registration number
pub const ATTR_DATE:      &str = "DATE";      // adoption date
pub const ATTR_SOURCE:    &str = "SOURCE";    // publishing authority
pub const ATTR_OWNER:     &str = "OWNER";     // issuing authority
pub const ATTR_KIND:      &str = "KIND";      // DecreeKind as string

/// Broad category of the normative act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecreeKind {
    Undefined,
    Kodex,       // кодекс
    Ustav,       // устав, конституция
    Law,         // закон
    Order,       // приказ, указ, постановление, распоряжение, директива
    Konvention,  // конвенция, пакт
    Contract,    // договор, контракт, соглашение
    Project,     // проект
    Program,     // программа
    Standard,    // ГОСТ, ТУ, ISO, ОСТ
    Classifier,  // классификатор
    License,     // лицензия
    Tz,          // техническое задание
}

impl DecreeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DecreeKind::Undefined   => "Undefined",
            DecreeKind::Kodex       => "Kodex",
            DecreeKind::Ustav       => "Ustav",
            DecreeKind::Law         => "Law",
            DecreeKind::Order       => "Order",
            DecreeKind::Konvention  => "Konvention",
            DecreeKind::Contract    => "Contract",
            DecreeKind::Project     => "Project",
            DecreeKind::Program     => "Program",
            DecreeKind::Standard    => "Standard",
            DecreeKind::Classifier  => "Classifier",
            DecreeKind::License     => "License",
            DecreeKind::Tz          => "Tz",
        }
    }
}

pub fn new_decree_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

pub fn add_slot_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

pub fn set_kind(r: &mut Referent, kind: &DecreeKind) {
    r.slots.retain(|s| s.type_name != ATTR_KIND);
    r.slots.push(Slot::new(ATTR_KIND, Some(SlotValue::Str(kind.as_str().to_string()))));
}

pub fn get_decree_type(r: &Referent)   -> Option<String> { get_str(r, ATTR_TYPE) }
pub fn get_decree_name(r: &Referent)   -> Option<String> { get_str(r, ATTR_NAME) }
pub fn get_decree_number(r: &Referent) -> Option<String> { get_str(r, ATTR_NUMBER) }
pub fn get_decree_kind(r: &Referent)   -> Option<String> { get_str(r, ATTR_KIND) }
