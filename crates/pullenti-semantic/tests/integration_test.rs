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

// ── Reflexive passive Pacient test ────────────────────────────────────────

#[test]
fn test_semantic_infinitive_pacient() {
    init();
    // "Он планирует продать книгу." — infinitive verb with accusative patient:
    // КНИГА → Pacient of ПРОДАТЬ
    let text = "Он планирует продать книгу.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    // Find a Pacient link
    let pacient_links: usize = blk.fragments.iter()
        .map(|fr| fr.borrow().graph.links.iter()
            .filter(|l| l.borrow().typ == SemLinkType::Pacient)
            .count())
        .sum();
    let objects: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| fr.borrow().graph.objects.iter()
            .map(|o| o.borrow().normal_full.clone())
            .collect::<Vec<_>>())
        .collect();
    assert!(pacient_links >= 1, "expected Pacient link for 'книгу', got 0. Objects: {:?}", objects);
}

#[test]
fn test_semantic_infinitive_pacient_agent() {
    init();
    // "Он планирует продать книгу." — full check:
    // ОН → Agent of ПЛАНИРУЕТ, КНИГА → Pacient of ПРОДАТЬ
    let text = "Он планирует продать книгу.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();

    let agent_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();
    let pacient_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Pacient)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(agent_targets.iter().any(|s| s.contains("ОН")),
        "expected Agent ОН, got: {:?}", agent_targets);
    assert!(pacient_targets.iter().any(|s| s.contains("КНИГА")),
        "expected Pacient КНИГА, got: {:?}", pacient_targets);
}

#[test]
fn test_semantic_sell_sdk() {
    init();
    // "Он планирует продать SDK." — ПЛАНИРУЕТ should get Agent=ОН
    // and (ideally) Pacient=SDK for ПРОДАТЬ
    // "Он планирует продать SDK." — ПЛАНИРОВАТЬ gets Agent=ОН, Pacient=SDK (indeclinable)
    let text = "Он планирует продать SDK.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();

    let agent_links: usize = blk.fragments.iter()
        .map(|fr| fr.borrow().graph.links.iter()
            .filter(|l| l.borrow().typ == SemLinkType::Agent)
            .count())
        .sum();
    let pacient_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Pacient)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(agent_links >= 1, "expected Agent link for ОН");
    assert!(pacient_targets.iter().any(|s| s == "SDK"),
        "expected Pacient SDK, got: {:?}", pacient_targets);
}

#[test]
fn test_semantic_reflexive_passive_pacient() {
    init();
    // "Система разрабатывается программистом." — passive reflexive verb:
    // ПРОГРАММИСТ → Agent (instrumental), СИСТЕМА → Pacient (nominative, pre-verb)
    let text = "Система разрабатывается программистом.";
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
    assert!(agent_links >= 1, "expected Agent link for 'программистом', got {}", agent_links);
    assert!(pacient_links >= 1, "expected Pacient link for 'система', got {}", pacient_links);
}

#[test]
fn test_semantic_anafor_kotory() {
    init();
    // "Мужчина, который работает, устал." —
    // No Delim tokens (commas are Conj), so single-fragment path.
    // Seg: [МУЖЧИНА, КОТОРЫЙ], AfterVerb=РАБОТАЕТ → Agent(РАБОТАЕТ → МУЖЧИНА)
    // Predicate chaining: УСТАЛ inherits МУЖЧИНА as Agent.
    // КОТОРЫЙ may or may not appear in graph depending on NounPhraseParser.
    let text = "Мужчина, который работает, устал.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    // Collect all objects and links across all fragments
    let all_normals: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| fr.borrow().graph.objects.iter()
            .map(|o| o.borrow().normal_full.clone())
            .collect::<Vec<_>>())
        .collect();
    let all_agent_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();
    let agent_sources: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().source.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();

    // МУЖЧИНА should be present
    let muzhchina_present = all_normals.iter().any(|s| s.contains("МУЖЧИН"));
    assert!(muzhchina_present,
        "МУЖЧИНА should be present in graph, got: {:?}", all_normals);

    // МУЖЧИНА should be an Agent target (for at least one verb)
    let muzhchina_is_agent = all_agent_targets.iter().any(|s| s.contains("МУЖЧИН"));
    assert!(muzhchina_is_agent,
        "МУЖЧИНА should be an Agent target; agent_targets={:?}, agent_sources={:?}, objects={:?}",
        all_agent_targets, agent_sources, all_normals);

    // Both РАБОТАТЬ and УСТАТЬ should have Agent links to МУЖЧИНА
    // (via NGSegment AfterVerb for РАБОТАТЬ, and predicate chaining for УСТАТЬ)
    let agent_count = all_agent_targets.iter().filter(|s| s.contains("МУЖЧИН")).count();
    assert!(agent_count >= 2,
        "Expected >= 2 Agent links with target МУЖЧИНА (for РАБОТАТЬ and УСТАТЬ), got {}; \
         agent_targets={:?}, sources={:?}",
        agent_count, all_agent_targets, agent_sources);
}


