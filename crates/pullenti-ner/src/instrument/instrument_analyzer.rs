/// InstrumentAnalyzer — stub implementation.
///
/// The full Instrument analyzer (~15K lines) parses hierarchical structure of
/// legal/normative documents (laws, contracts, standards). It is too complex
/// to port in full and is typically used as a specific analyzer.
///
/// This stub registers the analyzer name so the SDK compiles correctly.

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;

pub struct InstrumentAnalyzer;

impl InstrumentAnalyzer {
    pub fn new() -> Self { InstrumentAnalyzer }
}

impl Default for InstrumentAnalyzer {
    fn default() -> Self { InstrumentAnalyzer }
}

impl Analyzer for InstrumentAnalyzer {
    fn name(&self)       -> &'static str { "INSTRUMENT" }
    fn caption(&self)    -> &'static str { "Структура НПА" }
    fn is_specific(&self) -> bool        { true }

    fn process(&self, _kit: &mut AnalysisKit) {
        // Stub: full implementation requires ~15K lines of complex C# port
    }
}
