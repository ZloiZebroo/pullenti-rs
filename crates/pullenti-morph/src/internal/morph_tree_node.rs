use std::collections::HashMap;
use super::byte_array_wrapper::ByteArrayWrapper;
use super::morph_rule_variant_ref::MorphRuleVariantRef;
use super::morph_engine::MorphEngine;

pub struct MorphTreeNode {
    pub nodes: Option<HashMap<i16, MorphTreeNode>>,
    pub rule_ids: Option<Vec<i32>>,
    pub reverce_variants: Option<Vec<MorphRuleVariantRef>>,
    pub lazy_pos: usize,
}

impl MorphTreeNode {
    pub fn new() -> Self {
        MorphTreeNode {
            nodes: None,
            rule_ids: None,
            reverce_variants: None,
            lazy_pos: 0,
        }
    }

    fn deserialize_base(&mut self, str: &ByteArrayWrapper, pos: &mut usize) {
        let cou = str.deserialize_short(pos) as i32;
        if cou > 0 {
            let mut rule_ids = Vec::new();
            for _ in 0..cou {
                let id = str.deserialize_short(pos) as i32;
                rule_ids.push(id);
            }
            self.rule_ids = Some(rule_ids);
        }

        let cou = str.deserialize_short(pos) as i32;
        if cou > 0 {
            let mut reverce_variants = Vec::new();
            for _ in 0..cou {
                let rid = str.deserialize_short(pos) as i32;
                let id = str.deserialize_short(pos);
                let co = str.deserialize_short(pos);
                reverce_variants.push(MorphRuleVariantRef::new(rid, id, co));
            }
            self.reverce_variants = Some(reverce_variants);
        }
    }

    pub fn deserialize(&mut self, str: &ByteArrayWrapper, pos: &mut usize) -> i32 {
        let mut res = 0;
        self.deserialize_base(str, pos);

        let cou = str.deserialize_short(pos) as i32;
        if cou > 0 {
            let mut nodes = HashMap::new();
            for _ in 0..cou {
                let i = str.deserialize_short(pos);
                let _pp = str.deserialize_int(pos); // skip end position
                let mut child = MorphTreeNode::new();
                let res1 = child.deserialize(str, pos);
                res += 1 + res1;
                nodes.insert(i, child);
            }
            self.nodes = Some(nodes);
        }
        res
    }

    pub fn deserialize_lazy(&mut self, str: &ByteArrayWrapper, me: &mut MorphEngine, pos: &mut usize) {
        self.deserialize_base(str, pos);

        let cou = str.deserialize_short(pos) as i32;
        if cou > 0 {
            let mut nodes = HashMap::new();
            for _ in 0..cou {
                let i = str.deserialize_short(pos);
                let pp = str.deserialize_int(pos) as usize;
                let mut child = MorphTreeNode::new();
                child.lazy_pos = *pos;
                nodes.insert(i, child);
                *pos = pp;
            }
            self.nodes = Some(nodes);
        }

        let p = *pos;
        if let Some(ref rule_ids) = self.rule_ids {
            let rule_ids_copy: Vec<i32> = rule_ids.clone();
            for rid in &rule_ids_copy {
                if let Some(r) = me.get_mut_rule(*rid) {
                    if r.lazy_pos > 0 {
                        *pos = r.lazy_pos;
                        let _lp = r.lazy_pos;
                        // Need to deserialize - we have the buf
                        r.deserialize(str, pos);
                        r.lazy_pos = 0;
                    }
                }
            }
            *pos = p;
        }

        if let Some(ref reverce_variants) = self.reverce_variants {
            let rv_copy: Vec<(i32, usize)> = reverce_variants.iter()
                .map(|rv| (rv.rule_id, 0))
                .collect();
            for (rule_id, _) in &rv_copy {
                if let Some(r) = me.get_mut_rule(*rule_id) {
                    if r.lazy_pos > 0 {
                        *pos = r.lazy_pos;
                        r.deserialize(str, pos);
                        r.lazy_pos = 0;
                    }
                }
            }
            *pos = p;
        }
    }
}
