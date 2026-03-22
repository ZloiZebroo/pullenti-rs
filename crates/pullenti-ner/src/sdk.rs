use std::sync::Arc;
use pullenti_morph::{MorphologyService, MorphLang};
use crate::analyzer::Analyzer;
use crate::processor_service::ProcessorService;
use crate::processor::ORDER;
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

/// Top-level SDK initializer — mirrors C# `Sdk.InitializeAll()` / `Sdk.Initialize()`.
///
/// ## Usage patterns
///
/// ### Pattern 1 — Direct (preferred Rust API, no global state)
/// ```rust,ignore
/// let proc = Processor::new(MorphLang::RU, vec![Arc::new(PersonAnalyzer::new())]);
/// let result = proc.process(sofa, None);
/// ```
///
/// ### Pattern 2 — All analyzers, one call
/// ```rust,ignore
/// let proc = Processor::all(MorphLang::RU);
/// ```
///
/// ### Pattern 3 — All analyzers (C#-style global registry)
/// ```rust,ignore
/// Sdk::initialize_all(Some(MorphLang::RU));
/// let proc = ProcessorService::create_processor();
/// ```
///
/// ### Pattern 4 — Selective global registration
/// ```rust,ignore
/// Sdk::initialize_with(Some(MorphLang::RU), vec![Arc::new(PhoneAnalyzer::new())]);
/// let proc = ProcessorService::create_processor();
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
        sorted.sort_by_key(|a| ORDER.iter().position(|&n| n == a.name()).unwrap_or(usize::MAX));
        for a in sorted {
            ProcessorService::register_analyzer(a);
        }
    }

    /// Return the SDK version string.
    pub const VERSION: &'static str = ProcessorService::VERSION;
}
