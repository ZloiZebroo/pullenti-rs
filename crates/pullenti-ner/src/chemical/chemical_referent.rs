/// ChemicalFormulaReferent — mirrors `ChemicalFormulaReferent.cs`.

use crate::referent::{Referent, SlotValue};

pub const OBJ_TYPENAME: &str = "CHEMICALFORMULA";
pub const ATTR_VALUE: &str = "VALUE";
pub const ATTR_NAME: &str = "NAME";

pub fn new_chemical_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Get the formula value (e.g. "H2O")
pub fn get_value(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_VALUE).map(|s| s.to_string())
}

/// Get first textual name (e.g. "вода")
pub fn get_name(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_NAME).map(|s| s.to_string())
}

pub fn set_value(r: &mut Referent, v: &str) {
    r.add_slot(ATTR_VALUE, SlotValue::Str(v.to_string()), true);
}

pub fn add_name(r: &mut Referent, name: &str) {
    r.add_slot(ATTR_NAME, SlotValue::Str(name.to_string()), false);
}
