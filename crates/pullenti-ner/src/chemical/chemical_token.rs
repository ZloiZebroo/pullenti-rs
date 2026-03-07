/// ChemicalToken — mirrors `ChemicalToken.cs` + `ChemicalUnit.cs`.
///
/// Parses chemical formulas (H2O, CO2, NaCl, CH3OH) and named chemical substances.

use std::rc::Rc;
use std::collections::HashMap;
use std::sync::OnceLock;

use pullenti_morph::LanguageHelper;

use crate::token::{TokenRef, TokenKind, Token};
use crate::source_of_analysis::SourceOfAnalysis;
use crate::referent::Referent;
use super::chemical_referent as cr;

// ── Element table ──────────────────────────────────────────────────────────

struct ChemData {
    by_mnem:  HashMap<String, String>,  // mnem -> name_cyr
    termins:  Vec<String>,
    keywords: Vec<String>,
}

static CHEM_DATA: OnceLock<ChemData> = OnceLock::new();

fn data() -> &'static ChemData {
    CHEM_DATA.get_or_init(|| {
        // "Имя,Mnem" pairs for all 118 elements
        let elements = [
            ("ВОДОРОД","H"),("ГЕЛИЙ","He"),("ЛИТИЙ","Li"),("БЕРИЛЛИЙ","Be"),("БОР","B"),
            ("УГЛЕРОД","C"),("АЗОТ","N"),("КИСЛОРОД","O"),("ФТОР","F"),("НЕОН","Ne"),
            ("НАТРИЙ","Na"),("МАГНИЙ","Mg"),("АЛЮМИНИЙ","Al"),("КРЕМНИЙ","Si"),("ФОСФОР","P"),
            ("СЕРА","S"),("ХЛОР","Cl"),("АРГОН","Ar"),("КАЛИЙ","K"),("КАЛЬЦИЙ","Ca"),
            ("СКАНДИЙ","Sc"),("ТИТАН","Ti"),("ВАНАДИЙ","V"),("ХРОМ","Cr"),("МАРГАНЕЦ","Mn"),
            ("ЖЕЛЕЗО","Fe"),("КОБАЛЬТ","Co"),("НИКЕЛЬ","Ni"),("МЕДЬ","Cu"),("ЦИНК","Zn"),
            ("ГАЛЛИЙ","Ga"),("ГЕРМАНИЙ","Ge"),("МЫШЬЯК","As"),("СЕЛЕН","Se"),("БРОМ","Br"),
            ("КРИПТОН","Kr"),("РУБИДИЙ","Rb"),("СТРОНЦИЙ","Sr"),("ИТТРИЙ","Y"),("ЦИРКОНИЙ","Zr"),
            ("НИОБИЙ","Nb"),("МОЛИБДЕН","Mo"),("ТЕХНЕЦИЙ","Tc"),("РУТЕНИЙ","Ru"),("РОДИЙ","Rh"),
            ("ПАЛЛАДИЙ","Pd"),("СЕРЕБРО","Ag"),("КАДМИЙ","Cd"),("ИНДИЙ","In"),("ОЛОВО","Sn"),
            ("СУРЬМА","Sb"),("ТЕЛЛУР","Te"),("ИОД","I"),("КСЕНОН","Xe"),("ЦЕЗИЙ","Cs"),
            ("БАРИЙ","Ba"),("ЛАНТАН","La"),("ЦЕРИЙ","Ce"),("ПРАЗЕОДИМ","Pr"),("НЕОДИМ","Nd"),
            ("ПРОМЕТИЙ","Pm"),("САМАРИЙ","Sm"),("ЕВРОПИЙ","Eu"),("ГАДОЛИНИЙ","Gd"),("ТЕРБИЙ","Tb"),
            ("ДИСПРОЗИЙ","Dy"),("ГОЛЬМИЙ","Ho"),("ЭРБИЙ","Er"),("ТУЛИЙ","Tm"),("ИТТЕРБИЙ","Yb"),
            ("ЛЮТЕЦИЙ","Lu"),("ГАФНИЙ","Hf"),("ТАНТАЛ","Ta"),("ВОЛЬФРАМ","W"),("РЕНИЙ","Re"),
            ("ОСМИЙ","Os"),("ИРИДИЙ","Ir"),("ПЛАТИНА","Pt"),("ЗОЛОТО","Au"),("РТУТЬ","Hg"),
            ("ТАЛЛИЙ","Tl"),("СВИНЕЦ","Pb"),("ВИСМУТ","Bi"),("ПОЛОНИЙ","Po"),("АСТАТ","At"),
            ("РАДОН","Rn"),("ФРАНЦИЙ","Fr"),("РАДИЙ","Ra"),("АКТИНИЙ","Ac"),("ТОРИЙ","Th"),
            ("ПРОТАКТИНИЙ","Pa"),("УРАН","U"),("НЕПТУНИЙ","Np"),("ПЛУТОНИЙ","Pu"),("АМЕРИЦИЙ","Am"),
            ("КЮРИЙ","Cm"),("БЕРКЛИЙ","Bk"),("КАЛИФОРНИЙ","Cf"),("ЭЙНШТЕЙНИЙ","Es"),("ФЕРМИЙ","Fm"),
            ("МЕНДЕЛЕВИЙ","Md"),("НОБЕЛИЙ","No"),("ЛОУРЕНСИЙ","Lr"),("РЕЗЕРФОРДИЙ","Rf"),("ДУБНИЙ","Db"),
            ("СИБОРГИЙ","Sg"),("БОРИЙ","Bh"),("ХАССИЙ","Hs"),("МЕЙТНЕРИЙ","Mt"),("ДАРМШТАДТИЙ","Ds"),
            ("РЕНТГЕНИЙ","Rg"),("КОПЕРНИЦИЙ","Cn"),("НИХОНИЙ","Nh"),("ФЛЕРОВИЙ","Fl"),("МОСКОВИЙ","Mc"),
            ("ЛИВЕРМОРИЙ","Lv"),("ТЕННЕССИН","Ts"),("ОГАНЕСОН","Og"),
        ];
        let mut by_mnem = HashMap::new();
        for (name, mnem) in &elements {
            by_mnem.insert(mnem.to_string(), name.to_string());
        }

        let termins: Vec<String> = [
            "КИСЛОТА","РАСТВОР","СПИРТ","ВОДА","СОЛЬ","АММИАК","БУТАН","БЕНЗОЛ",
            "КЕРОСИН","АМИН","СКИПИДАР","ОКИСЬ","ГИДРИТ","АММОНИЙ","ПЕРЕКИСЬ","КАРБОНАТ",
        ].iter().map(|s| s.to_string()).collect();

        let keywords: Vec<String> = [
            "МЕТАЛЛ","ГАЗ","ТОПЛИВО","МОНОТОПЛИВО","СМЕСЬ","ХИМИЧЕСКИЙ","МОЛЕКУЛА",
            "АТОМ","МОЛЕКУЛЯРНЫЙ","АТОМАРНЫЙ",
        ].iter().map(|s| s.to_string()).collect();

        ChemData { by_mnem, termins, keywords }
    })
}

