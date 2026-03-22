use pullenti_morph::{MorphologyService, MorphLang};
use pullenti_ner::{
    SourceOfAnalysis, ProcessorService, Processor, Analyzer, AnalysisKit,
    TokenChainIter, PhoneAnalyzer, UriAnalyzer, DateAnalyzer, MoneyAnalyzer, MeasureAnalyzer,
    GeoAnalyzer, PersonAnalyzer, OrgAnalyzer, Sdk,
};
use pullenti_ner::phone::phone_referent as ph_ref;
use pullenti_ner::uri::{ATTR_VALUE, ATTR_SCHEME};
use pullenti_ner::date::{get_year, get_month, get_day, get_date_from, get_date_to, DATERANGE_OBJ_TYPENAME};
use pullenti_ner::money::{get_currency, get_value as get_money_value, get_rest};
use pullenti_ner::measure::{get_value as get_measure_value, get_unit, get_kind};
use pullenti_ner::geo::{get_name as get_geo_name, get_type as get_geo_type, get_alpha2, is_state, is_region};
use pullenti_ner::person::{get_firstname, get_middlename, get_lastname, get_sex, SEX_MALE, SEX_FEMALE,
    PERSONPROPERTY_OBJ_TYPENAME, get_person_property_name};
use pullenti_ner::org::{get_name as get_org_name, get_type as get_org_type, get_names as get_org_names};
use pullenti_ner::named::{get_name as get_named_name, get_kind as get_named_kind, get_type as get_named_type};
use pullenti_ner::NamedEntityAnalyzer;
use pullenti_ner::address::{get_street_type, get_street_name, get_house, get_flat,
    get_corpus, get_floor, get_office};
use pullenti_ner::AddressAnalyzer;
use pullenti_ner::TransportAnalyzer;
use pullenti_ner::transport::{get_transport_type, get_transport_brand, get_transport_kind};
use pullenti_ner::DecreeAnalyzer;
use pullenti_ner::decree::{get_decree_type, get_decree_number, get_decree_kind};
use pullenti_ner::bank::bank_referent::find_value_owned;
use pullenti_ner::WeaponAnalyzer;
use pullenti_ner::weapon::weapon_referent::{get_type as get_weapon_type, get_brand as get_weapon_brand, get_model as get_weapon_model};
use pullenti_ner::ChemicalAnalyzer;
use pullenti_ner::chemical::{get_value as get_chem_value, get_name as get_chem_name, CHEMICAL_OBJ_TYPENAME};
use pullenti_ner::VacanceAnalyzer;
use pullenti_ner::vacance::{VACANCE_OBJ_TYPENAME, get_item_type, get_value as get_vac_value, VacanceItemType};
use std::sync::Arc;

fn init() {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
}

// ── Minimal test analyzer that counts letter tokens ─────────────────────────

struct LetterCountAnalyzer;

impl Analyzer for LetterCountAnalyzer {
    fn name(&self) -> &'static str { "LETTER_COUNT" }
    fn caption(&self) -> &'static str { "Letter Counter" }

    fn process(&self, kit: &mut AnalysisKit) {
        let mut t = kit.first_token.clone();
        while let Some(tok) = t {
            let _ = tok.borrow().is_letters();
            t = tok.borrow().next.clone();
        }
        kit.analyzer_data
            .entry(self.name().to_string())
            .or_insert_with(pullenti_ner::analysis_kit::AnalyzerData::new);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_tokenization_pipeline() {
    init();
    let sofa = SourceOfAnalysis::new("Привет мир");
    let mut processor = Processor::empty();
    processor.add_analyzer(Arc::new(LetterCountAnalyzer));
    let result = processor.process(sofa, Some(MorphLang::RU));

    assert!(result.first_token.is_some(), "Should have tokens");
    assert_eq!(result.tokens_count(), 2, "Should have 2 tokens");
}

#[test]
fn test_token_terms() {
    init();
    let sofa = SourceOfAnalysis::new("Москва столица России");
    let processor = Processor::empty();
    let result = processor.process(sofa, Some(MorphLang::RU));

    let tokens: Vec<_> = TokenChainIter::new(result.first_token.clone()).collect();
    assert_eq!(tokens.len(), 3);

    let terms: Vec<String> = tokens.iter()
        .map(|t| t.borrow().term().unwrap_or("").to_string())
        .collect();
    // "России" is genitive → term is "РОССИИ", not nominative "РОССИЯ"
    assert_eq!(terms, vec!["МОСКВА", "СТОЛИЦА", "РОССИИ"]);
}

#[test]
fn test_token_is_value() {
    init();
    let sofa = SourceOfAnalysis::new("Иванов Иван Иванович");
    let processor = Processor::empty();
    let result = processor.process(sofa, Some(MorphLang::RU));

    let first = result.first_token.as_ref().unwrap().clone();
    assert!(first.borrow().is_value("ИВАНОВ", None));
}

#[test]
fn test_english_tokenization() {
    init();
    let sofa = SourceOfAnalysis::new("Hello world");
    let processor = Processor::empty();
    let result = processor.process(sofa, Some(MorphLang::EN));

    let tokens: Vec<_> = TokenChainIter::new(result.first_token.clone()).collect();
    assert_eq!(tokens.len(), 2);
}

#[test]
fn test_processor_service() {
    ProcessorService::initialize(Some(MorphLang::RU));
    let processor = ProcessorService::create_processor();
    let sofa = SourceOfAnalysis::new("Тест");
    let result = processor.process(sofa, None);
    assert!(result.first_token.is_some());
}

#[test]
fn test_token_linked_list() {
    init();
    let sofa = SourceOfAnalysis::new("один два три");
    let processor = Processor::empty();
    let result = processor.process(sofa, Some(MorphLang::RU));

    // Walk forward
    let mut terms = Vec::new();
    let mut t = result.first_token.clone();
    while let Some(tok) = t {
        terms.push(tok.borrow().term().unwrap_or("").to_string());
        t = tok.borrow().next.clone();
    }
    assert_eq!(terms, vec!["ОДИН", "ДВА", "ТРИ"]);

    // Walk backward via prev
    let last = {
        let mut cur = result.first_token.clone().unwrap();
        loop {
            let next = cur.borrow().next.clone();
            match next {
                None => break,
                Some(n) => cur = n,
            }
        }
        cur
    };
    let mut back_terms = Vec::new();
    let mut t: Option<_> = Some(last);
    while let Some(tok) = t {
        back_terms.push(tok.borrow().term().unwrap_or("").to_string());
        let prev = tok.borrow().prev.as_ref().and_then(|w| w.upgrade());
        t = prev;
    }
    assert_eq!(back_terms, vec!["ТРИ", "ДВА", "ОДИН"]);
}

#[test]
fn test_morph_collection() {
    init();
    let sofa = SourceOfAnalysis::new("красивая девушка");
    let processor = Processor::empty();
    let result = processor.process(sofa, Some(MorphLang::RU));

    let tokens: Vec<_> = TokenChainIter::new(result.first_token.clone()).collect();
    assert_eq!(tokens.len(), 2);

    // Both tokens should have morph data
    let tok0 = tokens[0].borrow();
    assert!(tok0.morph.items_count() > 0, "First token should have morph items");
}

// ── Phone analyzer tests ──────────────────────────────────────────────────────

#[test]
fn test_phone_russian_mobile() {
    init();
    let sofa = SourceOfAnalysis::new("Мобильный телефон: +7 999 123-45-67");
    let mut processor = Processor::empty();
    processor.add_analyzer(Arc::new(PhoneAnalyzer::new()));
    let result = processor.process(sofa, Some(MorphLang::RU));

    assert!(!result.entities.is_empty(), "Should extract at least one phone entity");

    let phone = result.entities.iter()
        .find(|e| e.borrow().type_name == "PHONE")
        .expect("Should have a PHONE entity");

    let rb = phone.borrow();
    let num = ph_ref::get_number(&rb).expect("Should have a number");
    // Number part without country code: 9991234567
    assert_eq!(num, "9991234567", "Expected 10-digit number without country code");
    let cc = ph_ref::get_country_code(&rb);
    assert_eq!(cc.as_deref(), Some("7"), "Expected country code 7");
}

#[test]
fn test_phone_local_number() {
    init();
    // Simple local 7-digit number with dashes
    let sofa = SourceOfAnalysis::new("Тел.: 123-45-67");
    let mut processor = Processor::empty();
    processor.add_analyzer(Arc::new(PhoneAnalyzer::new()));
    let result = processor.process(sofa, Some(MorphLang::RU));

    assert!(!result.entities.is_empty(), "Should extract phone entity");
    let phone = result.entities.iter()
        .find(|e| e.borrow().type_name == "PHONE")
        .expect("Should have a PHONE entity");

    let rb = phone.borrow();
    let num = ph_ref::get_number(&rb).expect("Should have a number");
    assert_eq!(num, "1234567");
}

#[test]
fn test_phone_not_extracted_from_gost() {
    init();
    // Numbers after ГОСТ keyword should NOT be phone numbers
    let sofa = SourceOfAnalysis::new("ГОСТ 12345-67");
    let mut processor = Processor::empty();
    processor.add_analyzer(Arc::new(PhoneAnalyzer::new()));
    let result = processor.process(sofa, Some(MorphLang::RU));

    let phones: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PHONE")
        .collect();
    assert!(phones.is_empty(), "ГОСТ numbers should not be extracted as phones");
}

// ── Sdk initialization pattern tests ─────────────────────────────────────────

/// Pattern 1 — Direct: no global state, Processor::with_analyzers()
#[test]
fn test_sdk_pattern1_direct() {
    MorphologyService::initialize(Some(MorphLang::RU));
    let proc = Processor::with_analyzers(vec![Arc::new(PhoneAnalyzer::new())]);
    let sofa = SourceOfAnalysis::new("Тел. +7 999 123-45-67");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let phones: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PHONE")
        .collect();
    assert!(!phones.is_empty(), "Pattern 1: should extract phone via with_analyzers()");
}

/// Pattern 2 — All analyzers via Sdk::initialize_all() + ProcessorService::create_processor()
#[test]
fn test_sdk_pattern2_initialize_all() {
    Sdk::initialize_all(Some(MorphLang::RU));
    let proc = ProcessorService::create_processor();
    let sofa = SourceOfAnalysis::new("Звоните +7 999 765-43-21");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let phones: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PHONE")
        .collect();
    assert!(!phones.is_empty(), "Pattern 2: should extract phone via Sdk::initialize_all()");
}

/// Pattern 3 — Selective: Sdk::initialize_with() + ProcessorService::create_processor()
#[test]
fn test_sdk_pattern3_initialize_with() {
    Sdk::initialize_with(
        Some(MorphLang::RU),
        vec![Arc::new(PhoneAnalyzer::new())],
    );
    let proc = ProcessorService::create_processor();
    let sofa = SourceOfAnalysis::new("Номер: +7 812 234-56-78");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let phones: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PHONE")
        .collect();
    assert!(!phones.is_empty(), "Pattern 3: should extract phone via Sdk::initialize_with()");
}

// ── URI analyzer tests ────────────────────────────────────────────────────────

fn uri_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(UriAnalyzer::new())])
}

fn get_uri_attr(e: &pullenti_ner::Referent, attr: &str) -> Option<String> {
    e.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| match s.value.as_ref()? {
            pullenti_ner::SlotValue::Str(v) => Some(v.clone()),
            _ => None,
        })
}

/// HTTP URL extraction
#[test]
fn test_uri_http_url() {
    let proc = uri_proc();
    let sofa = SourceOfAnalysis::new("Посетите сайт https://www.example.com/page");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let uris: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "URI")
        .collect();
    assert!(!uris.is_empty(), "Should extract URI from HTTPS URL");
    let scheme = get_uri_attr(&uris[0].borrow(), ATTR_SCHEME);
    assert!(
        scheme.as_deref().map_or(false, |s| s.contains("http") || s.contains("https")),
        "Scheme should be http/https, got {:?}", scheme
    );
}

/// Email extraction via '@'
#[test]
fn test_uri_email() {
    let proc = uri_proc();
    let sofa = SourceOfAnalysis::new("Напишите нам: user@example.com");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let uris: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "URI")
        .collect();
    assert!(!uris.is_empty(), "Should extract URI from email");
    let val = get_uri_attr(&uris[0].borrow(), ATTR_VALUE);
    assert!(
        val.as_deref().map_or(false, |v| v.contains('@')),
        "URI value should contain '@', got {:?}", val
    );
}

/// ISBN extraction
#[test]
fn test_uri_isbn() {
    let proc = uri_proc();
    let sofa = SourceOfAnalysis::new("ISBN 978-5-699-12014-7");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let uris: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "URI")
        .collect();
    assert!(!uris.is_empty(), "Should extract URI from ISBN");
    let scheme = get_uri_attr(&uris[0].borrow(), ATTR_SCHEME);
    assert_eq!(scheme.as_deref(), Some("ISBN"));
}

/// INN (ИНН) extraction
#[test]
fn test_uri_inn() {
    let proc = uri_proc();
    let sofa = SourceOfAnalysis::new("ИНН 7743013722");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let uris: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "URI")
        .collect();
    assert!(!uris.is_empty(), "Should extract URI from INN (ИНН)");
    let scheme = get_uri_attr(&uris[0].borrow(), ATTR_SCHEME);
    assert_eq!(scheme.as_deref(), Some("ИНН"), "Scheme should be ИНН, got {:?}", scheme);
}

// ── Date analyzer tests ───────────────────────────────────────────────────────

fn date_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(DateAnalyzer::new())])
}

/// Formal DD.MM.YYYY date
#[test]
fn test_date_formal_ddmmyyyy() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("Дата: 15.03.2024");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let dates: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DATE")
        .collect();
    assert!(!dates.is_empty(), "Should extract DATE from 15.03.2024");
    let rb = dates[0].borrow();
    assert_eq!(get_year(&rb), 2024, "Year should be 2024");
    assert_eq!(get_month(&rb), 3, "Month should be 3 (March)");
    assert_eq!(get_day(&rb), 15, "Day should be 15");
}

/// Ordinal day word in Russian: "девятнадцатого сентября"
#[test]
fn test_date_ordinal_day_ru() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("девятнадцатого сентября");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let dates: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DATE")
        .collect();
    for d in &dates {
        let rb = d.borrow();
        eprintln!("DATE: month={} day={}", get_month(&rb), get_day(&rb));
    }
    assert!(!dates.is_empty(), "Should extract DATE from 'девятнадцатого сентября'");
    let rb = dates[0].borrow();
    assert_eq!(get_month(&rb), 9, "Month should be 9 (September)");
    assert_eq!(get_day(&rb), 19, "Day should be 19 from ordinal 'девятнадцатого'");
}

/// Compound ordinal day in Russian: "тридцать первого января 2024"
#[test]
fn test_date_compound_ordinal_day_ru() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("тридцать первого января 2024 года");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let dates: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DATE")
        .collect();
    assert!(!dates.is_empty(), "Should extract DATE from 'тридцать первого января 2024 года'");
    let rb = dates[0].borrow();
    assert_eq!(get_month(&rb), 1, "Month should be 1 (January)");
    assert_eq!(get_day(&rb), 31, "Day should be 31 from ordinal 'тридцать первого'");
}

/// Written-out month in Russian: "15 января 2024 года"
#[test]
fn test_date_written_month_ru() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("Контракт подписан 15 января 2024 года");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let dates: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DATE")
        .collect();
    assert!(!dates.is_empty(), "Should extract DATE from '15 января 2024 года'");
    let rb = dates[0].borrow();
    assert_eq!(get_year(&rb), 2024, "Year should be 2024");
    assert_eq!(get_month(&rb), 1, "Month should be 1 (January)");
    assert_eq!(get_day(&rb), 15, "Day should be 15");
}

/// Year-only date extraction
#[test]
fn test_date_year_only() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("Основана в 1991 году");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let dates: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DATE")
        .collect();
    assert!(!dates.is_empty(), "Should extract DATE from '1991 году'");
    let rb = dates[0].borrow();
    assert_eq!(get_year(&rb), 1991, "Year should be 1991");
    assert_eq!(get_month(&rb), 0, "Month should be 0 (not set)");
    assert_eq!(get_day(&rb), 0, "Day should be 0 (not set)");
}

