/// Bank data referent — ports BankDataReferent.cs.

use std::rc::Rc;
use std::cell::RefCell;

use crate::referent::{Referent, SlotValue};
use crate::uri::uri_referent as ur;

pub const OBJ_TYPENAME: &str = "BANKDATA";

/// Attribute: a bank requisite item (URI referent — Р/С, ИНН, БИК, …)
pub const ATTR_ITEM: &str = "ITEM";
/// Attribute: the bank organisation (ORG referent)
pub const ATTR_BANK: &str = "BANK";
/// Attribute: the correspondent bank (ORG referent for К/С)
pub const ATTR_CORBANK: &str = "CORBANK";

pub fn new_bank_data_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn add_item(r: &mut Referent, uri: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_ITEM, SlotValue::Referent(uri), false);
}

pub fn set_bank(r: &mut Referent, org: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_BANK, SlotValue::Referent(org), false);
}

pub fn set_corbank(r: &mut Referent, org: Rc<RefCell<Referent>>) {
    r.add_slot(ATTR_CORBANK, SlotValue::Referent(org), false);
}

/// Return true if the scheme is a recognised bank requisite scheme.
pub fn is_bank_req_scheme(scheme: &str) -> bool {
    matches!(scheme,
        "Р/С" | "К/С" | "Л/С" |
        "ОКФС" | "ОКАТО" | "ОГРН" | "БИК" | "SWIFT" |
        "ОКПО" | "ОКВЭД" | "КБК" | "ИНН" | "КПП"
    )
}

/// Find the value (owned) of the URI item with the given scheme.
pub fn find_value_owned(r: &Referent, scheme: &str) -> Option<String> {
    for slot in &r.slots {
        if slot.type_name == ATTR_ITEM {
            if let Some(SlotValue::Referent(ref uri_rc)) = slot.value {
                let uri = uri_rc.borrow();
                if ur::get_scheme(&uri) == Some(scheme) {
                    return ur::get_value(&uri).map(|s| s.to_string());
                }
            }
        }
    }
    None
}
