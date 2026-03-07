/// InstrumentReferent — structure of legal/normative documents.
/// Mirrors `InstrumentReferent.cs` and `InstrumentBlockReferent.cs`.
///
/// This is a stub implementation — the full Instrument analyzer (~15K lines)
/// is too complex to port and is typically used as a specific analyzer.

use crate::referent::Referent;

// ── InstrumentReferent ─────────────────────────────────────────────────────

pub const OBJ_TYPENAME: &str = "INSTRUMENT";
pub const ATTR_TYPE:    &str = "TYPE";
pub const ATTR_NAME:    &str = "NAME";
pub const ATTR_NUMBER:  &str = "NUMBER";
pub const ATTR_DATE:    &str = "DATE";

// ── InstrumentBlockReferent ────────────────────────────────────────────────

pub const BLOCK_OBJ_TYPENAME: &str = "INSTRBLOCK";
pub const BLOCK_ATTR_KIND:    &str = "KIND";
pub const BLOCK_ATTR_NUMBER:  &str = "NUMBER";
pub const BLOCK_ATTR_NAME:    &str = "NAME";

pub fn new_instrument_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn new_block_referent() -> Referent {
    Referent::new(BLOCK_OBJ_TYPENAME)
}
