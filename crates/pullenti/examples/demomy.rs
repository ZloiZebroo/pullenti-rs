/// Pullenti SDK — Rust demo.
///
/// Mirrors the C# Demo/Program.cs — demonstrates the full NLP pipeline:
/// morphology, NER (named entity recognition), and semantic analysis.
///
/// Run with:
///   cargo run -p pullenti --example demo
use std::sync::Arc;
use pullenti::pullenti_morph::MorphLang;
use pullenti::pullenti_ner::{Sdk, ProcessorService, SourceOfAnalysis};
// use pullenti::pullenti_ner::core::noun_phrase::{try_parse as npt_try_parse, NounPhraseParseAttr};
use pullenti::pullenti_ner::token::{TokenKind};
// use pullenti::pullenti_semantic::{initialize as sem_initialize, process as sem_process};

use pullenti::pullenti_morph::{MorphologyService};
// use pullenti::pullenti_ner::{ProcessorService, SourceOfAnalysis};
use pullenti::pullenti_ner::analyzer::Analyzer;  // уточните путь
use pullenti::pullenti_ner::person::PersonAnalyzer;
use pullenti::pullenti_ner::address::AddressAnalyzer;
use pullenti::pullenti_ner::geo::GeoAnalyzer;
use std::fs;
use std::time::Instant; 

fn main() {
    // ── 1. Initialize SDK (morph + all analyzers) ─────────────────────────

    print!("Initializing Pullenti SDK v{} ... ", Sdk::VERSION);
    let init_start = Instant::now();  // ← Замер инициализации
    // Sdk::initialize_all(Some(MorphLang::RU));
    // sem_initialize();

    // 1. Инициализируем только морфологию (RU)
    MorphologyService::initialize(Some(MorphLang::RU));

    // 2. Регистрируем только 3 анализатора в глобальном реестре
    //    (это единственный способ, если ProcessorService — синглтон)
    for analyzer in [
        Arc::new(PersonAnalyzer::new()) as Arc<dyn Analyzer>,
        Arc::new(AddressAnalyzer::new()) as Arc<dyn Analyzer>,
        Arc::new(GeoAnalyzer::new()) as Arc<dyn Analyzer>,
    ] {
        ProcessorService::register_analyzer(analyzer);
    }

    // 3. Создаём процессор и запускаем
    let proc = ProcessorService::create_processor();
    println!("OK ({:.2} ms)", init_start.elapsed().as_secs_f64() * 1000.0); 
    // println!("OK");

    // MorphologyService::initialize(Some(MorphLang::RU));
    // use crate::person::PersonAnalyzer;
    // use crate::address::AddressAnalyzer;
    // use crate::geo::GeoAnalyzer;
    // let proc = Processor::with_analyzers(vec![Arc::new(PhoneAnalyzer::new())]);

    // Show registered analyzers
    // let analyzers = proc::analyzers();
    // println!("Registered analyzers ({}):", analyzers.len());
    // for a in &analyzers {
        // println!("  [{:8}]  {}", a.name(), a.caption());
    // }

    // ── 2. Demo text ──────────────────────────────────────────────────────

    // let file_path = "/Users/admin/Downloads/dialoge_1.txt";

    // let txt = fs::read_to_string(file_path)
        // .expect("Should have been able to read the file");

    let txt = String::from("Система разрабатывается в городе Казань. в Казань. я еду в Петербург. я еду в Санкт-Петербург.");

//         FAILED tests.py::test_masking_chat[3iTA-test_data/test_chat/natasha.csv] - AssertionError: Failed {'Казань'} , not found in ['Казань'], masked text: В {ADDRESS_1}
// FAILED tests.py::test_masking_chat[MaskingFacade-test_data/test_chat/natasha.csv] - AssertionError: Failed {'Казань'} , not found in ['Казань'], masked text: В {ADDRESS_1}
// FAILED tests.py::test_masking_chat[BTEST-test_data/test_chat/natasha.csv] - AssertionError: Failed {'Казань'} , not found in ['Казань'], masked text: В {ADDRESS_1}
// FAILED tests.py::test_masking_chat[MaskingTypeFull-test_data/test_chat/natasha.csv] - AssertionError: Failed {'Петербург'} , not found in ['Санкт-Петербург'], masked text: Санкт-{ADDRESS_1}

    // println!("\n── Text ──────────────────────────────────────────────────────────────");
    // println!("{}", txt);

    // ── 3. NER processing ─────────────────────────────────────────────────

    let ner_start = Instant::now();  // ← Замер NER
    let sofa = SourceOfAnalysis::new(&txt);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let ner_elapsed = ner_start.elapsed();
    println!("NER completed in {:.2} ms", ner_elapsed.as_secs_f64() * 1000.0);

    println!("\n── Entities ({}) ─────────────────────────────────────────────────────", ar.entities.len());
    for ent in &ar.entities {
        let e = ent.borrow();
        println!("{}: {}", e.type_name, display_referent(&e));
        for slot in &e.slots {
            let val_str = match &slot.value {
                Some(pullenti::pullenti_ner::referent::SlotValue::Str(s)) => s.clone(),
                Some(pullenti::pullenti_ner::referent::SlotValue::Referent(r)) => {
                    format!("[{}]", r.borrow().type_name)
                }
                None => "(none)".to_string(),
            };
            println!("   {}: {}", slot.type_name, val_str);
        }
    }

    // ── 4. Noun groups (from NER token chain) ─────────────────────────────

    // println!("\n── Noun groups ───────────────────────────────────────────────────────");
    // let mut cur = ar.first_token.clone();
    // while let Some(t) = cur {
    //     // Skip referent tokens (already recognized entities)
    //     let is_ref = matches!(t.borrow().kind, TokenKind::Referent(_));
    //     let next = t.borrow().next.clone();
    //     if !is_ref {
    //         let npt = npt_try_parse(&t, NounPhraseParseAttr::AdjectiveCanBeLast, 0, &ar.sofa);
    //         if let Some(npt) = npt {
    //             // Print source text and normalized form
    //             let begin = npt.begin_token.borrow().begin_char;
    //             let end   = npt.end_token.borrow().end_char;
    //             let surface = ar.sofa.substring(begin, end);
    //             // Get normalized nominative case text
    //             let normal = get_npt_normal(&npt);
    //             println!("  [{}] => [{}]", surface, normal);
    //             let after = npt.end_token.borrow().next.clone();
    //             cur = after;
    //             continue;
    //         }
    //     }
    //     cur = next;
    // }

    // ── 5. Semantic analysis ──────────────────────────────────────────────

    // println!("\n── Semantic analysis ─────────────────────────────────────────────────");
    // let sofa2 = SourceOfAnalysis::new(txt);
    // let proc2 = ProcessorService::create_processor();
    // let ar2   = proc2.process(sofa2, None);
    // let doc = sem_process(&ar2, None);

    // println!("Blocks: {}", doc.blocks.len());
    // for (bi, blk_ref) in doc.blocks.iter().enumerate() {
    //     let blk = blk_ref.borrow();
    //     println!("  Block {}: {} fragment(s)", bi, blk.fragments.len());
    //     for (fi, frag_ref) in blk.fragments.iter().enumerate() {
    //         let frag = frag_ref.borrow();
    //         let g = &frag.graph;
    //         println!("    Fragment {} [{}-{}]: {} objects, {} links",
    //             fi, frag.begin_char, frag.end_char,
    //             g.objects.len(), g.links.len());
    //         for obj_ref in &g.objects {
    //             let obj = obj_ref.borrow();
    //             println!("      {:?} \"{}\"", obj.typ, obj.normal);
    //         }
    //         for lnk_ref in &g.links {
    //             let lnk  = lnk_ref.borrow();
    //             let from = lnk.source.borrow();
    //             let to   = lnk.target.borrow();
    //             let q    = lnk.question.as_deref().unwrap_or("");
    //             println!("      {:?}: \"{}\" --[{}]-> \"{}\"",
    //                 lnk.typ, from.normal, q, to.normal);
    //         }
    //     }
    // }

    println!("\nDone.");
}

