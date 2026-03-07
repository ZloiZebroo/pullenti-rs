pub mod definition_referent;
pub mod definition_analyzer;

pub use definition_analyzer::DefinitionAnalyzer;
pub use definition_referent::{
    OBJ_TYPENAME as THESIS_OBJ_TYPENAME,
    ATTR_TERMIN,
    ATTR_TERMIN_ADD,
    ATTR_VALUE,
    ATTR_MISC,
    ATTR_KIND,
    ATTR_DECREE,
    DefinitionKind,
    new_thesis_referent,
    add_slot_str,
    set_kind,
    get_termin,
    get_termin_add,
    get_value,
    get_kind_str,
};
