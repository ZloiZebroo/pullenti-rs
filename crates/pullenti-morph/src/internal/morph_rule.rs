use std::collections::HashMap;
use super::byte_array_wrapper::ByteArrayWrapper;
use super::morph_rule_variant::MorphRuleVariant;

pub struct MorphRule {
    pub id: i32,
    tail_map: HashMap<String, usize>,
    pub morph_vars: Vec<Vec<MorphRuleVariant>>,
    pub lazy_pos: usize,
}

impl MorphRule {
    pub fn new() -> Self {
        MorphRule {
            id: 0,
            tail_map: HashMap::new(),
            morph_vars: Vec::new(),
            lazy_pos: 0,
        }
    }

    pub fn contains_var(&self, tail: &str) -> bool {
        self.tail_map.contains_key(tail)
    }

    pub fn get_vars(&self, key: &str) -> Option<&Vec<MorphRuleVariant>> {
        self.tail_map.get(key).map(|&i| &self.morph_vars[i])
    }

    pub fn find_var(&self, id: i16) -> Option<&MorphRuleVariant> {
        for li in &self.morph_vars {
            for v in li {
                if v.id == id {
                    return Some(v);
                }
            }
        }
        None
    }

    pub fn add(&mut self, tail: String, vars: Vec<MorphRuleVariant>) {
        let idx = self.morph_vars.len();
        self.tail_map.insert(tail, idx);
        self.morph_vars.push(vars);
    }

    pub fn tails(&self) -> impl Iterator<Item = &String> {
        self.tail_map.keys()
    }

    pub fn deserialize(&mut self, str: &ByteArrayWrapper, pos: &mut usize) {
        let ii = str.deserialize_short(pos);
        self.id = ii as i32;
        let mut id: i16 = 1;

        while !str.is_eof(*pos) {
            let b = str.deserialize_byte(pos);
            if b == 0xFF {
                break;
            }
            *pos -= 1;
            let key = str.deserialize_string(pos);

            let mut li = Vec::new();
            while !str.is_eof(*pos) {
                let mut mrv = MorphRuleVariant::new();
                if !mrv.deserialize(str, pos) {
                    break;
                }
                mrv.tail = key.clone();
                mrv.rule_id = ii;
                mrv.id = id;
                id += 1;
                li.push(mrv);
            }
            self.add(key, li);
        }
    }
}

impl std::fmt::Display for MorphRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.tail_map.keys().map(|t| format!("-{}", t)).collect();
        write!(f, "{}", parts.join(", "))
    }
}
