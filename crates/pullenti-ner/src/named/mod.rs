pub mod named_analyzer;
pub mod named_referent;
pub mod named_table;

pub use named_analyzer::NamedEntityAnalyzer;
pub use named_referent::{
    add_name, get_kind, get_name, get_names, get_type, new_named_referent, set_kind, set_type,
    ATTR_KIND, ATTR_NAME, ATTR_TYPE, OBJ_TYPENAME,
};
pub use named_table::NamedKind;
