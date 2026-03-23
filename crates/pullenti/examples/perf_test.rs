use std::sync::Arc;
use std::time::Instant;
use pullenti_morph::{MorphLang, MorphologyService};
use pullenti_ner::processor::Processor;
use pullenti_ner::source_of_analysis::SourceOfAnalysis;

// Larger realistic text with many person names, addresses, and entities
const TEXT: &str = "\
Директор ООО «Рога и копыта» Иванов Иван Иванович 15 марта 2024 года \
подписал договор с ПАО «Газпром» о поставке оборудования на сумму 1 500 000 рублей. \
Генеральный директор Петров Алексей Сергеевич и главный бухгалтер Сидорова Мария Петровна \
присутствовали на совещании в г. Москве, ул. Тверская, д. 15, кв. 3. \
Телефон для связи: +7 (495) 123-45-67. Электронная почта: info@example.com. \
Президент Российской Федерации Владимир Владимирович Путин провёл совещание \
в Кремле с участием министра обороны Сергея Кужугетовича Шойгу. \
Контактное лицо: г-жа Козлова Анна Дмитриевна, тел. 8-800-555-35-35. \
Адрес доставки: Санкт-Петербург, Невский проспект, д. 28, оф. 412. \
Заместитель генерального директора Смирнов Дмитрий Александрович подтвердил \
передачу документов представителю АО «Транснефть» Васильеву Николаю Петровичу \
по адресу: г. Казань, ул. Баумана, д. 44, корп. 2, кв. 17. \
Профессор Московского государственного университета Фёдоров Андрей Викторович \
выступил с докладом на конференции 22 апреля 2024 года в Новосибирске. \
Полковник Кузнецов Олег Борисович получил орден Мужества за выполнение боевых задач. \
Адвокат Романова Елена Игоревна представляла интересы г-на Соколова Артёма Валерьевича \
в Арбитражном суде г. Екатеринбурга, пр. Ленина, д. 101, оф. 7. \
Депутат Государственной Думы Козлов Сергей Анатольевич внёс законопроект. \
Губернатор Краснодарского края Попов Виктор Михайлович встретился с делегацией \
из Республики Беларусь во главе с послом Лебедевым Павлом Юрьевичем. \
Главный инженер ОАО «Мосводоканал» Новиков Максим Геннадьевич представил проект \
реконструкции водозаборных сооружений стоимостью 350 000 000 рублей. \
Прокурор Волгоградской области Морозов Игорь Владимирович возбудил уголовное дело. \
Доставка по адресу: Ростов-на-Дону, ул. Большая Садовая, д. 33, стр. 1, эт. 3. \
Секретарь Совета Безопасности Зайцев Пётр Николаевич провёл закрытое заседание. \
Ректор Санкт-Петербургского университета Соловьёва Ольга Васильевна объявила набор. \
Координатор проекта Mrs. Johnson Emily Kate направила отчёт Dr. Brown Robert James. \
Контактное лицо: Sir William Henry, тел. +44 20 7946 0958, адрес: London, Baker St., д. 221B.";

fn main() {
    let lang = MorphLang::RU;
    let n = 500u32;

    // ── 0. Morph-only baseline ───────────────────────────────────────────────
    MorphologyService::initialize(Some(lang));
    for _ in 0..10 { let _ = MorphologyService::process(TEXT, Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = MorphologyService::process(TEXT, Some(lang)); }
    let per_morph = start.elapsed() / n;

    // ── 1. All analyzers ─────────────────────────────────────────────────────
    let proc_all = Processor::all(lang);
    for _ in 0..10 { let _ = proc_all.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = proc_all.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let per_all = start.elapsed() / n;

    // ── 2. Without LinkAnalyzer ──────────────────────────────────────────────
    let mut proc_no_link = Processor::all(lang);
    proc_no_link.remove_analyzer("LINK");
    for _ in 0..10 { let _ = proc_no_link.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = proc_no_link.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let per_no_link = start.elapsed() / n;

    // ── 3. Person analyzer only ──────────────────────────────────────────────
    let proc_person = Processor::new(lang, vec![
        Arc::new(pullenti_ner::person::PersonAnalyzer::new()),
    ]);
    for _ in 0..10 { let _ = proc_person.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = proc_person.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let per_person = start.elapsed() / n;

    // ── 4. Address analyzer only ─────────────────────────────────────────────
    let proc_addr = Processor::new(lang, vec![
        Arc::new(pullenti_ner::address::AddressAnalyzer::new()),
    ]);
    for _ in 0..10 { let _ = proc_addr.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = proc_addr.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let per_addr = start.elapsed() / n;

    // ── 5. Geo analyzer only ─────────────────────────────────────────────────
    let proc_geo = Processor::new(lang, vec![
        Arc::new(pullenti_ner::geo::GeoAnalyzer::new()),
    ]);
    for _ in 0..10 { let _ = proc_geo.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let start = Instant::now();
    for _ in 0..n { let _ = proc_geo.process(SourceOfAnalysis::new(TEXT), Some(lang)); }
    let per_geo = start.elapsed() / n;

    // ── Summary ──────────────────────────────────────────────────────────────
    let morph_ms = per_morph.as_secs_f64() * 1000.0;
    println!("--- Results ({} iters, {} chars) ---", n, TEXT.len());
    println!("Morph only:    {:.3}ms/iter", morph_ms);
    println!("All analyzers: {:.3}ms/iter  (analyzer overhead: {:.3}ms)", per_all.as_secs_f64() * 1000.0, per_all.as_secs_f64() * 1000.0 - morph_ms);
    println!("Without Link:  {:.3}ms/iter  (analyzer overhead: {:.3}ms)", per_no_link.as_secs_f64() * 1000.0, per_no_link.as_secs_f64() * 1000.0 - morph_ms);
    println!("Person (net):  {:.3}ms/iter  (analyzer overhead: {:.3}ms)", per_person.as_secs_f64() * 1000.0, per_person.as_secs_f64() * 1000.0 - morph_ms);
    println!("Address (net): {:.3}ms/iter  (analyzer overhead: {:.3}ms)", per_addr.as_secs_f64() * 1000.0, per_addr.as_secs_f64() * 1000.0 - morph_ms);
    println!("Geo (net):     {:.3}ms/iter  (analyzer overhead: {:.3}ms)", per_geo.as_secs_f64() * 1000.0, per_geo.as_secs_f64() * 1000.0 - morph_ms);
}
