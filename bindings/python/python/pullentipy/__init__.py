"""
pullentipy — Python bindings for the Pullenti NLP SDK.

Quick start::

    from pullentipy import Processor, PersonAnalyzer, GeoAnalyzer

    # Option A — all built-in analyzers
    proc = Processor(lang='ru')
    result = proc.analyze("Иван Петров живёт в Москве")
    for r in result:
        print(r.entity_type, r.text)
        print(r["FIRSTNAME"])   # dict-like slot access

    # Option B — specific analyzers only
    proc = Processor(lang='ru', analyzers=[PersonAnalyzer(), GeoAnalyzer()])

    # Option C — legacy Sdk API (still works)
    from pullentipy import Sdk
    Sdk.initialize_all(lang='ru')
    proc = Sdk.create_processor()
    result = proc.analyze("Иван живёт в Москве")

Batch processing (parallel, uses all CPU cores)::

    proc = Processor(lang='ru')
    results = proc.analyze_batch(["Иван в Москве", "Петров в Казани"])

Person name normalization::

    from pullentipy import Sdk, analyze_person_name
    Sdk.initialize_all()
    d = analyze_person_name("Иванов И.И.")
    print(d.lastname, d.firstname, d.coef, d.result_type)

Morphological analysis::

    from pullentipy import Sdk, morph_analyze
    Sdk.initialize_all()
    for tok in morph_analyze("красные дома стоят"):
        print(tok.text, tok.lemma, [(f.pos, f.case) for f in tok.forms])

Semantic analysis::

    proc = Processor(lang='ru')
    doc = proc.analyze_semantic("Иван работает в Москве")
    for block in doc.blocks:
        for frag in block.fragments:
            for link in frag.links:
                print(link.typ, link.source.normal, "->", link.target.normal)
"""

from pullentipy._pullentipy import (
    PersonNormalData,
    py_analyze_person_name as analyze_person_name,
    # Morphology
    MorphForm,
    MorphToken,
    morph_analyze,
    MorphLang,
    Occurrence,
    Slot,
    Referent,
    AnalysisResult,
    # Semantic types
    SemObject,
    SemLink,
    SemFragment,
    SemFraglink,
    SemBlock,
    SemDocument,
    Processor,
    Sdk,
    PhoneAnalyzer,
    UriAnalyzer,
    DateAnalyzer,
    MoneyAnalyzer,
    MeasureAnalyzer,
    GeoAnalyzer,
    PersonAnalyzer,
    OrgAnalyzer,
    NamedEntityAnalyzer,
    AddressAnalyzer,
    TransportAnalyzer,
    DecreeAnalyzer,
    BankAnalyzer,
    WeaponAnalyzer,
    ChemicalAnalyzer,
    VacanceAnalyzer,
    DenominationAnalyzer,
    MailAnalyzer,
    KeywordAnalyzer,
    DefinitionAnalyzer,
    ResumeAnalyzer,
    InstrumentAnalyzer,
    TitlePageAnalyzer,
    GoodsAnalyzer,
    BookLinkAnalyzer,
    LinkAnalyzer,
)

__all__ = [
    "PersonNormalData",
    "analyze_person_name",
    "MorphForm",
    "MorphToken",
    "morph_analyze",
    "MorphLang",
    "Occurrence",
    "Slot",
    "Referent",
    "AnalysisResult",
    "SemObject",
    "SemLink",
    "SemFragment",
    "SemFraglink",
    "SemBlock",
    "SemDocument",
    "Processor",
    "Sdk",
    "PhoneAnalyzer",
    "UriAnalyzer",
    "DateAnalyzer",
    "MoneyAnalyzer",
    "MeasureAnalyzer",
    "GeoAnalyzer",
    "PersonAnalyzer",
    "OrgAnalyzer",
    "NamedEntityAnalyzer",
    "AddressAnalyzer",
    "TransportAnalyzer",
    "DecreeAnalyzer",
    "BankAnalyzer",
    "WeaponAnalyzer",
    "ChemicalAnalyzer",
    "VacanceAnalyzer",
    "DenominationAnalyzer",
    "MailAnalyzer",
    "KeywordAnalyzer",
    "DefinitionAnalyzer",
    "ResumeAnalyzer",
    "InstrumentAnalyzer",
    "TitlePageAnalyzer",
    "GoodsAnalyzer",
    "BookLinkAnalyzer",
    "LinkAnalyzer",
]
