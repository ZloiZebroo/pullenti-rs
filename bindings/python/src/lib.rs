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
///
/// Semantic analysis:
///   doc = proc.analyze_semantic("Иван работает в Москве")
///   for block in doc.blocks:
///       for frag in block.fragments:
///           for link in frag.links:
///               print(link.typ, link.source.normal, "->", link.target.normal)

use pyo3::prelude::*;
use std::sync::Arc;

use pullenti_morph::{MorphologyService, MorphLang, MorphClass, MorphCase, MorphGenderFlags, MorphNumber};
use pullenti_ner::{
    Sdk,
    ProcessorService,
    SourceOfAnalysis,
    processor::Processor,
};
use pullenti_ner::token::TokenKind;
use std::collections::HashMap;
use std::rc::Rc;
use pullenti_semantic::semantic_service;
use pullenti_semantic::types::{SemObjectType, SemLinkType, SemFraglinkType};
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
use pullenti_ner::person::analyze_person_name;
use pullenti_ner::person::PersonNormalResult;

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

// ── Semantic types ────────────────────────────────────────────────────────────

/// A semantic object (noun, verb, adjective, etc.) extracted from text.
#[pyclass(name = "SemObject")]
pub struct PySemObject {
    #[pyo3(get)] pub normal:      String,
    #[pyo3(get)] pub normal_full: String,
    #[pyo3(get)] pub typ:         String,   // "noun","verb","adjective",...
    #[pyo3(get)] pub not_:        bool,
    #[pyo3(get)] pub begin_char:  usize,
    #[pyo3(get)] pub end_char:    usize,
    pub links_from_data: Vec<Py<PySemLink>>,
    pub links_to_data:   Vec<Py<PySemLink>>,
}

#[pymethods]
impl PySemObject {
    /// Outgoing semantic links (this object is the source).
    #[getter]
    fn links_from(&self, py: Python<'_>) -> Vec<Py<PySemLink>> {
        self.links_from_data.iter().map(|l| l.clone_ref(py)).collect()
    }
    /// Incoming semantic links (this object is the target).
    #[getter]
    fn links_to(&self, py: Python<'_>) -> Vec<Py<PySemLink>> {
        self.links_to_data.iter().map(|l| l.clone_ref(py)).collect()
    }
    fn __repr__(&self) -> String {
        format!("SemObject({}, {:?})", self.normal, self.typ)
    }
}

/// A directed semantic link between two SemObjects.
#[pyclass(name = "SemLink")]
pub struct PySemLink {
    #[pyo3(get)] pub typ:        String,
    #[pyo3(get)] pub source:     Py<PySemObject>,
    #[pyo3(get)] pub target:     Py<PySemObject>,
    #[pyo3(get)] pub question:   Option<String>,
    #[pyo3(get)] pub preposition: Option<String>,
    #[pyo3(get)] pub is_or:      bool,
}

#[pymethods]
impl PySemLink {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!("SemLink({} {} -> {})", self.typ,
            self.source.borrow(py).normal, self.target.borrow(py).normal)
    }
}

/// A sentence fragment with its semantic graph (objects + links).
#[pyclass(name = "SemFragment")]
pub struct PySemFragment {
    #[pyo3(get)] pub begin_char: usize,
    #[pyo3(get)] pub end_char:   usize,
    pub objects_data: Vec<Py<PySemObject>>,
    pub links_data:   Vec<Py<PySemLink>>,
}

#[pymethods]
impl PySemFragment {
    #[getter]
    fn objects(&self, py: Python<'_>) -> Vec<Py<PySemObject>> {
        self.objects_data.iter().map(|o| o.clone_ref(py)).collect()
    }
    #[getter]
    fn links(&self, py: Python<'_>) -> Vec<Py<PySemLink>> {
        self.links_data.iter().map(|l| l.clone_ref(py)).collect()
    }
    fn __repr__(&self) -> String {
        format!("SemFragment([{}-{}])", self.begin_char, self.end_char)
    }
}

