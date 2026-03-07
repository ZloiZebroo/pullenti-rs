/// ResumeItemType + ResumeItemReferent constants.
/// Mirrors `ResumeItemType.cs` and `ResumeItemReferent.cs`.

use crate::referent::{Referent, SlotValue};

// ── Constants ─────────────────────────────────────────────────────────────

pub const OBJ_TYPENAME: &str = "RESUME";
pub const ATTR_TYPE:      &str = "TYPE";
pub const ATTR_VALUE:     &str = "VALUE";
pub const ATTR_REF:       &str = "REF";
pub const ATTR_DATERANGE: &str = "DATERANGE";
pub const ATTR_MISC:      &str = "MISC";

// ── ResumeItemType ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResumeItemType {
    #[default]
    Undefined,
    Person,
    Contact,
    Organization,
    Study,
    Position,
    Sex,
    Age,
    Money,
    Education,
    Experience,
    Language,
    DrivingLicense,
    License,
    Speciality,
    Skill,
    Moral,
    Hobby,
    Document,
}

impl ResumeItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            ResumeItemType::Undefined      => "Undefined",
            ResumeItemType::Person         => "Person",
            ResumeItemType::Contact        => "Contact",
            ResumeItemType::Organization   => "Organization",
            ResumeItemType::Study          => "Study",
            ResumeItemType::Position       => "Position",
            ResumeItemType::Sex            => "Sex",
            ResumeItemType::Age            => "Age",
            ResumeItemType::Money          => "Money",
            ResumeItemType::Education      => "Education",
            ResumeItemType::Experience     => "Experience",
            ResumeItemType::Language       => "Language",
            ResumeItemType::DrivingLicense => "DrivingLicense",
            ResumeItemType::License        => "License",
            ResumeItemType::Speciality     => "Speciality",
            ResumeItemType::Skill          => "Skill",
            ResumeItemType::Moral          => "Moral",
            ResumeItemType::Hobby          => "Hobby",
            ResumeItemType::Document       => "Document",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Person"        => ResumeItemType::Person,
            "Contact"       => ResumeItemType::Contact,
            "Organization"  => ResumeItemType::Organization,
            "Study"         => ResumeItemType::Study,
            "Position"      => ResumeItemType::Position,
            "Sex"           => ResumeItemType::Sex,
            "Age"           => ResumeItemType::Age,
            "Money"         => ResumeItemType::Money,
            "Education"     => ResumeItemType::Education,
            "Experience"    => ResumeItemType::Experience,
            "Language"      => ResumeItemType::Language,
            "DrivingLicense"=> ResumeItemType::DrivingLicense,
            "License"       => ResumeItemType::License,
            "Speciality"    => ResumeItemType::Speciality,
            "Skill"         => ResumeItemType::Skill,
            "Moral"         => ResumeItemType::Moral,
            "Hobby"         => ResumeItemType::Hobby,
            "Document"      => ResumeItemType::Document,
            _               => ResumeItemType::Undefined,
        }
    }
}

// ── Constructor & accessors ────────────────────────────────────────────────

pub fn new_resume_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

pub fn get_typ(r: &Referent) -> ResumeItemType {
    r.get_string_value(ATTR_TYPE)
        .map(ResumeItemType::from_str)
        .unwrap_or_default()
}

pub fn set_typ(r: &mut Referent, typ: ResumeItemType) {
    r.add_slot(ATTR_TYPE, SlotValue::Str(typ.as_str().to_string()), true);
}

pub fn get_value(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_VALUE)
}

pub fn set_value(r: &mut Referent, v: &str) {
    r.add_slot(ATTR_VALUE, SlotValue::Str(v.to_string()), true);
}
