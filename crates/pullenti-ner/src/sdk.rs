use crate::address::AddressAnalyzer;
use crate::analyzer::Analyzer;
use crate::bank::BankAnalyzer;
use crate::booklink::BookLinkAnalyzer;
use crate::chemical::ChemicalAnalyzer;
use crate::date::DateAnalyzer;
use crate::decree::DecreeAnalyzer;
use crate::definition::DefinitionAnalyzer;
use crate::denomination::DenominationAnalyzer;
use crate::geo::GeoAnalyzer;
use crate::goods::GoodsAnalyzer;
use crate::instrument::InstrumentAnalyzer;
use crate::keyword::KeywordAnalyzer;
use crate::link::LinkAnalyzer;
use crate::mail::MailAnalyzer;
use crate::measure::MeasureAnalyzer;
use crate::money::MoneyAnalyzer;
use crate::named::NamedEntityAnalyzer;
use crate::org::OrgAnalyzer;
use crate::person::PersonAnalyzer;
use crate::phone::PhoneAnalyzer;
use crate::processor::ORDER;
use crate::processor_service::ProcessorService;
use crate::resume::ResumeAnalyzer;
use crate::titlepage::TitlePageAnalyzer;
use crate::transport::TransportAnalyzer;
use crate::uri::UriAnalyzer;
use crate::vacance::VacanceAnalyzer;
use crate::weapon::WeaponAnalyzer;
use pullenti_morph::{MorphLang, MorphologyService};
use std::sync::Arc;

/// Top-level SDK initializer — mirrors C# `Sdk.InitializeAll()` / `Sdk.Initialize()`.
///
/// ## Usage patterns
///
/// ### Pattern 1 — Direct (preferred Rust API, no global state)
/// ```rust
/// # use std::sync::Arc;
/// # use pullenti_morph::MorphLang;
/// # use pullenti_ner::{PersonAnalyzer, Processor, SourceOfAnalysis};
/// let proc = Processor::new(MorphLang::RU, vec![Arc::new(PersonAnalyzer::new())]);
/// let sofa = SourceOfAnalysis::new("Иван Петров");
/// let result = proc.process(sofa, None);
/// # let _ = result;
/// ```
///
/// ### Pattern 2 — All analyzers, one call
/// ```rust
/// # use pullenti_morph::MorphLang;
/// # use pullenti_ner::Processor;
/// let proc = Processor::all(MorphLang::RU);
/// # assert!(proc.analyzer_count() > 0);
/// ```
///
/// ### Pattern 3 — All analyzers (C#-style global registry)
/// ```rust
/// # use pullenti_morph::MorphLang;
/// # use pullenti_ner::{ProcessorService, Sdk};
/// Sdk::initialize_all(Some(MorphLang::RU));
/// let proc = ProcessorService::create_processor();
/// # assert!(proc.analyzer_count() > 0);
/// ```
///
/// ### Pattern 4 — Selective global registration
/// ```rust
/// # use std::sync::Arc;
/// # use pullenti_morph::MorphLang;
/// # use pullenti_ner::{PhoneAnalyzer, ProcessorService, Sdk};
/// Sdk::initialize_with(Some(MorphLang::RU), vec![Arc::new(PhoneAnalyzer::new())]);
/// let proc = ProcessorService::create_processor();
/// # assert!(proc.find_analyzer("PHONE").is_some());
/// ```
pub struct Sdk;

impl Sdk {
    /// Initialize morphology for the given language(s) and register **all** built-in analyzers
    /// in the global `ProcessorService` registry.
    pub fn initialize_all(langs: Option<MorphLang>) {
        MorphologyService::initialize(langs);
        let all: Vec<Arc<dyn Analyzer>> = vec![
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
        ];
        for a in all {
            ProcessorService::register_analyzer(a);
        }
    }

    /// Initialize morphology and register only the **caller-supplied** analyzer instances
    /// in the global `ProcessorService` registry.
    ///
    /// Analyzers are automatically sorted into the canonical NER pipeline order.
    pub fn initialize_with(langs: Option<MorphLang>, analyzers: Vec<Arc<dyn Analyzer>>) {
        MorphologyService::initialize(langs);
        let mut sorted = analyzers;
        sorted.sort_by_key(|a| {
            ORDER
                .iter()
                .position(|&n| n == a.name())
                .unwrap_or(usize::MAX)
        });
        for a in sorted {
            ProcessorService::register_analyzer(a);
        }
    }

    /// Return the SDK version string.
    pub const VERSION: &'static str = ProcessorService::VERSION;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static SDK_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_registry(test: impl FnOnce()) {
        let _guard = SDK_TEST_LOCK.lock().unwrap();
        ProcessorService::clear();
        test();
        ProcessorService::clear();
    }

    #[test]
    fn initialize_all_registers_builtin_analyzers() {
        with_clean_registry(|| {
            Sdk::initialize_all(Some(MorphLang::RU));

            let proc = ProcessorService::create_processor();
            let names = proc.analyzer_names();

            assert!(names.contains(&"PHONE".to_string()));
            assert!(names.contains(&"PERSON".to_string()));
            assert!(!names.contains(&"THESIS".to_string()));
        });
    }

    #[test]
    fn initialize_with_sorts_analyzers_into_pipeline_order() {
        with_clean_registry(|| {
            Sdk::initialize_with(
                Some(MorphLang::RU),
                vec![
                    Arc::new(PersonAnalyzer::new()),
                    Arc::new(GeoAnalyzer::new()),
                    Arc::new(PhoneAnalyzer::new()),
                ],
            );

            let proc = ProcessorService::create_processor();
            assert_eq!(proc.analyzer_names(), vec!["PHONE", "GEO", "PERSON"]);
        });
    }

    #[test]
    fn duplicate_registration_keeps_one_analyzer_per_name() {
        with_clean_registry(|| {
            Sdk::initialize_with(Some(MorphLang::RU), vec![Arc::new(PhoneAnalyzer::new())]);
            Sdk::initialize_with(Some(MorphLang::RU), vec![Arc::new(PhoneAnalyzer::new())]);

            let proc = ProcessorService::create_processor();
            assert_eq!(proc.analyzer_names(), vec!["PHONE"]);
        });
    }

    #[test]
    fn empty_and_specific_processor_behavior_is_stable() {
        with_clean_registry(|| {
            Sdk::initialize_with(
                Some(MorphLang::RU),
                vec![
                    Arc::new(PhoneAnalyzer::new()),
                    Arc::new(DefinitionAnalyzer::new()),
                ],
            );

            let empty = ProcessorService::create_empty_processor();
            assert_eq!(empty.analyzer_count(), 0);

            let regular = ProcessorService::create_processor();
            assert_eq!(regular.analyzer_names(), vec!["PHONE"]);

            let specific = ProcessorService::create_specific_processor(&["THESIS"]);
            assert_eq!(specific.analyzer_names(), vec!["PHONE", "THESIS"]);
        });
    }
}
