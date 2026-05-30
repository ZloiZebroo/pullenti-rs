// Pullenti NER - Named Entity Recognition subsystem
// Ported from Pullenti C# SDK v4.33

#![allow(
    dead_code,
    dropping_copy_types,
    unreachable_code,
    unreachable_patterns,
    unused_assignments,
    unused_imports,
    unused_labels,
    unused_macros,
    unused_mut,
    unused_variables
)]

pub mod address;
pub mod analysis_kit;
pub mod analysis_result;
pub mod analyzer;
pub mod bank;
pub mod booklink;
pub mod chemical;
pub mod core;
pub mod date;
pub mod decree;
pub mod definition;
pub mod denomination;
pub mod deriv;
pub mod geo;
pub mod goods;
pub mod instrument;
pub mod keyword;
pub mod link;
pub mod mail;
pub mod measure;
pub mod money;
pub mod morph_collection;
pub mod named;
pub mod org;
pub mod person;
pub mod phone;
pub mod processor;
pub mod processor_service;
pub mod referent;
pub mod resume;
pub mod sdk;
pub mod source_of_analysis;
pub mod titlepage;
pub mod token;
pub mod transport;
pub mod uri;
pub mod vacance;
pub mod weapon;

pub use address::AddressAnalyzer;
pub use analysis_kit::AnalysisKit;
pub use analysis_result::AnalysisResult;
pub use analyzer::Analyzer;
pub use bank::BankAnalyzer;
pub use booklink::BookLinkAnalyzer;
pub use chemical::ChemicalAnalyzer;
pub use date::DateAnalyzer;
pub use decree::DecreeAnalyzer;
pub use definition::DefinitionAnalyzer;
pub use denomination::DenominationAnalyzer;
pub use geo::GeoAnalyzer;
pub use goods::GoodsAnalyzer;
pub use instrument::InstrumentAnalyzer;
pub use keyword::KeywordAnalyzer;
pub use link::LinkAnalyzer;
pub use mail::MailAnalyzer;
pub use measure::MeasureAnalyzer;
pub use money::MoneyAnalyzer;
pub use morph_collection::{MorphCollection, MorphVoice};
pub use named::NamedEntityAnalyzer;
pub use org::OrgAnalyzer;
pub use person::PersonAnalyzer;
pub use phone::PhoneAnalyzer;
pub use phone::PhoneKind;
pub use processor::Processor;
pub use processor_service::ProcessorService;
pub use referent::{Referent, Slot, SlotValue, TextAnnotation};
pub use resume::ResumeAnalyzer;
pub use sdk::Sdk;
pub use source_of_analysis::SourceOfAnalysis;
pub use titlepage::TitlePageAnalyzer;
pub use token::{
    build_token_chain, MetaTokenData, NumberSpellingType, NumberTokenData, ReferentTokenData,
    TextTokenData, Token, TokenChainIter, TokenKind, TokenRef,
};
pub use transport::TransportAnalyzer;
pub use uri::UriAnalyzer;
pub use vacance::VacanceAnalyzer;
pub use weapon::WeaponAnalyzer;
