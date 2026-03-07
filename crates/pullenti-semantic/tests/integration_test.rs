/// Integration tests for pullenti-semantic

use pullenti_morph::{MorphologyService, MorphLang};
use pullenti_ner::{Sdk, ProcessorService, SourceOfAnalysis};
use pullenti_semantic::{initialize, process, SemProcessParams, SemObjectType, SemLinkType, SemFraglinkType};

fn init() {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Sdk::initialize_all(Some(MorphLang::RU | MorphLang::EN));
    initialize();
}

// ── Smoke test: process returns a document ────────────────────────────────

#[test]
fn test_semantic_smoke() {
    init();
    let text = "Иван Петров работает в компании ООО «Ромашка».";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let doc = process(&ar, None);

    // Document should have at least one block with at least one fragment
    assert!(!doc.blocks.is_empty(), "expected at least one block");
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty(), "expected at least one fragment");

    // Fragment should cover the sentence
    let frag = blk.fragments[0].borrow();
    assert!(frag.begin_char < frag.end_char, "fragment should have non-zero span");
}

#[test]
fn test_semantic_multisentence() {
    init();
    let text = "Солнце светит ярко. Погода хорошая.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let doc = process(&ar, None);

    // Should have at least 1 block with at least 1 fragment per sentence
    assert!(!doc.blocks.is_empty(), "expected blocks");
    let total_frags: usize = doc.blocks.iter()
        .map(|b| b.borrow().fragments.len())
        .sum();
    assert!(total_frags >= 1, "expected at least one fragment, got {}", total_frags);
}

#[test]
fn test_semantic_empty() {
    init();
    let text = "";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);
    // Empty text should produce empty doc
    assert!(doc.blocks.is_empty(), "empty text should produce no blocks");
}

#[test]
fn test_deriv_service() {
    // Test that the DerivateService loads and works
    init();
    use pullenti_ner::deriv::deriv_service;
    use pullenti_morph::MorphLang;

    // "РАБОТА" should be found (common Russian noun)
    let found = deriv_service::find_words("РАБОТА", MorphLang::RU);
    assert!(!found.is_empty(), "РАБОТА should have deriv entries");

    // "РАБОТАТЬ" should also be found
    let ids = deriv_service::find_derivate_group_ids("РАБОТАТЬ", true, MorphLang::RU);
    assert!(!ids.is_empty(), "РАБОТАТЬ should have deriv group");
}

// ── SemObject creation tests ──────────────────────────────────────────────

#[test]
fn test_semantic_noun_objects() {
    init();
    // Simple noun phrase sentence — should produce Noun SemObjects
    let text = "Красный дом стоит на горе.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty(), "should have at least one block");
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty(), "should have fragments");
    let frag = blk.fragments[0].borrow();
    // Should have at least one SemObject (noun phrase found)
    assert!(!frag.graph.objects.is_empty(),
        "should have SemObjects in graph, got 0");
    // At least one should be Noun or Adjective type
    let has_noun_or_adj = frag.graph.objects.iter().any(|o| {
        let ot = o.borrow().typ;
        ot == SemObjectType::Noun || ot == SemObjectType::Adjective
    });
    assert!(has_noun_or_adj, "expected Noun or Adjective SemObject");
}

#[test]
fn test_semantic_verb_objects() {
    init();
    // Simple SVO sentence — should produce Verb SemObject
    let text = "Компания производит товары.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty(), "should have at least one block");
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty(), "should have fragments");
    let frag = blk.fragments[0].borrow();
    let has_verb = frag.graph.objects.iter().any(|o| {
        o.borrow().typ == SemObjectType::Verb
    });
    assert!(has_verb, "expected Verb SemObject in graph");
}

#[test]
fn test_semantic_agent_link() {
    init();
    // SVO: "Компания производит товары." — verb should have Agent link
    let text = "Компания производит товары.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    let frag = blk.fragments[0].borrow();
    // Should have at least one link (Agent or Pacient)
    let total_links: usize = frag.graph.links.len();
    assert!(total_links > 0,
        "expected semantic links, got {}; objects: {}",
        total_links, frag.graph.objects.len());
}

// ── Subsent / SemFraglink tests ───────────────────────────────────────────

#[test]
fn test_semantic_if_then_fraglink() {
    init();
    // "Если дождь, то Иван возьмёт зонт." — should produce 2 fragments with IfThen link
    let text = "Если дождь, то Иван возьмёт зонт.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty(), "expected at least one block");
    let blk = doc.blocks[0].borrow();

    // Should have at least 2 fragments (one per sub-sentence)
    assert!(blk.fragments.len() >= 2,
        "expected >= 2 fragments for if-then sentence, got {}", blk.fragments.len());

    // Should have a SemFraglink of type IfThen
    let has_if_then = blk.links.iter().any(|l| l.typ == SemFraglinkType::IfThen);
    assert!(has_if_then, "expected an IfThen SemFraglink, got links: {:?}",
        blk.links.iter().map(|l| format!("{:?}", l.typ)).collect::<Vec<_>>());
}

