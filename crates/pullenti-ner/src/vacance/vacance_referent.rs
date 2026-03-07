/// VacanceItemReferent — a parsed element of a job vacancy posting.
/// Mirrors `VacanceItemReferent.cs` / `VacanceItemType.cs`.

use crate::referent::{Referent, SlotValue};

pub const OBJ_TYPENAME: &str = "VACANCY";
pub const ATTR_TYPE:    &str = "TYPE";
pub const ATTR_VALUE:   &str = "VALUE";
pub const ATTR_REF:     &str = "REF";
pub const ATTR_EXPIRED: &str = "EXPIRED";

/// Type of a vacancy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VacanceItemType {
    #[default]
    Undefined,
    /// Job title / vacancy name
    Name,
    /// Publication / expiry date
    Date,
    /// Offered salary
    Money,
    /// Required education level
    Education,
    /// Required work experience
    Experience,
    /// Required language(s)
    Language,
    /// Driving license requirement
    DrivingLicense,
    /// Other certificates / permits
    License,
    /// Soft skills / personal qualities
    Moral,
    /// Required hard skill
    Skill,
    /// Optional / "nice to have" skill
    Plus,
}

impl VacanceItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            VacanceItemType::Undefined     => "undefined",
            VacanceItemType::Name          => "name",
            VacanceItemType::Date          => "date",
            VacanceItemType::Money         => "money",
            VacanceItemType::Education     => "education",
            VacanceItemType::Experience    => "experience",
            VacanceItemType::Language      => "language",
            VacanceItemType::DrivingLicense => "drivinglicense",
            VacanceItemType::License       => "license",
            VacanceItemType::Moral         => "moral",
            VacanceItemType::Skill         => "skill",
            VacanceItemType::Plus          => "plus",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "name"          => VacanceItemType::Name,
            "date"          => VacanceItemType::Date,
            "money"         => VacanceItemType::Money,
            "education"     => VacanceItemType::Education,
            "experience"    => VacanceItemType::Experience,
            "language"      => VacanceItemType::Language,
            "drivinglicense" => VacanceItemType::DrivingLicense,
            "license"       => VacanceItemType::License,
            "moral"         => VacanceItemType::Moral,
            "skill"         => VacanceItemType::Skill,
            "plus"          => VacanceItemType::Plus,
            _               => VacanceItemType::Undefined,
        }
    }
}

pub fn new_vacancy_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_item_type(r: &Referent) -> VacanceItemType {
    r.get_string_value(ATTR_TYPE)
        .map(VacanceItemType::from_str)
        .unwrap_or_default()
}

pub fn set_item_type(r: &mut Referent, typ: VacanceItemType) {
    r.add_slot(ATTR_TYPE, SlotValue::Str(typ.as_str().to_string()), true);
}

pub fn get_value(r: &Referent) -> Option<String> {
    r.get_string_value(ATTR_VALUE).map(|s| s.to_string())
}

pub fn set_value(r: &mut Referent, v: &str) {
    r.add_slot(ATTR_VALUE, SlotValue::Str(v.to_string()), true);
}

pub fn is_expired(r: &Referent) -> bool {
    r.get_string_value(ATTR_EXPIRED) == Some("true")
}

pub fn set_expired(r: &mut Referent) {
    r.add_slot(ATTR_EXPIRED, SlotValue::Str("true".to_string()), true);
}
