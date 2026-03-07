use crate::analysis_kit::AnalysisKit;

/// Base trait for all named-entity analyzers
pub trait Analyzer: Send + Sync {
    /// Unique name identifying this analyzer
    fn name(&self) -> &'static str;

    /// Human-readable caption
    fn caption(&self) -> &'static str {
        self.name()
    }

    /// Whether this analyzer is domain-specific (opt-in)
    fn is_specific(&self) -> bool {
        false
    }

    /// Approximate processing weight (for progress estimation)
    fn progress_weight(&self) -> i32 {
        10
    }

    /// Main analysis routine — traverse the token chain and extract entities
    fn process(&self, kit: &mut AnalysisKit);
}
