/// SemanticLink + SemanticRole.
/// Mirrors `SemanticLink.cs` and `SemanticRole.cs`.

use pullenti_ner::token::TokenRef;
use pullenti_ner::deriv::control_model::ControlModelQuestion;

// Re-export SemanticRole from deriv module
pub use pullenti_ner::deriv::control_model::SemanticRole;

/// A semantic relation between two phrases (master → slave).
#[derive(Clone, Debug)]
pub struct SemanticLink {
    /// The governing element (verb phrase or noun phrase)
    pub master:    Option<TokenRef>,
    /// The governed element
    pub slave:     Option<TokenRef>,
    /// The syntactic question (e.g. "кого/что?")
    pub question:  Option<String>,
    pub role:      SemanticRole,
    pub is_passive: bool,
    pub rank:      f64,
    pub modelled:  bool,
    pub idiom:     bool,
}

impl SemanticLink {
    pub fn new() -> Self {
        SemanticLink {
            master:    None,
            slave:     None,
            question:  None,
            role:      SemanticRole::Common,
            is_passive: false,
            rank:      0.0,
            modelled:  false,
            idiom:     false,
        }
    }
}

impl std::fmt::Display for SemanticLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modelled { write!(f, "?")?; }
        if self.idiom    { write!(f, "!")?; }
        if self.role != SemanticRole::Common { write!(f, "{:?}: ", self.role)?; }
        if self.is_passive { write!(f, "Passive ")?; }
        if self.rank > 0.0 { write!(f, "{} ", self.rank)?; }
        if let Some(ref q) = self.question { write!(f, "{}? ", q)?; }
        write!(f, "[master] <- [slave]")
    }
}