/// A cross-fragment link (IF/THEN, BUT, BECAUSE, etc.).
#[pyclass(name = "SemFraglink")]
pub struct PySemFraglink {
    #[pyo3(get)] pub typ:        String,
    #[pyo3(get)] pub source_idx: usize,
    #[pyo3(get)] pub target_idx: usize,
}

#[pymethods]
impl PySemFraglink {
    fn __repr__(&self) -> String {
        format!("SemFraglink({} {} -> {})", self.typ, self.source_idx, self.target_idx)
    }
}

/// A text block (paragraph/sentence group) containing fragments and cross-fragment links.
#[pyclass(name = "SemBlock")]
pub struct PySemBlock {
    pub fragments_data:  Vec<Py<PySemFragment>>,
    pub fraglinks_data:  Vec<Py<PySemFraglink>>,
}

#[pymethods]
impl PySemBlock {
    #[getter]
    fn fragments(&self, py: Python<'_>) -> Vec<Py<PySemFragment>> {
        self.fragments_data.iter().map(|f| f.clone_ref(py)).collect()
    }
    #[getter]
    fn fraglinks(&self, py: Python<'_>) -> Vec<Py<PySemFraglink>> {
        self.fraglinks_data.iter().map(|l| l.clone_ref(py)).collect()
    }
    fn __repr__(&self) -> String {
        format!("SemBlock({} fragments)", self.fragments_data.len())
    }
}

/// Full semantic analysis result — collection of SemBlocks.
#[pyclass(name = "SemDocument")]
pub struct PySemDocument {
    pub blocks_data: Vec<Py<PySemBlock>>,
}

#[pymethods]
impl PySemDocument {
    #[getter]
    fn blocks(&self, py: Python<'_>) -> Vec<Py<PySemBlock>> {
        self.blocks_data.iter().map(|b| b.clone_ref(py)).collect()
    }
    fn __repr__(&self) -> String {
        format!("SemDocument({} blocks)", self.blocks_data.len())
    }
}

fn obj_type_str(t: SemObjectType) -> &'static str {
    match t {
        SemObjectType::Undefined       => "undefined",
        SemObjectType::Noun            => "noun",
        SemObjectType::Adjective       => "adjective",
        SemObjectType::Verb            => "verb",
        SemObjectType::Participle      => "participle",
        SemObjectType::Adverb          => "adverb",
        SemObjectType::Pronoun         => "pronoun",
        SemObjectType::PersonalPronoun => "personal_pronoun",
        SemObjectType::Question        => "question",
    }
}

fn link_type_str(t: SemLinkType) -> &'static str {
    match t {
        SemLinkType::Undefined  => "undefined",
        SemLinkType::Detail     => "detail",
        SemLinkType::Naming     => "naming",
        SemLinkType::Agent      => "agent",
        SemLinkType::Pacient    => "pacient",
        SemLinkType::Participle => "participle",
        SemLinkType::Anafor     => "anafor",
    }
}

fn fraglinktype_str(t: SemFraglinkType) -> &'static str {
    match t {
        SemFraglinkType::Undefined => "undefined",
        SemFraglinkType::IfThen    => "if_then",
        SemFraglinkType::IfElse    => "if_else",
        SemFraglinkType::Because   => "because",
        SemFraglinkType::But       => "but",
        SemFraglinkType::For       => "for",
        SemFraglinkType::What      => "what",
    }
}

