"""
pullentipy — Python bindings for the Pullenti NLP SDK.

Quick start::

    from pullentipy import Sdk, MorphLang, PersonAnalyzer, GeoAnalyzer, OrgAnalyzer

    # Option A — all built-in analyzers
    Sdk.initialize_all(lang='ru')

    # Option B — specific analyzers only
    Sdk.initialize_with(lang='ru', analyzers=[PersonAnalyzer(), GeoAnalyzer(), OrgAnalyzer()])

    proc = Sdk.create_processor()
    result = proc.analyze("Иван Петров живёт в Москве")
    for r in result.referents:
        print(r.entity_type, r.text)
        for s in r.slots:
            print(" ", s.name, "=", s.value)

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
