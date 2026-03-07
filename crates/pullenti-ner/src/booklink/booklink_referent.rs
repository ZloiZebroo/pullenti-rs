/// BookLink referent types — ported from BookLinkReferent.cs and BookLinkRefReferent.cs.
///
/// BOOKLINK — a bibliographic reference (book, article, etc.)
/// BOOKLINKREF — an in-text citation pointing to a BOOKLINK

use crate::referent::{Referent, Slot, SlotValue};

// ── BOOKLINK ──────────────────────────────────────────────────────────────────

pub const OBJ_TYPENAME: &str = "BOOKLINK";
pub const ATTR_AUTHOR: &str = "AUTHOR";
pub const ATTR_NAME: &str = "NAME";
pub const ATTR_YEAR: &str = "YEAR";
pub const ATTR_LANG: &str = "LANG";
pub const ATTR_GEO: &str = "GEO";
pub const ATTR_URL: &str = "URL";
pub const ATTR_MISC: &str = "MISC";
pub const ATTR_TYPE: &str = "TYPE";

pub fn new_booklink_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_name(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_NAME).map(|s| s.to_string())
}

pub fn get_year(r: &Referent) -> i32 {
    r.get_string_value(ATTR_YEAR)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

pub fn set_name(r: &mut Referent, name: &str) {
    r.slots.retain(|s| s.type_name != ATTR_NAME);
    r.slots.push(Slot::new(ATTR_NAME, Some(SlotValue::Str(name.to_string()))));
}

pub fn set_year(r: &mut Referent, year: i32) {
    r.slots.retain(|s| s.type_name != ATTR_YEAR);
    r.slots.push(Slot::new(ATTR_YEAR, Some(SlotValue::Str(year.to_string()))));
}

pub fn set_type(r: &mut Referent, typ: &str) {
    r.slots.retain(|s| s.type_name != ATTR_TYPE);
    r.slots.push(Slot::new(ATTR_TYPE, Some(SlotValue::Str(typ.to_string()))));
}

pub fn add_author_str(r: &mut Referent, author: &str) {
    r.slots.push(Slot::new(ATTR_AUTHOR, Some(SlotValue::Str(author.to_string()))));
}

pub fn add_author_ref(r: &mut Referent, author_ref: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.push(Slot::new(ATTR_AUTHOR, Some(SlotValue::Referent(author_ref))));
}

pub fn add_url_ref(r: &mut Referent, url_ref: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.push(Slot::new(ATTR_URL, Some(SlotValue::Referent(url_ref))));
}

// ── BOOKLINKREF ───────────────────────────────────────────────────────────────

pub const REF_OBJ_TYPENAME: &str = "BOOKLINKREF";
pub const REF_ATTR_BOOK: &str = "BOOK";
pub const REF_ATTR_TYPE: &str = "TYPE";
pub const REF_ATTR_PAGES: &str = "PAGES";
pub const REF_ATTR_NUMBER: &str = "NUMBER";
pub const REF_ATTR_MISC: &str = "MISC";

#[derive(Debug, Clone, PartialEq)]
pub enum BookLinkRefType {
    Undefined,
    Inline,
}

impl BookLinkRefType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BookLinkRefType::Undefined => "Undefined",
            BookLinkRefType::Inline => "Inline",
        }
    }
}

pub fn new_booklinkref_referent() -> Referent {
    Referent::new(REF_OBJ_TYPENAME)
}

pub fn set_ref_number(r: &mut Referent, num: &str) {
    r.slots.retain(|s| s.type_name != REF_ATTR_NUMBER);
    r.slots.push(Slot::new(REF_ATTR_NUMBER, Some(SlotValue::Str(num.to_string()))));
}

pub fn set_ref_pages(r: &mut Referent, pages: &str) {
    r.slots.retain(|s| s.type_name != REF_ATTR_PAGES);
    r.slots.push(Slot::new(REF_ATTR_PAGES, Some(SlotValue::Str(pages.to_string()))));
}

pub fn set_ref_book(r: &mut Referent, book: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.slots.retain(|s| s.type_name != REF_ATTR_BOOK);
    r.slots.push(Slot::new(REF_ATTR_BOOK, Some(SlotValue::Referent(book))));
}

pub fn set_ref_type(r: &mut Referent, typ: BookLinkRefType) {
    r.slots.retain(|s| s.type_name != REF_ATTR_TYPE);
    r.slots.push(Slot::new(REF_ATTR_TYPE, Some(SlotValue::Str(typ.as_str().to_string()))));
}

pub fn get_ref_number(r: &Referent) -> Option<String> {
    r.get_string_value(REF_ATTR_NUMBER).map(|s| s.to_string())
}

pub fn get_ref_pages(r: &Referent) -> Option<String> {
    r.get_string_value(REF_ATTR_PAGES).map(|s| s.to_string())
}