fn convert_sem_document(
    py: Python<'_>,
    doc: pullenti_semantic::sem_document::SemDocument,
) -> PyResult<Py<PySemDocument>> {
    let mut py_blocks: Vec<Py<PySemBlock>> = Vec::new();

    for block_rc in &doc.blocks {
        let block = block_rc.borrow();
        let mut py_frags: Vec<Py<PySemFragment>> = Vec::new();

        for frag_rc in &block.fragments {
            let frag = frag_rc.borrow();

            // --- pass 1: create PySemObject for each SemObject (empty links) ---
            let mut obj_map: HashMap<*const std::cell::RefCell<pullenti_semantic::sem_graph::SemObject>, usize> = HashMap::new();
            let mut py_objs: Vec<Py<PySemObject>> = Vec::new();

            for obj_rc in &frag.graph.objects {
                let obj = obj_rc.borrow();
                let idx = py_objs.len();
                obj_map.insert(Rc::as_ptr(obj_rc), idx);
                py_objs.push(Py::new(py, PySemObject {
                    normal:      obj.normal.clone(),
                    normal_full: obj.normal_full.clone(),
                    typ:         obj_type_str(obj.typ).to_string(),
                    not_:        obj.not,
                    begin_char:  obj.begin_char,
                    end_char:    obj.end_char,
                    links_from_data: Vec::new(),
                    links_to_data:   Vec::new(),
                })?);
            }

            // --- pass 2: create PySemLink for each SemLink ---
            let mut py_links: Vec<Py<PySemLink>> = Vec::new();

            for link_rc in &frag.graph.links {
                let link = link_rc.borrow();
                let src_ptr = Rc::as_ptr(&link.source);
                let tgt_ptr = Rc::as_ptr(&link.target);
                if let (Some(&si), Some(&ti)) = (obj_map.get(&src_ptr), obj_map.get(&tgt_ptr)) {
                    let src_py = py_objs[si].clone_ref(py);
                    let tgt_py = py_objs[ti].clone_ref(py);
                    let py_link = Py::new(py, PySemLink {
                        typ:        link_type_str(link.typ).to_string(),
                        source:     src_py,
                        target:     tgt_py,
                        question:   link.question.clone(),
                        preposition: link.preposition.clone(),
                        is_or:      link.is_or,
                    })?;
                    // --- pass 3: back-populate links_from / links_to ---
                    py_objs[si].borrow_mut(py).links_from_data.push(py_link.clone_ref(py));
                    py_objs[ti].borrow_mut(py).links_to_data.push(py_link.clone_ref(py));
                    py_links.push(py_link);
                }
            }

            py_frags.push(Py::new(py, PySemFragment {
                begin_char:  frag.begin_char,
                end_char:    frag.end_char,
                objects_data: py_objs,
                links_data:   py_links,
            })?);
        }

        // Cross-fragment links
        let mut py_fraglinks: Vec<Py<PySemFraglink>> = Vec::new();
        let frag_ptr_to_idx: HashMap<*const std::cell::RefCell<pullenti_semantic::sem_document::SemFragment>, usize> =
            block.fragments.iter().enumerate()
                .map(|(i, f)| (Rc::as_ptr(f), i))
                .collect();
        for fl in &block.links {
            if let (Some(src), Some(tgt)) = (&fl.source, &fl.target) {
                if let (Some(&si), Some(&ti)) = (
                    frag_ptr_to_idx.get(&Rc::as_ptr(src)),
                    frag_ptr_to_idx.get(&Rc::as_ptr(tgt)),
                ) {
                    py_fraglinks.push(Py::new(py, PySemFraglink {
                        typ:        fraglinktype_str(fl.typ).to_string(),
                        source_idx: si,
                        target_idx: ti,
                    })?);
                }
            }
        }

        py_blocks.push(Py::new(py, PySemBlock {
            fragments_data: py_frags,
            fraglinks_data: py_fraglinks,
        })?);
    }

    Py::new(py, PySemDocument { blocks_data: py_blocks })
}

// ── Processor ────────────────────────────────────────────────────────────────

/// NER processor — run ``analyze()`` to extract entities from text.
#[pyclass(name = "Processor")]
pub struct PyProcessor {
    inner: Processor,
}

