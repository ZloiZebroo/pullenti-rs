"""Type stubs for the pullentipy._pullentipy Rust extension module."""

from __future__ import annotations
from typing import Optional, List, Union

# ── MorphLang ─────────────────────────────────────────────────────────────────

class MorphLang:
    """Morphology language selector.

    Use the class attributes::

        lang = MorphLang.RU
        lang = MorphLang.EN
    """

    UNKNOWN: MorphLang
    RU: MorphLang
    UA: MorphLang
    BY: MorphLang
    EN: MorphLang

    @property
    def value(self) -> int:
        """Raw bit-mask value."""
        ...

    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

# ── Occurrence ────────────────────────────────────────────────────────────────

class Occurrence:
    """A single text span where a :class:`Referent` appears in the document."""

    @property
    def begin_char(self) -> int:
        """Character offset of the first character (inclusive, Unicode scalar)."""
        ...

    @property
    def end_char(self) -> int:
        """Character offset of the last character (inclusive, Unicode scalar)."""
        ...

    @property
    def text(self) -> str:
        """Source text for this span."""
        ...

    def __repr__(self) -> str: ...

# ── Slot ──────────────────────────────────────────────────────────────────────

class Slot:
    """A named attribute on a :class:`Referent`."""

    @property
    def name(self) -> str:
        """Attribute name, e.g. ``"FIRSTNAME"``, ``"VALUE"``, ``"TYPE"``."""
        ...

    @property
    def value(self) -> str:
        """String representation of the attribute value."""
        ...

    def __repr__(self) -> str: ...

# ── Referent ──────────────────────────────────────────────────────────────────

class Referent:
    """A recognized named entity extracted from text."""

    @property
    def entity_type(self) -> str:
        """Entity type name, e.g. ``"PERSON"``, ``"GEO"``, ``"ORGANIZATION"``."""
        ...

    @property
    def text(self) -> str:
        """Source text of the first occurrence."""
        ...

    @property
    def begin_char(self) -> int:
        """Character offset of the first character of the first occurrence (inclusive)."""
        ...

    @property
    def end_char(self) -> int:
        """Character offset of the last character of the first occurrence (inclusive)."""
        ...

    @property
    def summary(self) -> str:
        """Full entity string including all slots, mirrors C# ``Referent.ToString()``."""
        ...

    @property
    def slots(self) -> List[Slot]:
        """All named attribute slots on this entity."""
        ...

    @property
    def occurrences(self) -> List[Occurrence]:
        """All text spans where this entity appears in the document, in order."""
        ...

    def get(self, name: str) -> Optional[str]:
        """Return the first value of the slot named *name*, or ``None``."""
        ...

    def __repr__(self) -> str: ...

# ── AnalysisResult ────────────────────────────────────────────────────────────

class AnalysisResult:
    """Result of :meth:`Processor.analyze`."""

    @property
    def referents(self) -> List[Referent]:
        """All recognized entities in document order."""
        ...

    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

# ── Processor ─────────────────────────────────────────────────────────────────

class Processor:
    """NER processor — created by :meth:`Sdk.create_processor`."""

    def analyze(
        self,
        text: str,
        lang: Optional[MorphLang] = None,
    ) -> AnalysisResult:
        """Analyze *text* and return the recognized entities.

        Args:
            text: Input text.
            lang: Morphology language hint (default: auto-detect).

        Returns:
            :class:`AnalysisResult` containing all found entities.
        """
        ...

    def __repr__(self) -> str: ...

# ── Analyzer types ────────────────────────────────────────────────────────────

class PhoneAnalyzer:
    """Analyzer for phone numbers."""
    def __init__(self) -> None: ...

class UriAnalyzer:
    """Analyzer for URIs and bank requisite codes."""
    def __init__(self) -> None: ...

class DateAnalyzer:
    """Analyzer for dates and time expressions."""
    def __init__(self) -> None: ...

class MoneyAnalyzer:
    """Analyzer for monetary amounts."""
    def __init__(self) -> None: ...

class MeasureAnalyzer:
    """Analyzer for physical measurements."""
    def __init__(self) -> None: ...

class GeoAnalyzer:
    """Analyzer for geographic locations (countries, regions, cities)."""
    def __init__(self) -> None: ...

class PersonAnalyzer:
    """Analyzer for person names."""
    def __init__(self) -> None: ...

class OrgAnalyzer:
    """Analyzer for organization names."""
    def __init__(self) -> None: ...

class NamedEntityAnalyzer:
    """Analyzer for well-known named entities (rivers, planets, monuments…)."""
    def __init__(self) -> None: ...

class AddressAnalyzer:
    """Analyzer for postal addresses."""
    def __init__(self) -> None: ...

class TransportAnalyzer:
    """Analyzer for transport names (vehicles, aircraft, vessels)."""
    def __init__(self) -> None: ...

class DecreeAnalyzer:
    """Analyzer for laws, orders, and standards (ГОСТ, ISO, ФЗ…)."""
    def __init__(self) -> None: ...

class BankAnalyzer:
    """Analyzer for bank requisite blocks (ИНН, КПП, Р/С, БИК…)."""
    def __init__(self) -> None: ...

class WeaponAnalyzer:
    """Analyzer for weapon references."""
    def __init__(self) -> None: ...

# Union of all analyzer types (for type-checking ``Sdk.register_analyzer`` calls)
AnyAnalyzer = Union[
    PhoneAnalyzer, UriAnalyzer, DateAnalyzer, MoneyAnalyzer,
    MeasureAnalyzer, GeoAnalyzer, PersonAnalyzer, OrgAnalyzer,
    NamedEntityAnalyzer, AddressAnalyzer, TransportAnalyzer,
    DecreeAnalyzer, BankAnalyzer, WeaponAnalyzer,
]

# ── Sdk ───────────────────────────────────────────────────────────────────────

class Sdk:
    """Top-level SDK entry point (static methods only).

    Typical usage — all analyzers::

        Sdk.initialize_all(lang='ru')
        proc = Sdk.create_processor()

    Typical usage — selected analyzers::

        Sdk.initialize_with(lang='ru', analyzers=[PersonAnalyzer(), GeoAnalyzer()])
        proc = Sdk.create_processor()
    """

    @staticmethod
    def initialize(lang: Optional[str] = None) -> None:
        """Initialize morphology only (no analyzers registered).

        Args:
            lang: Language code string, e.g. ``"ru"``, ``"en"``, ``"ru;en"``.
        """
        ...

    @staticmethod
    def initialize_all(lang: Optional[str] = None) -> None:
        """Initialize morphology **and** register all 14 built-in analyzers.

        Args:
            lang: Language code string (default: all supported languages).
        """
        ...

    @staticmethod
    def initialize_with(
        lang: Optional[str] = None,
        analyzers: Optional[List[AnyAnalyzer]] = None,
    ) -> None:
        """Initialize morphology and register **only** the supplied analyzers.

        Args:
            lang:      Language code string.
            analyzers: List of analyzer instances to register.

        Example::

            Sdk.initialize_with(
                lang='ru',
                analyzers=[PersonAnalyzer(), GeoAnalyzer(), OrgAnalyzer()],
            )
        """
        ...

    @staticmethod
    def register_analyzer(analyzer: AnyAnalyzer) -> None:
        """Register a single analyzer in the global registry.

        Can be called after :meth:`initialize` / :meth:`initialize_all`.
        """
        ...

    @staticmethod
    def create_processor() -> Processor:
        """Create a :class:`Processor` pre-loaded with all registered analyzers."""
        ...

    @staticmethod
    def version() -> str:
        """Return the SDK version string (e.g. ``"4.33"``)."""
        ...
