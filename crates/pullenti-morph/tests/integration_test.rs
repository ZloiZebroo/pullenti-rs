use pullenti_morph::{MorphologyService, MorphLang};

#[test]
fn test_initialize() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let langs = MorphologyService::loaded_languages();
    assert!(langs.is_ru(), "Russian should be loaded");
}

#[test]
fn test_tokenize() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let tokens = MorphologyService::tokenize("Привет мир");
    assert!(tokens.is_some());
    let tokens = tokens.unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].term.as_deref(), Some("ПРИВЕТ"));
    assert_eq!(tokens[1].term.as_deref(), Some("МИР"));
}

#[test]
fn test_process_russian() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let tokens = MorphologyService::process("Москва столица России", None);
    assert!(tokens.is_some());
    let tokens = tokens.unwrap();
    assert_eq!(tokens.len(), 3);
    // Each token should have word forms
    for tok in &tokens {
        println!("Token: {:?}, forms: {:?}", tok.term, tok.word_forms.as_ref().map(|wf| wf.len()));
    }
}

#[test]
fn test_process_english() {
    MorphologyService::initialize(Some(MorphLang::EN));
    let tokens = MorphologyService::process("Hello world", None);
    assert!(tokens.is_some());
    let tokens = tokens.unwrap();
    assert_eq!(tokens.len(), 2);
}
