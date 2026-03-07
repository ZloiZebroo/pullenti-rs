pub mod instrument_referent;
pub mod instrument_analyzer;

pub use instrument_analyzer::InstrumentAnalyzer;
pub use instrument_referent::{
    OBJ_TYPENAME as INSTRUMENT_OBJ_TYPENAME,
    BLOCK_OBJ_TYPENAME as INSTRUMENT_BLOCK_OBJ_TYPENAME,
    new_instrument_referent,
    new_block_referent,
};
