/// pullentipy — PyO3 Python bindings for the Pullenti NLP SDK.
///
/// Exposed Python API:
///
///   from pullentipy import Sdk, MorphLang, PersonAnalyzer
///   Sdk.initialize_all()
///   proc = Sdk.create_processor()
///   result = proc.analyze("Иван работает в Москве", lang=MorphLang.RU)
///   for r in result.referents:
///       print(r.entity_type, r.text)

use pyo3::prelude::*;
use std::sync::Arc;

use pullenti_morph::{MorphologyService, MorphLang};
use pullenti_ner::{
    Sdk,
    ProcessorService,
    SourceOfAnalysis,
    processor::Processor,
};
use pullenti_ner::token::TokenKind;
use std::collections::HashMap;
use std::rc::Rc;
use pullenti_ner::phone::PhoneAnalyzer;
use pullenti_ner::uri::UriAnalyzer;
use pullenti_ner::date::DateAnalyzer;
use pullenti_ner::money::MoneyAnalyzer;
use pullenti_ner::measure::MeasureAnalyzer;
use pullenti_ner::geo::GeoAnalyzer;
use pullenti_ner::person::PersonAnalyzer;
use pullenti_ner::org::OrgAnalyzer;
use pullenti_ner::named::NamedEntityAnalyzer;
use pullenti_ner::address::AddressAnalyzer;
use pullenti_ner::transport::TransportAnalyzer;
use pullenti_ner::decree::DecreeAnalyzer;
use pullenti_ner::bank::BankAnalyzer;
use pullenti_ner::weapon::WeaponAnalyzer;
use pullenti_ner::chemical::ChemicalAnalyzer;
use pullenti_ner::vacance::VacanceAnalyzer;
use pullenti_ner::denomination::DenominationAnalyzer;
use pullenti_ner::mail::MailAnalyzer;
use pullenti_ner::keyword::KeywordAnalyzer;
use pullenti_ner::definition::DefinitionAnalyzer;
use pullenti_ner::resume::ResumeAnalyzer;
use pullenti_ner::instrument::InstrumentAnalyzer;
use pullenti_ner::titlepage::TitlePageAnalyzer;
use pullenti_ner::goods::GoodsAnalyzer;
use pullenti_ner::booklink::BookLinkAnalyzer;
use pullenti_ner::link::LinkAnalyzer;

// ── MorphLang ────────────────────────────────────────────────────────────────

/// Morphology language selector.  Use the class attributes:
///   ``MorphLang.RU``, ``MorphLang.EN``, ``MorphLang.UA``, ``MorphLang.UNKNOWN``
#[pyclass(name = "MorphLang", skip_from_py_object)]
#[derive(Clone, Copy)]
pub struct PyMorphLang {
    value: i16,
}

impl PyMorphLang {
    fn to_rust(self) -> MorphLang {
        MorphLang::from_value(self.value)
    }
}

#[pymethods]
impl PyMorphLang {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const UNKNOWN: PyMorphLang = PyMorphLang { value: 0 };

    #[classattr]
    #[allow(non_upper_case_globals)]
    const RU: PyMorphLang = PyMorphLang { value: 1 };

    #[classattr]
    #[allow(non_upper_case_globals)]
    const UA: PyMorphLang = PyMorphLang { value: 2 };

    #[classattr]
    #[allow(non_upper_case_globals)]
    const BY: PyMorphLang = PyMorphLang { value: 4 };

    #[classattr]
    #[allow(non_upper_case_globals)]
    const EN: PyMorphLang = PyMorphLang { value: 8 };

    #[getter]
    fn value(&self) -> i16 { self.value }

    fn __repr__(&self) -> &str {
        match self.value {
            1 => "MorphLang.RU",
            2 => "MorphLang.UA",
            4 => "MorphLang.BY",
            8 => "MorphLang.EN",
            _ => "MorphLang.UNKNOWN",
        }
    }

    fn __eq__(&self, other: &PyMorphLang) -> bool { self.value == other.value }
    fn __hash__(&self) -> i16 { self.value }
}

// ── Occurrence ────────────────────────────────────────────────────────────────

