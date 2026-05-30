pub mod link_analyzer;
pub mod link_referent;

pub use link_analyzer::LinkAnalyzer;
pub use link_referent::{
    get_link_type, get_object1, get_object2, get_param, new_link_referent, set_datefrom,
    set_dateto, set_link_type, set_object1, set_object2, set_param, LinkType, ATTR_DATEFROM,
    ATTR_DATETO, ATTR_OBJECT1, ATTR_OBJECT2, ATTR_PARAM, ATTR_TYPE, OBJ_TYPENAME,
};
