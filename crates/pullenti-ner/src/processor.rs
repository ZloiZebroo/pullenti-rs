use std::sync::Arc;
use pullenti_morph::{MorphologyService, MorphLang};
use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::analysis_result::AnalysisResult;
use crate::source_of_analysis::SourceOfAnalysis;

use crate::phone::PhoneAnalyzer;
use crate::uri::UriAnalyzer;
use crate::date::DateAnalyzer;
use crate::money::MoneyAnalyzer;
use crate::measure::MeasureAnalyzer;
use crate::geo::GeoAnalyzer;
use crate::person::PersonAnalyzer;
use crate::org::OrgAnalyzer;
use crate::named::NamedEntityAnalyzer;
use crate::address::AddressAnalyzer;
use crate::transport::TransportAnalyzer;
use crate::decree::DecreeAnalyzer;
use crate::bank::BankAnalyzer;
use crate::weapon::WeaponAnalyzer;
use crate::chemical::ChemicalAnalyzer;
use crate::vacance::VacanceAnalyzer;
use crate::denomination::DenominationAnalyzer;
use crate::mail::MailAnalyzer;
use crate::keyword::KeywordAnalyzer;
use crate::definition::DefinitionAnalyzer;
use crate::resume::ResumeAnalyzer;
use crate::instrument::InstrumentAnalyzer;
use crate::titlepage::TitlePageAnalyzer;
use crate::booklink::BookLinkAnalyzer;
use crate::goods::GoodsAnalyzer;
use crate::link::LinkAnalyzer;

/// Canonical NER pipeline order.
/// Org must run before Person; GEO before PERSON; MONEY before MEASURE; etc.
pub(crate) static ORDER: &[&str] = &[
    "PHONE", "URI", "DATE", "MONEY", "MEASURE",
    "GEO", "ADDRESS", "ORGANIZATION", "PERSON",
    "NAMEDENTITY", "TRANSPORT", "DECREE", "BANKDATA",
    "WEAPON", "CHEMICALFORMULA",
    "VACANCY", "DENOMINATION", "MAIL", "KEYWORD",
    "DEFINITION", "RESUME", "INSTRUMENT",
    "TITLEPAGE", "BOOKLINK", "GOOD", "LINK",
];

/// Orchestrates text analysis — runs all registered analyzers sequentially
pub struct Processor {
    pub(crate) analyzers: Vec<Arc<dyn Analyzer>>,
    /// Optional processing timeout in seconds (0 = no limit)
    pub timeout_seconds: u64,
}

impl Processor {
    /// Create a processor with the given language and analyzers.
    /// Initializes morphology for the requested language and sorts analyzers
    /// into canonical NER pipeline order automatically.
    pub fn new(lang: MorphLang, analyzers: Vec<Arc<dyn Analyzer>>) -> Self {
        MorphologyService::initialize(Some(lang));
        let mut sorted = analyzers;
        sorted.sort_by_key(|a| ORDER.iter().position(|&n| n == a.name()).unwrap_or(usize::MAX));
        Processor {
            analyzers: sorted,
            timeout_seconds: 0,
        }
    }

    /// Create a processor with all built-in analyzers for the given language.
    pub fn all(lang: MorphLang) -> Self {
        Processor::new(lang, vec![
            Arc::new(PhoneAnalyzer::new()),
            Arc::new(UriAnalyzer::new()),
            Arc::new(DateAnalyzer::new()),
            Arc::new(MoneyAnalyzer::new()),
            Arc::new(MeasureAnalyzer::new()),
            Arc::new(GeoAnalyzer::new()),
            Arc::new(AddressAnalyzer::new()),
            Arc::new(OrgAnalyzer::new()),
            Arc::new(PersonAnalyzer::new()),
            Arc::new(NamedEntityAnalyzer::new()),
            Arc::new(TransportAnalyzer::new()),
            Arc::new(DecreeAnalyzer::new()),
            Arc::new(BankAnalyzer::new()),
            Arc::new(WeaponAnalyzer::new()),
            Arc::new(ChemicalAnalyzer::new()),
            Arc::new(VacanceAnalyzer::new()),
            Arc::new(DenominationAnalyzer::new()),
            Arc::new(MailAnalyzer::new()),
            Arc::new(KeywordAnalyzer::new()),
            Arc::new(DefinitionAnalyzer::new()),
            Arc::new(ResumeAnalyzer::new()),
            Arc::new(InstrumentAnalyzer::new()),
            Arc::new(TitlePageAnalyzer::new()),
            Arc::new(BookLinkAnalyzer::new()),
            Arc::new(GoodsAnalyzer::new()),
            Arc::new(LinkAnalyzer::new()),
        ])
    }

    /// Create an empty processor (no analyzers, no morph init)
    pub fn empty() -> Self {
        Processor {
            analyzers: Vec::new(),
            timeout_seconds: 0,
        }
    }

    /// Create a processor pre-loaded with the given analyzers (no morph init)
    pub fn with_analyzers(analyzers: Vec<Arc<dyn Analyzer>>) -> Self {
        Processor {
            analyzers,
            timeout_seconds: 0,
        }
    }

    pub fn add_analyzer(&mut self, a: Arc<dyn Analyzer>) {
        self.analyzers.push(a);
    }

    pub fn remove_analyzer(&mut self, name: &str) {
        self.analyzers.retain(|a| a.name() != name);
    }

    pub fn find_analyzer(&self, name: &str) -> Option<Arc<dyn Analyzer>> {
        self.analyzers.iter().find(|a| a.name() == name).cloned()
    }

    /// Number of registered analyzers
    pub fn analyzer_count(&self) -> usize {
        self.analyzers.len()
    }

    /// Names of registered analyzers
    pub fn analyzer_names(&self) -> Vec<String> {
        self.analyzers.iter().map(|a| a.name().to_string()).collect()
    }

    /// Main analysis entry point
    pub fn process(&self, sofa: SourceOfAnalysis, lang: Option<MorphLang>) -> AnalysisResult {
        let lang = lang.unwrap_or(MorphLang::UNKNOWN);

        // Morphological tokenization
        let morph_tokens = MorphologyService::process(&sofa.text, Some(lang))
            .unwrap_or_default();

        // Wrap in Arc to share between kit and result (avoids deep clone)
        let sofa = Arc::new(sofa);

        let mut kit = AnalysisKit::new(Arc::clone(&sofa));
        kit.build_tokens(morph_tokens);
        kit.define_base_language();

        // Run all analyzers
        for analyzer in &self.analyzers {
            analyzer.process(&mut kit);
        }

        // Build result
        let mut result = AnalysisResult::new(sofa);
        result.first_token = kit.first_token;
        result.entities = kit.entities;
        result.base_language = kit.base_language;
        result
    }

    /// Process multiple documents in parallel using Rayon.
    /// Each document gets its own AnalysisKit and token chain.
    /// Requires the `parallel` feature.
    #[cfg(feature = "parallel")]
    pub fn process_batch(
        &self,
        docs: Vec<(SourceOfAnalysis, Option<MorphLang>)>,
    ) -> Vec<AnalysisResult> {
        use rayon::prelude::*;
        let analyzers = &self.analyzers;
        docs.into_par_iter()
            .map(|(sofa, lang)| {
                let proc = Processor {
                    analyzers: analyzers.clone(),
                    timeout_seconds: 0,
                };
                proc.process(sofa, lang)
            })
            .collect()
    }
}

impl Default for Processor {
    fn default() -> Self { Processor::empty() }
}
