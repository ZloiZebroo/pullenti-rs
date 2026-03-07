use std::rc::Rc;
use std::cell::RefCell;
use crate::referent::{Referent, SlotValue};
use super::phone_kind::PhoneKind;

pub const OBJ_TYPENAME: &str = "PHONE";
pub const ATTR_NUMBER: &str = "NUMBER";
pub const ATTR_KIND: &str = "KIND";
pub const ATTR_COUNTRYCODE: &str = "COUNTRYCODE";
pub const ATTR_ADDNUMBER: &str = "ADDNUMBER";

/// Extra data stored in Referent::data for PHONE referents
#[derive(Debug)]
pub struct PhoneExtra {
    /// Phone number pattern template (e.g. "3-2-2")
    pub template: Option<String>,
    /// Secondary kind tag (used for fax/phone split)
    pub tag: Option<PhoneKind>,
}

impl PhoneExtra {
    pub fn new() -> Self {
        PhoneExtra { template: None, tag: None }
    }
}

/// Create a new PHONE referent
pub fn new_phone_referent() -> Rc<RefCell<Referent>> {
    let r = Referent::new_with_data(OBJ_TYPENAME, PhoneExtra::new());
    Rc::new(RefCell::new(r))
}

/// Get the main number (without country code)
pub fn get_number(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_NUMBER).map(|s| s.to_string())
}

/// Set the main number
pub fn set_number(r: &mut Referent, num: &str) {
    r.add_slot(ATTR_NUMBER, SlotValue::Str(num.to_string()), true);
}

/// Get country code
pub fn get_country_code(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_COUNTRYCODE).map(|s| s.to_string())
}

/// Set country code
pub fn set_country_code(r: &mut Referent, cc: &str) {
    r.add_slot(ATTR_COUNTRYCODE, SlotValue::Str(cc.to_string()), true);
}

/// Get additional (extension) number
pub fn get_add_number(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_ADDNUMBER).map(|s| s.to_string())
}

/// Set additional number
pub fn set_add_number(r: &mut Referent, num: &str) {
    r.add_slot(ATTR_ADDNUMBER, SlotValue::Str(num.to_string()), true);
}

/// Get phone kind
pub fn get_kind(r: &Referent) -> PhoneKind {
    r.get_string_value(ATTR_KIND)
        .and_then(|s| match s {
            "home" => Some(PhoneKind::Home),
            "mobile" => Some(PhoneKind::Mobile),
            "work" => Some(PhoneKind::Work),
            "fax" => Some(PhoneKind::Fax),
            _ => None,
        })
        .unwrap_or(PhoneKind::Undefined)
}

/// Set phone kind
pub fn set_kind(r: &mut Referent, kind: PhoneKind) {
    if kind != PhoneKind::Undefined {
        r.add_slot(ATTR_KIND, SlotValue::Str(kind.to_string()), true);
    }
}

/// Get template from extra data
pub fn get_template(r: &Referent) -> Option<&str> {
    r.data_as::<PhoneExtra>().and_then(|e| e.template.as_deref())
}

/// Set template in extra data
pub fn set_template(r: &mut Referent, tmpl: &str) {
    if let Some(extra) = r.data_as_mut::<PhoneExtra>() {
        extra.template = Some(tmpl.to_string());
    }
}

/// Get tag (secondary PhoneKind) from extra data
pub fn get_tag(r: &Referent) -> PhoneKind {
    r.data_as::<PhoneExtra>()
        .and_then(|e| e.tag)
        .unwrap_or(PhoneKind::Undefined)
}

/// Set tag in extra data
pub fn set_tag(r: &mut Referent, kind: PhoneKind) {
    if let Some(extra) = r.data_as_mut::<PhoneExtra>() {
        extra.tag = Some(kind);
    }
}

/// Auto-correct phone kind based on attributes
pub fn correct(r: &mut Referent) {
    if get_kind(r) != PhoneKind::Undefined { return; }
    if r.find_slot(ATTR_ADDNUMBER, None).is_some() {
        set_kind(r, PhoneKind::Work);
    } else {
        let cc = get_country_code(r);
        if cc.is_none() || cc.as_deref() == Some("7") {
            if let Some(num) = get_number(r) {
                if num.len() == 10 && num.starts_with('9') {
                    set_kind(r, PhoneKind::Mobile);
                }
            }
        }
    }
}

/// Check if two phone referents can be equal (simplified)
pub fn can_be_equals(a: &Referent, b: &Referent) -> bool {
    if a.type_name != OBJ_TYPENAME || b.type_name != OBJ_TYPENAME { return false; }
    let a_cc = get_country_code(a);
    let b_cc = get_country_code(b);
    if let (Some(acc), Some(bcc)) = (&a_cc, &b_cc) {
        if acc != bcc { return false; }
    }
    let a_add = get_add_number(a);
    let b_add = get_add_number(b);
    if a_add != b_add { return false; }
    let a_num = get_number(a);
    let b_num = get_number(b);
    match (a_num, b_num) {
        (None, _) | (_, None) => false,
        (Some(an), Some(bn)) => {
            an == bn
                || an.ends_with(&bn)
                || bn.ends_with(&an)
        }
    }
}

/// Merge slots from `src` into `dst`
pub fn merge_slots(dst: &mut Referent, src: &Referent) {
    if get_country_code(dst).is_none() {
        if let Some(cc) = get_country_code(src) {
            set_country_code(dst, &cc);
        }
    }
    if let (Some(dst_num), Some(src_num)) = (get_number(dst), get_number(src)) {
        if src_num.ends_with(&dst_num) {
            set_number(dst, &src_num);
        }
    }
}

/// Format phone for display
pub fn to_string_ex(r: &Referent) -> String {
    let mut res = String::new();
    if let Some(cc) = get_country_code(r) {
        if cc != "8" { res.push('+'); }
        res.push_str(&cc);
        res.push(' ');
    }
    let mut num = get_number(r);
    if let Some(ref n) = num {
        if n.len() >= 9 {
            let cou = if n.len() >= 11 { n.len() - 7 } else { 3 };
            res.push('(');
            res.push_str(&n[..cou]);
            res.push_str(") ");
            num = Some(n[cou..].to_string());
        } else if n.len() == 8 {
            res.push('(');
            res.push_str(&n[..2]);
            res.push_str(") ");
            num = Some(n[2..].to_string());
        }
    }
    match num {
        None => res.push_str("???-??-??"),
        Some(n) => {
            if n.len() > 5 {
                let mut s = n.clone();
                let insert_pos1 = s.len() - 4;
                s.insert(insert_pos1, '-');
                let insert_pos2 = s.len() - 2;
                s.insert(insert_pos2, '-');
                res.push_str(&s);
            } else {
                res.push_str(&n);
            }
        }
    }
    if let Some(add) = get_add_number(r) {
        res.push_str(" (доб.");
        res.push_str(&add);
        res.push(')');
    }
    res
}
