/// Enum types for the semantic subsystem.

// ── SemObjectType ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemObjectType {
    #[default]
    Undefined,
    Noun,
    Adjective,
    Verb,
    Participle,
    Adverb,
    Pronoun,
    PersonalPronoun,
    Question,
}

// ── SemFragmentType ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemFragmentType {
    #[default]
    Undefined,
}

// ── SemLinkType ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemLinkType {
    #[default]
    Undefined,
    Detail,
    Naming,
    Agent,
    Pacient,
    Participle,
    Anafor,
}

// ── SemFraglinkType ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemFraglinkType {
    #[default]
    Undefined,
    IfThen,
    IfElse,
    Because,
    But,
    For,
    What,
}

// ── SemAttributeType ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemAttributeType {
    #[default]
    Undefined,
    Very,
    Already,
    Still,
    All,
    Any,
    Some,
    One,
    OneOf,
    Other,
    EachOther,
    Himself,
    Whole,
    Less,
    Great,
}

// ── SemProcessParams ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct SemProcessParams {
    pub dont_create_anafor: bool,
    pub max_char:           usize,
}

// ── SemAttribute ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct SemAttribute {
    pub typ:     SemAttributeType,
    pub spelling: String,
    pub not:     bool,
}

impl std::fmt::Display for SemAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.spelling)
    }
}