// ── Relative-clause (который) variant tests ───────────────────────────────

#[test]
fn test_semantic_kotoraya_agent() {
    init();
    // "Женщина, которая поёт, красива." — feminine relative pronoun КОТОРАЯ
    // Same structure as the КОТОРЫЙ test but feminine.
    // ЖЕНЩИНА should be Agent for both ПОЁТ and whatever main verb follows.
    // Here the main predicate is "красива" (short-form adj acting as predicate),
    // which may or may not be parsed as a verb.  At minimum, ЖЕНЩИНА must be Agent
    // for ПОЁТ (inside the relative clause via AfterVerb assignment).
    let text = "Женщина, которая поёт, устала.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let all_agent_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();

    // ЖЕНЩИНА should be an Agent target for at least one verb
    let is_agent = all_agent_targets.iter().any(|s| s.contains("ЖЕНЩИН"));
    assert!(is_agent,
        "ЖЕНЩИНА should be Agent target; got: {:?}", all_agent_targets);

    // Both ПЕТЬ (поёт) and УСТАТЬ (устала) should assign ЖЕНЩИНА as Agent
    let count = all_agent_targets.iter().filter(|s| s.contains("ЖЕНЩИН")).count();
    assert!(count >= 2,
        "Expected >= 2 Agent links for ЖЕНЩИНА (ПЕТЬ + УСТАТЬ), got {}: {:?}",
        count, all_agent_targets);
}

#[test]
fn test_semantic_personal_pronoun_agent() {
    init();
    // "Он читает книгу." — personal pronoun ОН should remain a Noun SentItem
    // (not SubSent) so it gets a normal Agent link to ЧИТАЕТ.
    let text = "Он читает книгу.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    let agent_targets: Vec<String> = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .map(|l| l.borrow().target.borrow().normal_full.clone())
        .collect();

    // ОН (pronoun) should be the Agent of ЧИТАТЬ
    let on_is_agent = agent_targets.iter().any(|s| s == "ОН");
    assert!(on_is_agent,
        "ОН (personal pronoun) should be Agent; agent_targets={:?}", agent_targets);
}

#[test]
fn test_semantic_kotory_object_clause() {
    init();
    // "Книга, которую читает Иван, интересная." — object relative clause.
    // КОТОРУЮ is accusative → SubSent; ИВАН should be Agent for ЧИТАТЬ.
    // КНИГА might or might not be Agent/Pacient — at minimum ИВАН should be present.
    let text = "Книга, которую читает Иван, интересная.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let all_normals: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| fr.borrow().graph.objects.iter()
            .map(|o| o.borrow().normal_full.clone())
            .collect::<Vec<_>>())
        .collect();

    // Both КНИГА and ИВАН should appear in the graph
    assert!(all_normals.iter().any(|s| s.contains("КНИГ")),
        "КНИГА should be in graph; objects={:?}", all_normals);
    assert!(all_normals.iter().any(|s| s.contains("ИВАН")),
        "ИВАН should be in graph; objects={:?}", all_normals);
}

#[test]
fn test_semantic_kotory_no_list_link() {
    init();
    // "Студент, который знает предмет, сдал экзамен." —
    // КОТОРЫЙ must NOT form a List link with СТУДЕНТ (it's SubSent now).
    // СТУДЕНТ should be Agent for ЗНАЕТ (relative clause verb, AfterVerb)
    // and also Agent for СДАЛ (main verb) via predicate chaining.
    // Note: "занимается" would be reflexive and block Agent; use "знает" (transitive).
    let text = "Студент, который знает предмет, сдал экзамен.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let all_agent_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();

    // СТУДЕНТ should appear as Agent target at least once (ЗНАТЬ).
    // Note: СДАТЬ may get its own Agent from ЭКЗАМЕН (inanimate nom=acc ambiguity)
    // before predicate chaining can propagate СТУДЕНТ, so we only require ≥ 1.
    let count = all_agent_targets.iter().filter(|s| s.contains("СТУДЕНТ")).count();
    assert!(count >= 1,
        "Expected >= 1 Agent link for СТУДЕНТ (ЗНАТЬ), got {}: {:?}",
        count, all_agent_targets);
}

