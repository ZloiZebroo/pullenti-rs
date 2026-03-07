/// Static lookup tables for Transport analyzer.
///
/// Vehicle type keywords and brand names, mirroring data from
/// `TransItemToken.Initialize()` in C# source.

use std::collections::HashMap;
use std::sync::OnceLock;
use super::transport_referent::TransportKind;

pub struct TypeEntry {
    pub canonical: &'static str,
    pub kind: TransportKind,
}

pub struct BrandEntry {
    pub canonical: &'static str,
    pub kind: TransportKind,
}

struct Tables {
    types: HashMap<String, TypeEntry>,
    brands: HashMap<String, BrandEntry>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

fn get_tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let mut types: HashMap<String, TypeEntry> = HashMap::new();
        let mut brands: HashMap<String, BrandEntry> = HashMap::new();

        // ── Auto ──────────────────────────────────────────────────────────────
        for key in &[
            "АВТОМОБИЛЬ", "АВТОМАШИНА", "ТРАНСПОРТНОЕ СРЕДСТВО",
            "ВНЕДОРОЖНИК", "АВТОБУС", "МИКРОАВТОБУС", "ГРУЗОВИК",
            "МОТОЦИКЛ", "МОПЕД", "ЛЕГКОВОЙ АВТОМОБИЛЬ", "ГРУЗОВОЙ АВТОМОБИЛЬ",
            "ТС", // аббревиатура транспортного средства
        ] {
            types.insert(key.to_string(), TypeEntry { canonical: "автомобиль", kind: TransportKind::Auto });
        }
        // override canonical for specific subtypes
        types.insert("АВТОБУС".to_string(),      TypeEntry { canonical: "автобус",      kind: TransportKind::Auto });
        types.insert("МИКРОАВТОБУС".to_string(), TypeEntry { canonical: "микроавтобус", kind: TransportKind::Auto });
        types.insert("ГРУЗОВИК".to_string(),     TypeEntry { canonical: "грузовик",     kind: TransportKind::Auto });
        types.insert("МОТОЦИКЛ".to_string(),     TypeEntry { canonical: "мотоцикл",     kind: TransportKind::Auto });
        types.insert("МОПЕД".to_string(),        TypeEntry { canonical: "мопед",        kind: TransportKind::Auto });
        types.insert("ВНЕДОРОЖНИК".to_string(),  TypeEntry { canonical: "внедорожник",  kind: TransportKind::Auto });
        // keep generic for these
        types.insert("АВТОМОБИЛЬ".to_string(),         TypeEntry { canonical: "автомобиль",         kind: TransportKind::Auto });
        types.insert("АВТОМАШИНА".to_string(),         TypeEntry { canonical: "автомобиль",         kind: TransportKind::Auto });
        types.insert("ТРАНСПОРТНОЕ СРЕДСТВО".to_string(), TypeEntry { canonical: "автомобиль",      kind: TransportKind::Auto });

        // ── Train ─────────────────────────────────────────────────────────────
        for (key, canon) in &[
            ("ПОЕЗД", "поезд"), ("ЭЛЕКТРИЧКА", "электричка"),
            ("ЛОКОМОТИВ", "локомотив"), ("ПАРОВОЗ", "паровоз"),
            ("ВАГОН", "вагон"), ("ТРАМВАЙ", "трамвай"),
            ("МЕТРО", "метро"), ("СКОРОСТНОЙ ПОЕЗД", "поезд"),
        ] {
            types.insert(key.to_string(), TypeEntry { canonical: canon, kind: TransportKind::Train });
        }

        // ── Ship ──────────────────────────────────────────────────────────────
        for (key, canon) in &[
            ("ТЕПЛОХОД", "теплоход"), ("ПАРОХОД", "пароход"),
            ("ЯХТА", "яхта"), ("ЛОДКА", "лодка"),
            ("КАТЕР", "катер"), ("КОРАБЛЬ", "корабль"),
            ("СУДНО", "судно"), ("ПОДВОДНАЯ ЛОДКА", "подводная лодка"),
            ("ШХУНА", "шхуна"), ("ПАРОМ", "паром"),
            ("КРЕЙСЕР", "крейсер"), ("АВИАНОСЕЦ", "авианосец"),
            ("ЭСМИНЕЦ", "эсминец"), ("ФРЕГАТ", "фрегат"),
            ("ЛИНКОР", "линкор"), ("ЛЕДОКОЛ", "ледокол"),
            ("ТАНКЕР", "танкер"), ("ТРАУЛЕР", "траулер"),
            ("КРУИЗНЫЙ ЛАЙНЕР", "лайнер"), ("ЛАЙНЕР", "лайнер"),
            ("АТОМОХОД", "атомоход"), ("ПЛАВБАЗА", "плавбаза"),
            ("РЕФРИЖЕРАТОР", "рефрижератор"), ("СУХОГРУЗ", "сухогруз"),
        ] {
            types.insert(key.to_string(), TypeEntry { canonical: canon, kind: TransportKind::Ship });
        }