/// Produce a simple display string for a Referent.
fn display_referent(r: &pullenti::pullenti_ner::referent::Referent) -> String {
    // Collect all string slots
    let parts: Vec<String> = r.slots.iter()
        .filter_map(|s| {
            if let Some(pullenti::pullenti_ner::referent::SlotValue::Str(v)) = &s.value {
                Some(format!("{}={}", s.type_name, v))
            } else {
                None
            }
        })
        .collect();
    if parts.is_empty() {
        r.type_name.clone()
    } else {
        parts.join(", ")
    }
}

/// Get the normalized nominative case text for a noun phrase.
fn get_npt_normal(npt: &pullenti::pullenti_ner::core::noun_phrase::NounPhraseToken) -> String {
    use pullenti::pullenti_ner::core::noun_phrase::NounPhraseSpan;
    let mut parts = Vec::new();
    // Preposition
    if let Some(ref prep) = npt.preposition {
        let pb = prep.begin_token.borrow();
        if let TokenKind::Text(ref txt) = pb.kind {
            parts.push(txt.term.clone());
        }
    }
    // Adjectives
    for adj in &npt.adjectives {
        parts.push(span_normal_case(adj));
    }
    // Noun
    if let Some(ref noun) = npt.noun {
        parts.push(span_normal_case(noun));
    }
    if parts.is_empty() {
        // Fallback: raw source
        let begin = npt.begin_token.borrow().begin_char;
        let end   = npt.end_token.borrow().end_char;
        // We don't have sofa here, just return empty
        format!("[{}-{}]", begin, end)
    } else {
        parts.join(" ")
    }
}

/// Get normalized (nominative) text for a NounPhraseSpan.
fn span_normal_case(span: &pullenti::pullenti_ner::core::noun_phrase::NounPhraseSpan) -> String {
    let tb = span.begin_token.borrow();
    for wf in tb.morph.items() {
        if let Some(ref nc) = wf.normal_case { return nc.clone(); }
        if let Some(ref nf) = wf.normal_full { return nf.clone(); }
    }
    if let TokenKind::Text(ref txt) = tb.kind {
        txt.term.clone()
    } else {
        String::new()
    }
}
