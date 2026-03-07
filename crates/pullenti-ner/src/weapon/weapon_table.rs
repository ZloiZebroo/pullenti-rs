/// Static weapon ontology — ports WeaponItemToken.Initialize() in C#.

use std::sync::{Arc, OnceLock};
use crate::core::termin::{Termin, TerminCollection};

// ── WeaponItemTyp ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponItemTyp {
    Noun,
    Brand,
    Model,
    Number,
    Name,
    Class,
    Date,
    Caliber,
    Developer,
}

// Make it storable in Arc<dyn Any + Send + Sync>
unsafe impl Send for WeaponItemTyp {}
unsafe impl Sync for WeaponItemTyp {}

// ── ModelInner ─────────────────────────────────────────────────────────────────

/// Inner tokens for a model abbreviation (e.g., ТТ → Noun=ПИСТОЛЕТ, Brand=ТОКАРЕВ)
pub struct ModelInner {
    /// List of (type, value, alt_value)
    pub items: Vec<(WeaponItemTyp, &'static str, Option<&'static str>)>,
}

unsafe impl Send for ModelInner {}
unsafe impl Sync for ModelInner {}

// ── Ontology ──────────────────────────────────────────────────────────────────

static ONTOLOGY: OnceLock<TerminCollection> = OnceLock::new();

fn typ(t: WeaponItemTyp) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
    Some(Arc::new(t))
}

fn doubt_flag() -> Option<Arc<dyn std::any::Any + Send + Sync>> {
    Some(Arc::new(true_flag()))
}
struct TrueFlag;
fn true_flag() -> TrueFlag { TrueFlag }
unsafe impl Send for TrueFlag {}
unsafe impl Sync for TrueFlag {}

fn model_inner(items: Vec<(WeaponItemTyp, &'static str, Option<&'static str>)>) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
    Some(Arc::new(ModelInner { items }))
}

