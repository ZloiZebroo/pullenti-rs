/// DerivateDictionary — the word-family lookup database.
/// Mirrors `DerivateDictionary.cs`.

use pullenti_morph::{MorphLang, LanguageHelper};
use pullenti_morph::internal::byte_array_wrapper::ByteArrayWrapper;
use pullenti_morph::internal::morph_deserializer::MorphDeserializer;
use super::deriv_group::DerivateGroup;
use super::explan_tree_node::ExplanTreeNode;

pub struct DerivateDictionary {
    pub all_groups: Vec<DerivateGroup>,
    root:           ExplanTreeNode,
}

impl DerivateDictionary {
    pub fn new() -> Self {
        DerivateDictionary {
            all_groups: Vec::new(),
            root:       ExplanTreeNode::new(),
        }
    }

    pub fn is_loaded(&self) -> bool { !self.all_groups.is_empty() }

    pub fn load(&mut self, raw_gzip: &[u8]) {
        let data = MorphDeserializer::deflate_gzip(raw_gzip);
        let buf  = ByteArrayWrapper::new(&data);
        let mut pos = 0usize;

        self.all_groups.clear();

        let mut cou = buf.deserialize_int(&mut pos) as i32;
        while cou > 0 {
            cou -= 1;
            let p1 = buf.deserialize_int(&mut pos) as usize; // end-of-group offset
            let mut gr = DerivateGroup::new();
            gr.deserialize(&buf, &mut pos);
            gr.id = self.all_groups.len() + 1;
            self.all_groups.push(gr);
            let _ = p1; // not needed in eager mode
        }

        self.root = ExplanTreeNode::new();
        self.root.deserialize(&buf, &mut self.all_groups, &mut pos);
    }

    pub fn find(
        &self,
        word:       &str,
        try_create: bool,
        lang:       MorphLang,
    ) -> Option<Vec<&DerivateGroup>> {
        if word.is_empty() { return None; }

        let li = self.find_in_trie(word, lang);
        if li.is_some() { return li; }

        // Suffix-based fallbacks (matches C# DerivateDictionary.Find)
        if word.len() < 4 { return None; }

        let chars: Vec<char> = word.chars().collect();
        let n = chars.len();
        let ch0 = chars[n-1];
        let ch1 = chars[n-2];
        let ch2 = chars[n-3];

        if ch0 == 'О' || (ch0 == 'И' && ch1 == 'К') {
            let stem: String = chars[..n-1].iter().collect();
            if let Some(r) = self.find_in_trie(&(stem.clone() + "ИЙ"), lang) { return Some(r); }
            if let Some(r) = self.find_in_trie(&(stem.clone() + "ЫЙ"), lang) { return Some(r); }
            if ch0 == 'О' && ch1 == 'Н' {
                if let Some(r) = self.find_in_trie(&(stem.clone() + "СКИЙ"), lang) { return Some(r); }
            }
        } else if (ch0 == 'Я' || ch0 == 'Ь') && ch1 == 'С' {
            let stem: String = chars[..n-2].iter().collect();
            if stem == "ЯТЬ" { return None; }
            if let Some(r) = self.find_in_trie(&stem, lang) { return Some(r); }
        } else if ch0 == 'Е' && ch1 == 'Ь' {
            let stem: String = chars[..n-2].iter().collect();
            let word1 = stem + "ИЕ";
            if let Some(r) = self.find_in_trie(&word1, lang) { return Some(r); }
        } else if ch0 == 'Й' && ch2 == 'Н' && try_create {
            let n4 = if n > 3 { chars[n-4] } else { '\0' };
            let word1 = if n4 != 'Н' && LanguageHelper::is_cyrillic_vowel(n4) {
                // insert Н before last 3 chars
                let mut s: String = chars[..n-3].iter().collect();
                s.push('Н');
                s.push_str(&chars[n-3..].iter().collect::<String>());
                Some(s)
            } else if n4 == 'Н' {
                // remove one Н
                let mut s: String = chars[..n-4].iter().collect();
                s.push_str(&chars[n-3..].iter().collect::<String>());
                Some(s)
            } else {
                None
            };
            if let Some(w1) = word1 {
                if let Some(r) = self.find_in_trie(&w1, lang) { return Some(r); }
            }
        }

        if ch0 == 'Й' && ch1 == 'О' {
            let stem: String = chars[..n-2].iter().collect();
            if let Some(r) = self.find_in_trie(&(stem.clone() + "ИЙ"), lang) { return Some(r); }
            if let Some(r) = self.find_in_trie(&(stem.clone() + "ЫЙ"), lang) { return Some(r); }
        }

        None
    }

    fn find_in_trie(&self, word: &str, lang: MorphLang) -> Option<Vec<&DerivateGroup>> {
        let mut tn = &self.root;
        for ch in word.encode_utf16() {
            let k = ch as i16;
            match &tn.nodes {
                None => return None,
                Some(m) => match m.get(&k) {
                    None => return None,
                    Some(child) => tn = child,
                }
            }
        }
        let ids = tn.groups.as_ref()?;
        let mut li: Vec<&DerivateGroup> = ids.iter()
            .filter_map(|&id| self.get_group(id))
            .collect();
        if li.is_empty() { return None; }

        // If mix of generated and non-generated, keep only non-generated
        let has_gen   = li.iter().any(|g| g.is_generated);
        let has_nogen = li.iter().any(|g| !g.is_generated);
        if has_gen && has_nogen {
            li.retain(|g| !g.is_generated);
        }

        // Filter by language
        if !lang.is_undefined() {
            li.retain(|g| g.contains_word(word, lang));
        }

        if li.is_empty() { None } else { Some(li) }
    }

    pub fn get_group(&self, id: usize) -> Option<&DerivateGroup> {
        if id >= 1 && id <= self.all_groups.len() {
            Some(&self.all_groups[id - 1])
        } else {
            None
        }
    }
}