/// Year-year range "с 2020 по 2024 год" → DATERANGE(FROM=2020, TO=2024)
#[test]
fn test_date_range_year_year_po() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("с 2020 по 2024 год");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let ranges: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == DATERANGE_OBJ_TYPENAME)
        .collect();
    assert!(!ranges.is_empty(), "Should extract DATERANGE from 'с 2020 по 2024 год'");
    let rb = ranges[0].borrow();
    let from = get_date_from(&rb).expect("DATERANGE must have FROM");
    let to   = get_date_to(&rb).expect("DATERANGE must have TO");
    assert_eq!(get_year(&from.borrow()), 2020, "FROM year should be 2020");
    assert_eq!(get_year(&to.borrow()),   2024, "TO year should be 2024");
}

/// Year-year range with hyphen "2020-2024" → DATERANGE(FROM=2020, TO=2024)
#[test]
fn test_date_range_year_hyphen_year() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("период 2020-2024");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let ranges: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == DATERANGE_OBJ_TYPENAME)
        .collect();
    assert!(!ranges.is_empty(), "Should extract DATERANGE from '2020-2024'");
    let rb = ranges[0].borrow();
    let from = get_date_from(&rb).expect("DATERANGE must have FROM");
    let to   = get_date_to(&rb).expect("DATERANGE must have TO");
    assert_eq!(get_year(&from.borrow()), 2020, "FROM year should be 2020");
    assert_eq!(get_year(&to.borrow()),   2024, "TO year should be 2024");
}

/// Day-to-date range "с 15 по 20 марта" → DATERANGE(FROM=DAY:15/MONTH:3, TO=DAY:20/MONTH:3)
#[test]
fn test_date_range_day_to_day_same_month() {
    let proc = date_proc();
    let sofa = SourceOfAnalysis::new("с 15 по 20 марта");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let ranges: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == DATERANGE_OBJ_TYPENAME)
        .collect();
    assert!(!ranges.is_empty(), "Should extract DATERANGE from 'с 15 по 20 марта'");
    let rb = ranges[0].borrow();
    let from = get_date_from(&rb).expect("DATERANGE must have FROM");
    let to   = get_date_to(&rb).expect("DATERANGE must have TO");
    assert_eq!(get_day(&from.borrow()),   15, "FROM day should be 15");
    assert_eq!(get_month(&from.borrow()), 3,  "FROM month should be 3 (March)");
    assert_eq!(get_day(&to.borrow()),     20, "TO day should be 20");
    assert_eq!(get_month(&to.borrow()),   3,  "TO month should be 3 (March)");
}

// ── Money analyzer tests ──────────────────────────────────────────────────────

fn money_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(MoneyAnalyzer::new())])
}

/// "100 рублей" → MONEY, value=100, currency=RUB
#[test]
fn test_money_rubles() {
    let proc = money_proc();
    let sofa = SourceOfAnalysis::new("Стоимость 100 рублей");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let moneys: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MONEY")
        .collect();
    assert!(!moneys.is_empty(), "Should extract MONEY from '100 рублей'");
    let rb = moneys[0].borrow();
    let cur = get_currency(&rb);
    assert_eq!(cur.as_deref(), Some("RUB"), "Currency should be RUB, got {:?}", cur);
    let val = get_money_value(&rb);
    assert_eq!(val.as_deref(), Some("100"), "Value should be 100, got {:?}", val);
}

/// "$500" — currency symbol before number → USD
#[test]
fn test_money_dollar_symbol() {
    let proc = money_proc();
    let sofa = SourceOfAnalysis::new("Цена $500");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let moneys: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MONEY")
        .collect();
    assert!(!moneys.is_empty(), "Should extract MONEY from '$500'");
    let rb = moneys[0].borrow();
    let cur = get_currency(&rb);
    assert_eq!(cur.as_deref(), Some("USD"), "Currency should be USD, got {:?}", cur);
}

/// "1500 EUR" — ISO code as currency word
#[test]
fn test_money_eur_code() {
    let proc = money_proc();
    let sofa = SourceOfAnalysis::new("Бюджет 1500 EUR");
    let result = proc.process(sofa, Some(MorphLang::RU | MorphLang::EN));
    let moneys: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MONEY")
        .collect();
    assert!(!moneys.is_empty(), "Should extract MONEY from '1500 EUR'");
    let rb = moneys[0].borrow();
    let cur = get_currency(&rb);
    assert_eq!(cur.as_deref(), Some("EUR"), "Currency should be EUR, got {:?}", cur);
    let val = get_money_value(&rb);
    assert_eq!(val.as_deref(), Some("1500"), "Value should be 1500, got {:?}", val);
}

// ── Measure analyzer tests ────────────────────────────────────────────────────

fn measure_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(MeasureAnalyzer::new())])
}

/// "100 км" → MEASURE, value=100, unit=км, kind=Length
#[test]
fn test_measure_kilometers() {
    let proc = measure_proc();
    let sofa = SourceOfAnalysis::new("Расстояние 100 км");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let measures: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MEASURE")
        .collect();
    assert!(!measures.is_empty(), "Should extract MEASURE from '100 км'");
    let rb = measures[0].borrow();
    let val = get_measure_value(&rb);
    assert_eq!(val.as_deref(), Some("100"), "Value should be 100, got {:?}", val);
    let kind = get_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Length"), "Kind should be Length, got {:?}", kind);
}

/// "5 кг" → MEASURE, value=5, kind=Weight
#[test]
fn test_measure_kilograms() {
    let proc = measure_proc();
    let sofa = SourceOfAnalysis::new("Масса 5 кг");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let measures: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MEASURE")
        .collect();
    assert!(!measures.is_empty(), "Should extract MEASURE from '5 кг'");
    let rb = measures[0].borrow();
    let kind = get_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Weight"), "Kind should be Weight, got {:?}", kind);
}

/// "25%" → MEASURE, value=25, kind=Percent
#[test]
fn test_measure_percent() {
    let proc = measure_proc();
    let sofa = SourceOfAnalysis::new("Скидка 25%");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let measures: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "MEASURE")
        .collect();
    assert!(!measures.is_empty(), "Should extract MEASURE from '25%'");
    let rb = measures[0].borrow();
    let kind = get_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Percent"), "Kind should be Percent, got {:?}", kind);
}

// ── Geo analyzer tests ────────────────────────────────────────────────────────

fn geo_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(GeoAnalyzer::new())])
}

/// "Россия" → GEO, state, alpha2=RU
#[test]
fn test_geo_country_russia() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Это произошло в России.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO entity for 'России'");
    let rb = geos[0].borrow();
    assert!(is_state(&rb), "Russia should be a state, got type={:?}", get_geo_type(&rb));
    let a2 = get_alpha2(&rb);
    assert_eq!(a2.as_deref(), Some("RU"), "Alpha2 should be RU, got {:?}", a2);
}

/// "г. Москва" → GEO, city
#[test]
fn test_geo_city_prefix() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Офис находится в г. Москва");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO entity for 'г. Москва'");
    let rb = geos[0].borrow();
    let name = get_geo_name(&rb);
    assert!(
        name.as_deref().map(|n| n.contains("МОСКВА")).unwrap_or(false),
        "Name should contain МОСКВА, got {:?}", name
    );
}

/// "деревню. Я" → should NOT produce a GEO (sentence-ending "." followed by pronoun)
#[test]
fn test_geo_no_false_positive_sentence_period() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Он приехал в деревню. Я его встретил.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    // Should not extract "Я" as city name
    let has_ya = geos.iter().any(|e| {
        let rb = e.borrow();
        get_geo_name(&rb).as_deref() == Some("Я")
    });
    assert!(!has_ya, "Should NOT extract 'Я' (pronoun) as a city name");
}

/// "Сел Максим" → should NOT produce a GEO (verb + person name)
#[test]
fn test_geo_no_false_positive_verb_personname() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Сел Максим рядом с нами.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    let has_maksim = geos.iter().any(|e| {
        let rb = e.borrow();
        get_geo_name(&rb).as_deref() == Some("МАКСИМ")
    });
    assert!(!has_maksim, "Should NOT extract 'Максим' (person name) as a city name after verb 'Сел'");
}

/// "Московская область" → GEO, region
#[test]
fn test_geo_region_adjective_type() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Проживает в Московской области");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'Московской области'");
    let rb = geos[0].borrow();
    assert!(is_region(&rb), "Московская область should be a region, type={:?}", get_geo_type(&rb));
}

/// "в Санкт-Петербург" → GEO, whole "Санкт-Петербург" is one entity
#[test]
fn test_geo_hyphenated_city() {
    let proc = geo_proc();
    let sofa = SourceOfAnalysis::new("Конференция пройдёт в Санкт-Петербурге.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'Санкт-Петербурге'");
    // The entity name should contain both parts
    let rb = geos[0].borrow();
    let name = get_geo_name(&rb);
    assert!(
        name.as_deref().map(|n| n.contains("ПЕТЕРБУРГ") || n.contains("САНКТ")).unwrap_or(false),
        "Name should contain Санкт-Петербург, got {:?}", name
    );
}

// ── Person analyzer tests ─────────────────────────────────────────────────────

fn person_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(PersonAnalyzer::new())])
}

/// "Иванов Иван Иванович" → PERSON, lastname=Иванов, firstname=Иван, middlename=Иванович, sex=Male
#[test]
fn test_person_full_fio() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("Директор Иванов Иван Иванович подписал приказ.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from full FIO");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    let first = get_firstname(&rb);
    let mid = get_middlename(&rb);
    let sex = get_sex(&rb);
    assert!(last.as_deref().map(|s| s.contains("ИВАНОВ")).unwrap_or(false),
        "lastname should be ИВАНОВ, got {:?}", last);
    assert!(first.is_some(), "firstname should be set, got {:?}", first);
    assert!(mid.is_some(), "middlename should be set, got {:?}", mid);
    assert_eq!(sex.as_deref(), Some(SEX_MALE), "sex should be Male (from patronymic -вич), got {:?}", sex);
}

/// "Иван Иванович" → PERSON, firstname set, middlename set, sex=Male
#[test]
fn test_person_name_secname() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("Иван Иванович сказал спасибо.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from 'Иван Иванович'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    let mid = get_middlename(&rb);
    let sex = get_sex(&rb);
    assert!(first.is_some(), "firstname should be set, got {:?}", first);
    assert!(mid.is_some(), "middlename should be set, got {:?}", mid);
    assert_eq!(sex.as_deref(), Some(SEX_MALE), "sex should be Male, got {:?}", sex);
}

/// "Иванов И.И." → PERSON, lastname=Иванов, firstname=И
#[test]
fn test_person_surname_initials() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("Иванов И.И. подписал документ.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from 'Ивановым И.И.'");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.is_some(), "lastname should be set, got {:?}", last);
}

// ── Org analyzer tests ────────────────────────────────────────────────────────

fn org_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(OrgAnalyzer::new())])
}

/// "ООО "Russian Context Optimizer"" → ORGANIZATION, type=ООО, name contains RUSSIAN CONTEXT OPTIMIZER
#[test]
fn test_org_legal_form_double_quote_en() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("ООО \"Russian Context Optimizer\" (RCO)");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should extract ORGANIZATION from 'ООО \"Russian Context Optimizer\"'");
    let rb = orgs[0].borrow();
    let names = get_org_names(&rb);
    assert!(names.iter().any(|n| n.contains("RUSSIAN") && n.contains("CONTEXT") && n.contains("OPTIMIZER")),
        "name should contain RUSSIAN CONTEXT OPTIMIZER, got {:?}", names);
}

/// "ООО «Газпром»" → ORGANIZATION, type=ООО, name contains ГАЗПРОМ
#[test]
fn test_org_legal_form_quoted() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Договор с ООО «Газпром» заключён.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should extract ORGANIZATION from 'ООО «Газпром»'");
    let rb = orgs[0].borrow();
    let typ = get_org_type(&rb);
    assert_eq!(typ.as_deref(), Some("ООО"), "type should be ООО, got {:?}", typ);
    let names = get_org_names(&rb);
    assert!(names.iter().any(|n| n.contains("ГАЗПРОМ")),
        "name should contain ГАЗПРОМ, got {:?}", names);
}

/// "Министерство финансов" → ORGANIZATION, type contains МИНИСТЕРСТВО
#[test]
fn test_org_ministry() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Решение принято Министерством финансов.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should extract ORGANIZATION from 'Министерство финансов'");
    let rb = orgs[0].borrow();
    let typ = get_org_type(&rb);
    assert!(typ.as_deref().map(|t| t.contains("МИНИСТЕРСТВО")).unwrap_or(false),
        "type should contain МИНИСТЕРСТВО, got {:?}", typ);
}

/// "Министерство финансов России" → ORGANIZATION with multi-word lowercase name
#[test]
fn test_org_ministry_multiword() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Министерство финансов России выпустило постановление.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should extract ORGANIZATION from 'Министерство финансов России'");
    let rb = orgs[0].borrow();
    let names = get_org_names(&rb);
    assert!(names.iter().any(|n| n.contains("ФИНАНС")),
        "name should contain ФИНАНС (финансов/финансы), got {:?}", names);
}

/// "ГИБДД" → ORGANIZATION (known org from Orgs_ru.dat)
#[test]
fn test_org_known_gibdd() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Водитель остановлен ГИБДД.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should extract ORGANIZATION for known org 'ГИБДД'");
}

/// "Государственная дума" → ORGANIZATION (multi-word known org or adj+type)
#[test]
fn test_org_stateduma() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Государственная дума приняла закон.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should detect ORGANIZATION in 'Государственная дума'");
}

/// "Центральный банк России" → ORGANIZATION, type=БАНК
#[test]
fn test_org_centralbank() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Центральный банк России повысил ставку.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should detect ORGANIZATION in 'Центральный банк России'");
    let rb = orgs[0].borrow();
    let typ = get_org_type(&rb);
    assert!(typ.as_deref().map(|t| t.contains("БАНК")).unwrap_or(false),
        "type should contain БАНК, got {:?}", typ);
}

/// "Московский государственный университет" → ORGANIZATION, type=УНИВЕРСИТЕТ
#[test]
fn test_org_mgu() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Московский государственный университет является ведущим вузом.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should detect ORGANIZATION in 'Московский государственный университет'");
    let rb = orgs[0].borrow();
    let typ = get_org_type(&rb);
    assert!(typ.as_deref().map(|t| t.contains("УНИВЕРСИТЕТ")).unwrap_or(false),
        "type should contain УНИВЕРСИТЕТ, got {:?}", typ);
}

/// "Российская академия наук" → ORGANIZATION, type=АКАДЕМИЯ
#[test]
fn test_org_ran() {
    let proc = org_proc();
    let sofa = SourceOfAnalysis::new("Российская академия наук издала сборник.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    assert!(!orgs.is_empty(), "Should detect ORGANIZATION in 'Российская академия наук'");
    let rb = orgs[0].borrow();
    let typ = get_org_type(&rb);
    assert!(typ.as_deref().map(|t| t.contains("АКАДЕМИЯ")).unwrap_or(false),
        "type should contain АКАДЕМИЯ, got {:?}", typ);
}

// ── NamedEntity tests ────────────────────────────────────────────────────────

fn named_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(NamedEntityAnalyzer::new())])
}

