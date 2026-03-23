use crate::{MorphLang, MorphClass, MorphWordForm, MorphMiscInfo, MorphNumber, MorphGenderFlags, LanguageHelper};
use super::byte_array_wrapper::ByteArrayWrapper;
use super::morph_deserializer::MorphDeserializer;
use super::morph_rule::MorphRule;
use super::morph_rule_variant::MorphRuleVariant;
use super::morph_tree_node::MorphTreeNode;

pub struct MorphEngine {
    pub language: MorphLang,
    pub m_root: MorphTreeNode,
    pub m_root_reverce: MorphTreeNode,
    m_rules: Vec<MorphRule>,
    m_misc_infos: Vec<MorphMiscInfo>,
    m_lazy_buf: Option<Vec<u8>>,
}

impl MorphEngine {
    pub fn new() -> Self {
        MorphEngine {
            language: MorphLang::UNKNOWN,
            m_root: MorphTreeNode::new(),
            m_root_reverce: MorphTreeNode::new(),
            m_rules: Vec::new(),
            m_misc_infos: Vec::new(),
            m_lazy_buf: None,
        }
    }

    pub fn add_rule(&mut self, r: MorphRule) {
        self.m_rules.push(r);
    }

    pub fn get_rule(&self, id: i32) -> Option<&MorphRule> {
        if id > 0 && (id as usize) <= self.m_rules.len() {
            Some(&self.m_rules[(id - 1) as usize])
        } else {
            None
        }
    }

    pub fn get_mut_rule(&mut self, id: i32) -> Option<&mut MorphRule> {
        if id > 0 && (id as usize) <= self.m_rules.len() {
            Some(&mut self.m_rules[(id - 1) as usize])
        } else {
            None
        }
    }

    pub fn get_rule_var(&self, rid: i32, vid: i16) -> Option<&MorphRuleVariant> {
        self.get_rule(rid)?.find_var(vid)
    }

    pub fn add_misc_info(&mut self, mut mi: MorphMiscInfo) {
        if mi.id == 0 {
            mi.id = (self.m_misc_infos.len() + 1) as i32;
        }
        self.m_misc_infos.push(mi);
    }

    pub fn get_misc_info(&self, id: i16) -> Option<&MorphMiscInfo> {
        if id > 0 && (id as usize) <= self.m_misc_infos.len() {
            Some(&self.m_misc_infos[(id - 1) as usize])
        } else {
            None
        }
    }

    pub fn initialize_from_bytes(&mut self, data: &[u8], lang: MorphLang, lazy_load: bool) -> bool {
        if !self.language.is_undefined() {
            return false;
        }
        self.language = lang;
        self.deserialize(data, false, lazy_load);
        true
    }