        // ── Fly ───────────────────────────────────────────────────────────────
        for (key, canon) in &[
            ("САМОЛЕТ", "самолет"), ("САМОЛЁТ", "самолет"),
            ("АВИАЛАЙНЕР", "авиалайнер"), ("ИСТРЕБИТЕЛЬ", "истребитель"),
            ("БОМБАРДИРОВЩИК", "бомбардировщик"), ("ВЕРТОЛЕТ", "вертолет"),
            ("ВЕРТОЛЁТ", "вертолет"), ("ВОЗДУШНОЕ СУДНО", "самолет"),
        ] {
            types.insert(key.to_string(), TypeEntry { canonical: canon, kind: TransportKind::Fly });
        }

        // ── Space ─────────────────────────────────────────────────────────────
        for (key, canon) in &[
            ("КОСМИЧЕСКИЙ КОРАБЛЬ", "космический корабль"),
            ("ЗВЕЗДОЛЕТ", "звездолет"), ("ЗВЕЗДОЛЁТ", "звездолет"),
            ("КОСМИЧЕСКАЯ СТАНЦИЯ", "космическая станция"),
            ("РАКЕТА-НОСИТЕЛЬ", "ракета-носитель"),
            ("РАКЕТА", "ракета"),
            ("ШАТТЛ", "шаттл"),
        ] {
            types.insert(key.to_string(), TypeEntry { canonical: canon, kind: TransportKind::Space });
        }

        // ── Car brands ────────────────────────────────────────────────────────
        let car_brands: &[(&str, &str)] = &[
            ("AUDI", "Audi"), ("АУДИ", "Audi"),
            ("BMW", "BMW"), ("БМВ", "BMW"),
            ("FORD", "Ford"), ("ФОРД", "Ford"),
            ("TOYOTA", "Toyota"), ("ТОЙОТА", "Toyota"),
            ("HONDA", "Honda"), ("ХОНДА", "Honda"),
            ("VOLKSWAGEN", "Volkswagen"), ("ФОЛЬКСВАГЕН", "Volkswagen"),
            ("MERCEDES", "Mercedes"), ("МЕРСЕДЕС", "Mercedes"),
            ("MERCEDES-BENZ", "Mercedes-Benz"),
            ("NISSAN", "Nissan"), ("НИССАН", "Nissan"),
            ("HYUNDAI", "Hyundai"), ("ХЮНДАЙ", "Hyundai"), ("ХУНДАЙ", "Hyundai"),
            ("KIA", "Kia"), ("КИА", "Kia"),
            ("OPEL", "Opel"), ("ОПЕЛЬ", "Opel"),
            ("RENAULT", "Renault"), ("РЕНО", "Renault"),
            ("PEUGEOT", "Peugeot"), ("ПЕЖО", "Peugeot"),
            ("SKODA", "Skoda"), ("ШКОДА", "Skoda"),
            ("VOLVO", "Volvo"), ("ВОЛЬВО", "Volvo"),
            ("CHEVROLET", "Chevrolet"), ("ШЕВРОЛЕ", "Chevrolet"),
            ("SUBARU", "Subaru"), ("СУБАРУ", "Subaru"),
            ("MAZDA", "Mazda"), ("МАЗДА", "Mazda"),
            ("MITSUBISHI", "Mitsubishi"), ("МИЦУБИШИ", "Mitsubishi"),
            ("LEXUS", "Lexus"), ("ЛЕКСУС", "Lexus"),
            ("INFINITI", "Infiniti"),
            ("LAND ROVER", "Land Rover"), ("ЛЕНДРОВЕР", "Land Rover"),
            ("JEEP", "Jeep"), ("ДЖИП", "Jeep"),
            ("CHRYSLER", "Chrysler"), ("КРАЙСЛЕР", "Chrysler"),
            ("CADILLAC", "Cadillac"), ("КАДИЛЛАК", "Cadillac"),
            ("PORSCHE", "Porsche"), ("ПОРШЕ", "Porsche"),
            ("FERRARI", "Ferrari"), ("ФЕРРАРИ", "Ferrari"),
            ("LAMBORGHINI", "Lamborghini"), ("ЛАМБОРДЖИНИ", "Lamborghini"),
            ("ROLLS-ROYCE", "Rolls-Royce"), ("РОЛЛС-РОЙС", "Rolls-Royce"),
            ("BENTLEY", "Bentley"), ("БЕНТЛИ", "Bentley"),
            ("TESLA", "Tesla"),
            ("CITROEN", "Citroen"), ("СИТРОЕН", "Citroen"),
            ("DAEWOO", "Daewoo"), ("ДЭО", "Daewoo"),
            ("FIAT", "Fiat"), ("ФИАТ", "Fiat"),
            ("DODGE", "Dodge"), ("ДОДЖ", "Dodge"),
            ("HUMMER", "Hummer"), ("ХАММЕР", "Hummer"),
            ("ISUZU", "Isuzu"), ("ИСУЗУ", "Isuzu"),
            ("SUZUKI", "Suzuki"), ("СУДЗУКИ", "Suzuki"),
            ("DAIHATSU", "Daihatsu"),
            ("MASERATI", "Maserati"),
            ("BUICK", "Buick"),
            ("SAAB", "Saab"), ("СААБ", "Saab"),
            ("ALFA ROMEO", "Alfa Romeo"), ("АЛЬФА РОМЕО", "Alfa Romeo"),
            ("LANCIA", "Lancia"),
            ("SEAT", "Seat"),
            // Russian/Soviet brands
            ("ВАЗ", "ВАЗ"), ("VAZ", "ВАЗ"),
            ("ГАЗ", "ГАЗ"), ("GAZ", "ГАЗ"),
            ("ЗИЛ", "ЗИЛ"), ("ZIL", "ЗИЛ"),
            ("УАЗ", "УАЗ"), ("UAZ", "УАЗ"),
            ("АЗЛК", "АЗЛК"),
            ("МОСКВИЧ", "Москвич"),
            ("ЛАДА", "Лада"), ("ЖИГУЛИ", "Жигули"),
            ("ТАГАЗ", "ТагАЗ"),
            ("NIVA", "Нива"), ("НИВА", "Нива"),
            ("YAMAHA", "Yamaha"), ("ЯМАХА", "Yamaha"),
            ("HARLEY", "Harley"),
        ];
        for (key, canon) in car_brands {
            brands.entry(key.to_string()).or_insert(BrandEntry { canonical: canon, kind: TransportKind::Auto });
        }