// ── ChemicalToken ──────────────────────────────────────────────────────────

pub struct ChemicalToken {
    pub begin_token: TokenRef,
    pub end_token:   TokenRef,
    /// Element mnemonics if formula component
    pub items:       Option<Vec<String>>,
    /// Textual name if named compound
    pub name:        Option<String>,
    /// Sub-tokens for bracketed groups
    pub subtokens:   Option<Vec<ChemicalToken>>,
    pub bracket:     char,
    pub num:         i32,
    pub is_doubt:    bool,
    pub hiphen_before: bool,
}

impl ChemicalToken {
    fn new(begin: TokenRef, end: TokenRef) -> Self {
        ChemicalToken {
            begin_token: begin, end_token: end,
            items: None, name: None, subtokens: None,
            bracket: '\0', num: 0, is_doubt: false, hiphen_before: false,
        }
    }

    /// Build the formula string representation
    pub fn to_formula_string(&self) -> String {
        let mut res = String::new();
        if self.hiphen_before { res.push('-'); }
        if let Some(ref n) = self.name { res.push_str(n); }
        if let Some(ref subs) = self.subtokens {
            res.push(self.bracket);
            for s in subs { res.push_str(&s.to_formula_string()); }
            res.push(if self.bracket == '[' { ']' } else { ')' });
        }
        if let Some(ref items) = self.items {
            if self.name.is_none() {
                for mnem in items {
                    let mut chars = mnem.chars();
                    if let Some(c) = chars.next() { res.push(c); }
                    if let Some(c) = chars.next() { res.push(c.to_lowercase().next().unwrap_or(c)); }
                }
            }
        }
        if self.num > 0 { res.push_str(&self.num.to_string()); }
        res
    }
}

// ── TryParseList ─────────────────────────────────────────────────────────────