#[pymethods]
impl PyProcessor {
    /// Create a Processor.
    ///
    /// Two usage patterns:
    ///
    ///   1. ``Processor(lang='ru', analyzers=[PersonAnalyzer(), GeoAnalyzer()])``
    ///      — initializes morphology and creates a ready-to-use processor.
    ///   2. ``Processor()`` — empty processor (for use with ``Sdk.create_processor()``).
    #[new]
    #[pyo3(signature = (lang=None, analyzers=None))]
    fn new(lang: Option<&str>, analyzers: Option<Vec<Bound<'_, PyAny>>>) -> PyResult<Self> {
        match (lang, analyzers) {
            (Some(l), Some(list)) => {
                let morph_lang = MorphLang::try_parse(l)
                    .unwrap_or(MorphLang::RU | MorphLang::EN);
                let rust_analyzers: Result<Vec<_>, _> = list.iter()
                    .map(|a| analyzer_from_bound(a))
                    .collect();
                Ok(PyProcessor { inner: Processor::new(morph_lang, rust_analyzers?) })
            }
            (Some(l), None) => {
                // lang given, no analyzers → all analyzers
                let morph_lang = MorphLang::try_parse(l)
                    .unwrap_or(MorphLang::RU | MorphLang::EN);
                Ok(PyProcessor { inner: Processor::all(morph_lang) })
            }
            (None, Some(list)) => {
                // analyzers given, no lang → default lang (RU + EN)
                let rust_analyzers: Result<Vec<_>, _> = list.iter()
                    .map(|a| analyzer_from_bound(a))
                    .collect();
                Ok(PyProcessor { inner: Processor::new(MorphLang::RU | MorphLang::EN, rust_analyzers?) })
            }
            (None, None) => {
                // Empty processor (backward compat with Sdk.create_processor() pattern)
                Ok(PyProcessor { inner: Processor::empty() })
            }
        }
    }

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

    /// Analyze text with NER and semantic analysis.
    ///
    /// Returns a :class:`SemDocument` with blocks → fragments → objects and links.
    #[pyo3(signature = (text, lang=None))]
    fn analyze_semantic(
        &self,
        py: Python<'_>,
        text: &str,
        lang: Option<Bound<'_, PyMorphLang>>,
    ) -> PyResult<Py<PySemDocument>> {
        let rust_lang = lang.as_ref().map(|l| l.borrow().to_rust());
        let sofa = SourceOfAnalysis::new(text.to_string());
        let ar = self.inner.process(sofa, rust_lang);
        let doc = semantic_service::process(&ar, None);
        // ar is no longer needed; convert the semantic document to Python types
        drop(ar);
        convert_sem_document(py, doc)
    }