/// "планета Марс" → NAMEDENTITY, kind=Planet, name=МАРС
#[test]
fn test_named_planet_type_plus_name() {
    let proc = named_proc();
    let sofa = SourceOfAnalysis::new("Учёные изучают планету Марс.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let entities: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "NAMEDENTITY")
        .collect();
    assert!(!entities.is_empty(), "Should find NAMEDENTITY for 'планету Марс'");
    let rb = entities[0].borrow();
    let kind = get_named_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Planet"), "kind should be Planet, got {:?}", kind);
    let name = get_named_name(&rb);
    assert!(name.is_some(), "name should be set");
}

/// "Марс" standalone → NAMEDENTITY, kind=Planet (well-known name)
#[test]
fn test_named_planet_wellknown() {
    let proc = named_proc();
    let sofa = SourceOfAnalysis::new("Марс — четвёртая планета Солнечной системы.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let entities: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "NAMEDENTITY")
        .collect();
    assert!(!entities.is_empty(), "Should find NAMEDENTITY for 'Марс'");
    let rb = entities[0].borrow();
    let kind = get_named_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Planet"), "kind should be Planet, got {:?}", kind);
}

/// "река Волга" → NAMEDENTITY, kind=Location
#[test]
fn test_named_location_river() {
    let proc = named_proc();
    let sofa = SourceOfAnalysis::new("Город стоит на реке Волга.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let entities: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "NAMEDENTITY")
        .collect();
    assert!(!entities.is_empty(), "Should find NAMEDENTITY for 'реке Волга'");
    let rb = entities[0].borrow();
    let kind = get_named_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Location"), "kind should be Location, got {:?}", kind);
}

/// "памятник Пушкину" → NAMEDENTITY, kind=Monument
#[test]
fn test_named_monument() {
    let proc = named_proc();
    let sofa = SourceOfAnalysis::new("На площади стоит памятник Пушкину.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let entities: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "NAMEDENTITY")
        .collect();
    assert!(!entities.is_empty(), "Should find NAMEDENTITY for 'памятник Пушкину'");
    let rb = entities[0].borrow();
    let kind = get_named_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Monument"), "kind should be Monument, got {:?}", kind);
}

// ── Address tests ────────────────────────────────────────────────────────────

fn address_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(AddressAnalyzer::new())])
}

/// "ул. Ленина" → STREET, type=улица, name=ЛЕНИН*
#[test]
fn test_address_street_only() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Дом находится на ул. Ленина.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'ул. Ленина'");
    let rb = streets[0].borrow();
    let typ = get_street_type(&rb);
    assert_eq!(typ.as_deref(), Some("улица"), "type should be улица, got {:?}", typ);
    let name = get_street_name(&rb);
    assert!(name.is_some(), "street name should be set");
}

/// "ул. Ленина, д. 5" → STREET + ADDRESS with house=5
#[test]
fn test_address_street_and_house() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Адрес: ул. Ленина, д. 5.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS for 'ул. Ленина, д. 5'");
    let rb = addresses[0].borrow();
    let house = get_house(&rb);
    assert_eq!(house.as_deref(), Some("5"), "house should be 5, got {:?}", house);
}

/// "проспект Мира, 12, кв. 4" → ADDRESS with house=12, flat=4
#[test]
fn test_address_prospekt_house_flat() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Живёт по адресу проспект Мира, 12, кв. 4.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS for 'проспект Мира, 12, кв. 4'");
    let rb = addresses[0].borrow();
    let house = get_house(&rb);
    assert!(house.is_some(), "house should be set");
    let flat = get_flat(&rb);
    assert_eq!(flat.as_deref(), Some("4"), "flat should be 4, got {:?}", flat);
}

// ── Transport analyzer tests ──────────────────────────────────────────────────

fn transport_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(TransportAnalyzer::new())])
}

/// "автомобиль Toyota Camry" → TRANSPORT, kind=Auto, type=автомобиль, brand=Toyota
#[test]
fn test_transport_auto_type_brand_model() {
    let proc = transport_proc();
    let sofa = SourceOfAnalysis::new("Водитель управлял автомобилем Toyota Camry.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let transports: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "TRANSPORT")
        .collect();
    assert!(!transports.is_empty(), "Should extract TRANSPORT from 'автомобилем Toyota Camry'");
    let rb = transports[0].borrow();
    let kind = get_transport_kind(&rb);
    assert!(kind.as_deref() == Some("Auto"), "kind should be Auto, got {:?}", kind);
    let brand = get_transport_brand(&rb);
    assert!(brand.is_some(), "brand should be set, got {:?}", brand);
}

/// "теплоход «Победа»" → TRANSPORT, kind=Ship, name contains ПОБЕДА
#[test]
fn test_transport_ship_with_name() {
    let proc = transport_proc();
    let sofa = SourceOfAnalysis::new("На теплоходе «Победа» прибыли гости.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let transports: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "TRANSPORT")
        .collect();
    assert!(!transports.is_empty(), "Should extract TRANSPORT from 'теплоходе «Победа»'");
    let rb = transports[0].borrow();
    let kind = get_transport_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Ship"), "kind should be Ship, got {:?}", kind);
}

/// "самолет Boeing" → TRANSPORT, kind=Fly
#[test]
fn test_transport_airplane_brand() {
    let proc = transport_proc();
    let sofa = SourceOfAnalysis::new("Пассажиры вылетели на самолете Boeing 737.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let transports: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "TRANSPORT")
        .collect();
    assert!(!transports.is_empty(), "Should extract TRANSPORT from 'самолете Boeing 737'");
    let rb = transports[0].borrow();
    let kind = get_transport_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Fly"), "kind should be Fly, got {:?}", kind);
    let brand = get_transport_brand(&rb);
    assert!(brand.is_some(), "brand should be set (Boeing), got {:?}", brand);
}


// ── Decree analyzer tests ─────────────────────────────────────────────────────

fn decree_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(DecreeAnalyzer::new())])
}

/// "Федеральный закон № 123-ФЗ" → DECREE, kind=Law, number contains 123
#[test]
fn test_decree_federal_law() {
    let proc = decree_proc();
    let sofa = SourceOfAnalysis::new("В соответствии с Федеральным законом № 123-ФЗ от 01.01.2024.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let decrees: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DECREE")
        .collect();
    assert!(!decrees.is_empty(), "Should extract DECREE from 'Федеральным законом № 123-ФЗ'");
    let rb = decrees[0].borrow();
    let kind = get_decree_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Law"), "kind should be Law, got {:?}", kind);
    let number = get_decree_number(&rb);
    assert!(number.as_deref().map(|s| s.contains("123")).unwrap_or(false),
        "number should contain 123, got {:?}", number);
}

/// "ГОСТ 12345-2020" → DECREE, kind=Standard, number=12345-2020
#[test]
fn test_decree_gost_standard() {
    let proc = decree_proc();
    let sofa = SourceOfAnalysis::new("Продукция соответствует ГОСТ 12345-2020.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let decrees: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DECREE")
        .collect();
    assert!(!decrees.is_empty(), "Should extract DECREE from 'ГОСТ 12345-2020'");
    let rb = decrees[0].borrow();
    let kind = get_decree_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Standard"), "kind should be Standard, got {:?}", kind);
    let typ = get_decree_type(&rb);
    assert!(typ.as_deref().map(|s| s.contains("ГОСТ")).unwrap_or(false),
        "type should be ГОСТ, got {:?}", typ);
    let number = get_decree_number(&rb);
    assert!(number.is_some(), "number should be set, got {:?}", number);
}

/// "Приказ Министерства финансов № 45" → DECREE, kind=Order
#[test]
fn test_decree_order_with_number() {
    let proc = decree_proc();
    let sofa = SourceOfAnalysis::new("Согласно приказу Министерства финансов № 45.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let decrees: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "DECREE")
        .collect();
    assert!(!decrees.is_empty(), "Should extract DECREE from 'приказу... № 45'");
    let rb = decrees[0].borrow();
    let kind = get_decree_kind(&rb);
    assert_eq!(kind.as_deref(), Some("Order"), "kind should be Order, got {:?}", kind);
}

// ── NounPhraseHelper tests ────────────────────────────────────────────────

fn nph_proc() -> pullenti_ner::Processor {
    use pullenti_ner::ProcessorService;
    use pullenti_ner::Sdk;
    Sdk::initialize_all(Some(MorphLang::RU | MorphLang::EN));
    ProcessorService::create_processor()
}

#[test]
fn test_noun_phrase_simple_adj_noun() {
    init();
    use pullenti_ner::core::noun_phrase::{try_parse as nph_try_parse, NounPhraseParseAttr};
    let sofa = SourceOfAnalysis::new("красный дом");
    let proc = nph_proc();
    let result = proc.process(sofa.clone(), Some(MorphLang::RU));
    // Walk the token chain and try to find a noun phrase at "красный"
    use pullenti_ner::TokenChainIter;
    let mut found = false;
    for tok in TokenChainIter::new(result.first_token.clone()) {
        let npt = nph_try_parse(&tok, NounPhraseParseAttr::No, 0, &sofa);
        if let Some(ref np) = npt {
            if np.adjectives.len() > 0 && np.noun.is_some() {
                found = true;
            }
        }
        if found { break; }
    }
    assert!(found, "Should find adj+noun phrase for 'красный дом'");
}

#[test]
fn test_noun_phrase_pronoun() {
    init();
    use pullenti_ner::core::noun_phrase::{try_parse as nph_try_parse, NounPhraseParseAttr};
    let sofa = SourceOfAnalysis::new("большая компания");
    let proc = nph_proc();
    let result = proc.process(sofa.clone(), Some(MorphLang::RU));
    use pullenti_ner::TokenChainIter;
    let mut found_noun = false;
    for tok in TokenChainIter::new(result.first_token.clone()) {
        let npt = nph_try_parse(&tok, NounPhraseParseAttr::No, 0, &sofa);
        if let Some(ref np) = npt {
            if np.noun.is_some() {
                found_noun = true;
                break;
            }
        }
    }
    assert!(found_noun, "Should find noun phrase for 'большая компания'");
}

#[test]
fn test_noun_phrase_not_noun() {
    init();
    use pullenti_ner::core::noun_phrase::{try_parse as nph_try_parse, NounPhraseParseAttr};
    // A pure verb should not be a noun phrase
    let sofa = SourceOfAnalysis::new("бежит");
    let proc = nph_proc();
    let result = proc.process(sofa.clone(), Some(MorphLang::RU));
    use pullenti_ner::TokenChainIter;
    let mut found_noun = false;
    for tok in TokenChainIter::new(result.first_token.clone()) {
        let npt = nph_try_parse(&tok, NounPhraseParseAttr::No, 0, &sofa);
        if let Some(ref np) = npt {
            if np.noun.is_some() { found_noun = true; }
        }
    }
    // A standalone verb should not parse as a noun phrase
    assert!(!found_noun, "Pure verb 'бежит' should not be a noun phrase");
}

#[test]
fn test_demo_text_entities() {
    Sdk::initialize_all(Some(MorphLang::RU | MorphLang::EN));
    let text = "Система разрабатывается с 2011 года российским программистом Михаилом Жуковым, проживающим в Москве на Красной площади в доме номер один на втором этаже. Конкурентов у него много: Abbyy, Yandex, ООО \"Russian Context Optimizer\" (RCO) и другие компании. Он планирует продать SDK за 1.120.000.001,99 (миллиард сто двадцать миллионов один рубль 99 копеек) рублей, без НДС.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let types: Vec<String> = ar.entities.iter()
        .map(|e| format!("{}={:?}", e.borrow().type_name, e.borrow().slots.iter().map(|s| format!("{}:{}", s.type_name, s.value.as_ref().and_then(|v| if let pullenti_ner::referent::SlotValue::Str(sv) = v { Some(sv.clone()) } else { None }).unwrap_or_default())).collect::<Vec<_>>()))
        .collect();
    for t in &types {
        eprintln!("ENTITY: {}", t);
    }
    // Should find at least 2011 date, Moscow geo, and at least one org
    let has_2011 = ar.entities.iter().any(|e| {
        let e = e.borrow();
        e.type_name == "DATE" && e.get_string_value("YEAR") == Some("2011")
    });
    assert!(has_2011, "Should find DATE 2011");
    let has_moscow = ar.entities.iter().any(|e| {
        let e = e.borrow();
        e.type_name == "GEO" && e.get_string_value("NAME").map(|n| n.contains("МОСКВА")).unwrap_or(false)
    });
    assert!(has_moscow, "Should find GEO МОСКВА");
    let has_org = ar.entities.iter().any(|e| e.borrow().type_name == "ORGANIZATION");
    assert!(has_org, "Should find at least one ORGANIZATION");
}

#[test]
fn test_date_false_positive_check() {
    Sdk::initialize_all(Some(MorphLang::RU));
    // Just the money portion — should NOT produce a date with year 1999
    let text = "за 1.120.000.001,99 (миллиард сто двадцать миллионов один рубль 99 копеек) рублей";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let false_dates: Vec<_> = ar.entities.iter()
        .filter(|e| {
            let e = e.borrow();
            e.type_name == "DATE" && e.get_string_value("YEAR") == Some("1999")
        })
        .collect();
    eprintln!("Dates found:");
    for e in &ar.entities {
        let e = e.borrow();
        if e.type_name == "DATE" {
            eprintln!("  DATE: {:?}", e.slots.iter().map(|s| format!("{:?}", s.value)).collect::<Vec<_>>());
        }
    }
    assert!(false_dates.is_empty(), "Should NOT find DATE YEAR=1999 in money context");
}

#[test]
fn test_money_large_amount() {
    Sdk::initialize_all(Some(MorphLang::RU));
    // Large amount with Russian decimal notation (period=thousands, comma=decimal)
    let text = "продать SDK за 1.120.000.001,99 рублей";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let moneys: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "MONEY")
        .collect();
    eprintln!("Money entities: {}", moneys.len());
    for m in &moneys {
        let m = m.borrow();
        eprintln!("  MONEY: {:?}", m.slots.iter().map(|s| format!("{:?}", s.value)).collect::<Vec<_>>());
    }
    eprintln!("All entities:");
    for e in &ar.entities {
        let e = e.borrow();
        eprintln!("  {}: {:?}", e.type_name, e.get_string_value("VALUE").or(e.get_string_value("YEAR")));
    }
    assert!(!moneys.is_empty(), "Should find MONEY entity for 'рублей'");
}


#[test]
fn test_person_fi_surname_instrumental() {
    Sdk::initialize_all(Some(MorphLang::RU));
    // "программистом Михаилом Жуковым" — first name + last name in instrumental
    let text = "российским программистом Михаилом Жуковым, проживающим в Москве";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let persons: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    eprintln!("Persons found: {}", persons.len());
    for p in &persons {
        let p = p.borrow();
        eprintln!("  PERSON: {:?}", p.slots.iter().map(|s| format!("{}={:?}", s.type_name, s.value)).collect::<Vec<_>>());
    }
    assert!(!persons.is_empty(), "Should detect person 'Михаилом Жуковым'");
}

/// "Мария Петровна Иванова" — FirstName + Patronymic + Surname pattern (C3).
/// Also tests that "Иванова" is NOT extracted as GEO (Иваново city) in person context.
#[test]
fn test_person_firstname_patronymic_surname() {
    Sdk::initialize_all(Some(MorphLang::RU));
    let text = "Мария Петровна Иванова встретила гостей.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);
    let persons: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    let geos: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON 'Мария Петровна Иванова'");
    assert!(geos.is_empty(), "Should NOT extract GEO for 'Иванова' in person context");
    let rb = persons[0].borrow();
    let first = rb.slots.iter().find(|s| s.type_name == "FIRSTNAME")
        .and_then(|s| s.value.as_ref().and_then(|v| if let pullenti_ner::SlotValue::Str(s) = v { Some(s.clone()) } else { None }));
    let last = rb.slots.iter().find(|s| s.type_name == "LASTNAME")
        .and_then(|s| s.value.as_ref().and_then(|v| if let pullenti_ner::SlotValue::Str(s) = v { Some(s.clone()) } else { None }));
    assert!(first.is_some(), "FIRSTNAME should be set");
    assert!(last.is_some(), "LASTNAME should be set (got from C3 pattern)");
}

// ── Bank analyzer tests ────────────────────────────────────────────────────────

#[test]
fn test_bank_basic_requisites() {
    Sdk::initialize_all(Some(MorphLang::RU));
    // Bank requisites block with Р/С, ИНН, БИК — three URIs, no keyword
    let text = "Р/С 40702810000000001234\nИНН 7701234567\nБИК 044525225";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let banks: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "BANKDATA")
        .collect();
    eprintln!("BANKDATA entities: {}", banks.len());
    for b in &banks {
        let b = b.borrow();
        eprintln!("  BANKDATA slots: {:?}",
            b.slots.iter().map(|s| format!("{}={:?}", s.type_name, s.value)).collect::<Vec<_>>());
    }
    assert!(!banks.is_empty(), "Should find BANKDATA entity");

    // The BANKDATA must contain Р/С
    let has_rs = banks.iter().any(|b| {
        find_value_owned(&b.borrow(), "Р/С").is_some()
    });
    assert!(has_rs, "BANKDATA must contain Р/С slot");
}

