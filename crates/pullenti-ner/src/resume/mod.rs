pub mod resume_analyzer;
pub mod resume_referent;

pub use resume_analyzer::ResumeAnalyzer;
pub use resume_referent::{
    get_typ as get_resume_typ, get_value as get_resume_value, new_resume_referent, set_typ,
    set_value, ResumeItemType, ATTR_REF as RESUME_ATTR_REF, ATTR_TYPE as RESUME_ATTR_TYPE,
    ATTR_VALUE as RESUME_ATTR_VALUE, OBJ_TYPENAME as RESUME_OBJ_TYPENAME,
};

pub use resume_analyzer::{parse_org, parse_org2};
