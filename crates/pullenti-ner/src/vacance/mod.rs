pub mod vacance_analyzer;
pub mod vacance_referent;
pub mod vacance_token;

pub use vacance_analyzer::VacanceAnalyzer;
pub use vacance_referent::{
    get_item_type, get_value, is_expired, new_vacancy_referent, set_expired, set_item_type,
    set_value, VacanceItemType, ATTR_EXPIRED, ATTR_REF, ATTR_TYPE, ATTR_VALUE,
    OBJ_TYPENAME as VACANCE_OBJ_TYPENAME,
};
