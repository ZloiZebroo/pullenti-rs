use std::sync::{Arc, Mutex};
use pullenti_morph::{MorphologyService, MorphLang};
use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::analysis_result::AnalysisResult;
use crate::source_of_analysis::SourceOfAnalysis;

/// Orchestrates text analysis — runs all registered analyzers sequentially
pub struct Processor {
    analyzers: Mutex<Vec<Arc<dyn Analyzer>>>,
    /// Optional processing timeout in seconds (0 = no limit)
    pub timeout_seconds: u64,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            analyzers: Mutex::new(Vec::new()),
            timeout_seconds: 0,
        }
    }

    /// Create a processor pre-loaded with the given analyzers
    pub fn with_analyzers(analyzers: Vec<Arc<dyn Analyzer>>) -> Self {
        Processor {
            analyzers: Mutex::new(analyzers),
            timeout_seconds: 0,
        }
    }

    pub fn add_analyzer(&self, a: Arc<dyn Analyzer>) {
        self.analyzers.lock().unwrap().push(a);
    }

    pub fn remove_analyzer(&self, name: &str) {
        let mut analyzers = self.analyzers.lock().unwrap();
        analyzers.retain(|a| a.name() != name);
    }

    pub fn find_analyzer(&self, name: &str) -> Option<Arc<dyn Analyzer>> {
        let analyzers = self.analyzers.lock().unwrap();
        analyzers.iter().find(|a| a.name() == name).cloned()
    }

    /// Main analysis entry point
    pub fn process(&self, sofa: SourceOfAnalysis, lang: Option<MorphLang>) -> AnalysisResult {
        let lang = lang.unwrap_or(MorphLang::UNKNOWN);

        // Morphological tokenization
        let morph_tokens = MorphologyService::process(&sofa.text, Some(lang))
            .unwrap_or_default();

        let mut kit = AnalysisKit::new(sofa.clone());
        kit.build_tokens(morph_tokens);
        kit.define_base_language();

        // Run all analyzers
        let analyzers: Vec<Arc<dyn Analyzer>> = {
            self.analyzers.lock().unwrap().clone()
        };

        for analyzer in &analyzers {
            analyzer.process(&mut kit);
        }

        // Build result
        let mut result = AnalysisResult::new(sofa);
        result.first_token = kit.first_token;
        result.entities = kit.entities;
        result.base_language = kit.base_language;
        result
    }
}

impl Default for Processor {
    fn default() -> Self { Processor::new() }
}
