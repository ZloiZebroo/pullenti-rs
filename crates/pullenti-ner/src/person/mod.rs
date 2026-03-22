pub mod person_referent;
pub mod person_analyzer;
pub mod person_property_referent;
pub mod person_attr_table;
pub mod person_identity_referent;
pub mod person_id_token;
pub mod person_normal_result;
pub mod short_name_helper;
pub mod person_item_token;
pub mod person_normal_node;
pub mod person_normal_data;

pub use person_analyzer::PersonAnalyzer;
pub use person_referent::{
    OBJ_TYPENAME as PERSON_OBJ_TYPENAME,
    ATTR_FIRSTNAME, ATTR_MIDDLENAME, ATTR_LASTNAME, ATTR_SEX,
    SEX_MALE, SEX_FEMALE,
    get_firstname, get_middlename, get_lastname, get_sex,
    set_firstname, set_middlename, set_lastname, set_sex,
    new_person_referent, to_string_short,
};
pub use person_property_referent::{
    OBJ_TYPENAME as PERSONPROPERTY_OBJ_TYPENAME,
    ATTR_NAME as PERSONPROPERTY_ATTR_NAME,
    get_name as get_person_property_name,
    new_person_property_referent,
};
pub use person_normal_result::PersonNormalResult;
pub use person_normal_data::{PersonNormalData, analyze as analyze_person_name};
