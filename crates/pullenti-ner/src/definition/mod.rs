pub mod definition_analyzer;
pub mod definition_referent;

pub use definition_analyzer::DefinitionAnalyzer;
pub use definition_referent::{
    add_slot_str, get_kind_str, get_termin, get_termin_add, get_value, new_thesis_referent,
    set_kind, DefinitionKind, ATTR_DECREE, ATTR_KIND, ATTR_MISC, ATTR_TERMIN, ATTR_TERMIN_ADD,
    ATTR_VALUE, OBJ_TYPENAME as THESIS_OBJ_TYPENAME,
};