pub fn try_parse_list(t: &TokenRef, sofa: &SourceOfAnalysis, lev: i32) -> Option<Vec<ChemicalToken>> {
    let mut res: Vec<ChemicalToken> = Vec::new();
    let mut cur = Some(t.clone());

    while let Some(tt) = cur.clone() {
        let item = try_parse_one(&tt, sofa, lev);

        if let Some(it) = item {
            // Formula items (not name) break at whitespace if prev was also formula
            if it.name.is_none() && !res.is_empty() {
                let last = res.last().unwrap();
                if last.name.is_none() && tt.borrow().is_whitespace_before(sofa) {
                    break;
                }
            }
            let end = it.end_token.clone();
            res.push(it);
            cur = end.borrow().next.clone();
            continue;
        }

        // "ИЛИ" between two text names
        if tt.borrow().is_value("ИЛИ", None) && res.len() == 1 && res[0].name.is_some() {
            if let Some(next_t) = tt.borrow().next.clone() {
                if let Some(ni) = try_parse_one(&next_t, sofa, lev) {
                    if ni.name.is_some() {
                        let end = ni.end_token.clone();
                        res.push(ni);
                        cur = end.borrow().next.clone();
                        continue;
                    }
                }
            }
        }
        break;
    }

    if res.is_empty() { None } else { Some(res) }
}

fn try_parse_one(t: &TokenRef, sofa: &SourceOfAnalysis, lev: i32) -> Option<ChemicalToken> {
    if lev > 3 { return None; }
    let tb = t.borrow();
    if !matches!(tb.kind, TokenKind::Text(_)) { return None; }

    let src = sofa.substring(tb.begin_char, tb.end_char);

    // Case 1: bracket group
    if src == "(" || src == "[" {
        drop(tb);
        let next = t.borrow().next.clone()?;
        let subs = try_parse_list(&next, sofa, lev + 1)?;
        let last_end = subs.last().unwrap().end_token.clone();
        let close = last_end.borrow().next.clone()?;
        let close_src = sofa.substring(close.borrow().begin_char, close.borrow().end_char);
        if close_src != ")" && close_src != "]" { return None; }
        let mut ct = ChemicalToken::new(t.clone(), close.clone());
        ct.bracket = src.chars().next().unwrap();
        ct.subtokens = Some(subs);
        add_num(&mut ct, sofa);
        return Some(ct);
    }

    // Case 2: hyphen
    let first_char = src.chars().next().unwrap_or('\0');
    if LanguageHelper::is_hiphen(first_char) && tb.length_char() == 1 {
        let ws_before = tb.is_whitespace_before(sofa);
        drop(tb);
        let next = t.borrow().next.clone()?;
        let ws_after = next.borrow().is_whitespace_before(sofa);
        let must_be_name = ws_before || ws_after;
        let mut sub = try_parse_one(&next, sofa, lev + 1)?;
        if must_be_name && sub.name.is_none() { return None; }
        sub.hiphen_before = true;
        sub.begin_token = t.clone();
        return Some(sub);
    }

    // Case 3: match element symbols
    if tb.chars.is_letter() && !tb.chars.is_all_lower() {
        let chars: Vec<char> = src.chars().collect();
        let d = data();
        let mut mnems: Vec<String> = Vec::new();
        let mut i = 0;
        let mut formula_ok = true;

        while i < chars.len() {
            let ch0 = chars[i];
            if ch0.is_lowercase() { formula_ok = false; break; }
            let lat0 = if ch0 as u32 > 0x80 {
                match LanguageHelper::get_lat_for_cyr(ch0) {
                    Some(c) => c,
                    None => { formula_ok = false; break; }
                }
            } else { ch0 };

            // Try 2-char mnemonic first
            let mnem2 = if i + 1 < chars.len() && chars[i+1].is_lowercase() {
                let ch1 = chars[i+1];
                let lat1 = if ch1 as u32 > 0x80 {
                    LanguageHelper::get_lat_for_cyr(ch1.to_uppercase().next().unwrap_or(ch1))
                        .unwrap_or('\0')
                } else { ch1.to_ascii_uppercase() };
                format!("{}{}", lat0, lat1)
            } else {
                String::new()
            };

            if !mnem2.is_empty() && d.by_mnem.contains_key(&mnem2) {
                i += 2; // consumed 2 chars
                mnems.push(mnem2);
            } else {
                let mnem1 = format!("{}", lat0);
                if d.by_mnem.contains_key(&mnem1) {
                    i += 1;
                    mnems.push(mnem1);
                } else {
                    formula_ok = false;
                    break;
                }
            }
        }

        if formula_ok && !mnems.is_empty() {
            drop(tb);
            let mut ct = ChemicalToken::new(t.clone(), t.clone());
            ct.items = Some(mnems.clone());
            add_num(&mut ct, sofa);
            // Single-element tokens may be doubtful
            if ct.num == 0 {
                let len = ct.begin_token.borrow().length_char();
                let in_dict = !ct.begin_token.borrow().morph.items().is_empty();
                if in_dict || len < 2 { ct.is_doubt = true; }
            }
            if mnems.len() > 6 { ct.is_doubt = true; }
            'outer: for ii in 0..mnems.len() {
                for jj in (ii+1)..mnems.len() {
                    if mnems[ii] == mnems[jj] { ct.is_doubt = true; break 'outer; }
                }
            }
            return Some(ct);
        }
    }

    // Case 4: textual substance name
    let val_upper = token_normal_upper(&tb);
    drop(tb);
    let d = data();
    let is_termin = d.termins.iter().any(|te| te == &val_upper);
    if is_termin || can_be_part_name(&val_upper) {
        // Scan forward for more name words
        let mut end_tok = t.clone();
        let mut cur = t.borrow().next.clone();
        while let Some(ct) = cur.clone() {
            let cv = {
                let ctb = ct.borrow();
                if !matches!(ctb.kind, TokenKind::Text(_)) { break; }
                token_normal_upper(&ctb)
            };
            if d.termins.iter().any(|te| te == &cv) || can_be_part_name(&cv) {
                end_tok = ct.clone();
                cur = ct.borrow().next.clone();
            } else {
                break;
            }
        }

        let name = build_name_str(t, &end_tok, sofa);
        let mut ct = ChemicalToken::new(t.clone(), end_tok.clone());
        ct.name = Some(name);

        // Single-token names are doubtful unless recognized
        if Rc::ptr_eq(t, &end_tok) {
            ct.is_doubt = true;
            if d.termins.iter().any(|te| te == &val_upper) {
                ct.is_doubt = false;
            }
            // Check "ФОРМУЛА" before
            if let Some(prev) = t.borrow().prev.as_ref().and_then(|w| w.upgrade()) {
                if prev.borrow().is_value("ФОРМУЛА", None) { ct.is_doubt = false; }
            }
        }
        return Some(ct);
    }

    None
}