/// A single text span where a :class:`Referent` was found.
#[pyclass(name = "Occurrence", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOccurrence {
    /// Character offset of the first character (inclusive).
    #[pyo3(get)]
    pub begin_char: i32,
    /// Character offset of the last character (inclusive).
    #[pyo3(get)]
    pub end_char: i32,
    /// Source text for this span.
    #[pyo3(get)]
    pub text: String,
}

#[pymethods]
impl PyOccurrence {
    fn __repr__(&self) -> String {
        format!("Occurrence({:?}, {}..{})", self.text, self.begin_char, self.end_char)
    }
}

// ── Slot ─────────────────────────────────────────────────────────────────────

/// A named attribute on a recognized entity.
#[pyclass(name = "Slot", skip_from_py_object)]
#[derive(Clone)]
pub struct PySlot {
    /// Attribute name (e.g. "FIRSTNAME", "LASTNAME", "VALUE")
    #[pyo3(get)]
    pub name: String,
    /// String representation of the attribute value
    #[pyo3(get)]
    pub value: String,
}

#[pymethods]
impl PySlot {
    fn __repr__(&self) -> String { format!("Slot({}={})", self.name, self.value) }
}

// ── Referent ─────────────────────────────────────────────────────────────────

/// A recognized named entity.
#[pyclass(name = "Referent")]
pub struct PyReferent {
    /// Entity type name (e.g. "PERSON", "GEO", "ORGANIZATION", "DATE")
    #[pyo3(get)]
    pub entity_type: String,
    /// Source text span where this entity was found (first occurrence).
    #[pyo3(get)]
    pub text: String,
    /// Character offset of the first character of the first occurrence (inclusive).
    #[pyo3(get)]
    pub begin_char: i32,
    /// Character offset of the last character of the first occurrence (inclusive).
    #[pyo3(get)]
    pub end_char: i32,
    /// Full entity string (type + all slots), mirrors C# Referent.ToString()
    #[pyo3(get)]
    pub summary: String,
    slots_data: Vec<PySlot>,
    occurrences_data: Vec<PyOccurrence>,
}

#[pymethods]
impl PyReferent {
    /// List of named attribute slots on this entity.
    #[getter]
    fn slots(&self, py: Python<'_>) -> PyResult<Vec<Py<PySlot>>> {
        self.slots_data.iter().map(|s| Py::new(py, s.clone())).collect()
    }

    /// All text spans where this entity appears in the document.
    #[getter]
    fn occurrences(&self, py: Python<'_>) -> PyResult<Vec<Py<PyOccurrence>>> {
        self.occurrences_data.iter().map(|o| Py::new(py, o.clone())).collect()
    }

    /// Get the first value of the named slot, or None.
    fn get(&self, name: &str) -> Option<String> {
        self.slots_data.iter().find(|s| s.name == name).map(|s| s.value.clone())
    }

    fn __repr__(&self) -> String {
        format!("Referent({}: {})", self.entity_type, self.text)
    }
}

// ── AnalysisResult ────────────────────────────────────────────────────────────

/// Result of NER text analysis — holds the list of recognized entities.
#[pyclass(name = "AnalysisResult")]
pub struct PyAnalysisResult {
    referents_data: Vec<Py<PyReferent>>,
}

#[pymethods]
impl PyAnalysisResult {
    /// All recognized entities, in document order.
    #[getter]
    fn referents(&self, py: Python<'_>) -> Vec<Py<PyReferent>> {
        self.referents_data.iter().map(|r| r.clone_ref(py)).collect()
    }

    fn __len__(&self) -> usize { self.referents_data.len() }

    fn __repr__(&self) -> String {
        format!("AnalysisResult({} entities)", self.referents_data.len())
    }
}

