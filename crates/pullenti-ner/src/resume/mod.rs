pub mod resume_referent;
pub mod resume_analyzer;

pub use resume_analyzer::ResumeAnalyzer;
pub use resume_referent::{
    OBJ_TYPENAME as RESUME_OBJ_TYPENAME,
    ATTR_TYPE as RESUME_ATTR_TYPE,
    ATTR_VALUE as RESUME_ATTR_VALUE,
    ATTR_REF as RESUME_ATTR_REF,
    ResumeItemType,
    new_resume_referent,
    get_typ as get_resume_typ,
    get_value as get_resume_value,
    set_typ,
    set_value,
};

pub use resume_analyzer::{parse_org, parse_org2};