pub fn get_ontology() -> &'static TerminCollection {
    ONTOLOGY.get_or_init(|| {
        let mut tc = TerminCollection::new();

        // ── Models (added FIRST so multi-word matches are tried before single-word nouns) ──

        // ТУЛЬСКИЙ ТОКАРЕВА → ТТ
        let mut t = Termin::new_canonic("ТУЛЬСКИЙ ТОКАРЕВА", "ТТ");
        t.tag = typ(WeaponItemTyp::Model);
        t.tag2 = model_inner(vec![
            (WeaponItemTyp::Noun,  "ПИСТОЛЕТ", None),
            (WeaponItemTyp::Brand, "ТОКАРЕВ",  None),
        ]);
        t.add_abridge("ТТ");
        tc.add(t);

        // ПИСТОЛЕТ МАКАРОВА МОДЕРНИЗИРОВАННЫЙ → ПММ (must be before ПМ!)
        let mut t = Termin::new_canonic("ПИСТОЛЕТ МАКАРОВА МОДЕРНИЗИРОВАННЫЙ", "ПММ");
        t.tag = typ(WeaponItemTyp::Model);
        t.tag2 = model_inner(vec![
            (WeaponItemTyp::Noun,  "ПИСТОЛЕТ", Some("МОДЕРНИЗИРОВАННЫЙ ПИСТОЛЕТ")),
            (WeaponItemTyp::Brand, "МАКАРОВ",  None),
        ]);
        t.add_abridge("ПММ");
        tc.add(t);

        // ПИСТОЛЕТ МАКАРОВА → ПМ
        let mut t = Termin::new_canonic("ПИСТОЛЕТ МАКАРОВА", "ПМ");
        t.tag = typ(WeaponItemTyp::Model);
        t.tag2 = model_inner(vec![
            (WeaponItemTyp::Noun,  "ПИСТОЛЕТ", None),
            (WeaponItemTyp::Brand, "МАКАРОВ",  None),
        ]);
        t.add_abridge("ПМ");
        tc.add(t);

        // АВТОМАТ КАЛАШНИКОВА → АК
        let mut t = Termin::new_canonic("АВТОМАТ КАЛАШНИКОВА", "АК");
        t.tag = typ(WeaponItemTyp::Model);
        t.tag2 = model_inner(vec![
            (WeaponItemTyp::Noun,  "АВТОМАТ",   None),
            (WeaponItemTyp::Brand, "КАЛАШНИКОВ", None),
        ]);
        t.add_abridge("АК");
        tc.add(t);

        // ── Nouns ──────────────────────────────────────────────────────────────

        macro_rules! noun {
            ($text:expr) => {{
                let mut t = Termin::new($text);
                t.tag = typ(WeaponItemTyp::Noun);
                tc.add(t);
            }};
            ($text:expr, doubt) => {{
                let mut t = Termin::new($text);
                t.tag = typ(WeaponItemTyp::Noun);
                t.tag2 = doubt_flag();
                tc.add(t);
            }};
            ($text:expr, variant: $v:expr) => {{
                let mut t = Termin::new($text);
                t.tag = typ(WeaponItemTyp::Noun);
                t.add_variant($v);
                tc.add(t);
            }};
            ($text:expr, abridge: $a:expr) => {{
                let mut t = Termin::new($text);
                t.tag = typ(WeaponItemTyp::Noun);
                t.add_abridge($a);
                tc.add(t);
            }};
        }

        noun!("ПИСТОЛЕТ");
        noun!("РЕВОЛЬВЕР");
        noun!("ВИНТОВКА");
        noun!("РУЖЬЕ");
        noun!("АВТОМАТ", doubt);
        noun!("КАРАБИН", doubt);
        noun!("ПИСТОЛЕТ-ПУЛЕМЕТ");
        noun!("ПУЛЕМЕТ");
        {
            let mut t = Termin::new("ГРАНАТОМЕТ");
            t.tag = typ(WeaponItemTyp::Noun);
            t.add_variant("СТРЕЛКОВО ГРАНАТОМЕТНЫЙ КОМПЛЕКС");
            tc.add(t);
        }
        noun!("ОГНЕМЕТ");
        noun!("МИНОМЕТ");
        {
            let mut t = Termin::new_canonic("ПЕРЕНОСНОЙ ЗЕНИТНО РАКЕТНЫЙ КОМПЛЕКС", "ПЕРЕНОСНОЙ ЗЕНИТНО РАКЕТНЫЙ КОМПЛЕКС");
            t.tag = typ(WeaponItemTyp::Noun);
            t.add_abridge("ПЗРК");
            tc.add(t);
        }
        {
            let mut t = Termin::new_canonic("ПРОТИВОТАНКОВЫЙ РАКЕТНЫЙ КОМПЛЕКС", "ПРОТИВОТАНКОВЫЙ РАКЕТНЫЙ КОМПЛЕКС");
            t.tag = typ(WeaponItemTyp::Noun);
            t.add_abridge("ПТРК");
            t.add_variant("ПЕРЕНОСНОЙ ПРОТИВОТАНКОВЫЙ РАКЕТНЫЙ КОМПЛЕКС");
            tc.add(t);
        }
        {
            let mut t = Termin::new("АВИАЦИОННАЯ ПУШКА");
            t.tag = typ(WeaponItemTyp::Noun);
            t.add_variant("АВИАПУШКА");
            tc.add(t);
        }
        noun!("НАРУЧНИКИ");
        noun!("БРОНЕЖИЛЕТ");
        noun!("ГРАНАТА");
        noun!("ЛИМОНКА");
        noun!("НОЖ");
        noun!("ВЗРЫВАТЕЛЬ");

        // ── Brands ─────────────────────────────────────────────────────────────

        for name in &[
            "МАКАРОВ", "КАЛАШНИКОВ", "СИМОНОВ", "СТЕЧКИН", "ШМАЙСЕР",
            "МОСИН", "СЛОСТИН", "НАГАН", "МАКСИМ", "ДРАГУНОВ",
            "СЕРДЮКОВ", "ЯРЫГИН", "НИКОНОВ", "МАУЗЕР", "БРАУНИНГ",
            "КОЛЬТ", "ВИНЧЕСТЕР",
        ] {
            let mut t = Termin::new(*name);
            t.tag = typ(WeaponItemTyp::Brand);
            tc.add(t);
        }

        // ── Names ───────────────────────────────────────────────────────────────

        for name in &["УЗИ"] {
            let mut t = Termin::new(*name);
            t.tag = typ(WeaponItemTyp::Name);
            tc.add(t);
        }

        tc
    })
}

/// Return the WeaponItemTyp tag from a termin, if present.
pub fn get_typ(termin: &std::sync::Arc<crate::core::termin::Termin>) -> Option<WeaponItemTyp> {
    termin.tag.as_ref()
        .and_then(|a| a.downcast_ref::<WeaponItemTyp>())
        .copied()
}

/// Return true if the noun termin is a doubt-noun (АВТОМАТ, КАРАБИН).
pub fn is_noun_doubt(termin: &std::sync::Arc<crate::core::termin::Termin>) -> bool {
    termin.tag2.as_ref()
        .and_then(|a| a.downcast_ref::<TrueFlag>())
        .is_some()
}

/// Return the inner items for a model termin.
pub fn get_model_inner(termin: &std::sync::Arc<crate::core::termin::Termin>) -> Option<&ModelInner> {
    termin.tag2.as_ref()
        .and_then(|a| a.downcast_ref::<ModelInner>())
}