fn token_normal_upper(tb: &std::cell::Ref<Token>) -> String {
    tb.morph.items().first()
        .and_then(|wf| wf.normal_case.as_deref().or(wf.normal_full.as_deref()))
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| {
            if let TokenKind::Text(ref td) = tb.kind { td.term.clone() }
            else { String::new() }
        })
}

fn build_name_str(t0: &TokenRef, t1: &TokenRef, sofa: &SourceOfAnalysis) -> String {
    let mut parts = Vec::new();
    let t1_end = t1.borrow().end_char;
    let mut cur = Some(t0.clone());
    while let Some(t) = cur.take() {
        if t.borrow().begin_char > t1_end { break; }
        let word = {
            let tb = t.borrow();
            tb.morph.items().first()
                .and_then(|wf| wf.normal_case.as_deref().or(wf.normal_full.as_deref()))
                .map(|s| s.to_uppercase())
                .unwrap_or_else(|| {
                    if let TokenKind::Text(ref td) = tb.kind { td.term.clone() }
                    else { sofa.substring(tb.begin_char, tb.end_char).to_uppercase() }
                })
        };
        parts.push(word);
        if t.borrow().end_char >= t1_end { break; }
        cur = t.borrow().next.clone();
    }
    parts.join(" ")
}

fn can_be_part_name(val: &str) -> bool {
    if val.ends_with("ИД") || val.ends_with("ОЛ") || val.ends_with("КИСЬ")
        || val.ends_with("ТАН") || val.ends_with("ОЗА") || val.ends_with("ИН")
        || val.ends_with("АТ") || val.ends_with("СИД") || val.starts_with("ДИ")
        || val.starts_with("ТРИ") || val.starts_with("ЧЕТЫР") { return true; }
    if val.starts_with("ГЕКСА") || val.starts_with("ДЕКА") || val.starts_with("ТЕТРА")
        || val.starts_with("ПЕНТА") || val.starts_with("ГЕПТА") || val.starts_with("ОКТА")
        || val.starts_with("НОНА") || val.starts_with("УНДЕКА") || val.starts_with("ДОДЕКА")
        || val.starts_with("ЭЙКОЗА") || val.starts_with("ГЕКТА") || val.starts_with("КИЛА")
        || val.starts_with("МИРИА") { return true; }
    if val.ends_with("КИСЬ") || val.ends_with("ТОРИД") || val.ends_with("ЦЕТАТ")
        || val.ends_with("РАЗИН") || val.ends_with("КСИД") || val.ends_with("ФИД")
        || val.ends_with("РИД") || val.ends_with("ОНАТ") { return true; }
    false
}

