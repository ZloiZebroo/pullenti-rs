pub mod link_referent;
pub mod link_analyzer;

pub use link_analyzer::LinkAnalyzer;
pub use link_referent::{
    OBJ_TYPENAME,
    ATTR_TYPE, ATTR_PARAM, ATTR_OBJECT1, ATTR_OBJECT2, ATTR_DATEFROM, ATTR_DATETO,
    LinkType,
    new_link_referent,
    get_link_type, set_link_type,
    get_param, set_param,
    get_object1, set_object1,
    get_object2, set_object2,
    set_datefrom, set_dateto,
};
