use std::time::Instant;
use std::sync::Arc;
use pullenti_morph::{MorphologyService, MorphLang};
use pullenti_ner::{Sdk, ProcessorService, SourceOfAnalysis, Processor, Analyzer};

fn main() {
    let start = Instant::now();
    Sdk::initialize_all(Some(MorphLang::RU));
    eprintln!("Init: {:?}", start.elapsed());

    let proc = ProcessorService::create_processor();

    let medium = "Иванов Иван Петрович родился 15 января 1985 года в городе Москве. \
        Работает в ООО «Рога и Копыта» с 2010 года. \
        Телефон: +7 (999) 123-45-67. Email: ivanov@example.com. \
        Адрес: г. Москва, ул. Ленина, д. 5, кв. 4. \
        Петрова Мария Ивановна работает в ПАО «Газпром». \
        ГОСТ 12345-2020 определяет стандарты. \
        Автомобиль Toyota Camry зарегистрирован на имя Сидорова.";

    // Large text
    let large = (0..10).map(|_| medium).collect::<Vec<_>>().join("\n");
    eprintln!("Text size: {} chars", large.chars().count());

    // With ALL analyzers (including Link)
    let start2 = Instant::now();
    for _ in 0..10 {
        let sofa = SourceOfAnalysis::new(&large);
        let _result = proc.process(sofa, Some(MorphLang::RU));
    }
    eprintln!("All analyzers  (10x): {:?} ({:?}/iter)", start2.elapsed(), start2.elapsed() / 10);

    // Without Link
    let proc_no_link = Processor::with_analyzers(vec![
        Arc::new(pullenti_ner::PhoneAnalyzer::new()),
        Arc::new(pullenti_ner::UriAnalyzer::new()),
        Arc::new(pullenti_ner::DateAnalyzer::new()),
        Arc::new(pullenti_ner::MoneyAnalyzer::new()),
        Arc::new(pullenti_ner::MeasureAnalyzer::new()),
        Arc::new(pullenti_ner::GeoAnalyzer::new()),
        Arc::new(pullenti_ner::PersonAnalyzer::new()),
        Arc::new(pullenti_ner::OrgAnalyzer::new()),
        Arc::new(pullenti_ner::NamedEntityAnalyzer::new()),
        Arc::new(pullenti_ner::AddressAnalyzer::new()),
        Arc::new(pullenti_ner::TransportAnalyzer::new()),
        Arc::new(pullenti_ner::DecreeAnalyzer::new()),
    ]);
    let start3 = Instant::now();
    for _ in 0..10 {
        let sofa = SourceOfAnalysis::new(&large);
        let _result = proc_no_link.process(sofa, Some(MorphLang::RU));
    }
    eprintln!("No Link (10x): {:?} ({:?}/iter)", start3.elapsed(), start3.elapsed() / 10);

    // Just morph + token build (baseline)
    let proc_empty = Processor::with_analyzers(vec![]);
    let start4 = Instant::now();
    for _ in 0..10 {
        let sofa = SourceOfAnalysis::new(&large);
        let _result = proc_empty.process(sofa, Some(MorphLang::RU));
    }
    eprintln!("Morph only (10x): {:?} ({:?}/iter)", start4.elapsed(), start4.elapsed() / 10);

    // Per-analyzer profiling
    eprintln!("\n--- Per-analyzer (large text, single run) ---");
    let analyzer_sets: Vec<(&str, Vec<Arc<dyn Analyzer>>)> = vec![
        ("Phone", vec![Arc::new(pullenti_ner::PhoneAnalyzer::new())]),
        ("URI", vec![Arc::new(pullenti_ner::UriAnalyzer::new())]),
        ("Date", vec![Arc::new(pullenti_ner::DateAnalyzer::new())]),
        ("Geo", vec![Arc::new(pullenti_ner::GeoAnalyzer::new())]),
        ("Person", vec![Arc::new(pullenti_ner::GeoAnalyzer::new()), Arc::new(pullenti_ner::PersonAnalyzer::new())]),
        ("Org", vec![Arc::new(pullenti_ner::OrgAnalyzer::new())]),
        ("Address", vec![Arc::new(pullenti_ner::AddressAnalyzer::new())]),
        ("Named", vec![Arc::new(pullenti_ner::NamedEntityAnalyzer::new())]),
        ("Transport", vec![Arc::new(pullenti_ner::TransportAnalyzer::new())]),
        ("Decree", vec![Arc::new(pullenti_ner::DecreeAnalyzer::new())]),
    ];

    for (name, analyzers) in analyzer_sets {
        let p = Processor::with_analyzers(analyzers);
        let start = Instant::now();
        let sofa = SourceOfAnalysis::new(&large);
        let result = p.process(sofa, Some(MorphLang::RU));
        eprintln!("  {:12} {:?} ({} entities)", name, start.elapsed(), result.entities.len());
    }
}