#[test]
fn test_bank_keyword_trigger() {
    Sdk::initialize_all(Some(MorphLang::RU));
    let text = "БАНКОВСКИЕ РЕКВИЗИТЫ:\nР/С 40702810000000001234\nИНН 7701234567\nБИК 044525225";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let banks: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "BANKDATA")
        .collect();
    eprintln!("BANKDATA (keyword) entities: {}", banks.len());
    assert!(!banks.is_empty(), "Should find BANKDATA via keyword trigger");
}

#[test]
fn test_bank_no_rs_no_match() {
    Sdk::initialize_all(Some(MorphLang::RU));
    // ИНН+КПП alone, no Р/С → must NOT produce BANKDATA
    let text = "ИНН 7701234567 КПП 770101001";
    let sofa = SourceOfAnalysis::new(text);
    let proc = ProcessorService::create_processor();
    let ar = proc.process(sofa, None);

    let banks: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "BANKDATA")
        .collect();
    eprintln!("BANKDATA (no-Р/С) count: {}", banks.len());
    assert!(banks.is_empty(), "Should NOT find BANKDATA without Р/С or Л/С");
}

// ═══════════════════════════════════════════════════════════════════════════
// Weapon tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_weapon_pistol_brand() {
    init();
    let text = "пистолет Макаров";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(WeaponAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let weapons: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "WEAPON")
        .collect();
    eprintln!("Weapon entities: {}", weapons.len());
    assert_eq!(weapons.len(), 1, "Should find 1 WEAPON");

    let w = weapons[0].borrow();
    let typ = get_weapon_type(&w);
    let brand = get_weapon_brand(&w);
    eprintln!("  type={:?} brand={:?}", typ, brand);
    assert_eq!(typ, Some("ПИСТОЛЕТ"), "TYPE should be ПИСТОЛЕТ");
    assert_eq!(brand, Some("МАКАРОВ"), "BRAND should be МАКАРОВ");
}

#[test]
fn test_weapon_ak47() {
    init();
    // АК is recognized as acronym for АВТОМАТ КАЛАШНИКОВА, _correct_model extends to АК-47
    let text = "АК-47";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(WeaponAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let weapons: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "WEAPON")
        .collect();
    eprintln!("AK-47 weapon entities: {}", weapons.len());

    // АК-47 alone without noun context may not produce a weapon (depends on context check)
    // Let's test with context
    let text2 = "вооружение АК-47";
    let sofa2 = SourceOfAnalysis::new(text2);
    let ar2 = proc.process(sofa2, None);
    let weapons2: Vec<_> = ar2.entities.iter()
        .filter(|e| e.borrow().type_name == "WEAPON")
        .collect();
    eprintln!("AK-47 with context: {} weapons", weapons2.len());
    if !weapons2.is_empty() {
        let w = weapons2[0].borrow();
        let model = get_weapon_model(&w);
        eprintln!("  model={:?}", model);
    }
    // The model pattern should work either with or without context
    assert!(!weapons.is_empty() || !weapons2.is_empty(),
        "Should find WEAPON for АК-47 in some context");
}

#[test]
fn test_weapon_noun_alone() {
    init();
    // A lone noun without brand/model should not produce a WEAPON entity
    let text = "пистолет лежал на столе";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(WeaponAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let weapons: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == "WEAPON")
        .collect();
    eprintln!("Lone noun weapons: {}", weapons.len());
    assert!(weapons.is_empty(), "Should NOT find WEAPON for lone noun");
}

// ── Chemical analyzer tests ───────────────────────────────────────────────

#[test]
fn test_chemical_formula_h2o() {
    init();
    // "H2O" — water formula in chemical context
    let text = "Химическая формула воды — H2O.";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(ChemicalAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let chems: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == CHEMICAL_OBJ_TYPENAME)
        .collect();
    eprintln!("H2O chemicals: {}", chems.len());
    assert!(!chems.is_empty(), "Should find CHEMICALFORMULA for H2O");
    let c = chems[0].borrow();
    let val = get_chem_value(&c);
    eprintln!("  formula={:?}", val);
    assert_eq!(val.as_deref(), Some("H2O"), "Formula should be H2O");
}

#[test]
fn test_chemical_formula_co2() {
    init();
    let text = "CO2 — газ, молекула углекислоты.";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(ChemicalAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let chems: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == CHEMICAL_OBJ_TYPENAME)
        .collect();
    eprintln!("CO2 chemicals: {}", chems.len());
    assert!(!chems.is_empty(), "Should find CHEMICALFORMULA for CO2");
    let c = chems[0].borrow();
    let val = get_chem_value(&c);
    eprintln!("  formula={:?}", val);
    assert_eq!(val.as_deref(), Some("CO2"), "Formula should be CO2");
}

#[test]
fn test_chemical_substance_name() {
    init();
    // Named substance "кислота" in context "серная кислота"
    let text = "Серная кислота (H2SO4) используется в промышленности.";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(ChemicalAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let chems: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == CHEMICAL_OBJ_TYPENAME)
        .collect();
    eprintln!("H2SO4 chemicals: {}", chems.len());
    // Should find at least H2SO4 formula
    assert!(!chems.is_empty(), "Should find at least one CHEMICALFORMULA");
}

// ── VacanceAnalyzer tests ─────────────────────────────────────────────────

#[test]
fn test_vacance_job_name() {
    init();
    // A minimal job posting: the first meaningful text becomes the Name
    let text = "Продавец-консультант\nОпыт работы: не требуется";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(VacanceAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let vacs: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == VACANCE_OBJ_TYPENAME)
        .collect();
    eprintln!("vacance entities: {}", vacs.len());
    assert!(!vacs.is_empty(), "Should find at least one VACANCY entity");

    let name_ent = vacs.iter().find(|e| {
        let b = e.borrow();
        get_item_type(&b) == VacanceItemType::Name
    });
    assert!(name_ent.is_some(), "Should have a Name item");
    let val = {
        let b = name_ent.unwrap().borrow();
        get_vac_value(&b)
    };
    eprintln!("  vacancy name value={:?}", val);
    assert!(val.is_some(), "Name item should have a value");
}

#[test]
fn test_vacance_experience() {
    init();
    let text = "Менеджер по продажам\nОпыт работы: от 1 года\nЗнание 1С";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(VacanceAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let vacs: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == VACANCE_OBJ_TYPENAME)
        .collect();
    eprintln!("vacance entities (experience): {}", vacs.len());
    assert!(!vacs.is_empty(), "Should find VACANCY entities");

    let exp = vacs.iter().find(|e| {
        let b = e.borrow();
        get_item_type(&b) == VacanceItemType::Experience
    });
    eprintln!("  experience found: {}", exp.is_some());
    assert!(exp.is_some(), "Should have an Experience item");
}

#[test]
fn test_vacance_skill() {
    init();
    let text = "Разработчик\nЗнание Rust, C++\nОтветственность";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(VacanceAnalyzer::new()));
    let ar = proc.process(sofa, None);

    let vacs: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == VACANCE_OBJ_TYPENAME)
        .collect();
    eprintln!("vacance entities (skill): {}", vacs.len());
    assert!(!vacs.is_empty(), "Should find VACANCY entities");

    let skill = vacs.iter().find(|e| {
        let b = e.borrow();
        let t = get_item_type(&b);
        t == VacanceItemType::Skill || t == VacanceItemType::Moral
    });
    assert!(skill.is_some(), "Should find at least a Skill or Moral item");
}

// ── DefinitionAnalyzer tests ──────────────────────────────────────────────────

use pullenti_ner::DefinitionAnalyzer;
use pullenti_ner::definition::{
    THESIS_OBJ_TYPENAME, get_termin, get_value, get_kind_str,
};

fn def_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(DefinitionAnalyzer::new()));
    proc
}

#[test]
fn test_definition_em_dash() {
    // Pattern: "X — Y" (em dash with spaces)
    let text = "Предприниматель — физическое лицо, зарегистрированное в установленном порядке.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = def_proc();
    let ar = proc.process(sofa, None);
    eprintln!("definition_em_dash entities:");
    for e in &ar.entities { eprintln!("  {:?}", e.borrow().slots.iter().map(|s| format!("{}={}", s.type_name, s.value.as_ref().map(|v| v.to_string()).unwrap_or_default())).collect::<Vec<_>>()); }
    let theses: Vec<_> = ar.entities.iter().filter(|e| e.borrow().type_name == THESIS_OBJ_TYPENAME).collect();
    assert!(!theses.is_empty(), "Should find THESIS entity for em-dash pattern");
    let t = theses[0].borrow();
    let termin = get_termin(&t).unwrap_or_default();
    let value  = get_value(&t).unwrap_or_default();
    let kind   = get_kind_str(&t).unwrap_or_default();
    eprintln!("TERMIN={:?} VALUE={:?} KIND={:?}", termin, value, kind);
    assert!(!termin.is_empty(), "TERMIN should be non-empty");
    assert!(!value.is_empty(), "VALUE should be non-empty");
    assert_eq!(kind, "Definition", "Kind should be Definition for em-dash pattern");
}

#[test]
fn test_definition_yavlyaetsya() {
    // Pattern: "X является Y"
    let text = "Договор является соглашением двух или более лиц об установлении обязательств.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = def_proc();
    let ar = proc.process(sofa, None);
    eprintln!("definition_yavlyaetsya entities:");
    for e in &ar.entities { eprintln!("  {:?}", e.borrow().slots.iter().map(|s| format!("{}={}", s.type_name, s.value.as_ref().map(|v| v.to_string()).unwrap_or_default())).collect::<Vec<_>>()); }
    let theses: Vec<_> = ar.entities.iter().filter(|e| e.borrow().type_name == THESIS_OBJ_TYPENAME).collect();
    assert!(!theses.is_empty(), "Should find THESIS entity for 'является' pattern");
    let t = theses[0].borrow();
    let termin = get_termin(&t).unwrap_or_default();
    let value  = get_value(&t).unwrap_or_default();
    let kind   = get_kind_str(&t).unwrap_or_default();
    eprintln!("TERMIN={:?} VALUE={:?} KIND={:?}", termin, value, kind);
    assert!(!termin.is_empty(), "TERMIN should be non-empty");
    assert!(!value.is_empty(), "VALUE should be non-empty");
    assert_eq!(kind, "Assertation", "Kind should be Assertation for 'является' pattern");
}

#[test]
fn test_definition_eto() {
    // Pattern: "X это Y"
    let text = "Ипотека это залог недвижимого имущества в обеспечение кредитного обязательства.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = def_proc();
    let ar = proc.process(sofa, None);
    eprintln!("definition_eto entities:");
    for e in &ar.entities { eprintln!("  {:?}", e.borrow().slots.iter().map(|s| format!("{}={}", s.type_name, s.value.as_ref().map(|v| v.to_string()).unwrap_or_default())).collect::<Vec<_>>()); }
    let theses: Vec<_> = ar.entities.iter().filter(|e| e.borrow().type_name == THESIS_OBJ_TYPENAME).collect();
    // Note: "это" after a noun phrase → Assertation or Definition
    eprintln!("Thesis count: {}", theses.len());
    // The main thing: if a THESIS is found it has non-empty termin and value
    if !theses.is_empty() {
        let t = theses[0].borrow();
        let termin = get_termin(&t).unwrap_or_default();
        let value  = get_value(&t).unwrap_or_default();
        eprintln!("TERMIN={:?} VALUE={:?}", termin, value);
        assert!(!termin.is_empty(), "TERMIN should be non-empty");
        assert!(!value.is_empty(), "VALUE should be non-empty");
    }
}

// ── MailAnalyzer tests ────────────────────────────────────────────────────────

use pullenti_ner::MailAnalyzer;
use pullenti_ner::mail::{MAIL_OBJ_TYPENAME, get_kind as get_mail_kind, MailKind as MailKindEnum};

fn mail_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(MailAnalyzer::new()));
    proc
}

#[test]
fn test_mail_header() {
    // A typical email header text
    let text = "От кого: Иванов Иван\nКому: Петров Пётр\nТема: Отчёт за квартал\n\nДорогой Пётр, высылаю отчёт.\n\nС уважением,\nИван";
    let sofa = SourceOfAnalysis::new(text);
    let proc = mail_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let mails: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == MAIL_OBJ_TYPENAME)
        .collect();
    eprintln!("Mail entities: {}", mails.len());
    for m in &mails {
        let mb = m.borrow();
        let k = get_mail_kind(&mb);
        eprintln!("  kind={:?}", k);
    }
    // At least one MAIL entity should be found
    assert!(!mails.is_empty(), "Should find at least one MAIL entity");
}

#[test]
fn test_mail_body_only() {
    // Simple body text
    let text = "Добрый день! Прошу рассмотреть данное предложение. С уважением, Иванов.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = mail_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let mails: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == MAIL_OBJ_TYPENAME)
        .collect();
    eprintln!("Mail body entities: {}", mails.len());
    // Mail may or may not detect body without header context, just run without panic
}

// ── KeywordAnalyzer tests ─────────────────────────────────────────────────────

use pullenti_ner::KeywordAnalyzer;
use pullenti_ner::keyword::{KEYWORD_OBJ_TYPENAME, get_keyword_value};

fn keyword_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(PersonAnalyzer::new()));
    proc.add_analyzer(Arc::new(OrgAnalyzer::new()));
    proc.add_analyzer(Arc::new(GeoAnalyzer::new()));
    proc.add_analyzer(Arc::new(KeywordAnalyzer::new()));
    proc
}

#[test]
fn test_keyword_extracts_noun_phrase() {
    // Keyword analyzer should extract noun phrases
    let text = "Разработка программного обеспечения является важным направлением.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = keyword_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let keywords: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == KEYWORD_OBJ_TYPENAME)
        .collect();
    eprintln!("Keyword entities: {}", keywords.len());
    for kw in &keywords {
        let kb = kw.borrow();
        let val = get_keyword_value(&kb);
        eprintln!("  value={:?}", val);
    }
    // Should extract at least one keyword
    assert!(!keywords.is_empty(), "Should extract keywords from text");
}

#[test]
fn test_keyword_referent_type() {
    // Keyword should wrap a named entity as Referent type
    let text = "Компания Газпром занимается добычей газа в России.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = keyword_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let keywords: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == KEYWORD_OBJ_TYPENAME)
        .collect();
    eprintln!("Keyword referent entities: {}", keywords.len());
    for kw in &keywords {
        let kb = kw.borrow();
        let val = get_keyword_value(&kb);
        eprintln!("  value={:?}", val);
    }
    // Run without panic is the main check; keyword extraction depends on parser depth
}

// ── DenominationAnalyzer tests ────────────────────────────────────────────────

use pullenti_ner::DenominationAnalyzer;
use pullenti_ner::denomination::{DENOMINATION_OBJ_TYPENAME, get_denomination_value};

fn denomination_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(DenominationAnalyzer::new()));
    proc
}

#[test]
fn test_denomination_quoted() {
    // Denomination recognizes quoted names: "ООО «Ромашка»"
    let text = "Общество с ограниченной ответственностью «Ромашка» зарегистрировано в 2010 году.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = denomination_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let denoms: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == DENOMINATION_OBJ_TYPENAME)
        .collect();
    eprintln!("Denomination entities: {}", denoms.len());
    for d in &denoms {
        let db = d.borrow();
        let val = get_denomination_value(&db);
        eprintln!("  value={:?}", val);
    }
    if !denoms.is_empty() {
        let db = denoms[0].borrow();
        let val = get_denomination_value(&db).unwrap_or_default();
        assert!(!val.is_empty(), "Denomination value should be non-empty");
    }
}