    fn __repr__(&self) -> String {
        let n = self.inner.analyzer_count();
        if n == 0 {
            "Processor()".to_string()
        } else {
            format!("Processor({} analyzers)", n)
        }
    }
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

// ── PersonNormalData ──────────────────────────────────────────────────────────

/// Result of ``analyze_person_name()``.
///
/// Fields:
///
///   - ``lastname``       — surname (Фамилия), or ``None``
///   - ``firstname``      — first name (Имя), or ``None``
///   - ``middlename``     — patronymic (Отчество), or ``None``
///   - ``firstname_alt``  — original short/informal form (e.g. ``"ВАСЯ"`` when
///                          ``firstname`` was expanded to ``"ВАСИЛИЙ"``), or ``None``
///   - ``gender``         — 1 = male, 2 = female, 0 = unknown
///   - ``coef``           — confidence 0–100
///   - ``result_type``    — ``"OK"`` | ``"Manual"`` | ``"NotPerson"`` | ``"Undefined"``
///   - ``error_message``  — diagnostic string when not recognized, or ``None``
#[pyclass(name = "PersonNormalData")]
pub struct PyPersonNormalData {
    #[pyo3(get)] pub lastname:      Option<String>,
    #[pyo3(get)] pub firstname:     Option<String>,
    #[pyo3(get)] pub middlename:    Option<String>,
    #[pyo3(get)] pub firstname_alt: Option<String>,
    #[pyo3(get)] pub gender:        i32,
    #[pyo3(get)] pub coef:          i32,
    #[pyo3(get)] pub result_type:   String,
    #[pyo3(get)] pub error_message: Option<String>,
}

#[pymethods]
impl PyPersonNormalData {
    fn __repr__(&self) -> String {
        format!(
            "PersonNormalData(result_type={:?}, coef={}, lastname={:?}, firstname={:?}, middlename={:?}, gender={})",
            self.result_type, self.coef,
            self.lastname, self.firstname, self.middlename, self.gender
        )
    }
}

/// Parse a short text (≤200 chars) expected to contain a Russian person name (ФИО).
///
/// Requires ``Sdk.initialize_all()`` (or at minimum
/// ``Sdk.initialize_with(analyzers=[PersonAnalyzer(), GeoAnalyzer(), OrgAnalyzer()])``
/// to have been called first.
///
/// Returns a :class:`PersonNormalData` describing the result.
///
/// Example::
///
///   from pullentipy import Sdk, analyze_person_name
///   Sdk.initialize_all()
///   d = analyze_person_name("Иванов Иван Иванович")
///   print(d.lastname, d.firstname, d.middlename)  # ИВАНОВ ИВАН ИВАНОВИЧ
#[pyfunction]
fn py_analyze_person_name(text: &str) -> PyPersonNormalData {
    let d = analyze_person_name(text);
    let result_type = match d.res_typ {
        PersonNormalResult::OK        => "OK",
        PersonNormalResult::Manual    => "Manual",
        PersonNormalResult::NotPerson => "NotPerson",
        PersonNormalResult::Undefined => "Undefined",
    }.to_string();
    PyPersonNormalData {
        lastname:      d.lastname,
        firstname:     d.firstname,
        middlename:    d.middlename,
        firstname_alt: d.firstname_alt,
        gender:        d.gender,
        coef:          d.coef,
        result_type,
        error_message: d.error_message,
    }
}

// ── Morphology ────────────────────────────────────────────────────────────────

/// One word form variant for a morphological token.
///
/// Fields:
///   - ``normal``      — normalised (dictionary) form of this variant
///   - ``pos``         — part-of-speech string: ``"noun"``, ``"verb"``, ``"adjective"``,
///                       ``"adverb"``, ``"pronoun"``, ``"preposition"``, ``"conjunction"``,
///                       ``"misc"`` or ``""``
///   - ``case``        — case string, e.g. ``"именит."``, ``"родит."`` … or ``""``
///   - ``gender``      — ``"masc"``, ``"fem"``, ``"neut"`` or ``""``
///   - ``number``      — ``"sg"``, ``"pl"`` or ``""``
///   - ``is_proper``   — True if the form is a proper noun/name
///   - ``in_dict``     — True if the form was found in the morphological dictionary
#[pyclass(name = "MorphForm")]
#[derive(Clone)]
pub struct PyMorphForm {
    #[pyo3(get)] pub normal:    String,
    #[pyo3(get)] pub pos:       String,
    #[pyo3(get)] pub case:      String,
    #[pyo3(get)] pub gender:    String,
    #[pyo3(get)] pub number:    String,
    #[pyo3(get)] pub is_proper: bool,
    #[pyo3(get)] pub in_dict:   bool,
}

#[pymethods]
impl PyMorphForm {
    fn __repr__(&self) -> String {
        format!("MorphForm(normal={:?}, pos={:?}, case={:?})", self.normal, self.pos, self.case)
    }
}

fn morph_class_str(cls: MorphClass) -> &'static str {
    if cls.is_noun()        { "noun" }
    else if cls.is_verb()   { "verb" }
    else if cls.is_adjective() { "adjective" }
    else if cls.is_adverb() { "adverb" }
    else if cls.is_pronoun() { "pronoun" }
    else if cls.is_preposition() { "preposition" }
    else if cls.is_conjunction() { "conjunction" }
    else if cls.is_misc()   { "misc" }
    else { "" }
}

fn morph_case_str(case: MorphCase) -> String {
    case.to_string()
}

fn morph_gender_str(g: MorphGenderFlags) -> &'static str {
    if (g.0 & MorphGenderFlags::MASCULINE.0) != 0 { "masc" }
    else if (g.0 & MorphGenderFlags::FEMINIE.0) != 0 { "fem" }
    else if (g.0 & MorphGenderFlags::NEUTER.0) != 0 { "neut" }
    else { "" }
}

fn morph_number_str(n: MorphNumber) -> &'static str {
    if n == MorphNumber::SINGULAR { "sg" }
    else if n == MorphNumber::PLURAL { "pl" }
    else { "" }
}

