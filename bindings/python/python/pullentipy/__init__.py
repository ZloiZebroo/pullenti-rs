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
"""

from pullentipy._pullentipy import (
    MorphLang,
    Occurrence,
    Slot,
    Referent,
    AnalysisResult,
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
)

__all__ = [
    "MorphLang",
    "Occurrence",
    "Slot",
    "Referent",
    "AnalysisResult",
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
]
