pub mod person_referent;
pub mod person_analyzer;

pub use person_analyzer::PersonAnalyzer;
pub use person_referent::{
    OBJ_TYPENAME as PERSON_OBJ_TYPENAME,
    ATTR_FIRSTNAME, ATTR_MIDDLENAME, ATTR_LASTNAME, ATTR_SEX,
    SEX_MALE, SEX_FEMALE,
    get_firstname, get_middlename, get_lastname, get_sex,
    set_firstname, set_middlename, set_lastname, set_sex,
    new_person_referent, to_string_short,
};
