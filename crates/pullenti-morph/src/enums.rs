use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

/// Gender (Род)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphGender {
    #[default]
    Undefined = 0,
    Masculine = 1,
    Feminie = 2,
    Neuter = 4,
}

// MorphGender is used as a bitfield in the C# code (values OR'd together)
// We need a wrapper that supports bitwise operations
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct MorphGenderFlags(pub i16);

impl MorphGenderFlags {
    pub const UNDEFINED: MorphGenderFlags = MorphGenderFlags(0);
    pub const MASCULINE: MorphGenderFlags = MorphGenderFlags(1);
    pub const FEMINIE: MorphGenderFlags = MorphGenderFlags(2);
    pub const NEUTER: MorphGenderFlags = MorphGenderFlags(4);
}

impl From<MorphGender> for MorphGenderFlags {
    fn from(g: MorphGender) -> Self {
        MorphGenderFlags(g as i16)
    }
}

impl BitOr for MorphGenderFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { MorphGenderFlags(self.0 | rhs.0) }
}

impl BitOrAssign for MorphGenderFlags {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl BitAnd for MorphGenderFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { MorphGenderFlags(self.0 & rhs.0) }
}

impl BitAndAssign for MorphGenderFlags {
    fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
}

/// Number (Число)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct MorphNumber(pub i16);

impl MorphNumber {
    pub const UNDEFINED: MorphNumber = MorphNumber(0);
    pub const SINGULAR: MorphNumber = MorphNumber(1);
    pub const PLURAL: MorphNumber = MorphNumber(2);
}

impl BitOr for MorphNumber {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { MorphNumber(self.0 | rhs.0) }
}

impl BitOrAssign for MorphNumber {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl BitAnd for MorphNumber {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { MorphNumber(self.0 & rhs.0) }
}

impl BitAndAssign for MorphNumber {
    fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
}

/// Tense (Время)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphTense {
    #[default]
    Undefined = 0,
    Past = 1,
    Present = 2,
    Future = 4,
}

/// Voice (Залог)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphVoice {
    #[default]
    Undefined = 0,
    Active = 1,
    Passive = 2,
    Middle = 4,
}

/// Aspect (Аспект)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphAspect {
    #[default]
    Undefined = 0,
    Perfective = 1,
    Imperfective = 2,
}

/// Mood (Наклонение)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphMood {
    #[default]
    Undefined = 0,
    Indicative = 1,
    Subjunctive = 2,
    Imperative = 4,
}

/// Person (Лицо)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct MorphPerson(pub i16);

impl MorphPerson {
    pub const UNDEFINED: MorphPerson = MorphPerson(0);
    pub const FIRST: MorphPerson = MorphPerson(1);
    pub const SECOND: MorphPerson = MorphPerson(2);
    pub const THIRD: MorphPerson = MorphPerson(4);
}

impl BitOr for MorphPerson {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { MorphPerson(self.0 | rhs.0) }
}

impl BitOrAssign for MorphPerson {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl BitAnd for MorphPerson {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { MorphPerson(self.0 & rhs.0) }
}

impl BitAndAssign for MorphPerson {
    fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
}

/// Form (Форма)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphForm {
    #[default]
    Undefined = 0,
    Short = 1,
    Synonym = 2,
}

/// Finite (for English verbs)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
#[repr(i16)]
pub enum MorphFinite {
    #[default]
    Undefined = 0,
    Finite = 1,
    Infinitive = 2,
    Participle = 4,
    Gerund = 8,
}