/// Convert Rust `AnalysisResult` to `PyAnalysisResult`, eagerly converting
/// all data so no `Rc<RefCell<...>>` leaks past the Python boundary.
fn convert_analysis_result(
    py: Python<'_>,
    ar: pullenti_ner::analysis_result::AnalysisResult,
) -> PyResult<Py<PyAnalysisResult>> {
    // Walk token chain and collect ALL spans for each referent entity.
    // The spans appear in document order because we traverse left-to-right.
    let mut all_spans: HashMap<
        *const std::cell::RefCell<pullenti_ner::referent::Referent>,
        Vec<(i32, i32)>,
    > = HashMap::new();
    let mut cur = ar.first_token.clone();
    while let Some(tok) = cur {
        let next = tok.borrow().next.clone();
        {
            let tb = tok.borrow();
            if let TokenKind::Referent(ref rd) = tb.kind {
                let ptr = Rc::as_ptr(&rd.referent);
                all_spans.entry(ptr).or_default().push((tb.begin_char, tb.end_char));
            }
        }
        cur = next;
    }

    let sofa = &ar.sofa;
    let mut py_refs: Vec<Py<PyReferent>> = Vec::with_capacity(ar.entities.len());

    for ent in &ar.entities {
        let r = ent.borrow();
        let ptr = Rc::as_ptr(ent);

        let occurrences_data: Vec<PyOccurrence> = if let Some(spans) = all_spans.get(&ptr) {
            spans.iter().map(|&(b, e)| PyOccurrence {
                begin_char: b,
                end_char:   e,
                text:       sofa.substring(b, e).to_string(),
            }).collect()
        } else {
            // Fallback: use occurrence list stored on the referent itself
            r.occurrence.iter().map(|occ| PyOccurrence {
                begin_char: occ.begin_char,
                end_char:   occ.end_char,
                text:       sofa.substring(occ.begin_char, occ.end_char).to_string(),
            }).collect()
        };

        let (first_begin, first_end, first_text) = occurrences_data.first()
            .map(|o| (o.begin_char, o.end_char, o.text.clone()))
            .unwrap_or_else(|| (0, 0, r.to_string()));

        let slots_data: Vec<PySlot> = r.slots.iter()
            .filter(|s| !s.is_internal())
            .map(|s| PySlot {
                name:  s.type_name.clone(),
                value: s.value.as_ref().map_or_else(String::new, |v| v.to_string()),
            })
            .collect();

        py_refs.push(Py::new(py, PyReferent {
            entity_type: r.type_name.clone(),
            text:        first_text,
            begin_char:  first_begin,
            end_char:    first_end,
            summary:     r.to_string(),
            slots_data,
            occurrences_data,
        })?);
    }

    Py::new(py, PyAnalysisResult { referents_data: py_refs })
}

// ── Processor ────────────────────────────────────────────────────────────────

/// NER processor — run ``analyze()`` to extract entities from text.
#[pyclass(name = "Processor")]
pub struct PyProcessor {
    inner: Processor,
}

#[pymethods]
impl PyProcessor {
    /// Analyze text and return the recognized entities.
    ///
    /// Args:
    ///   text: Input text (str).
    ///   lang: Optional ``MorphLang`` (default: auto-detect).
    #[pyo3(signature = (text, lang=None))]
    fn analyze(
        &self,
        py: Python<'_>,
        text: &str,
        lang: Option<Bound<'_, PyMorphLang>>,
    ) -> PyResult<Py<PyAnalysisResult>> {
        let rust_lang = lang.as_ref().map(|l| l.borrow().to_rust());
        let sofa = SourceOfAnalysis::new(text.to_string());
        let ar = self.inner.process(sofa, rust_lang);
        convert_analysis_result(py, ar)
    }

    fn __repr__(&self) -> &str { "Processor()" }
}

// ── Analyzer wrapper classes ──────────────────────────────────────────────────
//
// Each is a zero-size Python class whose sole purpose is to be passed to
// ``Sdk.register_analyzer()``.

