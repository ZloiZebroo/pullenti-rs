pub mod named_referent;
pub mod named_table;
pub mod named_analyzer;

pub use named_analyzer::NamedEntityAnalyzer;
pub use named_referent::{
    OBJ_TYPENAME, ATTR_NAME, ATTR_KIND, ATTR_TYPE,
    new_named_referent, add_name, set_kind, set_type,
    get_name, get_names, get_kind, get_type,
};
pub use named_table::NamedKind;
