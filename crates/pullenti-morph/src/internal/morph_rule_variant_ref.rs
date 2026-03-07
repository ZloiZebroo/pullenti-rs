use std::cmp::Ordering;

#[derive(Clone)]
pub struct MorphRuleVariantRef {
    pub rule_id: i32,
    pub variant_id: i16,
    pub coef: i16,
}

impl MorphRuleVariantRef {
    pub fn new(rule_id: i32, variant_id: i16, coef: i16) -> Self {
        MorphRuleVariantRef { rule_id, variant_id, coef }
    }
}

impl Ord for MorphRuleVariantRef {
    fn cmp(&self, other: &Self) -> Ordering {
        other.coef.cmp(&self.coef) // descending by coef
    }
}

impl PartialOrd for MorphRuleVariantRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MorphRuleVariantRef {
    fn eq(&self, other: &Self) -> bool {
        self.coef == other.coef
    }
}

impl Eq for MorphRuleVariantRef {}

impl std::fmt::Display for MorphRuleVariantRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.rule_id, self.variant_id)
    }
}