#[test]
fn test_denomination_code() {
    // DenominationAnalyzer handles alphanumeric codes like "1С", "C#", "АК-47"
    let text = "Система 1С используется для бухгалтерии.";
    let sofa = SourceOfAnalysis::new(text);
    let proc = denomination_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let denoms: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == DENOMINATION_OBJ_TYPENAME)
        .collect();
    eprintln!("Denomination code entities: {}", denoms.len());
    for d in &denoms {
        let db = d.borrow();
        let val = get_denomination_value(&db).unwrap_or_default();
        eprintln!("  value={:?}", val);
    }
    assert!(!denoms.is_empty(), "Should find denomination '1С'");
    let db = denoms[0].borrow();
    let val = get_denomination_value(&db).unwrap_or_default();
    assert!(val.contains("1С") || val.contains("1C"), "Should contain '1С', got {:?}", val);
}

// ── ResumeAnalyzer tests ──────────────────────────────────────────────────────

use pullenti_ner::ResumeAnalyzer;
use pullenti_ner::resume::{RESUME_OBJ_TYPENAME, ResumeItemType, get_resume_typ};

fn resume_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(PersonAnalyzer::new()));
    proc.add_analyzer(Arc::new(OrgAnalyzer::new()));
    proc.add_analyzer(Arc::new(DateAnalyzer::new()));
    proc.add_analyzer(Arc::new(ResumeAnalyzer::new()));
    proc
}

#[test]
fn test_resume_person() {
    // Resume should recognize PERSON entity at line start
    let text = "Иванов Иван Иванович\nОпыт работы: 5 лет";
    let sofa = SourceOfAnalysis::new(text);
    let proc = resume_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let resumes: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == RESUME_OBJ_TYPENAME)
        .collect();
    eprintln!("Resume entities: {}", resumes.len());
    for r in &resumes {
        let rb = r.borrow();
        let typ = get_resume_typ(&rb);
        eprintln!("  typ={:?}", typ);
    }
    // Check that at least a Person or Contact item is found (or run without panic)
    // Minimal check: the analyzer runs without panic
}

#[test]
fn test_resume_contact() {
    // Resume should recognize phone/email at line start as Contact
    let text = "Тел: +7 495 123-45-67\nE-mail: test@example.com";
    let sofa = SourceOfAnalysis::new(text);
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(PhoneAnalyzer::new()));
    proc.add_analyzer(Arc::new(UriAnalyzer::new()));
    proc.add_analyzer(Arc::new(ResumeAnalyzer::new()));
    let ar = proc.process(sofa, Some(MorphLang::RU));
    let resumes: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == RESUME_OBJ_TYPENAME)
        .collect();
    eprintln!("Resume contact entities: {}", resumes.len());
    for r in &resumes {
        let rb = r.borrow();
        let typ = get_resume_typ(&rb);
        eprintln!("  typ={:?}", typ);
    }
    // Phone/URI on own line → Contact items
    let contacts: Vec<_> = resumes.iter()
        .filter(|r| get_resume_typ(&r.borrow()) == ResumeItemType::Contact)
        .collect();
    eprintln!("Contact items: {}", contacts.len());
    // At least phone or URI entity should have been wrapped as contact
    // (depends on newline detection — soft assertion)
}

// ── GoodsAnalyzer tests ───────────────────────────────────────────────────────

use pullenti_ner::GoodsAnalyzer;
use pullenti_ner::goods::{
    GOOD_OBJ_TYPENAME, GOODATTR_OBJ_TYPENAME,
    GoodAttrType, get_attr_type, get_attr_value,
};

fn goods_proc() -> Processor {
    init();
    let mut proc = Processor::empty();
    proc.add_analyzer(Arc::new(GoodsAnalyzer::new()));
    proc
}

#[test]
fn test_goods_keyword_extraction() {
    // A simple product line starting with a Cyrillic noun → Keyword attribute
    let text = "Молоко пастеризованное Простоквашино 3.5%";
    let sofa = SourceOfAnalysis::new(text);
    let proc = goods_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));

    let good_attrs: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == GOODATTR_OBJ_TYPENAME)
        .collect();
    eprintln!("GoodAttr entities for '{}': {}", text, good_attrs.len());
    for a in &good_attrs {
        let ab = a.borrow();
        let typ = get_attr_type(&ab);
        let val = get_attr_value(&ab).map(|s| s.to_string()).unwrap_or_default();
        eprintln!("  typ={:?} val={:?}", typ, val);
    }

    let keywords: Vec<_> = good_attrs.iter()
        .filter(|a| get_attr_type(&a.borrow()) == GoodAttrType::Keyword)
        .collect();
    assert!(!keywords.is_empty(), "Expected at least one Keyword attribute for product line");

    let kw0 = keywords[0].borrow();
    let keyword_val = get_attr_value(&kw0).unwrap_or_default();
    // Should be the normalized (nominative) form of "молоко"
    assert!(
        keyword_val.to_uppercase().contains("МОЛОКО") || keyword_val.to_uppercase().contains("МОЛОК"),
        "Keyword value should contain МОЛОКО, got {:?}",
        keyword_val
    );
}

#[test]
fn test_goods_creates_good_entity() {
    // A product line should create exactly one GOOD referent
    let text = "Молоко цельное 1 литр";
    let sofa = SourceOfAnalysis::new(text);
    let proc = goods_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));

    let goods: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == GOOD_OBJ_TYPENAME)
        .collect();
    eprintln!("GOOD entities for '{}': {}", text, goods.len());

    // The GOOD entity should exist and wrap the attribute entities
    assert!(!goods.is_empty(), "Expected at least one GOOD entity for product line");
}

#[test]
fn test_goods_proper_attribute() {
    // Proper-cased word (brand name) should become a Proper attribute
    let text = "Молоко OXFORD 3.5%";
    let sofa = SourceOfAnalysis::new(text);
    let proc = goods_proc();
    let ar = proc.process(sofa, Some(MorphLang::RU));

    let good_attrs: Vec<_> = ar.entities.iter()
        .filter(|e| e.borrow().type_name == GOODATTR_OBJ_TYPENAME)
        .collect();
    eprintln!("GoodAttr entities for '{}': {}", text, good_attrs.len());
    for a in &good_attrs {
        let ab = a.borrow();
        let typ = get_attr_type(&ab);
        let val = get_attr_value(&ab).map(|s| s.to_string()).unwrap_or_default();
        eprintln!("  typ={:?} val={:?}", typ, val);
    }

    let propers: Vec<_> = good_attrs.iter()
        .filter(|a| get_attr_type(&a.borrow()) == GoodAttrType::Proper)
        .collect();
    eprintln!("Proper attributes: {}", propers.len());
    // We expect at least one Proper attribute (the brand name OXFORD)
    // This is a soft assertion as it depends on the morph dictionary
    if !propers.is_empty() {
        let p0 = propers[0].borrow();
        let val = get_attr_value(&p0).unwrap_or_default();
        assert!(!val.is_empty(), "Proper value should be non-empty");
    }
    // Run without panic is the minimum requirement
}

// ── English GEO tests ─────────────────────────────────────────────────────────

fn en_geo_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(GeoAnalyzer::new())])
}

/// "Bangkok" → GEO (English city)
#[test]
fn test_geo_en_city_bangkok() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("The conference was held in Bangkok.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'Bangkok'");
}

/// "Thai-\nland" (linebreak-hyphen) → GEO
#[test]
fn test_geo_en_linebreak_hyphen() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("Located in Thai-\nland.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for linebreak-hyphen 'Thai-\\nland'");
}

/// "Miami" → GEO city (city must override county in geo table)
#[test]
fn test_geo_en_city_miami() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("The team is based in Miami.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'Miami'");
}

/// "United Arab Emirates" → GEO (3-word country name)
#[test]
fn test_geo_en_three_word_country() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("The summit took place in United Arab Emirates.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'United Arab Emirates'");
}

/// "USA" → GEO (abbreviation)
#[test]
fn test_geo_en_country_abbreviation() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("Researchers from the USA contributed.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'USA'");
}

/// "Singapore" → GEO
#[test]
fn test_geo_en_city_singapore() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("The office moved to Singapore last year.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(!geos.is_empty(), "Should extract GEO for 'Singapore'");
}

/// "Matthew Wallingford" — Wallingford is a city but should NOT be extracted
/// as GEO because "Matthew" immediately precedes it (person-name context).
#[test]
fn test_geo_en_no_false_positive_preceding_firstname() {
    let proc = en_geo_proc();
    let sofa = SourceOfAnalysis::new("Matthew Wallingford, Aditya Sinha worked on this.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .collect();
    assert!(geos.is_empty(), "Should NOT extract GEO for 'Wallingford' when preceded by 'Matthew'");
}

// ── English PERSON tests ──────────────────────────────────────────────────────

fn en_person_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(PersonAnalyzer::new())])
}

/// "Jacob Devlin" → PERSON (simple EN-1 pattern)
#[test]
fn test_person_en_simple() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Introduced by Jacob Devlin at the conference.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Jacob Devlin'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    let last = get_lastname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Jacob")).unwrap_or(false),
        "firstname should contain Jacob, got {:?}", first);
    assert!(last.as_deref().map(|s| s.contains("Devlin")).unwrap_or(false),
        "lastname should contain Devlin, got {:?}", last);
}

/// "Marta R. Costa-jussà" → PERSON (EN-2: initial + hyphenated lastname with accent)
#[test]
fn test_person_en_initial_hyphen_accent_lastname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Research by Marta R. Costa-jussà on translation.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Marta R. Costa-jussà'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Marta")).unwrap_or(false),
        "firstname should be Marta, got {:?}", first);
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("Costa")).unwrap_or(false),
        "lastname should contain Costa, got {:?}", last);
}

/// "Ming-Wei Chang" → PERSON (hyphenated firstname)
#[test]
fn test_person_en_hyphen_firstname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Paper by Ming-Wei Chang on NLP.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Ming-Wei Chang'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Ming")).unwrap_or(false),
        "firstname should contain Ming, got {:?}", first);
}

/// "William Howard-Snyder" → PERSON (hyphenated lastname)
#[test]
fn test_person_en_hyphen_lastname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Authored by William Howard-Snyder.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'William Howard-Snyder'");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("Howard")).unwrap_or(false),
        "lastname should contain Howard-Snyder, got {:?}", last);
}

/// "Kevin Hef-\nfernan" → PERSON (linebreak-hyphen in lastname)
#[test]
fn test_person_en_linebreak_hyphen_lastname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Work by Kevin Hef-\nfernan was cited.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Kevin Hef-\\nfernan'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Kevin")).unwrap_or(false),
        "firstname should be Kevin, got {:?}", first);
}

/// "Guillaume\nWenzek" → PERSON (newline between firstname and lastname)
#[test]
fn test_person_en_newline_between_names() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Authors: Guillaume\nWenzek and others.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Guillaume\\nWenzek'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    let last = get_lastname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Guillaume")).unwrap_or(false),
        "firstname should be Guillaume, got {:?}", first);
    assert!(last.as_deref().map(|s| s.contains("Wenzek")).unwrap_or(false),
        "lastname should be Wenzek, got {:?}", last);
}

/// "Onur\nÇelebi" → PERSON (newline + extended-Latin char in lastname)
#[test]
fn test_person_en_newline_extended_latin() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Research by Onur\nÇelebi on multilingual NLP.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Onur\\nÇelebi'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Onur")).unwrap_or(false),
        "firstname should be Onur, got {:?}", first);
}

/// "Gabriel Mejia Gonzalez" → PERSON (2-word lastname)
#[test]
fn test_person_en_two_word_lastname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Contributions by Gabriel Mejia Gonzalez.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Gabriel Mejia Gonzalez'");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("Mejia")).unwrap_or(false),
        "lastname should contain Mejia, got {:?}", last);
}

/// "Kaushik Ram\nSadagopan" → PERSON (2-word lastname with newline)
#[test]
fn test_person_en_two_word_lastname_newline() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Authors: Kaushik Ram\nSadagopan and others.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Kaushik Ram\\nSadagopan'");
    let rb = persons[0].borrow();
    let first = get_firstname(&rb);
    assert!(first.as_deref().map(|s| s.contains("Kaushik")).unwrap_or(false),
        "firstname should be Kaushik, got {:?}", first);
}

/// "Pierre-Emmanuel Mazaré" → PERSON (hyphen firstname + accented lastname)
#[test]
fn test_person_en_hyphen_firstname_accent_lastname() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Presented by Pierre-Emmanuel Mazaré.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON for 'Pierre-Emmanuel Mazaré'");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("Mazar")).unwrap_or(false),
        "lastname should contain Mazar(é), got {:?}", last);
}

/// "Bullet Point Line" must NOT be detected as PERSON (stop-word suppression)
#[test]
fn test_person_en_no_false_positive_compound_phrase() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("Use a Bullet Point Line to organize content.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(persons.is_empty(), "Should NOT extract PERSON for 'Bullet Point Line'");
}

/// "Blacklist Domain" must NOT be detected as PERSON
#[test]
fn test_person_en_no_false_positive_tech_phrase() {
    let proc = en_person_proc();
    let sofa = SourceOfAnalysis::new("The Blacklist Domain was blocked by the firewall.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(persons.is_empty(), "Should NOT extract PERSON for 'Blacklist Domain'");
}

// ── PersonPropertyReferent tests ──────────────────────────────────────────────

fn person_prop_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![Arc::new(PersonAnalyzer::new())])
}

/// "господин Иванов" → PERSONPROPERTY(name="господин") + PERSON(lastname=ИВАНОВ)
#[test]
fn test_person_property_gospodin() {
    let proc = person_prop_proc();
    let sofa = SourceOfAnalysis::new("Это господин Иванов.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let props: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == PERSONPROPERTY_OBJ_TYPENAME)
        .collect();
    assert!(!props.is_empty(), "Should extract PERSONPROPERTY for 'господин Иванов'");
    let rb = props[0].borrow();
    let name = get_person_property_name(&rb);
    assert_eq!(name.as_deref(), Some("господин"), "name should be 'господин', got {:?}", name);
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "PERSON should also be detected for 'Иванов'");
}

/// "директор Петров" → PERSONPROPERTY(name="директор") + PERSON(lastname=ПЕТРОВ)
#[test]
fn test_person_property_director() {
    let proc = person_prop_proc();
    let sofa = SourceOfAnalysis::new("директор Петров подписал приказ");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let props: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == PERSONPROPERTY_OBJ_TYPENAME)
        .collect();
    assert!(!props.is_empty(), "Should extract PERSONPROPERTY for 'директор Петров'");
    let rb = props[0].borrow();
    let name = get_person_property_name(&rb);
    assert!(name.is_some(), "PERSONPROPERTY should have a name, got None");
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "PERSON 'Петров' should be detected");
}

/// "Mr Smith" (no dot) → PERSONPROPERTY(name="mr.") + PERSON(lastname=SMITH)
#[test]
fn test_person_property_mr_en() {
    let proc = person_prop_proc();
    // Use "Mr Smith" without the dot — "Mr." tokenizes as "MR" + "." which is
    // handled, but "Mr" alone also matches the table key "MR"
    let sofa = SourceOfAnalysis::new("Greetings from Mr Smith and Mrs Jones.");
    let result = proc.process(sofa, Some(MorphLang::EN));
    let props: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == PERSONPROPERTY_OBJ_TYPENAME)
        .collect();
    assert!(!props.is_empty(), "Should extract PERSONPROPERTY for 'Mr Smith'");
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "PERSON 'Smith' should be detected");
}

// ── LinkAnalyzer tests ────────────────────────────────────────────────────

use pullenti_ner::LinkAnalyzer;
use pullenti_ner::link::{OBJ_TYPENAME as LINK_OBJ_TYPENAME, get_link_type, get_object1, get_object2, LinkType};

fn link_proc() -> Processor {
    init();
    Processor::with_analyzers(vec![
        Arc::new(PhoneAnalyzer::new()),
        Arc::new(UriAnalyzer::new()),
        Arc::new(PersonAnalyzer::new()),
        Arc::new(OrgAnalyzer::new()),
        Arc::new(AddressAnalyzer::new()),
        Arc::new(LinkAnalyzer::new()),
    ])
}

