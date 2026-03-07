pub mod decree_referent;
pub mod decree_table;
pub mod decree_analyzer;

pub use decree_analyzer::DecreeAnalyzer;
pub use decree_referent::{
    OBJ_TYPENAME as DECREE_OBJ_TYPENAME,
    ATTR_TYPE as DECREE_ATTR_TYPE,
    ATTR_NAME as DECREE_ATTR_NAME,
    ATTR_NUMBER as DECREE_ATTR_NUMBER,
    ATTR_KIND as DECREE_ATTR_KIND,
    DecreeKind,
    get_decree_type,
    get_decree_name,
    get_decree_number,
    get_decree_kind,
    new_decree_referent,
};
