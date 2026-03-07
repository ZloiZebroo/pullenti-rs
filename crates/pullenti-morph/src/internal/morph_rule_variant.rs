use crate::{MorphBaseInfo, MorphClass, MorphCase, MorphGenderFlags, MorphNumber};
use super::byte_array_wrapper::ByteArrayWrapper;

#[derive(Clone, Default)]
pub struct MorphRuleVariant {
    pub base: MorphBaseInfo,
    pub tail: String,
    pub misc_info_id: i16,
    pub rule_id: i16,
    pub id: i16,
    pub normal_tail: Option<String>,
    pub full_normal_tail: Option<String>,
}

impl MorphRuleVariant {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copy_from_variant(&mut self, src: &MorphRuleVariant) {
        self.tail = src.tail.clone();
        self.base.copy_from(&src.base);
        self.misc_info_id = src.misc_info_id;
        self.normal_tail = src.normal_tail.clone();
        self.full_normal_tail = src.full_normal_tail.clone();
        self.rule_id = src.rule_id;
    }

    pub fn compare(&self, mrv: &MorphRuleVariant) -> bool {
        if mrv.base.class != self.base.class
            || mrv.base.gender != self.base.gender
            || mrv.base.number != self.base.number
            || mrv.base.case != self.base.case
        {
            return false;
        }
        if mrv.misc_info_id != self.misc_info_id {
            return false;
        }
        if mrv.normal_tail != self.normal_tail {
            return false;
        }
        true
    }

    pub fn deserialize(&mut self, str: &ByteArrayWrapper, pos: &mut usize) -> bool {
        let id = str.deserialize_short(pos);
        if id <= 0 {
            return false;
        }
        self.misc_info_id = id;

        let iii = str.deserialize_short(pos);
        let mut mc = MorphClass::from_value(iii);
        if mc.is_misc() && mc.is_proper() {
            mc.set_misc(false);
        }
        self.base.class = mc;

        let bbb = str.deserialize_byte(pos);
        self.base.gender = MorphGenderFlags(bbb as i16);

        let bbb = str.deserialize_byte(pos);
        self.base.number = MorphNumber(bbb as i16);

        let bbb = str.deserialize_byte(pos);
        self.base.case = MorphCase::from_value(bbb as i16);

        let s = str.deserialize_string(pos);
        self.normal_tail = if s.is_empty() { None } else { Some(s) };

        let s = str.deserialize_string(pos);
        self.full_normal_tail = if s.is_empty() { None } else { Some(s) };

        true
    }
}

impl std::fmt::Display for MorphRuleVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = format!("-{}", self.tail);
        if let Some(ref nt) = self.normal_tail {
            res.push_str(&format!(" [-{}]", nt));
        }
        if let Some(ref fnt) = self.full_normal_tail {
            if self.full_normal_tail != self.normal_tail {
                res.push_str(&format!(" [-{}]", fnt));
            }
        }
        res.push_str(&format!(" {}", self.base));
        write!(f, "{}", res.trim())
    }
}