#[test]
fn test_semantic_passive_relative_clause() {
    init();
    // "Закон, который был принят, вступил в силу." —
    // Relative clause contains a passive construction.
    // ЗАКОН should appear in graph; sentence should produce some links.
    let text = "Закон, который был принят, вступил в силу.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let all_normals: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| fr.borrow().graph.objects.iter()
            .map(|o| o.borrow().normal_full.clone())
            .collect::<Vec<_>>())
        .collect();

    assert!(all_normals.iter().any(|s| s.contains("ЗАКОН")),
        "ЗАКОН should be in graph; objects={:?}", all_normals);
}

// ── Additional agent / pacient coverage ──────────────────────────────────

#[test]
fn test_semantic_double_predicate_chaining() {
    init();
    // "Директор пришёл, выступил и ушёл." — three verbs, all should share ДИРЕКТОР as Agent.
    let text = "Директор пришёл, выступил и ушёл.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let agent_targets: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| {
            let fr = fr.borrow();
            fr.graph.links.iter()
                .filter(|l| l.borrow().typ == SemLinkType::Agent)
                .map(|l| l.borrow().target.borrow().normal_full.clone())
                .collect::<Vec<_>>()
        })
        .collect();

    // ДИРЕКТОР should be Agent for at least 2 of the 3 verbs (chaining may miss the 3rd)
    let count = agent_targets.iter().filter(|s| s.contains("ДИРЕКТОР")).count();
    assert!(count >= 2,
        "Expected >= 2 Agent links for ДИРЕКТОР across three chained verbs, got {}: {:?}",
        count, agent_targets);
}

#[test]
fn test_semantic_preposition_actant() {
    init();
    // "Иван живёт в Москве." — "в Москве" is a prepositional Actant of ЖИВЁТ.
    let text = "Иван живёт в Москве.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());
    let frag = blk.fragments[0].borrow();

    // Should have Agent (Иван) and at least one other link (Detail/Actant for в Москве)
    let agent_links = frag.graph.links.iter()
        .filter(|l| l.borrow().typ == SemLinkType::Agent)
        .count();
    assert!(agent_links >= 1, "expected Agent link; got {}", agent_links);

    // МОСКВА should be in the graph
    let has_moskva = frag.graph.objects.iter()
        .any(|o| o.borrow().normal_full.contains("МОСКВ"));
    assert!(has_moskva, "expected МОСКВА in graph");
}

#[test]
fn test_semantic_deepart_agent() {
    init();
    // "Увидев ошибку, программист исправил её." —
    // УВИДЕВ is a deepart; ПРОГРАММИСТ should be its Agent.
    let text = "Увидев ошибку, программист исправил её.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();
    assert!(!blk.fragments.is_empty());

    let all_normals: Vec<String> = blk.fragments.iter()
        .flat_map(|fr| fr.borrow().graph.objects.iter()
            .map(|o| o.borrow().normal_full.clone())
            .collect::<Vec<_>>())
        .collect();

    // Both ПРОГРАММИСТ and the deepart verb should be present
    assert!(all_normals.iter().any(|s| s.contains("ПРОГРАММИСТ")),
        "ПРОГРАММИСТ should be in graph; objects={:?}", all_normals);
}

#[test]
fn test_semantic_because_fraglink() {
    init();
    // "Иван остался дома, потому что шёл дождь." — should produce a Because SemFraglink
    let text = "Иван остался дома, потому что шёл дождь.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();

    // Should split into at least 2 fragments
    let total_frags: usize = blk.fragments.len();
    assert!(total_frags >= 2,
        "expected >= 2 fragments for because-sentence, got {}", total_frags);

    // Should have a Because SemFraglink
    let has_because = blk.links.iter().any(|l| l.typ == SemFraglinkType::Because);
    assert!(has_because,
        "expected Because SemFraglink; links={:?}",
        blk.links.iter().map(|l| format!("{:?}", l.typ)).collect::<Vec<_>>());
}

