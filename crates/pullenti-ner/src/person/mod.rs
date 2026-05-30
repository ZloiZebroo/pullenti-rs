pub mod person_analyzer;
pub mod person_attr_table;
pub mod person_id_token;
pub mod person_identity_referent;
pub mod person_item_token;
pub mod person_normal_data;
pub mod person_normal_node;
pub mod person_normal_result;
pub mod person_property_referent;
pub mod person_referent;
pub mod short_name_helper;

pub use person_analyzer::PersonAnalyzer;
pub use person_normal_data::{analyze as analyze_person_name, PersonNormalData};
pub use person_normal_result::PersonNormalResult;
pub use person_property_referent::{
    get_name as get_person_property_name, new_person_property_referent,
    ATTR_NAME as PERSONPROPERTY_ATTR_NAME, OBJ_TYPENAME as PERSONPROPERTY_OBJ_TYPENAME,
};
pub use person_referent::{
    get_firstname, get_lastname, get_middlename, get_sex, new_person_referent, set_firstname,
    set_lastname, set_middlename, set_sex, to_string_short, ATTR_FIRSTNAME, ATTR_LASTNAME,
    ATTR_MIDDLENAME, ATTR_SEX, OBJ_TYPENAME as PERSON_OBJ_TYPENAME, SEX_FEMALE, SEX_MALE,
};