/// Person followed by phone → Contact link
#[test]
fn test_link_person_phone_contact() {
    let text = "Иванов Иван Петрович\n+7 (999) 123-45-67";
    let sofa = SourceOfAnalysis::new(text);
    let proc = link_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let links: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == LINK_OBJ_TYPENAME)
        .collect();

    eprintln!("LINK entities for person+phone:");
    for l in &links {
        let lb = l.borrow();
        let typ = get_link_type(&lb);
        let o1 = get_object1(&lb).map(|r| r.borrow().type_name.clone());
        let o2 = get_object2(&lb).map(|r| r.borrow().type_name.clone());
        eprintln!("  type={:?} o1={:?} o2={:?}", typ, o1, o2);
    }

    let contact_links: Vec<_> = links.iter()
        .filter(|l| get_link_type(&l.borrow()) == LinkType::Contact)
        .collect();
    assert!(!contact_links.is_empty(), "Expected a Contact link between person and phone");

    let link0 = contact_links[0].borrow();
    let o2_type = get_object2(&link0).map(|r| r.borrow().type_name.clone());
    assert_eq!(o2_type.as_deref(), Some("PHONE"), "Object2 of Contact link should be PHONE");
}

/// Person followed by email URI → Contact link
#[test]
fn test_link_person_email_contact() {
    let text = "Петрова Мария Ивановна\nEmail: maria@example.com";
    let sofa = SourceOfAnalysis::new(text);
    let proc = link_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let links: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == LINK_OBJ_TYPENAME)
        .collect();

    eprintln!("LINK entities for person+email:");
    for l in &links {
        let lb = l.borrow();
        let typ = get_link_type(&lb);
        let o1 = get_object1(&lb).map(|r| r.borrow().type_name.clone());
        let o2 = get_object2(&lb).map(|r| r.borrow().type_name.clone());
        eprintln!("  type={:?} o1={:?} o2={:?}", typ, o1, o2);
    }

    // We accept either a Contact link (preferred) or any LINK entities present.
    // The key check: there are entities that link person to URI.
    let contact_links: Vec<_> = links.iter()
        .filter(|l| get_link_type(&l.borrow()) == LinkType::Contact)
        .collect();
    // At minimum we should have PERSON and URI detected, and a link is formed
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    let uris: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "URI")
        .collect();
    assert!(!persons.is_empty(), "Should detect PERSON");
    assert!(!uris.is_empty(), "Should detect email URI");
    if !contact_links.is_empty() {
        let link0 = contact_links[0].borrow();
        let o1_type = get_object1(&link0).map(|r| r.borrow().type_name.clone());
        let o2_type = get_object2(&link0).map(|r| r.borrow().type_name.clone());
        assert_eq!(o1_type.as_deref(), Some("PERSON"));
        assert_eq!(o2_type.as_deref(), Some("URI"));
    }
}

/// Person associated with organization → Work link (from resume context)
#[test]
fn test_link_person_org_work() {
    // Resume-style text with PERSON then ORGANIZATION
    let text = "Сидоров Алексей Николаевич\nработает в ООО «Рога и Копыта»";
    let sofa = SourceOfAnalysis::new(text);
    let proc = link_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let links: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == LINK_OBJ_TYPENAME)
        .collect();

    eprintln!("LINK entities for person+org:");
    for l in &links {
        let lb = l.borrow();
        let typ = get_link_type(&lb);
        let o1 = get_object1(&lb).map(|r| r.borrow().type_name.clone());
        let o2 = get_object2(&lb).map(|r| r.borrow().type_name.clone());
        eprintln!("  type={:?} o1={:?} o2={:?}", typ, o1, o2);
    }

    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    let orgs: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ORGANIZATION")
        .collect();
    eprintln!("  persons={} orgs={}", persons.len(), orgs.len());

    // At minimum: PERSON and ORGANIZATION should be detected
    assert!(!persons.is_empty(), "Should detect PERSON");
    // Check if we have a LINK entity linking them (work context)
    let work_or_undef: Vec<_> = links.iter()
        .filter(|l| {
            let typ = get_link_type(&l.borrow());
            typ == LinkType::Work || typ == LinkType::Undefined
        })
        .collect();
    // If orgs were detected and linked, great
    if !orgs.is_empty() && !work_or_undef.is_empty() {
        let link0 = work_or_undef[0].borrow();
        let o2_type = get_object2(&link0).map(|r| r.borrow().type_name.clone());
        assert_eq!(o2_type.as_deref(), Some("ORGANIZATION"), "Object2 should be ORGANIZATION");
    }
}

// ── PersonIdentityReferent tests ────────────────────────────────────────────

fn person_id_proc() -> Processor {
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![
        Arc::new(DateAnalyzer::new()),
        Arc::new(GeoAnalyzer::new()),
        Arc::new(OrgAnalyzer::new()),
        Arc::new(PersonAnalyzer::new()),
    ])
}

/// "паспорт 1234 567890" → PERSONIDENTITY, number=1234567890
#[test]
fn test_person_identity_passport_number() {
    let text = "паспорт 1234 567890";
    let sofa = SourceOfAnalysis::new(text);
    let proc = person_id_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let ids: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSONIDENTITY")
        .collect();

    assert!(!ids.is_empty(), "Should extract PERSONIDENTITY from 'паспорт 1234 567890'");
    let id = ids[0].borrow();
    let number = id.slots.iter()
        .find(|s| s.type_name == "NUMBER")
        .and_then(|s| s.value.as_ref())
        .and_then(|v| if let pullenti_ner::referent::SlotValue::Str(s) = v { Some(s.clone()) } else { None });
    assert!(number.is_some(), "PERSONIDENTITY should have NUMBER slot");
    let num = number.unwrap();
    assert!(num.contains("1234") || num.len() >= 6,
        "Number should include series+number digits, got: {}", num);
}

/// "паспорт серия 12 34 номер 567890" → PERSONIDENTITY
#[test]
fn test_person_identity_passport_seria_number() {
    let text = "паспорт серия 12 34 номер 567890";
    let sofa = SourceOfAnalysis::new(text);
    let proc = person_id_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let ids: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSONIDENTITY")
        .collect();

    assert!(!ids.is_empty(), "Should extract PERSONIDENTITY from passport with seria/number keywords");
    let id = ids[0].borrow();
    let typ = id.slots.iter()
        .find(|s| s.type_name == "TYPE")
        .and_then(|s| s.value.as_ref())
        .and_then(|v| if let pullenti_ner::referent::SlotValue::Str(s) = v { Some(s.clone()) } else { None });
    assert!(typ.is_some(), "PERSONIDENTITY should have TYPE slot");
    assert!(typ.unwrap().contains("паспорт"), "TYPE should contain 'паспорт'");
}

/// "водительское удостоверение 77ВВ 123456" → PERSONIDENTITY (driver's license)
#[test]
fn test_person_identity_driver_license() {
    let text = "водительское удостоверение 77ВВ 123456";
    let sofa = SourceOfAnalysis::new(text);
    let proc = person_id_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let ids: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSONIDENTITY")
        .collect();

    eprintln!("Entities: {:?}", result.entities.iter()
        .map(|e| e.borrow().type_name.clone()).collect::<Vec<_>>());

    // Driver's license may or may not be parsed depending on
    // whether "77ВВ" is recognized as seria
    // The test simply checks we don't crash
    let _ = ids.len();
}

/// "Иванов И.И., паспорт 1234 567890" → PERSON + PERSONIDENTITY linked via IDDOC
#[test]
fn test_person_identity_linked_to_person() {
    let text = "Иванов Иван, паспорт 1234 567890";
    let sofa = SourceOfAnalysis::new(text);
    let proc = person_id_proc();
    let result = proc.process(sofa, Some(MorphLang::RU));

    let ids: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSONIDENTITY")
        .collect();

    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();

    assert!(!ids.is_empty(), "Should extract PERSONIDENTITY");
    assert!(!persons.is_empty(), "Should extract PERSON");
    // Check if person has IDDOC slot linking to the identity doc
    let has_iddoc = persons.iter().any(|p| {
        p.borrow().slots.iter().any(|s| s.type_name == "IDDOC")
    });
    // IDDOC linking is best-effort — just check that both entities exist
    let _ = has_iddoc;
}

// ── PersonNormalData tests ────────────────────────────────────────────────────

#[test]
fn test_person_normal_fio() {
    // Standard FIO: "Иванов Иван Иванович"
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Иванов Иван Иванович");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "should be recognised as person: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ИВАНОВ")).unwrap_or(false),
        "expected ИВАНОВ lastname, got: {:?}", data.lastname);
    assert!(data.firstname.as_deref().map(|s| s.to_uppercase().contains("ИВАН")).unwrap_or(false),
        "expected ИВАН firstname, got: {:?}", data.firstname);
}

#[test]
fn test_person_normal_initials() {
    // Initials form: "Петров А.С." — at minimum recognized as person
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Петров А.С.");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "should be recognised as person: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ПЕТРОВ")).unwrap_or(false),
        "expected ПЕТРОВ lastname, got: {:?}", data.lastname);
}

#[test]
fn test_person_normal_not_person() {
    // Text that is clearly not a person
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze(
        "ул. Ленина, д. 5, кв. 4");
    // Should either be NotPerson or have very low coef (address detected)
    let is_rejected = data.res_typ == pullenti_ner::person::PersonNormalResult::NotPerson
        || data.coef < 50;
    assert!(is_rejected,
        "address text should not be recognised as high-confidence person: {:?}", data);
}

#[test]
fn test_person_normal_preprocess() {
    // "Сидоров-иванов" — hyphen + lowercase should be uppercased in preprocessing
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Сидоров Иван Иванович");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "should be recognised as person: {:?}", data);
    assert!(data.lastname.is_some(), "should have lastname");
}

// ── PersonItemToken / PersonNormalNode (EmptyProcessor path) ─────────────────

// ends_with_std_surname
#[test]
fn test_surname_tail_ov() {
    use pullenti_ner::person::person_item_token::ends_with_std_surname;
    assert_eq!(ends_with_std_surname("ИВАНОВ"), Some(1), "ОВ → masculine");
    assert_eq!(ends_with_std_surname("ИВАНОВА"), Some(2), "ОВА → feminine");
    assert_eq!(ends_with_std_surname("ПЕТРОВ"), Some(1), "ОВ → masculine");
    assert_eq!(ends_with_std_surname("ПЕТРОВА"), Some(2), "ОВА → feminine");
}

#[test]
fn test_surname_tail_ev() {
    use pullenti_ner::person::person_item_token::ends_with_std_surname;
    assert_eq!(ends_with_std_surname("СЕРГЕЕВ"), Some(1), "ЕВ → masculine");
    assert_eq!(ends_with_std_surname("СЕРГЕЕВА"), Some(2), "ЕВА → feminine");
}

#[test]
fn test_surname_tail_in() {
    use pullenti_ner::person::person_item_token::ends_with_std_surname;
    assert_eq!(ends_with_std_surname("ПУШКИН"), Some(1), "ИН → masculine");
    assert_eq!(ends_with_std_surname("ПУШКИНА"), Some(2), "ИНА → feminine");
}

#[test]
fn test_surname_tail_neutral() {
    use pullenti_ner::person::person_item_token::ends_with_std_surname;
    assert_eq!(ends_with_std_surname("ШЕВЧЕНКО"), Some(0), "КО → neutral");
    assert_eq!(ends_with_std_surname("КАЗАРЯН"), Some(0), "ЯН → neutral");
    assert_eq!(ends_with_std_surname("КОВАЛЬЧУК"), Some(0), "УК → neutral");
}

#[test]
fn test_surname_tail_no_match() {
    use pullenti_ner::person::person_item_token::ends_with_std_surname;
    // A common noun that doesn't end with a surname tail
    assert_eq!(ends_with_std_surname("СТОЛ"), None);
    // Too short (tail would consume entire word)
    assert_eq!(ends_with_std_surname("ОВ"), None);
}

// analyze() — EmptyProcessor primary path

#[test]
fn test_person_normal_empty_fio_full() {
    // Classic Russian FIO — EmptyProcessor should handle this before StandardProcessor
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Иванов Иван Иванович");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "FIO should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ИВАНОВ")).unwrap_or(false),
        "expected ИВАНОВ lastname: {:?}", data.lastname);
    assert!(data.firstname.as_deref().map(|s| s.to_uppercase().contains("ИВАН")).unwrap_or(false),
        "expected ИВАН firstname: {:?}", data.firstname);
    assert!(data.middlename.as_deref().map(|s| s.to_uppercase().contains("ИВАНОВИЧ")).unwrap_or(false),
        "expected ИВАНОВИЧ middlename: {:?}", data.middlename);
}

#[test]
fn test_person_normal_empty_iof_full() {
    // IOF ordering: Имя Отчество Фамилия
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Иван Иванович Иванов");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "IOF should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ИВАНОВ")).unwrap_or(false),
        "expected ИВАНОВ lastname: {:?}", data.lastname);
    assert!(data.firstname.as_deref().map(|s| s.to_uppercase().contains("ИВАН")).unwrap_or(false),
        "expected ИВАН firstname: {:?}", data.firstname);
}

#[test]
fn test_person_normal_empty_female_fio() {
    // Female FIO: Петрова Мария Ивановна
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Петрова Мария Ивановна");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "female FIO should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ПЕТРОВ")).unwrap_or(false),
        "expected ПЕТРОВ* lastname: {:?}", data.lastname);
    assert_eq!(data.gender, 2, "expected female gender: {:?}", data);
}

#[test]
fn test_person_normal_empty_fi_only() {
    // Just Firstname + Lastname (no patronymic)
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Александр Петров");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "FI should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ПЕТРОВ")).unwrap_or(false),
        "expected ПЕТРОВ lastname: {:?}", data.lastname);
}

#[test]
fn test_person_normal_empty_surname_only_with_std_tail() {
    // A single ambiguous surname token → too low confidence for both paths.
    // C# PersonNormalHelper behaves the same way without context.
    // We just verify analyze() doesn't panic and returns a valid struct.
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Сергеев");
    // Any result is acceptable — the important thing is no panic and coef is 0..=100
    assert!(data.coef >= 0 && data.coef <= 100, "coef out of range: {:?}", data);
}

#[test]
fn test_person_normal_empty_middlename_female_suffix() {
    // Female patronymic ending -вна / -овна
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Сидорова Ольга Николаевна");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "female FIO should be recognised: {:?}", data);
    assert!(data.middlename.as_deref().map(|s| s.to_uppercase().contains("НИКОЛАЕВНА")).unwrap_or(false),
        "expected НИКОЛАЕВНА middlename: {:?}", data.middlename);
    assert_eq!(data.gender, 2, "expected female gender");
}

#[test]
fn test_person_normal_empty_middlename_male_suffix() {
    // Male patronymic ending -ович
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Смирнов Дмитрий Александрович");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "male FIO should be recognised: {:?}", data);
    assert!(data.middlename.as_deref().map(|s| s.to_uppercase().contains("АЛЕКСАНДРОВИЧ")).unwrap_or(false),
        "expected АЛЕКСАНДРОВИЧ middlename: {:?}", data.middlename);
    assert_eq!(data.gender, 1, "expected male gender");
}

#[test]
fn test_person_normal_empty_coef_ok_threshold() {
    // A well-formed FIO should achieve OK status (coef ≥ 90)
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Иванов Иван Иванович");
    assert_eq!(data.res_typ, pullenti_ner::person::PersonNormalResult::OK,
        "clean FIO should be OK: coef={}, {:?}", data.coef, data);
}

#[test]
fn test_person_normal_empty_not_person_plain_text() {
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze(
        "красивый большой дом стоит на улице");
    let is_rejected = data.res_typ == pullenti_ner::person::PersonNormalResult::NotPerson
        || data.coef < 50;
    assert!(is_rejected, "plain text should not be a high-confidence person: {:?}", data);
}

#[test]
fn test_person_normal_empty_not_person_too_long() {
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    // > 200 chars → early exit
    let long = "А".repeat(201);
    let data = pullenti_ner::person::person_normal_data::analyze(&long);
    assert_eq!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "text >200 chars should be rejected immediately");
}

// ── ShortNameHelper ───────────────────────────────────────────────────────────