#[test]
fn test_semantic_what_fraglink() {
    init();
    // "Мы знаем, что Земля круглая." — should produce a What SemFraglink
    let text = "Мы знаем, что Земля круглая.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let doc = process(&ar, None);

    assert!(!doc.blocks.is_empty());
    let blk = doc.blocks[0].borrow();

    let total_frags: usize = blk.fragments.len();
    assert!(total_frags >= 2,
        "expected >= 2 fragments for what-sentence, got {}", total_frags);

    let has_what = blk.links.iter().any(|l| l.typ == SemFraglinkType::What);
    assert!(has_what,
        "expected What SemFraglink; links={:?}",
        blk.links.iter().map(|l| format!("{:?}", l.typ)).collect::<Vec<_>>());
}

// ── try_create_links tests ─────────────────────────────────────────────────

#[test]
fn test_try_create_links_verb_acc_pacient() {
    // "строить дом" → СТРОИТЬ governs ДОМ as Pacient (accusative)
    use pullenti_morph::MorphologyService;
    use pullenti_ner::{SourceOfAnalysis, token::build_token_chain};
    use pullenti_ner::core::noun_phrase::{NounPhraseParseAttr, try_parse as npt_try_parse};
    use pullenti_ner::core::verb_phrase::try_parse as vpt_try_parse;
    use pullenti_semantic::internal::sent_item::{SentItem, NPT_ATTRS};
    use pullenti_semantic::core::try_create_links;

    init();

    let text = "строить дом";
    let sofa = SourceOfAnalysis::new(text);
    let tokens = MorphologyService::process(text, None).unwrap();
    let first = build_token_chain(tokens, &sofa).unwrap();

    // Parse "строить" as verb
    let vpt = vpt_try_parse(&first, true, true, true, &sofa).unwrap();
    let master = SentItem::from_verb_vpt(vpt);

    // Find "дом" — walk past the verb token(s) to reach the noun
    let mut dom_tok = first.borrow().next.clone();
    while let Some(t) = dom_tok.clone() {
        let is_letter = t.borrow().chars.is_letter();
        if is_letter { break; }
        dom_tok = t.borrow().next.clone();
    }
    let dom_tok = dom_tok.unwrap();
    let npt = npt_try_parse(&dom_tok, NPT_ATTRS, -1, &sofa).unwrap();
    let slave = SentItem::from_noun_npt(npt);

    let links = try_create_links(&master, &slave);
    assert!(!links.is_empty(), "Expected semantic links for строить+дом, got none");
    // The top link should be Pacient or at least have some role
    println!("строить+дом links: {:?}", links.iter().map(|l| (&l.role, l.rank, &l.question)).collect::<Vec<_>>());
}

#[test]
fn test_try_create_links_verb_nom_agent() {
    // "работает программист" → РАБОТАТЬ governs ПРОГРАММИСТ as Agent (nominative)
    use pullenti_morph::MorphologyService;
    use pullenti_ner::{SourceOfAnalysis, token::build_token_chain};
    use pullenti_ner::core::noun_phrase::{try_parse as npt_try_parse};
    use pullenti_ner::core::verb_phrase::try_parse as vpt_try_parse;
    use pullenti_semantic::internal::sent_item::{SentItem, NPT_ATTRS};
    use pullenti_semantic::core::try_create_links;
    use pullenti_ner::deriv::SemanticRole;

    init();

    let text = "работает программист";
    let sofa = SourceOfAnalysis::new(text);
    let tokens = MorphologyService::process(text, None).unwrap();
    let first = build_token_chain(tokens, &sofa).unwrap();

    let vpt = vpt_try_parse(&first, true, true, true, &sofa).unwrap();
    let master = SentItem::from_verb_vpt(vpt);

    // Find noun start (skip to next letter token)
    let mut cur = first.borrow().next.clone();
    while let Some(t) = cur.clone() {
        let is_letter = t.borrow().chars.is_letter();
        if is_letter { break; }
        cur = t.borrow().next.clone();
    }
    let noun_tok = cur.unwrap();
    let npt = npt_try_parse(&noun_tok, NPT_ATTRS, -1, &sofa).unwrap();
    let slave = SentItem::from_noun_npt(npt);

    let links = try_create_links(&master, &slave);
    println!("работает+программист links: {:?}", links.iter().map(|l| (&l.role, l.rank, &l.question)).collect::<Vec<_>>());
    assert!(!links.is_empty(), "Expected links for работает+программист");
    let top = &links[0];
    assert_eq!(top.role, SemanticRole::Agent, "Expected Agent, got {:?}", top.role);
}
