pub mod keyword_referent;
pub mod keyword_analyzer;

pub use keyword_analyzer::KeywordAnalyzer;
pub use keyword_referent::{
    OBJ_TYPENAME as KEYWORD_OBJ_TYPENAME,
    ATTR_TYPE  as KEYWORD_ATTR_TYPE,
    ATTR_VALUE as KEYWORD_ATTR_VALUE,
    ATTR_NORMAL as KEYWORD_ATTR_NORMAL,
    ATTR_REF   as KEYWORD_ATTR_REF,
    KeywordType,
    get_typ as get_keyword_type,
    get_value as get_keyword_value,
    get_normal as get_keyword_normal,
    get_rank as get_keyword_rank,
};
