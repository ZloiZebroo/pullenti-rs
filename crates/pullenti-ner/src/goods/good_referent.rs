/// GoodReferent and GoodAttributeReferent — product/goods entities and their attributes.
/// Mirrors `GoodReferent.cs`, `GoodAttributeReferent.cs`, `GoodAttrType.cs`.

use crate::referent::{Referent, SlotValue};

// ── GoodReferent constants ─────────────────────────────────────────────────

/// TypeName for the "good" (product) entity: "GOOD"
pub const OBJ_TYPENAME: &str = "GOOD";

/// Attribute name for the product's attributes list (holds GoodAttributeReferent)
pub const ATTR_ATTR: &str = "ATTR";

// ── GoodAttributeReferent constants ───────────────────────────────────────

/// TypeName for a single product attribute entity: "GOODATTR"
pub const GOODATTR_OBJ_TYPENAME: &str = "GOODATTR";

/// Attribute name for the type of attribute (GoodAttrType as string)
pub const ATTR_TYPE: &str = "TYPE";

/// Attribute name for the value of the attribute
pub const ATTR_VALUE: &str = "VALUE";

/// Attribute name for alternate value
pub const ATTR_ALTVALUE: &str = "ALTVALUE";

/// Attribute name for unit of measurement
pub const ATTR_UNIT: &str = "UNIT";

/// Attribute name for the human-readable name of this attribute
pub const ATTR_NAME: &str = "NAME";

/// Attribute name for reference to an external entity
pub const ATTR_REF: &str = "REF";

// ── GoodAttrType ───────────────────────────────────────────────────────────

/// Type classification of a product attribute (mirrors `GoodAttrType` C# enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoodAttrType {
    /// Not set / undefined
    #[default]
    Undefined,
    /// Keyword — the type/category of the good (e.g. "молоко")
    Keyword,
    /// Qualitative property / characteristic (e.g. "пастеризованное")
    Character,
    /// Proper name / brand (e.g. "Простоквашино")
    Proper,
    /// Model / article number (e.g. "АК-47", "12345-ФЗ")
    Model,
    /// Quantitative / numeric attribute (e.g. "3.5%", "1 кг")
    Numeric,
    /// Reference to another entity (organization, geo, decree, etc.)
    Referent,
}

impl GoodAttrType {
    pub fn as_str(self) -> &'static str {
        match self {
            GoodAttrType::Undefined  => "UNDEFINED",
            GoodAttrType::Keyword    => "KEYWORD",
            GoodAttrType::Character  => "CHARACTER",
            GoodAttrType::Proper     => "PROPER",
            GoodAttrType::Model      => "MODEL",
            GoodAttrType::Numeric    => "NUMERIC",
            GoodAttrType::Referent   => "REFERENT",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "KEYWORD"   => GoodAttrType::Keyword,
            "CHARACTER" => GoodAttrType::Character,
            "PROPER"    => GoodAttrType::Proper,
            "MODEL"     => GoodAttrType::Model,
            "NUMERIC"   => GoodAttrType::Numeric,
            "REFERENT"  => GoodAttrType::Referent,
            _           => GoodAttrType::Undefined,
        }
    }
}

// ── Constructors ───────────────────────────────────────────────────────────

/// Create a new empty GOOD referent.
pub fn new_good_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Create a new empty GOODATTR referent.
pub fn new_goodattr_referent() -> Referent {
    Referent::new(GOODATTR_OBJ_TYPENAME)
}

// ── Accessors for GoodAttributeReferent ───────────────────────────────────

/// Get the type of a GOODATTR referent.
pub fn get_attr_type(r: &Referent) -> GoodAttrType {
    r.get_string_value(ATTR_TYPE)
        .map(GoodAttrType::from_str)
        .unwrap_or_default()
}

/// Set the type of a GOODATTR referent.
pub fn set_attr_type(r: &mut Referent, typ: GoodAttrType) {
    r.add_slot(ATTR_TYPE, SlotValue::Str(typ.as_str().to_string()), true);
}

/// Get the primary value string of a GOODATTR referent.
pub fn get_attr_value(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_VALUE)
}

/// Add a VALUE slot to a GOODATTR referent.
pub fn add_attr_value(r: &mut Referent, v: impl Into<String>) {
    r.add_slot(ATTR_VALUE, SlotValue::Str(v.into()), false);
}

/// Add an ALTVALUE slot to a GOODATTR referent.
pub fn add_attr_altvalue(r: &mut Referent, v: impl Into<String>) {
    r.add_slot(ATTR_ALTVALUE, SlotValue::Str(v.into()), false);
}

/// Set the NAME slot (human-readable attribute label, e.g. "ФАСОВКА").
pub fn set_attr_name(r: &mut Referent, name: impl Into<String>) {
    r.add_slot(ATTR_NAME, SlotValue::Str(name.into()), false);
}

/// Set the REF slot (reference to external entity).
pub fn set_attr_ref(r: &mut Referent, ref_r: std::rc::Rc<std::cell::RefCell<Referent>>) {
    r.add_slot(ATTR_REF, SlotValue::Referent(ref_r), true);
}

/// Build a display string for a GOODATTR referent (short variant).
pub fn goodattr_to_string(r: &Referent) -> String {
    let typ = get_attr_type(r);
    let mut res = String::new();
    if let Some(v) = r.get_string_value(ATTR_VALUE) {
        match typ {
            GoodAttrType::Keyword | GoodAttrType::Character => {
                res.push_str(&v.to_lowercase());
            }
            _ => {
                res.push_str(v);
            }
        }
    }
    if let Some(ref_slot) = r.find_slot(ATTR_REF, None) {
        if let Some(sv) = &ref_slot.value {
            if let Some(ref_r) = sv.as_referent() {
                let ref_str = ref_r.borrow().get_string_value("NAME")
                    .map(|s| s.to_string())
                    .or_else(|| ref_r.borrow().get_string_value("VALUE").map(|s| s.to_string()))
                    .unwrap_or_else(|| ref_r.borrow().type_name.clone());
                if !res.is_empty() { res.push(' '); }
                res.push_str(&ref_str);
            }
        }
    }
    res
}

/// Build a display string for a GOOD referent (joins all attribute strings).
pub fn good_to_string(r: &Referent) -> String {
    let mut parts: Vec<String> = Vec::new();
    for slot in &r.slots {
        if slot.type_name != ATTR_ATTR { continue; }
        if let Some(sv) = &slot.value {
            if let Some(ref_r) = sv.as_referent() {
                let s = goodattr_to_string(&ref_r.borrow());
                if !s.is_empty() { parts.push(s); }
            }
        }
    }
    parts.join(" ")
}
