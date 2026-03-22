/// Deriv subsystem — DerivateService, DerivateGroup, DerivateWord, ControlModel, etc.
/// These live in pullenti-ner so that VerbPhraseHelper (NER) can use them
/// without creating a circular dependency with pullenti-semantic.

pub mod explan_word_attr;
pub mod control_model;
pub mod deriv_word;
pub mod deriv_group;
pub mod explan_tree_node;
pub mod deriv_dict;
pub mod deriv_service;

pub use explan_word_attr::ExplanWordAttr;
pub use control_model::{
    SemanticRole, QuestionType, ControlModelItemType,
    ControlModelQuestion, ControlModelItem, ControlModel,
    items as control_model_questions, get_by_id, find_by_spel,
};
pub use deriv_word::DerivateWord;
pub use deriv_group::DerivateGroup;
pub use deriv_dict::DerivateDictionary;
pub use deriv_service::{
    initialize as deriv_initialize,
    find_derivate_group_ids, find_groups_cloned, is_animated, is_named, find_words,
    for_each_word, find_verb_role,
};