    pub fn process(&self, word: &str, ignore_no_dict: bool) -> Option<Vec<MorphWordForm>> {
        if word.is_empty() {
            return None;
        }

        let mut res: Option<Vec<MorphWordForm>> = None;
        let chars: Vec<char> = word.chars().collect();

        // Check for at least one vowel in multi-char words
        if chars.len() > 1 {
            let has_vowel = chars.iter().any(|&ch|
                LanguageHelper::is_cyrillic_vowel(ch) || LanguageHelper::is_latin_vowel(ch)
            );
            if !has_vowel {
                return res;
            }
        }

        // Walk forward tree
        let mut tn = &self.m_root;
        let mut i = 0;
        let mut word_begin_buf = String::new();
        let mut word_end_buf = String::new();
        loop {
            if i > chars.len() { break; }

            if let Some(ref rule_ids) = tn.rule_ids {
                word_end_buf.clear();
                if i == 0 {
                    word_end_buf.push_str(word);
                } else if i < chars.len() {
                    word_end_buf.extend(chars[i..].iter());
                }

                if res.is_none() {
                    res = Some(Vec::new());
                }

                let mut word_begin_set = false;

                for &rid in rule_ids {
                    if let Some(r) = self.get_rule(rid) {
                        if let Some(mvs) = r.get_vars(&word_end_buf) {
                            if !word_begin_set {
                                word_begin_buf.clear();
                                if i == chars.len() {
                                    word_begin_buf.push_str(word);
                                } else if i > 0 {
                                    word_begin_buf.extend(chars[..i].iter());
                                }
                                word_begin_set = true;
                            }
                            self.process_result(res.as_mut().unwrap(), &word_begin_buf, mvs);
                        }
                    }
                }
            }

            if tn.nodes.is_none() || i >= chars.len() {
                break;
            }
            let ch = chars[i] as i16;
            match tn.nodes.as_ref().unwrap().get(&ch) {
                Some(next) => {
                    tn = next;
                    i += 1;
                }
                None => break,
            }
        }

        // Determine if we need to test unknown variants
        let mut need_test_unknown = true;
        let mut to_first: Option<usize> = None;

        if let Some(ref r) = res {
            for (idx, wf) in r.iter().enumerate() {
                if wf.base.class.is_pronoun() || wf.base.class.is_noun()
                    || wf.base.class.is_adjective()
                    || (wf.base.class.is_misc() && wf.base.class.is_conjunction())
                    || wf.base.class.is_preposition()
                {
                    need_test_unknown = false;
                } else if wf.base.class.is_adverb() {
                    if let Some(ref nc) = wf.normal_case {
                        if !LanguageHelper::ends_with_ex(nc, &["О", "А"]) {
                            need_test_unknown = false;
                        } else if nc == "МНОГО" {
                            need_test_unknown = false;
                        }
                    }
                } else if wf.base.class.is_verb() && r.len() > 1 {
                    let ok = r.iter().any(|rr| !std::ptr::eq(rr, wf) && rr.base.class != wf.base.class);
                    if ok && !LanguageHelper::ends_with(word, "ИМ") {
                        need_test_unknown = false;
                    }
                }

                // Check for priority reordering
                if r.len() > 1 && to_first.is_none() {
                    if let Some(ref nf) = wf.normal_full {
                        if matches!(nf.as_str(), "КОПИЯ" | "ПОЛК" | "СУД" | "ПАРК" | "БАНК" | "ПОЛОЖЕНИЕ") {
                            to_first = Some(idx);
                        }
                    }
                    if let Some(ref nc) = wf.normal_case {
                        if matches!(nc.as_str(), "МОРЕ" | "МАРИЯ" | "ВЕТЕР" | "КИЕВ") {
                            to_first = Some(idx);
                        }
                    }
                }
            }
        }

        // Reorder if needed
        if let Some(idx) = to_first {
            if let Some(ref mut r) = res {
                if idx > 0 {
                    let item = r.remove(idx);
                    r.insert(0, item);
                }
            }
        }

        // Check for unknown variants using reverse tree
        if need_test_unknown && LanguageHelper::is_cyrillic_char(chars[0]) {
            let mut gl = 0;
            let mut sog = 0;
            for &ch in &chars {
                if LanguageHelper::is_cyrillic_vowel(ch) { gl += 1; } else { sog += 1; }
            }
            if gl < 2 || sog < 2 {
                need_test_unknown = false;
            }
        }

        if need_test_unknown {
            if let Some(ref r) = res {
                if r.len() == 1 {
                    let wf = &r[0];
                    if wf.base.class.is_verb() {
                        if let Some(ref misc) = wf.misc {
                            let has = |s: &str| misc.attrs.iter().any(|a| a == s);
                            if has("н.вр.") && has("нес.в.") && !has("страд.з.") {
                                need_test_unknown = false;
                            } else if has("б.вр.") && has("сов.в.") {
                                need_test_unknown = false;
                            } else if has("инф.") && has("сов.в.") {
                                need_test_unknown = false;
                            }
                        }
                        if let Some(ref nc) = wf.normal_case {
                            if LanguageHelper::ends_with(nc, "СЯ") {
                                need_test_unknown = false;
                            }
                        }
                    }
                    if wf.base.class.is_undefined() {
                        if let Some(ref misc) = wf.misc {
                            if misc.attrs.iter().any(|a| a == "прдктв.") {
                                need_test_unknown = false;
                            }
                        }
                    }
                }
            }
        }

        if need_test_unknown && !ignore_no_dict {
            // Walk reverse tree
            let mut tn = &self.m_root_reverce;
            let mut tn0 = &self.m_root_reverce;
            let mut rev_i: Option<usize> = None;

            for j in (0..chars.len()).rev() {
                let ch = chars[j] as i16;
                if tn.nodes.is_none() { break; }
                match tn.nodes.as_ref().unwrap().get(&ch) {
                    Some(next) => {
                        tn = next;
                        if tn.reverce_variants.is_some() {
                            tn0 = tn;
                            rev_i = Some(j);
                            break;
                        }
                    }
                    None => break,
                }
            }

            if !std::ptr::eq(tn0, &self.m_root_reverce) {
                let i_val = rev_i.unwrap_or(0);
                let mut glas = i_val < 4;
                if !glas {
                    for k in (0..i_val).rev() {
                        if LanguageHelper::is_cyrillic_vowel(chars[k]) || LanguageHelper::is_latin_vowel(chars[k]) {
                            glas = true;
                            break;
                        }
                    }
                }

                if glas {
                    if let Some(ref rvs) = tn0.reverce_variants {
                        for mvref in rvs {
                            if let Some(mv) = self.get_rule_var(mvref.rule_id, mvref.variant_id) {
                                if !mv.base.class.is_verb() && !mv.base.class.is_adjective()
                                    && !mv.base.class.is_noun() && !mv.base.class.is_proper_surname()
                                    && !mv.base.class.is_proper_geo() && !mv.base.class.is_proper_secname()
                                {
                                    continue;
                                }

                                if let Some(ref r) = res {
                                    let ok = r.iter().any(|rr| {
                                        rr.is_in_dictionary() && (rr.base.class == mv.base.class || rr.base.class.is_noun() || (!mv.base.class.is_adjective() && rr.base.class.is_verb()))
                                    });
                                    if ok { continue; }
                                }

                                if !mv.tail.is_empty() && !LanguageHelper::ends_with(word, &mv.tail) {
                                    continue;
                                }

                                let mi = self.get_misc_info(mv.misc_info_id).cloned().unwrap_or_default();
                                let mut r_wf = MorphWordForm::from_rule_variant(mv, word, mi);

                                if res.is_none() {
                                    res = Some(Vec::new());
                                }
                                if !r_wf.has_morph_equals(res.as_mut().unwrap()) {
                                    r_wf.undef_coef = mvref.coef;
                                    res.as_mut().unwrap().push(r_wf);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Special case for "ПРИ"
        if word == "ПРИ" {
            if let Some(ref mut r) = res {
                r.retain(|wf| !wf.base.class.is_proper_geo());
            }
        }

        // Check if result is empty
        if let Some(ref r) = res {
            if r.is_empty() { return None; }
        } else {
            return None;
        }

        // Sort and finalize
        if let Some(ref mut r) = res {
            self.sort(r, word);
            for v in r.iter_mut() {
                if v.normal_case.is_none() {
                    v.normal_case = Some(word.to_string());
                }
                if v.base.class.is_verb() {
                    if v.normal_full.is_none() {
                        if let Some(ref nc) = v.normal_case {
                            if LanguageHelper::ends_with(nc, "ТЬСЯ") {
                                v.normal_full = Some(nc[..nc.len() - 2].to_string());
                            }
                        }
                    }
                }
                v.base.language = self.language;
                if v.base.class.is_preposition() {
                    if let Some(ref nc) = v.normal_case {
                        v.normal_case = Some(LanguageHelper::normalize_preposition(nc));
                    }
                }
            }

            // Remove suspicious unknown adjectives
            let mut mc = MorphClass::new();
            let mut to_remove = Vec::new();
            for i in (0..r.len()).rev() {
                if !r[i].is_in_dictionary() && r[i].base.class.is_adjective() && r.len() > 1 {
                    if let Some(ref misc) = r[i].misc {
                        if misc.attrs.iter().any(|a| a == "к.ф." || a == "неизм.") {
                            to_remove.push(i);
                            continue;
                        }
                    }
                }
                if r[i].is_in_dictionary() {
                    mc.value |= r[i].base.class.value;
                }
            }
            for i in to_remove {
                r.remove(i);
            }

            // Special verb+adjective handling
            if mc == MorphClass::VERB && r.len() > 1 {
                for wf in r.iter_mut() {
                    if wf.undef_coef > 100 && wf.base.class == MorphClass::ADJECTIVE {
                        wf.undef_coef = 0;
                    }
                }
            }

            if r.is_empty() {
                return None;
            }
        }

        res
    }

    fn process_result(&self, res: &mut Vec<MorphWordForm>, word_begin: &str, mvs: &[MorphRuleVariant]) {
        let mut buf = String::new();
        for mv in mvs {
            let mi = self.get_misc_info(mv.misc_info_id).cloned().unwrap_or_default();
            let mut r = MorphWordForm::from_rule_variant(mv, "", mi);

            // Construct normal_case from word_begin + normal_tail
            if let Some(ref nt) = mv.normal_tail {
                if !nt.is_empty() && !nt.starts_with('-') {
                    buf.clear();
                    buf.push_str(word_begin);
                    buf.push_str(nt);
                    r.normal_case = Some(buf.clone());
                } else {
                    r.normal_case = Some(word_begin.to_string());
                }
            } else {
                r.normal_case = Some(word_begin.to_string());
            }

            if let Some(ref fnt) = mv.full_normal_tail {
                if !fnt.is_empty() && !fnt.starts_with('-') {
                    buf.clear();
                    buf.push_str(word_begin);
                    buf.push_str(fnt);
                    r.normal_full = Some(buf.clone());
                } else {
                    r.normal_full = Some(word_begin.to_string());
                }
            }

            if !r.has_morph_equals(res) {
                r.undef_coef = 0;
                res.push(r);
            }
        }
    }

    fn compare(&self, x: &MorphWordForm, y: &MorphWordForm) -> i32 {
        if x.is_in_dictionary() && !y.is_in_dictionary() { return -1; }
        if !x.is_in_dictionary() && y.is_in_dictionary() { return 1; }
        if x.undef_coef > 0 {
            if x.undef_coef > y.undef_coef * 2 { return -1; }
            if x.undef_coef * 2 < y.undef_coef { return 1; }
        }
        if x.base.class != y.base.class {
            if x.base.class.is_preposition() || x.base.class.is_conjunction() || x.base.class.is_pronoun() || x.base.class.is_personal_pronoun() {
                return -1;
            }
            if y.base.class.is_preposition() || y.base.class.is_conjunction() || y.base.class.is_pronoun() || y.base.class.is_personal_pronoun() {
                return 1;
            }
            if x.base.class.is_verb() { return 1; }
            if y.base.class.is_verb() { return -1; }
            if x.base.class.is_noun() { return -1; }
            if y.base.class.is_noun() { return 1; }
        }
        let cx = self.calc_coef(x);
        let cy = self.calc_coef(y);
        if cx > cy { return -1; }
        if cx < cy { return 1; }
        if x.base.number == MorphNumber::PLURAL && y.base.number != MorphNumber::PLURAL { return 1; }
        if y.base.number == MorphNumber::PLURAL && x.base.number != MorphNumber::PLURAL { return -1; }
        0
    }

    fn calc_coef(&self, wf: &MorphWordForm) -> i32 {
        let mut k = 0;
        if !wf.base.case.is_undefined() { k += 1; }
        if wf.base.gender != MorphGenderFlags::UNDEFINED { k += 1; }
        if wf.base.number != MorphNumber::UNDEFINED { k += 1; }
        if let Some(ref misc) = wf.misc {
            if misc.is_synonym_form() { k -= 3; }
        }
        if wf.normal_case.is_none() || wf.normal_case.as_ref().map_or(true, |nc| nc.len() < 4) {
            return k;
        }
        let nc = wf.normal_case.as_ref().unwrap();

        if wf.base.class.is_adjective() {
            // Get last two chars without Vec<char> allocation
            let mut last1 = '\0';
            let mut last = '\0';
            let mut count = 0u32;
            for ch in nc.chars() {
                last1 = last;
                last = ch;
                count += 1;
            }
            if count >= 2 {
                if wf.base.number != MorphNumber::PLURAL {
                    let mut ok = false;
                    if wf.base.gender == MorphGenderFlags::FEMINIE && last == 'Я' { ok = true; }
                    if wf.base.gender == MorphGenderFlags::MASCULINE {
                        if last == 'Й' {
                            if last1 == 'И' { k += 1; }
                            ok = true;
                        }
                    }
                    if wf.base.gender == MorphGenderFlags::NEUTER && last == 'Е' { ok = true; }
                    if ok && LanguageHelper::is_cyrillic_vowel(last1) { k += 1; }
                } else {
                    if last == 'Й' || last == 'Е' { k += 1; }
                }
            }
        }
        k
    }

    fn sort(&self, res: &mut Vec<MorphWordForm>, _word: &str) {
        if res.len() < 2 { return; }

        res.sort_by(|a, b| {
            let c = self.compare(a, b);
            if c < 0 { std::cmp::Ordering::Less }
            else if c > 0 { std::cmp::Ordering::Greater }
            else { std::cmp::Ordering::Equal }
        });

        // Remove duplicates
        let mut i = 0;
        while i < res.len().saturating_sub(1) {
            let mut j = i + 1;
            while j < res.len() {
                if self.comp1(&res[i], &res[j]) {
                    if res[i].base.class.is_adjective() && res[j].base.class.is_noun() && !res[j].is_in_dictionary() && !res[i].is_in_dictionary() {
                        res.remove(j);
                    } else if res[i].base.class.is_noun() && res[j].base.class.is_adjective() && !res[j].is_in_dictionary() && !res[i].is_in_dictionary() {
                        res.remove(i);
                        if i > 0 { i -= 1; }
                        break;
                    } else if res[i].base.class.is_adjective() && res[j].base.class.is_pronoun() {
                        res.remove(i);
                        if i > 0 { i -= 1; }
                        break;
                    } else if res[i].base.class.is_pronoun() && res[j].base.class.is_adjective() {
                        if res[j].normal_full.as_deref() == Some("ОДИН") || res[j].normal_case.as_deref() == Some("ОДИН") {
                            j += 1;
                            continue;
                        }
                        res.remove(j);
                    } else {
                        j += 1;
                        continue;
                    }
                    continue;
                }
                j += 1;
            }
            i += 1;
        }
    }

    fn comp1(&self, r1: &MorphWordForm, r2: &MorphWordForm) -> bool {
        r1.base.number == r2.base.number
            && r1.base.gender == r2.base.gender
            && r1.base.case == r2.base.case
            && r1.normal_case == r2.normal_case
    }

    pub fn deserialize(&mut self, data: &[u8], ignore_rev_tree: bool, lazy_load: bool) {
        let arr = MorphDeserializer::deflate_gzip(data);
        let buf = ByteArrayWrapper::new(&arr);
        let mut pos = 0;

        // Read misc infos
        let cou = buf.deserialize_int(&mut pos);
        for _ in 0..cou {
            let mut mi = MorphMiscInfo::new();
            mi.deserialize(&arr, &mut pos);
            self.add_misc_info(mi);
        }

        // Read rules
        let cou = buf.deserialize_int(&mut pos);
        for _ in 0..cou {
            let p1 = buf.deserialize_int(&mut pos) as usize;
            let mut r = MorphRule::new();
            if lazy_load {
                r.lazy_pos = pos;
                pos = p1;
            } else {
                r.deserialize(&buf, &mut pos);
            }
            self.add_rule(r);
        }

        // Read forward tree
        let mut root = MorphTreeNode::new();
        if lazy_load {
            root.deserialize_lazy(&buf, self, &mut pos);
        } else {
            root.deserialize(&buf, &mut pos);
        }
        self.m_root = root;

        // Read reverse tree
        if !ignore_rev_tree {
            let mut root_rev = MorphTreeNode::new();
            if lazy_load {
                root_rev.deserialize_lazy(&buf, self, &mut pos);
            } else {
                root_rev.deserialize(&buf, &mut pos);
            }
            self.m_root_reverce = root_rev;
        }

        if lazy_load {
            self.m_lazy_buf = Some(arr);
        }
    }
}
