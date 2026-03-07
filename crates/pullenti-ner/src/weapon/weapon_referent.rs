/// Weapon referent — ports WeaponReferent.cs.

use crate::referent::Referent;

pub const OBJ_TYPENAME: &str = "WEAPON";

pub const ATTR_TYPE:    &str = "TYPE";
pub const ATTR_BRAND:   &str = "BRAND";
pub const ATTR_MODEL:   &str = "MODEL";
pub const ATTR_NAME:    &str = "NAME";
pub const ATTR_NUMBER:  &str = "NUMBER";
pub const ATTR_DATE:    &str = "DATE";
pub const ATTR_REF:     &str = "REF";
pub const ATTR_CALIBER: &str = "CALIBER";

pub fn new_weapon_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_type(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_TYPE)
}

pub fn get_brand(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_BRAND)
}

pub fn get_model(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_MODEL)
}

pub fn get_name(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_NAME)
}

pub fn get_number(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_NUMBER)
}
