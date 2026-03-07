pub mod good_referent;
pub mod goods_analyzer;

pub use goods_analyzer::GoodsAnalyzer;

pub use good_referent::{
    OBJ_TYPENAME as GOOD_OBJ_TYPENAME,
    GOODATTR_OBJ_TYPENAME,
    ATTR_ATTR,
    ATTR_TYPE  as GOODATTR_ATTR_TYPE,
    ATTR_VALUE as GOODATTR_ATTR_VALUE,
    ATTR_NAME  as GOODATTR_ATTR_NAME,
    ATTR_REF   as GOODATTR_ATTR_REF,
    GoodAttrType,
    get_attr_type, get_attr_value,
    good_to_string, goodattr_to_string,
};
