pub mod keyword_analyzer;
pub mod keyword_referent;

pub use keyword_analyzer::KeywordAnalyzer;
pub use keyword_referent::{
    get_normal as get_keyword_normal, get_rank as get_keyword_rank, get_typ as get_keyword_type,
    get_value as get_keyword_value, KeywordType, ATTR_NORMAL as KEYWORD_ATTR_NORMAL,
    ATTR_REF as KEYWORD_ATTR_REF, ATTR_TYPE as KEYWORD_ATTR_TYPE, ATTR_VALUE as KEYWORD_ATTR_VALUE,
    OBJ_TYPENAME as KEYWORD_OBJ_TYPENAME,
};
