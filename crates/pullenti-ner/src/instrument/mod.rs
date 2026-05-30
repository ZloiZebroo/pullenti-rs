pub mod instrument_analyzer;
pub mod instrument_referent;

pub use instrument_analyzer::InstrumentAnalyzer;
pub use instrument_referent::{
    new_block_referent, new_instrument_referent,
    BLOCK_OBJ_TYPENAME as INSTRUMENT_BLOCK_OBJ_TYPENAME, OBJ_TYPENAME as INSTRUMENT_OBJ_TYPENAME,
};
