use std::sync::{Arc, Mutex, OnceLock};
use pullenti_morph::{MorphologyService, MorphLang};
use crate::analyzer::Analyzer;
use crate::processor::Processor;

static ANALYZERS: OnceLock<Mutex<Vec<Arc<dyn Analyzer>>>> = OnceLock::new();

fn global_analyzers() -> &'static Mutex<Vec<Arc<dyn Analyzer>>> {
    ANALYZERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Static service for processor/analyzer lifecycle management
pub struct ProcessorService;

impl ProcessorService {
    /// SDK version
    pub const VERSION: &'static str = "4.33";

    /// Initialize morphology and all subsystems for the given language(s)
    pub fn initialize(langs: Option<MorphLang>) {
        MorphologyService::initialize(langs);
    }

    /// Register an analyzer globally
    pub fn register_analyzer(a: Arc<dyn Analyzer>) {
        let mut analyzers = global_analyzers().lock().unwrap();
        // Avoid duplicate registration
        if !analyzers.iter().any(|x| x.name() == a.name()) {
            analyzers.push(a);
        }
    }

    /// Get all registered analyzers
    pub fn analyzers() -> Vec<Arc<dyn Analyzer>> {
        global_analyzers().lock().unwrap().clone()
    }

    /// Create a standard processor with all non-specific analyzers
    pub fn create_processor() -> Processor {
        let p = Processor::new();
        let analyzers = global_analyzers().lock().unwrap();
        for a in analyzers.iter() {
            if !a.is_specific() {
                p.add_analyzer(a.clone());
            }
        }
        p
    }

    /// Create a processor with specific named analyzers added
    pub fn create_specific_processor(names: &[&str]) -> Processor {
        let p = Self::create_processor();
        let analyzers = global_analyzers().lock().unwrap();
        for name in names {
            if let Some(a) = analyzers.iter().find(|a| a.name() == *name) {
                p.add_analyzer(a.clone());
            }
        }
        p
    }

    /// Create an empty processor (no analyzers)
    pub fn create_empty_processor() -> Processor {
        Processor::new()
    }
}
