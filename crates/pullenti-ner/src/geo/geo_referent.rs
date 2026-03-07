/// GeoReferent — territorial entity (country, region, city).
/// Mirrors Pullenti C# `GeoReferent`.

use crate::referent::{Referent, Slot, SlotValue};

pub const OBJ_TYPENAME: &str = "GEO";

pub const ATTR_NAME:     &str = "NAME";
pub const ATTR_TYPE:     &str = "TYPE";
pub const ATTR_ALPHA2:   &str = "ALPHA2";
pub const ATTR_HIGHER:   &str = "HIGHER";
pub const ATTR_MISC:     &str = "MISC";
pub const ATTR_PROBABLE: &str = "PROBABLE";
pub const ATTR_REF:      &str = "REF";

// ── Constructor ───────────────────────────────────────────────────────────────

pub fn new_geo_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_str(r: &Referent, attr: &str) -> Option<String> {
    r.slots.iter()
        .find(|s| s.type_name == attr)
        .and_then(|s| s.value.as_ref())
        .and_then(|v| match v { SlotValue::Str(s) => Some(s.clone()), _ => None })
}

fn set_str(r: &mut Referent, attr: &str, val: &str) {
    r.slots.retain(|s| s.type_name != attr);
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(val.to_string()))));
}

fn add_str(r: &mut Referent, attr: &str, val: &str) {
    // avoid duplicates
    let upper = val.to_string();
    if r.slots.iter().any(|s| s.type_name == attr && s.value.as_ref().map(|v| matches!(v, SlotValue::Str(x) if x == &upper)).unwrap_or(false)) {
        return;
    }
    r.slots.push(Slot::new(attr, Some(SlotValue::Str(upper))));
}

// ── Mutators ──────────────────────────────────────────────────────────────────

/// Add a NAME slot (stored uppercase; deduplicates).
pub fn add_name(r: &mut Referent, name: &str) {
    add_str(r, ATTR_NAME, &name.to_uppercase());
}

/// Add a TYPE slot (stored lowercase; deduplicates).
pub fn add_type(r: &mut Referent, typ: &str) {
    add_str(r, ATTR_TYPE, &typ.to_lowercase());
}

/// Set ISO 3166-1 alpha-2 code.
pub fn set_alpha2(r: &mut Referent, code: &str) {
    set_str(r, ATTR_ALPHA2, &code.to_uppercase());
}

// ── Getters ───────────────────────────────────────────────────────────────────

/// First NAME slot value (uppercase).
pub fn get_name(r: &Referent) -> Option<String> { get_str(r, ATTR_NAME) }

/// All NAME slot values.
pub fn get_names(r: &Referent) -> Vec<String> {
    r.slots.iter()
        .filter(|s| s.type_name == ATTR_NAME)
        .filter_map(|s| if let Some(SlotValue::Str(v)) = &s.value { Some(v.clone()) } else { None })
        .collect()
}

/// First TYPE slot value (lowercase).
pub fn get_type(r: &Referent) -> Option<String> { get_str(r, ATTR_TYPE) }

/// ISO alpha-2 code.
pub fn get_alpha2(r: &Referent) -> Option<String> { get_str(r, ATTR_ALPHA2) }

// ── Predicates ────────────────────────────────────────────────────────────────

pub fn is_city(r: &Referent) -> bool {
    r.slots.iter()
        .filter(|s| s.type_name == ATTR_TYPE)
        .any(|s| if let Some(SlotValue::Str(v)) = &s.value { _is_city_type(v) } else { false })
}

pub fn is_state(r: &Referent) -> bool {
    if get_alpha2(r).is_some() { return true; }
    r.slots.iter()
        .filter(|s| s.type_name == ATTR_TYPE)
        .any(|s| if let Some(SlotValue::Str(v)) = &s.value { _is_state_type(v) } else { false })
}

pub fn is_region(r: &Referent) -> bool {
    r.slots.iter()
        .filter(|s| s.type_name == ATTR_TYPE)
        .any(|s| if let Some(SlotValue::Str(v)) = &s.value { _is_region_type(v) } else { false })
}

fn _is_city_type(v: &str) -> bool {
    matches!(v, "город" | "місто" | "city" | "town" | "municipality" | "locality" |
        "поселок" | "посёлок" | "село" | "деревня" | "станица" | "хутор" |
        "аул" | "станция" | "village" | "hamlet" | "settlement" |
        "населенный пункт" | "населений пункт")
}

fn _is_state_type(v: &str) -> bool {
    v.contains("государство") || v.contains("страна") || v.contains("держава") ||
    v.contains("country") || v.contains("империя") || v.contains("королевство") ||
    v.contains("княжество") || v == "союз"
}

fn _is_region_type(v: &str) -> bool {
    matches!(v, "область" | "район" | "край" | "округ" | "республика" | "штат" |
        "провинция" | "префектура" | "регион" | "графство" | "губерния" |
        "уезд" | "автономия" | "district" | "county" | "state" | "province" |
        "prefecture" | "region" | "autonomy" | "borough" | "parish") ||
    v.contains("автономн") || v.contains("федерал") || v.contains("округ")
}
