/// Semantic Core module — SemanticLink, SemanticHelper, ISemanticOnto.
/// Mirrors `Pullenti/Semantic/Core/`.

pub mod semantic_link;
pub mod semantic_helper;

pub use semantic_link::{SemanticLink, SemanticRole};
pub use semantic_helper::{try_create_links, get_keyword, check_morph_accord};
