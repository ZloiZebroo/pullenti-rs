pub mod chemical_referent;
pub mod chemical_token;
pub mod chemical_analyzer;

pub use chemical_analyzer::ChemicalAnalyzer;
pub use chemical_referent::{
    OBJ_TYPENAME as CHEMICAL_OBJ_TYPENAME,
    ATTR_VALUE, ATTR_NAME,
    get_value, get_name,
    set_value, add_name,
    new_chemical_referent,
};