// ── AddNum ────────────────────────────────────────────────────────────────

fn add_num(ct: &mut ChemicalToken, sofa: &SourceOfAnalysis) {
    let next = match ct.end_token.borrow().next.clone() { Some(n) => n, None => return };
    if next.borrow().is_whitespace_before(sofa) { return; }

    let next_src = {
        let nb = next.borrow();
        sofa.substring(nb.begin_char, nb.end_char).to_string()
    };

    // Number token
    {
        let nb = next.borrow();
        if let TokenKind::Number(ref nd) = nb.kind {
            if let Ok(n) = nd.value.parse::<i32>() {
                drop(nb);
                ct.num = n;
                ct.end_token = next;
                return;
            }
        }
    }

    // Unicode subscript ₂–₉ (U+2082–U+2089)
    if next_src.chars().count() == 1 {
        let ch = next_src.chars().next().unwrap();
        let code = ch as u32;
        if code >= 0x2082 && code <= 0x2089 {
            ct.num = (code - 0x2080) as i32;
            ct.end_token = next;
            return;
        }
    }

    // <N> or [N] pattern
    if next_src == "<" || next_src == "[" {
        if let Some(num_tok) = next.borrow().next.clone() {
            let num_val = {
                let ntb = num_tok.borrow();
                if let TokenKind::Number(ref nd) = ntb.kind {
                    nd.value.parse::<i32>().ok()
                } else { None }
            };
            if let Some(n) = num_val {
                if let Some(close) = num_tok.borrow().next.clone() {
                    let close_src = sofa.substring(close.borrow().begin_char, close.borrow().end_char);
                    if close_src == ">" || close_src == "]" {
                        ct.num = n;
                        ct.end_token = close;
                    }
                }
            }
        }
    }
}

// ── CreateReferent ────────────────────────────────────────────────────────

pub fn create_referent(li: &[ChemicalToken], sofa: &SourceOfAnalysis) -> Option<Referent> {
    if li.len() == 1 {
        let tok = &li[0];
        if tok.is_doubt { return None; }
        if tok.bracket != '\0' { return None; }
        // Simple single-element or short formula needs context
        let single_elem = tok.items.as_ref().map_or(false, |items| items.len() == 1);
        let short = tok.end_token.borrow().end_char - tok.begin_token.borrow().begin_char + 1 < 5;
        if single_elem || short {
            if !has_chemical_context(&tok.begin_token, sofa) { return None; }
        }
    }

    let mut r = cr::new_chemical_referent();
    let mut formula = String::new();
    for tok in li {
        if let Some(ref n) = tok.name {
            cr::add_name(&mut r, n);
        } else {
            formula.push_str(&tok.to_formula_string());
        }
    }
    if !formula.is_empty() {
        cr::set_value(&mut r, &formula);
    }
    Some(r)
}

fn has_chemical_context(t: &TokenRef, sofa: &SourceOfAnalysis) -> bool {
    let d = data();
    // Scan backward up to 40 tokens
    let mut cur = t.borrow().prev.as_ref().and_then(|w| w.upgrade());
    for _ in 0..40 {
        let prev = match cur { Some(p) => p, None => break };
        {
            let pb = prev.borrow();
            if let TokenKind::Referent(ref rd) = pb.kind {
                if rd.referent.borrow().type_name == cr::OBJ_TYPENAME { return true; }
            }
            let val = token_normal_upper(&pb);
            if d.termins.iter().any(|te| te == &val) || d.keywords.iter().any(|k| k == &val) {
                return true;
            }
            cur = pb.prev.as_ref().and_then(|w| w.upgrade());
        }
    }
    // Scan forward up to 40 tokens
    let mut cur = t.borrow().next.clone();
    for _ in 0..40 {
        let next = match cur { Some(n) => n, None => break };
        {
            let nb = next.borrow();
            if let TokenKind::Referent(ref rd) = nb.kind {
                if rd.referent.borrow().type_name == cr::OBJ_TYPENAME { return true; }
            }
            let val = token_normal_upper(&nb);
            if d.termins.iter().any(|te| te == &val) || d.keywords.iter().any(|k| k == &val) {
                return true;
            }
            cur = nb.next.clone();
        }
    }
    false
}
