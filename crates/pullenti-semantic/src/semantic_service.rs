/// SemanticService — entry point for semantic analysis.
/// Mirrors `SemanticService.cs`.

use pullenti_ner::analysis_result::AnalysisResult;
use pullenti_morph::MorphLang;
use crate::types::SemProcessParams;
use crate::sem_document::SemDocument;
use crate::analyze_helper;

pub const VERSION: &str = "0.3";

static INITED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// Initialize the semantic service (call once, after ProcessorService/MorphologyService init).
pub fn initialize() {
    INITED.get_or_init(|| {
        // Initialize the deriv dictionary (for RU by default)
        pullenti_ner::deriv::deriv_service::initialize(MorphLang::RU);
    });
}

/// Perform semantic analysis over the NER analysis result.
pub fn process(ar: &AnalysisResult, pars: Option<SemProcessParams>) -> SemDocument {
    INITED.get_or_init(|| {
        pullenti_ner::deriv::deriv_service::initialize(MorphLang::RU);
    });
    let pars = pars.unwrap_or_default();
    let mut doc = analyze_helper::process(ar, &pars);
    crate::internal::optimizer_helper::optimize(&mut doc, &pars);
    doc
}