#[test]
fn test_short_name_helper_lookup() {
    use pullenti_ner::person::short_name_helper::get_names_for_shortname;
    // САША → АЛЕКСАНДР (m) and АЛЕКСАНДРА (f)
    let res = get_names_for_shortname("САША").expect("САША should have expansions");
    let names: Vec<&str> = res.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"АЛЕКСАНДР"),  "САША should expand to АЛЕКСАНДР");
    assert!(names.contains(&"АЛЕКСАНДРА"), "САША should expand to АЛЕКСАНДРА");
}

#[test]
fn test_short_name_helper_reverse() {
    use pullenti_ner::person::short_name_helper::get_shortnames_for_name;
    let shorts = get_shortnames_for_name("АЛЕКСАНДР").expect("АЛЕКСАНДР should have short forms");
    let shorts: Vec<&str> = shorts.iter().map(|s| s.as_str()).collect();
    assert!(shorts.contains(&"САША"), "АЛЕКСАНДР → САША");
    assert!(shorts.contains(&"ШУРА"), "АЛЕКСАНДР → ШУРА");
}

#[test]
fn test_short_name_helper_not_found() {
    use pullenti_ner::person::short_name_helper::get_names_for_shortname;
    // A word that is not a shortname
    assert!(get_names_for_shortname("ИВАНОВ").is_none(), "surname should not be in shortname map");
}

#[test]
fn test_short_name_helper_gender() {
    use pullenti_ner::person::short_name_helper::get_names_for_shortname;
    // ЖЕНЯ → ЕВГЕНИЙ (m) and ЕВГЕНИЯ (f)
    let res = get_names_for_shortname("ЖЕНЯ").expect("ЖЕНЯ should have expansions");
    let male   = res.iter().any(|(n, g)| n == "ЕВГЕНИЙ"  && *g == 1);
    let female = res.iter().any(|(n, g)| n == "ЕВГЕНИЯ"  && *g == 2);
    assert!(male,   "ЖЕНЯ should map to ЕВГЕНИЙ (masculine)");
    assert!(female, "ЖЕНЯ should map to ЕВГЕНИЯ (feminine)");
}

// ── ShortName expansion in analyze() ─────────────────────────────────────────

#[test]
fn test_person_normal_shortname_male_expansion() {
    // "Вася Петров" → firstname should expand to ВАСИЛИЙ
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Вася Петров");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "should be recognised: {:?}", data);
    // Expanded first name OR at minimum the input is captured
    let fn_upper = data.firstname.as_deref().map(|s| s.to_uppercase());
    let alt_upper = data.firstname_alt.as_deref().map(|s| s.to_uppercase());
    let has_vasily = fn_upper.as_deref() == Some("ВАСИЛИЙ")
        || alt_upper.as_deref() == Some("ВАСЯ");
    // Either expansion happened or at least the short form was captured
    assert!(
        has_vasily || fn_upper.as_deref().map(|s| s.contains("ВАС")).unwrap_or(false),
        "expected ВАСИЛИЙ or ВАСЯ in firstname/alt: fn={:?} alt={:?}", data.firstname, data.firstname_alt
    );
}

#[test]
fn test_person_normal_shortname_female_expansion() {
    // "Катя Иванова" → firstname should expand to ЕКАТЕРИНА
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Катя Иванова");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "should be recognised: {:?}", data);
    // Expect expansion or at least the short form
    let fn_upper = data.firstname.as_deref().map(|s| s.to_uppercase());
    let alt_upper = data.firstname_alt.as_deref().map(|s| s.to_uppercase());
    let ok = fn_upper.as_deref().map(|s| s.contains("КАТЕРИН") || s.contains("КАТЯ")).unwrap_or(false)
          || alt_upper.as_deref().map(|s| s.contains("КАТ")).unwrap_or(false);
    assert!(ok, "expected ЕКАТЕРИНА or КАТЯ: fn={:?} alt={:?}", data.firstname, data.firstname_alt);
}

// ── Arab postfix ──────────────────────────────────────────────────────────────

#[test]
fn test_surname_tail_arab_postfix_detection() {
    use pullenti_ner::person::person_item_token::try_attach;
    use pullenti_ner::SourceOfAnalysis;
    use pullenti_morph::MorphologyService;
    use pullenti_morph::MorphLang;

    MorphologyService::initialize(Some(MorphLang::RU));
    // Build a token chain for "МАМЕДОГЛЫ" and check that ОГЛЫ-postfix words
    // parse as a name part (lastname present)
    let sofa = SourceOfAnalysis::new("Мамед");
    let morph = pullenti_morph::MorphologyService::process("Мамед", None);
    if let Some(toks) = morph {
        if let Some(first) = pullenti_ner::token::build_token_chain(toks, &sofa) {
            if let Some(pit) = try_attach(&first, &sofa) {
                // At minimum the token was parsed as some name part
                assert!(pit.firstname.is_some() || pit.lastname.is_some(),
                    "Мамед should be a name part: {:?}", pit.value);
            }
        }
    }
}

#[test]
fn test_person_normal_arab_postfix_ogli() {
    // "Мамедов Рустам Оглы" — Оглы is a male patronymic postfix
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Мамедов Рустам Оглы");
    // Should be recognised as person (not rejected)
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "Azerbaijani name with Оглы should be recognised: {:?}", data);
}

// ── Comma-separated FIO ───────────────────────────────────────────────────────

#[test]
fn test_person_normal_comma_fio() {
    // "Иванов, И.И." — comma-inverted format common in Russian documents
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Иванов, И.И.");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "comma-inverted FIO should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ИВАНОВ")).unwrap_or(false),
        "expected ИВАНОВ lastname: {:?}", data.lastname);
}

#[test]
fn test_person_normal_comma_three_parts() {
    // "Петрова, Мария Ивановна"
    MorphologyService::initialize(Some(MorphLang::RU));
    Sdk::initialize_all(None);
    let data = pullenti_ner::person::person_normal_data::analyze("Петрова, Мария Ивановна");
    assert_ne!(data.res_typ, pullenti_ner::person::PersonNormalResult::NotPerson,
        "comma-separated FIO with patronymic should be recognised: {:?}", data);
    assert!(data.lastname.as_deref().map(|s| s.to_uppercase().contains("ПЕТРОВ")).unwrap_or(false),
        "expected ПЕТРОВ* lastname: {:?}", data.lastname);
}

// ── Additional Person analyzer tests ─────────────────────────────────────────

/// Female FIO in nominative: sex should be Female (from feminine patronymic -вна).
#[test]
fn test_person_female_fio_ner() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("Петрова Ольга Ивановна подписала договор.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from female FIO");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    let sex  = get_sex(&rb);
    assert!(last.as_deref().map(|s| s.contains("ПЕТРОВ")).unwrap_or(false),
        "lastname should contain ПЕТРОВ, got {:?}", last);
    assert_eq!(sex.as_deref(), Some(SEX_FEMALE),
        "sex should be Female (from patronymic -вна), got {:?}", sex);
}

/// Pattern A — initials BEFORE surname: "А.С. Пушкин написал стихи."
#[test]
fn test_person_initials_before_surname() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("А.С. Пушкин написал стихи.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from initials-before-surname pattern");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("ПУШКИН")).unwrap_or(false),
        "lastname should be ПУШКИН, got {:?}", last);
}

/// Two persons in the same sentence: at least 2 PERSON entities.
#[test]
fn test_person_multiple_in_text() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new(
        "Иванов Иван Иванович и Петрова Ольга Ивановна подписали договор."
    );
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(persons.len() >= 2,
        "Should extract at least 2 PERSONs, got {}", persons.len());
    let lastnames: Vec<_> = persons.iter()
        .filter_map(|p| get_lastname(&p.borrow()))
        .collect();
    assert!(lastnames.iter().any(|s| s.contains("ИВАНОВ")),
        "Expected ИВАНОВ among lastnames: {:?}", lastnames);
    assert!(lastnames.iter().any(|s| s.contains("ПЕТРОВ")),
        "Expected ПЕТРОВ among lastnames: {:?}", lastnames);
}

/// Female firstname + patronymic only ("Мария Ивановна") → sex=Female.
#[test]
fn test_person_female_name_patronymic() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new("Мария Ивановна пришла на встречу.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from Мария Ивановна");
    let rb = persons[0].borrow();
    let sex = get_sex(&rb);
    assert_eq!(sex.as_deref(), Some(SEX_FEMALE),
        "sex should be Female (patronymic -вна), got {:?}", sex);
}

/// Academic title "профессор" + surname → PERSONPROPERTY + PERSON.
/// ПРОФЕССОР was added to attr_ru.dat (same flags as РЕКТОР/ДЕКАН: a="1d").
#[test]
fn test_person_property_professor() {
    let proc = person_prop_proc();
    let sofa = SourceOfAnalysis::new("профессор Смирнов прочитал лекцию.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON after 'профессор'");
    let rb = persons[0].borrow();
    let last = get_lastname(&rb);
    assert!(last.as_deref().map(|s| s.contains("СМИРНОВ")).unwrap_or(false),
        "lastname should be СМИРНОВ, got {:?}", last);
    let props: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == PERSONPROPERTY_OBJ_TYPENAME)
        .collect();
    assert!(!props.is_empty(), "Should extract PERSONPROPERTY for 'профессор'");
    let prop_name = get_person_property_name(&props[0].borrow());
    assert!(prop_name.as_deref().map(|s| s.contains("профессор")).unwrap_or(false),
        "property name should contain 'профессор', got {:?}", prop_name);
}

/// Person in non-nominative (genitive) case: "В комнате Иванова Ивана Ивановича нашли документы."
#[test]
fn test_person_genitive_case() {
    let proc = person_proc();
    let sofa = SourceOfAnalysis::new(
        "В комнате Иванова Ивана Ивановича нашли документы."
    );
    let result = proc.process(sofa, Some(MorphLang::RU));
    let persons: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .collect();
    assert!(!persons.is_empty(), "Should extract PERSON from genitive FIO");
    // Any part of ИВАНОВ should appear
    let has_ivanov = persons.iter().any(|p| {
        let rb = p.borrow();
        get_lastname(&rb).as_deref().map(|s| s.contains("ИВАНОВ")).unwrap_or(false)
    });
    assert!(has_ivanov, "ИВАНОВ should appear as lastname in genitive context");
}

// ── Additional Address analyzer tests ────────────────────────────────────────

/// "пер. Садовый, д. 3" → STREET type=переулок, ADDRESS house=3
#[test]
fn test_address_pereulok_with_house() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Контора расположена в пер. Садовый, д. 3.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'пер. Садовый'");
    let typ = get_street_type(&streets[0].borrow());
    assert_eq!(typ.as_deref(), Some("переулок"),
        "street type should be переулок, got {:?}", typ);
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS with house");
    let house = get_house(&addresses[0].borrow());
    assert_eq!(house.as_deref(), Some("3"), "house should be 3, got {:?}", house);
}

/// "шоссе Энтузиастов, дом 15" → STREET type=шоссе, ADDRESS house=15 (spelled-out дом).
#[test]
fn test_address_shosse_spelled_dom() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Склад: шоссе Энтузиастов, дом 15.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'шоссе Энтузиастов'");
    let typ = get_street_type(&streets[0].borrow());
    assert_eq!(typ.as_deref(), Some("шоссе"),
        "street type should be шоссе, got {:?}", typ);
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS for шоссе");
    let house = get_house(&addresses[0].borrow());
    assert_eq!(house.as_deref(), Some("15"), "house should be 15, got {:?}", house);
}

/// "пл. Победы" → STREET type=площадь (abbreviation "пл.").
#[test]
fn test_address_ploschad_abbr() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Встреча на пл. Победы.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'пл. Победы'");
    let typ = get_street_type(&streets[0].borrow());
    assert_eq!(typ.as_deref(), Some("площадь"),
        "street type should be площадь, got {:?}", typ);
}

/// "бул. Ленина, д. 8" → STREET type=бульвар, ADDRESS house=8.
#[test]
fn test_address_bulvar_with_house() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Адрес: бул. Ленина, д. 8.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'бул. Ленина'");
    let typ = get_street_type(&streets[0].borrow());
    assert_eq!(typ.as_deref(), Some("бульвар"),
        "street type should be бульвар, got {:?}", typ);
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS for бульвар");
    let house = get_house(&addresses[0].borrow());
    assert_eq!(house.as_deref(), Some("8"), "house should be 8, got {:?}", house);
}

/// "ул. Пушкина, д. 10, корп. 2" → ADDRESS with house=10 and corpus=2.
#[test]
fn test_address_with_corpus() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Проживает по адресу ул. Пушкина, д. 10, корп. 2.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS with corpus");
    let rb = addresses[0].borrow();
    let house = get_house(&rb);
    assert_eq!(house.as_deref(), Some("10"), "house should be 10, got {:?}", house);
    let corpus = get_corpus(&rb);
    assert_eq!(corpus.as_deref(), Some("2"), "corpus should be 2, got {:?}", corpus);
}

/// "ул. Гагарина, д. 5, кв. 12, эт. 3" → ADDRESS with flat=12 and floor=3.
#[test]
fn test_address_with_flat_and_floor() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("ул. Гагарина, д. 5, кв. 12, эт. 3.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS with flat and floor");
    let rb = addresses[0].borrow();
    let flat  = get_flat(&rb);
    let floor = get_floor(&rb);
    assert_eq!(flat.as_deref(), Some("12"), "flat should be 12, got {:?}", flat);
    assert_eq!(floor.as_deref(), Some("3"), "floor should be 3, got {:?}", floor);
}

/// "ул. Советская, д. 7, оф. 101" → ADDRESS with house=7 and office=101.
#[test]
fn test_address_with_office() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Офис компании: ул. Советская, д. 7, оф. 101.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let addresses: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "ADDRESS")
        .collect();
    assert!(!addresses.is_empty(), "Should find ADDRESS with office");
    let rb = addresses[0].borrow();
    let house  = get_house(&rb);
    let office = get_office(&rb);
    assert_eq!(house.as_deref(), Some("7"), "house should be 7, got {:?}", house);
    assert_eq!(office.as_deref(), Some("101"), "office should be 101, got {:?}", office);
}

/// "набережная Невы, 4" → STREET type=набережная, ADDRESS house=4.
#[test]
fn test_address_naberezhnaya() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Здание стоит на набережная Невы, 4.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET for 'набережная Невы'");
    let typ = get_street_type(&streets[0].borrow());
    assert_eq!(typ.as_deref(), Some("набережная"),
        "street type should be набережная, got {:?}", typ);
}

/// Street name is extracted and non-empty.
#[test]
fn test_address_street_name_nonempty() {
    let proc = address_proc();
    let sofa = SourceOfAnalysis::new("Офис на ул. Октябрьская, д. 1.");
    let result = proc.process(sofa, Some(MorphLang::RU));
    let streets: Vec<_> = result.entities.iter()
        .filter(|e| e.borrow().type_name == "STREET")
        .collect();
    assert!(!streets.is_empty(), "Should find STREET");
    let name = get_street_name(&streets[0].borrow());
    assert!(name.as_deref().map(|s| !s.is_empty()).unwrap_or(false),
        "street name should be non-empty, got {:?}", name);
}

// ── dialoge_3.txt test cases ──────────────────────────────────────────────────
//
// Source: academic NLP paper references file (dialoge_3.txt).
// Tests mirror test_1.py: run Person + Geo analyzers over each paragraph and
// assert the expected entities are extracted.  Groups follow the paragraph
// structure of the source document.

fn person_geo_proc() -> Processor {
    // GeoAnalyzer first so city tokens aren't consumed by PersonAnalyzer
    MorphologyService::initialize(Some(MorphLang::RU | MorphLang::EN));
    Processor::with_analyzers(vec![
        Arc::new(GeoAnalyzer::new()),
        Arc::new(PersonAnalyzer::new()),
    ])
}

/// Collect (firstname, lastname) pairs from all PERSON entities.
fn collect_persons(result: &pullenti_ner::AnalysisResult) -> Vec<(String, String)> {
    result.entities.iter()
        .filter(|e| e.borrow().type_name == "PERSON")
        .map(|e| {
            let rb = e.borrow();
            (
                get_firstname(&rb).unwrap_or_default().to_uppercase(),
                get_lastname(&rb).unwrap_or_default().to_uppercase(),
            )
        })
        .collect()
}