/// A morphologically-analysed text token.
///
/// Fields:
///   - ``text``        — surface text of the token
///   - ``begin_char``  — start character offset (inclusive)
///   - ``end_char``    — end character offset (inclusive)
///   - ``lemma``       — canonical lemma (best form), or ``None``
///   - ``forms``       — list of :class:`MorphForm` (all alternative parses)
#[pyclass(name = "MorphToken")]
pub struct PyMorphToken {
    #[pyo3(get)] pub text:       String,
    #[pyo3(get)] pub begin_char: i32,
    #[pyo3(get)] pub end_char:   i32,
    #[pyo3(get)] pub lemma:      Option<String>,
    pub forms_data: Vec<PyMorphForm>,
}

#[pymethods]
impl PyMorphToken {
    #[getter]
    fn forms(&self, py: Python<'_>) -> PyResult<Vec<Py<PyMorphForm>>> {
        self.forms_data.iter().map(|f| Py::new(py, f.clone())).collect()
    }
    fn __repr__(&self) -> String {
        format!("MorphToken({:?}, lemma={:?})", self.text, self.lemma)
    }
}

/// Perform morphological analysis on *text* and return a list of :class:`MorphToken`.
///
/// Requires ``Sdk.initialize_all()`` or at minimum ``Sdk.initialize(lang='ru')`` to
/// have been called first.
///
/// Example::
///
///   from pullentipy import Sdk, morph_analyze
///   Sdk.initialize_all()
///   for tok in morph_analyze("красные дома стоят"):
///       print(tok.text, tok.lemma, [(f.pos, f.case) for f in tok.forms])
#[pyfunction]
#[pyo3(signature = (text, lang=None))]
fn morph_analyze(
    py: Python<'_>,
    text: &str,
    lang: Option<Bound<'_, PyMorphLang>>,
) -> PyResult<Vec<Py<PyMorphToken>>> {
    let rust_lang = lang.as_ref().map(|l| l.borrow().to_rust());
    let tokens = MorphologyService::process(text, rust_lang)
        .unwrap_or_default();

    let mut result = Vec::with_capacity(tokens.len());
    for mt in tokens {
        let lemma = if mt.get_lemma().is_empty() { None } else { Some(mt.get_lemma()) };
        let surface = mt.get_source_text(text).to_string();
        let forms_data: Vec<PyMorphForm> = mt.word_forms.as_deref().unwrap_or(&[]).iter().map(|wf| {
            let normal = wf.normal_full.clone()
                .or_else(|| wf.normal_case.clone())
                .unwrap_or_default();
            PyMorphForm {
                normal,
                pos:       morph_class_str(wf.base.class).to_string(),
                case:      morph_case_str(wf.base.case),
                gender:    morph_gender_str(wf.base.gender).to_string(),
                number:    morph_number_str(wf.base.number).to_string(),
                is_proper: wf.base.class.is_proper(),
                in_dict:   wf.is_in_dictionary(),
            }
        }).collect();
        result.push(Py::new(py, PyMorphToken {
            text:       surface,
            begin_char: mt.begin_char,
            end_char:   mt.end_char,
            lemma,
            forms_data,
        })?);
    }
    Ok(result)
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
    m.add_class::<PyPersonNormalData>()?;
    m.add_function(wrap_pyfunction!(py_analyze_person_name, m)?)?;
    // Morphology types
    m.add_class::<PyMorphForm>()?;
    m.add_class::<PyMorphToken>()?;
    m.add_function(wrap_pyfunction!(morph_analyze, m)?)?;
    m.add_class::<PyMorphLang>()?;
    m.add_class::<PyOccurrence>()?;
    m.add_class::<PySlot>()?;
    m.add_class::<PyReferent>()?;
    m.add_class::<PyAnalysisResult>()?;
    // Semantic types
    m.add_class::<PySemObject>()?;
    m.add_class::<PySemLink>()?;
    m.add_class::<PySemFragment>()?;
    m.add_class::<PySemFraglink>()?;
    m.add_class::<PySemBlock>()?;
    m.add_class::<PySemDocument>()?;
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