        // ── Aircraft brands ───────────────────────────────────────────────────
        let fly_brands: &[(&str, &str)] = &[
            ("BOEING", "Boeing"), ("БОИНГ", "Boeing"),
            ("AIRBUS", "Airbus"), ("АЭРОБУС", "Airbus"),
            ("ИЛ", "Ил"), ("ИЛЮШИН", "Ил"),  // Ilyushin
            ("ТУ", "Ту"), ("ТУПОЛЕВ", "Ту"),  // Tupolev
            ("АН", "Ан"), ("АНТОНОВ", "Ан"),  // Antonov
            ("СУ", "Су"), ("СУХОЙ", "Су"),    // Sukhoi
            ("ЯК", "Як"), ("ЯКОВЛЕВ", "Як"),  // Yakovlev
            ("МИ", "Ми"),                       // Mil helicopters
            ("EMBRAER", "Embraer"), ("ЭМБРАЕР", "Embraer"),
            ("BOMBARDIER", "Bombardier"), ("БОМБАРДЬЕ", "Bombardier"),
            ("CESSNA", "Cessna"), ("ЦЕССНА", "Cessna"),
            ("SAAB", "Saab"),
            ("ATR", "ATR"),
        ];
        for (key, canon) in fly_brands {
            brands.entry(key.to_string()).or_insert(BrandEntry { canonical: canon, kind: TransportKind::Fly });
        }

        Tables { types, brands }
    })
}

/// Look up a vehicle type keyword (uppercase).
pub fn lookup_type(key: &str) -> Option<&'static TypeEntry> {
    get_tables().types.get(key).map(|e| {
        // SAFETY: `Tables` is stored in `OnceLock` which lives for `'static`
        unsafe { &*(e as *const TypeEntry) }
    })
}

/// Look up a vehicle brand (uppercase).
pub fn lookup_brand(key: &str) -> Option<&'static BrandEntry> {
    get_tables().brands.get(key).map(|e| {
        unsafe { &*(e as *const BrandEntry) }
    })
}