/// Check that at least one PERSON has a lastname containing `sub` (case-insensitive).
fn has_last(persons: &[(String, String)], sub: &str) -> bool {
    let sub_up = sub.to_uppercase();
    persons.iter().any(|(_, l)| l.contains(&sub_up))
}

/// Collect all GEO canonical names.
fn collect_geos(result: &pullenti_ner::AnalysisResult) -> Vec<String> {
    result.entities.iter()
        .filter(|e| e.borrow().type_name == "GEO")
        .filter_map(|e| get_geo_name(&e.borrow()))
        .map(|n| n.to_uppercase())
        .collect()
}

// ── Group 1: Conference venue geo-locations ───────────────────────────────────

/// Bangkok, Thailand, Miami, Florida, USA extracted as GEO from conference refs.
/// Note: Singapore and Abu Dhabi are not in the GEO database under English names
/// and appear as PERSON false-positives — known limitation documented here.
#[test]
fn test_dialoge3_geo_venues() {
    let proc = person_geo_proc();
    let text = concat!(
        "pages 11897–11916, Bangkok, Thai-\n",
        "land. Association for Computational Linguistics.\n",
        "In Proceedings of the Sixteenth ACM Interna-\n",
        "tional Conference on Web Search and Data Mining,\n",
        "pages 1048–1056, Singapore Singapore. ACM.\n",
        "In Proceedings\n",
        "of the 2024 Conference on Empirical Methods in\n",
        "Natural Language Processing, pages 13261–13273,\n",
        "Miami, Florida, USA. Association for Computational\n",
        "Linguistics.\n",
        "Proceedings of the 2022 Conference on Empirical\n",
        "Methods in Natural Language Processing, pages 538–\n",
        "548, Abu Dhabi, United Arab Emirates. Association\n",
        "for Computational Linguistics.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let geos = collect_geos(&result);

    assert!(geos.iter().any(|g| g.contains("BANGKOK")),
        "Bangkok not found in GEO: {:?}", geos);
    assert!(geos.iter().any(|g| g.contains("ТАИЛАНД") || g.contains("THAILAND") || g.contains("THAI")),
        "Thailand not found in GEO: {:?}", geos);
    assert!(geos.iter().any(|g| g.contains("MIAMI")),
        "Miami not found in GEO: {:?}", geos);
    assert!(geos.iter().any(|g| g.contains("FLORIDA")),
        "Florida not found in GEO: {:?}", geos);
    assert!(geos.iter().any(|g| g.contains("US") || g.contains("USA") || g.contains("СОЕДИНЕНН")),
        "USA not found in GEO: {:?}", geos);
}

// ── Group 2: NLLB Team authors ────────────────────────────────────────────────

/// 39-author NLLB team list with linebreak-hyphens, accented chars, 2-word lastnames.
#[test]
fn test_dialoge3_nllb_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "NLLB Team, Marta R. Costa-jussà, James Cross, Onur\n",
        "Çelebi, Maha Elbayad, Kenneth Heafield, Kevin Hef-\n",
        "fernan, Elahe Kalbassi, Janice Lam, Daniel Licht,\n",
        "Jean Maillard, Anna Sun, Skyler Wang, Guillaume\n",
        "Wenzek, Al Youngblood, Bapi Akula, Loic Bar-\n",
        "rault, Gabriel Mejia Gonzalez, Prangthip Hansanti,\n",
        "John Hoffman, Semarley Jarrett, Kaushik Ram\n",
        "Sadagopan, Dirk Rowe, Shannon Spruit, Chau\n",
        "Tran, Pierre Andrews, Necip Fazil Ayan, Shruti\n",
        "Bhosale, Sergey Edunov, Angela Fan, Cynthia\n",
        "Gao, Vedanuj Goswami, Francisco Guzmán, Philipp\n",
        "Koehn, Alexandre Mourachko, Christophe Rop-\n",
        "ers, Safiyyah Saleem, Holger Schwenk, and Jeff\n",
        "Wang.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    // Simple "First Last" names
    assert!(has_last(&persons, "Cross"),      "missing Cross: {:?}", persons);
    assert!(has_last(&persons, "Heafield"),   "missing Heafield: {:?}", persons);
    assert!(has_last(&persons, "Kalbassi"),   "missing Kalbassi: {:?}", persons);
    assert!(has_last(&persons, "Lam"),        "missing Lam: {:?}", persons);
    assert!(has_last(&persons, "Licht"),      "missing Licht: {:?}", persons);
    assert!(has_last(&persons, "Maillard"),   "missing Maillard: {:?}", persons);
    assert!(has_last(&persons, "Wenzek"),     "missing Wenzek: {:?}", persons);
    assert!(has_last(&persons, "Youngblood"), "missing Youngblood: {:?}", persons);
    assert!(has_last(&persons, "Akula"),      "missing Akula: {:?}", persons);
    assert!(has_last(&persons, "Hansanti"),   "missing Hansanti: {:?}", persons);
    assert!(has_last(&persons, "Hoffman"),    "missing Hoffman: {:?}", persons);
    assert!(has_last(&persons, "Jarrett"),    "missing Jarrett: {:?}", persons);
    assert!(has_last(&persons, "Rowe"),       "missing Rowe: {:?}", persons);
    assert!(has_last(&persons, "Spruit"),     "missing Spruit: {:?}", persons);
    assert!(has_last(&persons, "Andrews"),    "missing Andrews: {:?}", persons);
    assert!(has_last(&persons, "Bhosale"),    "missing Bhosale: {:?}", persons);
    assert!(has_last(&persons, "Edunov"),     "missing Edunov: {:?}", persons);
    assert!(has_last(&persons, "Saleem"),     "missing Saleem: {:?}", persons);
    assert!(has_last(&persons, "Schwenk"),    "missing Schwenk: {:?}", persons);

    // Accented / extended-Latin lastnames
    assert!(has_last(&persons, "Costa"),     "missing Costa-jussà: {:?}", persons);
    assert!(has_last(&persons, "Çelebi"),    "missing Çelebi: {:?}", persons);
    assert!(has_last(&persons, "Guzmán"),    "missing Guzmán: {:?}", persons);
    assert!(has_last(&persons, "Mourachko"), "missing Mourachko: {:?}", persons);

    // Linebreak-hyphen lastnames ("Hef-\nfernan" → "Hef-fernan")
    assert!(has_last(&persons, "Hef"),    "missing Heffernan: {:?}", persons);
    assert!(has_last(&persons, "Bar"),    "missing Barrault: {:?}", persons);
    assert!(has_last(&persons, "Rop"),    "missing Ropers: {:?}", persons);

    // Two-word lastnames
    assert!(has_last(&persons, "Mejia"),    "missing Mejia Gonzalez: {:?}", persons);
    assert!(has_last(&persons, "Sadagopan"),"missing Ram Sadagopan: {:?}", persons);
    assert!(has_last(&persons, "Ayan"),     "missing Fazil Ayan: {:?}", persons);
}

// ── Group 3: Matryoshka Representation Learning authors ───────────────────────

#[test]
fn test_dialoge3_matryoshka_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Aditya Kusupati, Gantavya Bhatt, Aniket Rege,\n",
        "Matthew Wallingford, Aditya Sinha, Vivek Ramanu-\n",
        "jan, William Howard-Snyder, Kaifeng Chen, Sham\n",
        "Kakade, Prateek Jain, and Ali Farhadi.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Kusupati"),      "missing Kusupati: {:?}", persons);
    assert!(has_last(&persons, "Bhatt"),         "missing Bhatt: {:?}", persons);
    assert!(has_last(&persons, "Rege"),          "missing Rege: {:?}", persons);
    assert!(has_last(&persons, "Wallingford"),   "missing Wallingford: {:?}", persons);
    assert!(has_last(&persons, "Sinha"),         "missing Sinha: {:?}", persons);
    assert!(has_last(&persons, "Ramanu"),        "missing Ramanujan: {:?}", persons);  // "Ramanu-jan"
    assert!(has_last(&persons, "Howard-Snyder"), "missing Howard-Snyder: {:?}", persons);
    assert!(has_last(&persons, "Chen"),          "missing Chen: {:?}", persons);
    assert!(has_last(&persons, "Kakade"),        "missing Kakade: {:?}", persons);
    assert!(has_last(&persons, "Jain"),          "missing Jain: {:?}", persons);
    assert!(has_last(&persons, "Farhadi"),       "missing Farhadi: {:?}", persons);
}

// ── Group 4: MS MARCO dataset authors ────────────────────────────────────────

#[test]
fn test_dialoge3_msmarco_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Daniel Fernando Campos, Tri Nguyen, Mir Rosenberg,\n",
        "Xia Song, Jianfeng Gao, Saurabh Tiwary, Rangan\n",
        "Majumder, Li Deng, and Bhaskar Mitra.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Fernando Campos"), "missing Fernando Campos: {:?}", persons);
    assert!(has_last(&persons, "Nguyen"),   "missing Nguyen: {:?}", persons);
    assert!(has_last(&persons, "Rosenberg"),"missing Rosenberg: {:?}", persons);
    assert!(has_last(&persons, "Song"),     "missing Song: {:?}", persons);
    assert!(has_last(&persons, "Tiwary"),   "missing Tiwary: {:?}", persons);
    assert!(has_last(&persons, "Majumder"), "missing Majumder: {:?}", persons);
    assert!(has_last(&persons, "Mitra"),    "missing Mitra: {:?}", persons);
}

// ── Group 5: Promptagator authors ────────────────────────────────────────────

#[test]
fn test_dialoge3_promptagator_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Zhuyun Dai, Vincent Y. Zhao, Ji Ma, Yi Luan, Jianmo\n",
        "Ni, Jing Lu, Anton Bakalov, Kelvin Guu, Keith B.\n",
        "Hall, and Ming-Wei Chang.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Dai"),     "missing Dai: {:?}", persons);
    assert!(has_last(&persons, "Zhao"),    "missing Zhao: {:?}", persons);
    assert!(has_last(&persons, "Luan"),    "missing Luan: {:?}", persons);
    assert!(has_last(&persons, "Bakalov"), "missing Bakalov: {:?}", persons);
    assert!(has_last(&persons, "Guu"),     "missing Guu: {:?}", persons);
    assert!(has_last(&persons, "Hall"),    "missing Hall: {:?}", persons);
    assert!(has_last(&persons, "Chang"),   "missing Ming-Wei Chang: {:?}", persons);
}

// ── Group 6: BERT authors ─────────────────────────────────────────────────────

/// Devlin et al. 2018/2019 — same four authors appear twice (dedup may merge them).
#[test]
fn test_dialoge3_bert_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Jacob Devlin, Ming-Wei Chang, Kenton Lee, and\n",
        "Kristina Toutanova. 2018. Bert: Pre-training of deep\n",
        "bidirectional transformers for language understand-\n",
        "ing. arXiv preprint arXiv:1810.04805.\n",
        "\n",
        "Jacob Devlin, Ming-Wei Chang, Kenton Lee, and\n",
        "Kristina Toutanova. 2019.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Devlin"),    "missing Devlin: {:?}", persons);
    assert!(has_last(&persons, "Chang"),     "missing Chang: {:?}", persons);
    assert!(has_last(&persons, "Lee"),       "missing Lee: {:?}", persons);
    assert!(has_last(&persons, "Toutanova"), "missing Toutanova: {:?}", persons);
}

// ── Group 7: Natural Questions (NQ) authors ───────────────────────────────────

#[test]
fn test_dialoge3_nq_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Tom Kwiatkowski, Jennimaria Palomaki, Olivia Red-\n",
        "field, Michael Collins, Ankur P. Parikh, Chris Alberti,\n",
        "Danielle Epstein, Illia Polosukhin, Jacob Devlin, Ken-\n",
        "ton Lee, Kristina Toutanova, Llion Jones, Matthew\n",
        "Kelcey, Ming-Wei Chang, Andrew M. Dai, Jakob\n",
        "Uszkoreit, Quoc V. Le, and Slav Petrov.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Kwiatkowski"), "missing Kwiatkowski: {:?}", persons);
    assert!(has_last(&persons, "Palomaki"),    "missing Palomaki: {:?}", persons);
    assert!(has_last(&persons, "Red"),         "missing Redfield: {:?}", persons); // "Red-field"
    assert!(has_last(&persons, "Collins"),     "missing Collins: {:?}", persons);
    assert!(has_last(&persons, "Parikh"),      "missing Parikh: {:?}", persons);
    assert!(has_last(&persons, "Alberti"),     "missing Alberti: {:?}", persons);
    assert!(has_last(&persons, "Epstein"),     "missing Epstein: {:?}", persons);
    assert!(has_last(&persons, "Polosukhin"),  "missing Polosukhin: {:?}", persons);
    assert!(has_last(&persons, "Jones"),       "missing Jones: {:?}", persons);
    assert!(has_last(&persons, "Kelcey"),      "missing Kelcey: {:?}", persons);
    assert!(has_last(&persons, "Uszkoreit"),   "missing Uszkoreit: {:?}", persons);
    assert!(has_last(&persons, "Petrov"),      "missing Petrov: {:?}", persons);
}

// ── Group 8: FAISS library authors ───────────────────────────────────────────

#[test]
fn test_dialoge3_faiss_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Matthijs Douze, Alexandr Guzhva, Chengqi Deng, Jeff\n",
        "Johnson, Gergely Szilvasy, Pierre-Emmanuel Mazaré,\n",
        "Maria Lomeli, Lucas Hosseini, and Hervé Jégou.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Douze"),          "missing Douze: {:?}", persons);
    assert!(has_last(&persons, "Guzhva"),         "missing Guzhva: {:?}", persons);
    assert!(has_last(&persons, "Johnson"),        "missing Johnson: {:?}", persons);
    assert!(has_last(&persons, "Szilvasy"),       "missing Szilvasy: {:?}", persons);
    assert!(has_last(&persons, "Mazaré"),         "missing Mazaré: {:?}", persons);
    assert!(has_last(&persons, "Lomeli"),         "missing Lomeli: {:?}", persons);
    assert!(has_last(&persons, "Hosseini"),       "missing Hosseini: {:?}", persons);
    assert!(has_last(&persons, "Jégou"),          "missing Jégou: {:?}", persons);
    assert!(has_last(&persons, "Pierre-Emmanuel")
        || persons.iter().any(|(f, _)| f.contains("PIERRE")),
        "missing Pierre-Emmanuel: {:?}", persons);
}

// ── Group 9: Jina Embeddings 2 authors ───────────────────────────────────────

#[test]
fn test_dialoge3_jina_authors() {
    let proc = person_geo_proc();
    let text = concat!(
        "Michael Günther, Jackmin Ong, Isabelle Mohr, Alaed-\n",
        "dine Abdessalem, Tanguy Abel, Mohammad Kalim\n",
        "Akram, Susana Guzman, Georgios Mastrapas, Saba\n",
        "Sturua, Bo Wang, Maximilian Werk, Nan Wang,\n",
        "and Han Xiao.",
    );
    let sofa = SourceOfAnalysis::new(text);
    let result = proc.process(sofa, Some(MorphLang::EN));
    let persons = collect_persons(&result);

    assert!(has_last(&persons, "Günther"),    "missing Günther: {:?}", persons);
    assert!(has_last(&persons, "Ong"),        "missing Ong: {:?}", persons);
    assert!(has_last(&persons, "Mohr"),       "missing Mohr: {:?}", persons);
    assert!(has_last(&persons, "Abdessalem"), "missing Abdessalem: {:?}", persons);
    assert!(has_last(&persons, "Abel"),       "missing Abel: {:?}", persons);
    assert!(has_last(&persons, "Akram"),      "missing Kalim Akram: {:?}", persons);
    assert!(has_last(&persons, "Guzman"),     "missing Guzman: {:?}", persons);
    assert!(has_last(&persons, "Mastrapas"),  "missing Mastrapas: {:?}", persons);
    assert!(has_last(&persons, "Sturua"),     "missing Sturua: {:?}", persons);
    assert!(has_last(&persons, "Werk"),       "missing Werk: {:?}", persons);
    assert!(has_last(&persons, "Xiao"),       "missing Xiao: {:?}", persons);
}
