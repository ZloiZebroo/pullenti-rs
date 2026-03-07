/// ExplanWordAttr — bitfield attributes for DerivateWord.
/// Mirrors `ExplanWordAttr.cs`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplanWordAttr {
    pub value: i16,
}

impl ExplanWordAttr {
    fn get_bit(&self, i: u32) -> bool { ((self.value >> i) & 1) != 0 }
    fn set_bit(&mut self, i: u32, v: bool) {
        if v { self.value |= 1 << i; } else { self.value &= !(1 << i); }
    }

    pub fn is_undefined(self)         -> bool { self.value == 0 }
    pub fn is_animated(self)          -> bool { self.get_bit(0) }
    pub fn set_animated(&mut self, v: bool)   { self.set_bit(0, v); }
    pub fn is_named(self)             -> bool { self.get_bit(1) }
    pub fn set_named(&mut self, v: bool)      { self.set_bit(1, v); }
    pub fn is_numbered(self)          -> bool { self.get_bit(2) }
    pub fn is_measured(self)          -> bool { self.get_bit(3) }
    pub fn is_emo_positive(self)      -> bool { self.get_bit(4) }
    pub fn is_emo_negative(self)      -> bool { self.get_bit(5) }
    pub fn is_animal(self)            -> bool { self.get_bit(6) }
    pub fn is_man(self)               -> bool { self.get_bit(7) }
    pub fn is_can_person_after(self)  -> bool { self.get_bit(8) }
    pub fn is_space_object(self)      -> bool { self.get_bit(9) }
    pub fn is_time_object(self)       -> bool { self.get_bit(10) }
    pub fn is_verb_noun(self)         -> bool { self.get_bit(11) }
}

impl std::ops::BitAnd for ExplanWordAttr {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self { ExplanWordAttr { value: self.value & rhs.value } }
}

impl std::ops::BitOr for ExplanWordAttr {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { ExplanWordAttr { value: self.value | rhs.value } }
}