#[pyclass(name = "PhoneAnalyzer")]     pub struct PyPhoneAnalyzer;
#[pyclass(name = "UriAnalyzer")]       pub struct PyUriAnalyzer;
#[pyclass(name = "DateAnalyzer")]      pub struct PyDateAnalyzer;
#[pyclass(name = "MoneyAnalyzer")]     pub struct PyMoneyAnalyzer;
#[pyclass(name = "MeasureAnalyzer")]   pub struct PyMeasureAnalyzer;
#[pyclass(name = "GeoAnalyzer")]       pub struct PyGeoAnalyzer;
#[pyclass(name = "PersonAnalyzer")]    pub struct PyPersonAnalyzer;
#[pyclass(name = "OrgAnalyzer")]       pub struct PyOrgAnalyzer;
#[pyclass(name = "NamedEntityAnalyzer")] pub struct PyNamedEntityAnalyzer;
#[pyclass(name = "AddressAnalyzer")]   pub struct PyAddressAnalyzer;
#[pyclass(name = "TransportAnalyzer")] pub struct PyTransportAnalyzer;
#[pyclass(name = "DecreeAnalyzer")]    pub struct PyDecreeAnalyzer;
#[pyclass(name = "BankAnalyzer")]      pub struct PyBankAnalyzer;
#[pyclass(name = "WeaponAnalyzer")]       pub struct PyWeaponAnalyzer;
#[pyclass(name = "ChemicalAnalyzer")]     pub struct PyChemicalAnalyzer;
#[pyclass(name = "VacanceAnalyzer")]      pub struct PyVacanceAnalyzer;
#[pyclass(name = "DenominationAnalyzer")] pub struct PyDenominationAnalyzer;
#[pyclass(name = "MailAnalyzer")]         pub struct PyMailAnalyzer;
#[pyclass(name = "KeywordAnalyzer")]      pub struct PyKeywordAnalyzer;
#[pyclass(name = "DefinitionAnalyzer")]   pub struct PyDefinitionAnalyzer;
#[pyclass(name = "ResumeAnalyzer")]       pub struct PyResumeAnalyzer;
#[pyclass(name = "InstrumentAnalyzer")]   pub struct PyInstrumentAnalyzer;
#[pyclass(name = "TitlePageAnalyzer")]    pub struct PyTitlePageAnalyzer;
#[pyclass(name = "GoodsAnalyzer")]        pub struct PyGoodsAnalyzer;
#[pyclass(name = "BookLinkAnalyzer")]     pub struct PyBookLinkAnalyzer;
#[pyclass(name = "LinkAnalyzer")]         pub struct PyLinkAnalyzer;

macro_rules! impl_analyzer_new {
    ($($t:ty),+) => {$(
        #[pymethods]
        impl $t {
            #[new]
            fn new() -> Self { <$t>::default() }
        }
        impl Default for $t { fn default() -> Self { unsafe { std::mem::zeroed() } } }
    )+};
}

impl_analyzer_new!(
    PyPhoneAnalyzer, PyUriAnalyzer, PyDateAnalyzer, PyMoneyAnalyzer,
    PyMeasureAnalyzer, PyGeoAnalyzer, PyPersonAnalyzer, PyOrgAnalyzer,
    PyNamedEntityAnalyzer, PyAddressAnalyzer, PyTransportAnalyzer,
    PyDecreeAnalyzer, PyBankAnalyzer, PyWeaponAnalyzer, PyChemicalAnalyzer,
    PyVacanceAnalyzer, PyDenominationAnalyzer, PyMailAnalyzer, PyKeywordAnalyzer,
    PyDefinitionAnalyzer, PyResumeAnalyzer, PyInstrumentAnalyzer,
    PyTitlePageAnalyzer, PyGoodsAnalyzer, PyBookLinkAnalyzer,
    PyLinkAnalyzer
);

// ── Analyzer dispatch helper ──────────────────────────────────────────────────

