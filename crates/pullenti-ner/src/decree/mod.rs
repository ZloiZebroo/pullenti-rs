pub mod decree_analyzer;
pub mod decree_referent;
pub mod decree_table;

pub use decree_analyzer::DecreeAnalyzer;
pub use decree_referent::{
    get_decree_kind, get_decree_name, get_decree_number, get_decree_type, new_decree_referent,
    DecreeKind, ATTR_KIND as DECREE_ATTR_KIND, ATTR_NAME as DECREE_ATTR_NAME,
    ATTR_NUMBER as DECREE_ATTR_NUMBER, ATTR_TYPE as DECREE_ATTR_TYPE,
    OBJ_TYPENAME as DECREE_OBJ_TYPENAME,
};