#[test]
fn test_semantic_but_fraglink() {
    init();
    // "Иван работает, но Пётр отдыхает." — should produce But SemFraglink
    let text = "Иван работает, но Пётр отдыхает.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();

    // Should have at least 2 fragments
    assert!(blk.fragments.len() >= 2,
        "expected >= 2 fragments for 'but' sentence, got {}", blk.fragments.len());

    // Should have a SemFraglink of type But
    let has_but = blk.links.iter().any(|l| l.typ == SemFraglinkType::But);
    assert!(has_but, "expected a But SemFraglink, links: {:?}",
        blk.links.iter().map(|l| format!("{:?}", l.typ)).collect::<Vec<_>>());
}

#[test]
fn test_semantic_no_split_on_list_conj() {
    init();
    // "Иван и Пётр работают." — И is in a noun list; should NOT split into 2 sub-sentences
    let text = "Иван и Пётр работают.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    // No Delim tokens in this sentence → single fragment
    assert_eq!(blk.fragments.len(), 1,
        "list conjunction should NOT split the sentence, got {} fragments", blk.fragments.len());
}

#[test]
fn test_semantic_list_expansion() {
    init();
    // "Иван и Пётр работают." — both should get Agent links to работают
    let text = "Иван и Пётр работают.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Count Agent links — should be at least 1 (ideally 2 for list expansion)
    let agent_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .count();
    assert!(agent_links >= 1,
        "expected at least 1 Agent link for 'Иван и Пётр работают', got {}", agent_links);
}

// ── Genitive / Detail link tests ──────────────────────────────────────────

#[test]
fn test_semantic_genitive_detail() {
    init();
    // "Книга учителя." — should produce a Detail("чего") link from книга to учитель
    let text = "Книга учителя.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty(), "expected at least one block");
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty(), "expected fragments");
    let frag = blk.fragments[0].borrow();

    // Should have at least 2 objects (книга + учитель)
    assert!(frag.graph.objects.len() >= 2,
        "expected >= 2 objects, got {}", frag.graph.objects.len());

    // Should have at least one Detail link
    let detail_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Detail)
        .count();
    assert!(detail_links >= 1,
        "expected at least 1 Detail link for genitive, got {}", detail_links);
}

#[test]
fn test_semantic_adjective_detail() {
    init();
    // "Красный автомобиль стоит." — adjective should have Detail link to noun
    let text = "Красный автомобиль стоит.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Should have at least: автомобиль (Noun), красный (Adjective), стоит (Verb)
    let has_adj = frag.graph.objects.iter().any(|o| {
        o.borrow().typ == SemObjectType::Adjective
    });
    assert!(has_adj, "expected Adjective SemObject for 'Красный'");

    // There should be a Detail link from автомобиль to красный
    let detail_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Detail)
        .count();
    assert!(detail_links >= 1,
        "expected Detail link for adjective, got {}", detail_links);

    // Should have Agent link (стоит ← автомобиль)
    let agent_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .count();
    assert!(agent_links >= 1, "expected Agent link, got {}", agent_links);
}

// ── Pacient link tests ─────────────────────────────────────────────────────

#[test]
fn test_semantic_pacient_link() {
    init();
    // "Строители строят дом." — should have Agent=строители AND Pacient=дом
    let text = "Строители строят дом.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    let agent_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .count();
    let pacient_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Pacient)
        .count();
    assert!(agent_links >= 1, "expected Agent link for 'Строители строят дом', got {}", agent_links);
    assert!(pacient_links >= 1, "expected Pacient link for 'дом', got {}", pacient_links);
}

// ── Multi-block tests ──────────────────────────────────────────────────────

#[test]
fn test_semantic_two_sentences() {
    init();
    // Two separate sentences on separate lines → two blocks
    let text = "Иван читает книгу.\nМария пишет письмо.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    // Should have at least 2 blocks
    assert!(doc.blocks.len() >= 2,
        "expected >= 2 blocks for two newline-separated sentences, got {}", doc.blocks.len());
    for blk_rc in &doc.blocks {
        let blk = blk_rc.borrow();
        assert!(!blk.fragments.is_empty(), "each block should have fragments");
    }
}

// ── Negation test ──────────────────────────────────────────────────────────

#[test]
fn test_semantic_negation() {
    init();
    // "Компания не производит товары." — not flag on verb
    let text = "Компания не производит товары.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Should have a Verb object and some links
    let has_verb = frag.graph.objects.iter().any(|o| o.borrow().typ == SemObjectType::Verb);
    assert!(has_verb, "expected Verb SemObject");
    // At minimum there should be some objects
    assert!(frag.graph.objects.len() >= 2,
        "expected multiple SemObjects, got {}", frag.graph.objects.len());
}

// ── Adverb Detail link tests ───────────────────────────────────────────────

#[test]
fn test_semantic_adverb_detail_link() {
    init();
    // "Иван быстро бежит." — adverb "быстро" should have Detail link from "бежит"
    let text = "Иван быстро бежит.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Should have at least one Detail link (adverb → verb)
    let detail_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Detail)
        .count();
    assert!(detail_links >= 1,
        "expected Detail link for manner adverb 'быстро', got {}; objects: {}",
        detail_links, frag.graph.objects.len());
}

// ── Predicate chaining test ───────────────────────────────────────────────

#[test]
fn test_semantic_predicate_chaining() {
    init();
    // "Иван бежит и прыгает." — both verbs should get Agent=Иван
    let text = "Иван бежит и прыгает.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Both бежит and прыгает should have Agent links to Иван
    let agent_links: usize = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .count();
    assert!(agent_links >= 2,
        "expected >= 2 Agent links (one per verb in 'Иван бежит и прыгает'), got {}",
        agent_links);
}

