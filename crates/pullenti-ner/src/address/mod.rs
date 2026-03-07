pub mod address_referent;
pub mod street_table;
pub mod address_analyzer;

pub use address_analyzer::AddressAnalyzer;
pub use address_referent::{
    STREET_TYPENAME, STREET_ATTR_TYPE, STREET_ATTR_NAME, STREET_ATTR_NUMBER,
    ADDRESS_TYPENAME, ADDRESS_ATTR_STREET, ADDRESS_ATTR_HOUSE, ADDRESS_ATTR_FLAT,
    ADDRESS_ATTR_CORPUS, ADDRESS_ATTR_FLOOR, ADDRESS_ATTR_OFFICE, ADDRESS_ATTR_POST,
    new_street_referent, new_address_referent,
    get_street_type, get_street_name, get_street_number,
    get_house, get_flat, get_corpus, get_floor, get_office, get_post,
    add_slot_str, set_slot_str, get_slot_str,
};