fn analyzer_from_bound(a: &Bound<'_, PyAny>) -> PyResult<Arc<dyn pullenti_ner::analyzer::Analyzer>> {
    if a.is_instance_of::<PyPhoneAnalyzer>()         { return Ok(Arc::new(PhoneAnalyzer::new())); }
    if a.is_instance_of::<PyUriAnalyzer>()           { return Ok(Arc::new(UriAnalyzer::new())); }
    if a.is_instance_of::<PyDateAnalyzer>()          { return Ok(Arc::new(DateAnalyzer::new())); }
    if a.is_instance_of::<PyMoneyAnalyzer>()         { return Ok(Arc::new(MoneyAnalyzer::new())); }
    if a.is_instance_of::<PyMeasureAnalyzer>()       { return Ok(Arc::new(MeasureAnalyzer::new())); }
    if a.is_instance_of::<PyGeoAnalyzer>()           { return Ok(Arc::new(GeoAnalyzer::new())); }
    if a.is_instance_of::<PyPersonAnalyzer>()        { return Ok(Arc::new(PersonAnalyzer::new())); }
    if a.is_instance_of::<PyOrgAnalyzer>()           { return Ok(Arc::new(OrgAnalyzer::new())); }
    if a.is_instance_of::<PyNamedEntityAnalyzer>()   { return Ok(Arc::new(NamedEntityAnalyzer::new())); }
    if a.is_instance_of::<PyAddressAnalyzer>()       { return Ok(Arc::new(AddressAnalyzer::new())); }
    if a.is_instance_of::<PyTransportAnalyzer>()     { return Ok(Arc::new(TransportAnalyzer::new())); }
    if a.is_instance_of::<PyDecreeAnalyzer>()        { return Ok(Arc::new(DecreeAnalyzer::new())); }
    if a.is_instance_of::<PyBankAnalyzer>()          { return Ok(Arc::new(BankAnalyzer::new())); }
    if a.is_instance_of::<PyWeaponAnalyzer>()        { return Ok(Arc::new(WeaponAnalyzer::new())); }
    if a.is_instance_of::<PyChemicalAnalyzer>()      { return Ok(Arc::new(ChemicalAnalyzer::new())); }
    if a.is_instance_of::<PyVacanceAnalyzer>()       { return Ok(Arc::new(VacanceAnalyzer::new())); }
    if a.is_instance_of::<PyDenominationAnalyzer>()  { return Ok(Arc::new(DenominationAnalyzer::new())); }
    if a.is_instance_of::<PyMailAnalyzer>()          { return Ok(Arc::new(MailAnalyzer::new())); }
    if a.is_instance_of::<PyKeywordAnalyzer>()       { return Ok(Arc::new(KeywordAnalyzer::new())); }
    if a.is_instance_of::<PyDefinitionAnalyzer>()    { return Ok(Arc::new(DefinitionAnalyzer::new())); }
    if a.is_instance_of::<PyResumeAnalyzer>()        { return Ok(Arc::new(ResumeAnalyzer::new())); }
    if a.is_instance_of::<PyInstrumentAnalyzer>()    { return Ok(Arc::new(InstrumentAnalyzer::new())); }
    if a.is_instance_of::<PyTitlePageAnalyzer>()     { return Ok(Arc::new(TitlePageAnalyzer::new())); }
    if a.is_instance_of::<PyGoodsAnalyzer>()         { return Ok(Arc::new(GoodsAnalyzer::new())); }
    if a.is_instance_of::<PyBookLinkAnalyzer>()      { return Ok(Arc::new(BookLinkAnalyzer::new())); }
    if a.is_instance_of::<PyLinkAnalyzer>()          { return Ok(Arc::new(LinkAnalyzer::new())); }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Unknown analyzer type: {}", a.get_type().name()?
    )))
}

// ── Sdk ───────────────────────────────────────────────────────────────────────

/// Top-level SDK entry point (static methods only).
///
/// Typical usage::
///
///   from pullentipy import Sdk, MorphLang
///   Sdk.initialize_all()
///   proc = Sdk.create_processor()
///   result = proc.analyze("Иван живёт в Москве")
///   for r in result.referents:
///       print(r.entity_type, r.text)
#[pyclass(name = "Sdk")]
pub struct PySdk;

#[pymethods]
impl PySdk {
    /// Initialize morphology for the given language string (``"ru"``, ``"en"``, ``"ua"``…)
    /// without registering any analyzers.
    #[staticmethod]
    #[pyo3(signature = (lang=None))]
    fn initialize(lang: Option<&str>) {
        let morph_lang = lang.and_then(|s| MorphLang::try_parse(s));
        MorphologyService::initialize(morph_lang);
    }

    /// Initialize morphology **and** register all built-in analyzers globally.
    #[staticmethod]
    #[pyo3(signature = (lang=None))]
    fn initialize_all(lang: Option<&str>) {
        let morph_lang = lang.and_then(|s| MorphLang::try_parse(s));
        Sdk::initialize_all(morph_lang);
    }

    /// Initialize morphology and register a **specific** list of analyzers.
    ///
    /// Example::
    ///
    ///   Sdk.initialize_with(lang='ru', analyzers=[PersonAnalyzer(), GeoAnalyzer(), OrgAnalyzer()])
    #[staticmethod]
    #[pyo3(signature = (lang=None, analyzers=None))]
    fn initialize_with(lang: Option<&str>, analyzers: Option<Vec<Bound<'_, PyAny>>>) -> PyResult<()> {
        let morph_lang = lang.and_then(|s| MorphLang::try_parse(s));
        if let Some(list) = analyzers {
            // Collect first, then call Sdk::initialize_with so the canonical NER pipeline
            // order (GEO before PERSON, etc.) is enforced regardless of the Python list order.
            let rust_analyzers: Result<Vec<_>, _> = list.iter()
                .map(|a| analyzer_from_bound(a))
                .collect();
            Sdk::initialize_with(morph_lang, rust_analyzers?);
        } else {
            MorphologyService::initialize(morph_lang);
        }
        Ok(())
    }

    /// Register a single analyzer in the global registry.
    ///
    /// Pass an instance of one of the analyzer classes, e.g.
    /// ``Sdk.register_analyzer(PersonAnalyzer())``.
    #[staticmethod]
    fn register_analyzer(analyzer: Bound<'_, PyAny>) -> PyResult<()> {
        ProcessorService::register_analyzer(analyzer_from_bound(&analyzer)?);
        Ok(())
    }

    /// Create a ``Processor`` pre-loaded with all globally registered analyzers.
    #[staticmethod]
    fn create_processor() -> PyProcessor {
        PyProcessor { inner: ProcessorService::create_processor() }
    }

    /// SDK version string.
    #[staticmethod]
    fn version() -> &'static str { Sdk::VERSION }
}

// ── Module ────────────────────────────────────────────────────────────────────

#[pymodule]
#[pyo3(name = "_pullentipy")]
fn module_init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMorphLang>()?;
    m.add_class::<PyOccurrence>()?;
    m.add_class::<PySlot>()?;
    m.add_class::<PyReferent>()?;
    m.add_class::<PyAnalysisResult>()?;
    m.add_class::<PyProcessor>()?;
    m.add_class::<PySdk>()?;
    m.add_class::<PyPhoneAnalyzer>()?;
    m.add_class::<PyUriAnalyzer>()?;
    m.add_class::<PyDateAnalyzer>()?;
    m.add_class::<PyMoneyAnalyzer>()?;
    m.add_class::<PyMeasureAnalyzer>()?;
    m.add_class::<PyGeoAnalyzer>()?;
    m.add_class::<PyPersonAnalyzer>()?;
    m.add_class::<PyOrgAnalyzer>()?;
    m.add_class::<PyNamedEntityAnalyzer>()?;
    m.add_class::<PyAddressAnalyzer>()?;
    m.add_class::<PyTransportAnalyzer>()?;
    m.add_class::<PyDecreeAnalyzer>()?;
    m.add_class::<PyBankAnalyzer>()?;
    m.add_class::<PyWeaponAnalyzer>()?;
    m.add_class::<PyChemicalAnalyzer>()?;
    m.add_class::<PyVacanceAnalyzer>()?;
    m.add_class::<PyDenominationAnalyzer>()?;
    m.add_class::<PyMailAnalyzer>()?;
    m.add_class::<PyKeywordAnalyzer>()?;
    m.add_class::<PyDefinitionAnalyzer>()?;
    m.add_class::<PyResumeAnalyzer>()?;
    m.add_class::<PyInstrumentAnalyzer>()?;
    m.add_class::<PyTitlePageAnalyzer>()?;
    m.add_class::<PyGoodsAnalyzer>()?;
    m.add_class::<PyBookLinkAnalyzer>()?;
    m.add_class::<PyLinkAnalyzer>()?;
    Ok(())
}
