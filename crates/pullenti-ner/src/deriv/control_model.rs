/// ControlModel + supporting types.
/// Mirrors: SemanticRole.cs, QuestionType.cs, ControlModelItemType.cs,
///          ControlModelQuestion.cs, ControlModelItem.cs, ControlModel.cs

use std::collections::HashMap;
use std::sync::OnceLock;
use pullenti_morph::{MorphCase, MorphLang};
use pullenti_morph::internal::byte_array_wrapper::ByteArrayWrapper;

// ── SemanticRole ──────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticRole {
    Common  = 0,
    Agent   = 1,
    Pacient = 2,
    Strong  = 3,
}

impl SemanticRole {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => SemanticRole::Agent,
            2 => SemanticRole::Pacient,
            3 => SemanticRole::Strong,
            _ => SemanticRole::Common,
        }
    }
}

// ── QuestionType ──────────────────────────────────────────────────────────

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuestionType {
    Undefined = 0,
    Where     = 1,
    WhereFrom = 2,
    WhereTo   = 4,
    When      = 8,
    WhatToDo  = 0x10,
}

// ── ControlModelItemType ──────────────────────────────────────────────────

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlModelItemType {
    Undefined = 0,
    Word      = 1,
    Verb      = 2,
    Reflexive = 3,
    Noun      = 4,
}

impl ControlModelItemType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Word,
            2 => Self::Verb,
            3 => Self::Reflexive,
            4 => Self::Noun,
            _ => Self::Undefined,
        }
    }
}

// ── ControlModelQuestion ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ControlModelQuestion {
    pub question:    QuestionType,
    pub preposition: Option<String>,
    pub case:        MorphCase,
    pub spelling:    String,
    pub spelling_ex: String,
    pub id:          usize,
    pub is_base:     bool,
    pub is_abstract: bool,
}

impl ControlModelQuestion {
    fn new(prep: Option<&str>, case: MorphCase, typ: QuestionType) -> Self {
        let mut spelling = String::new();
        let mut spelling_ex = String::new();
        if let Some(p) = prep {
            let pl = p.to_lowercase();
            spelling = if case.is_genitive()     { format!("{} чего",  pl) }
                  else if case.is_dative()       { format!("{} чему",  pl) }
                  else if case.is_accusative()   { format!("{} что",   pl) }
                  else if case.is_instrumental() { format!("{} чем",   pl) }
                  else if case.is_prepositional(){ format!("{} чём",   pl) }
                  else                           { pl.clone() };
            spelling_ex = spelling.clone();
            match typ {
                QuestionType::When      => { spelling_ex = format!("{}/когда",  spelling); }
                QuestionType::Where     => { spelling_ex = format!("{}/где",    spelling); }
                QuestionType::WhereFrom => { spelling_ex = format!("{}/откуда", spelling); }
                QuestionType::WhereTo   => { spelling_ex = format!("{}/куда",   spelling); }
                _ => {}
            }
        } else if !case.is_undefined() {
            if case.is_nominative() {
                spelling    = "кто".into();
                spelling_ex = "кто/что".into();
            } else if case.is_genitive() {
                spelling    = "чего".into();
                spelling_ex = "кого/чего".into();
            } else if case.is_dative() {
                spelling    = "чему".into();
                spelling_ex = "кому/чему".into();
            } else if case.is_accusative() {
                spelling    = "что".into();
                spelling_ex = "кого/что".into();
            } else if case.is_instrumental() {
                spelling    = "чем".into();
                spelling_ex = "кем/чем".into();
            }
        } else {
            match typ {
                QuestionType::WhatToDo  => { spelling = "что делать".into(); spelling_ex = spelling.clone(); }
                QuestionType::When      => { spelling = "когда".into();      spelling_ex = spelling.clone(); }
                QuestionType::Where     => { spelling = "где".into();        spelling_ex = spelling.clone(); }
                QuestionType::WhereFrom => { spelling = "откуда".into();     spelling_ex = spelling.clone(); }
                QuestionType::WhereTo   => { spelling = "куда".into();       spelling_ex = spelling.clone(); }
                _ => {}
            }
        }
        ControlModelQuestion {
            question:    typ,
            preposition: prep.map(|s| s.to_string()),
            case,
            spelling,
            spelling_ex,
            id:          0,
            is_base:     false,
            is_abstract: false,
        }
    }

    pub fn check(&self, prep: Option<&str>, cas: MorphCase) -> bool {
        if self.is_abstract {
            return items().iter().any(|it| {
                !it.is_abstract && it.question == self.question && it.check(prep, cas)
            });
        }
        let case_match = (cas & self.case);
        if case_match.is_undefined() {
            if self.preposition.as_deref() == Some("В") && prep == Some("В") {
                if self.case.is_accusative() && (cas.is_undefined() || cas.is_nominative()) {
                    return true;
                }
            }
            return false;
        }
        if let (Some(p), Some(sp)) = (prep, &self.preposition) {
            if p == sp { return true; }
            if sp == "ОТ" && p == "ОТ ИМЕНИ" { return true; }
        }
        prep.is_none() && self.preposition.is_none()
    }
}

// ── Global items list ──────────────────────────────────────────────────────

static ITEMS: OnceLock<Vec<ControlModelQuestion>> = OnceLock::new();
static HASH_BY_SPEL: OnceLock<HashMap<String, usize>> = OnceLock::new();

// Indices of well-known questions
pub const IDX_BASE_NOM:    usize = 0;
pub const IDX_BASE_GEN:    usize = 1;
pub const IDX_BASE_ACC:    usize = 2;
pub const IDX_BASE_INS:    usize = 3;
pub const IDX_BASE_DAT:    usize = 4;
pub const IDX_TODO:        usize = 5;

pub fn items() -> &'static Vec<ControlModelQuestion> {
    ITEMS.get_or_init(|| {
        let mut list: Vec<ControlModelQuestion> = Vec::new();

        let g = MorphCase::GENITIVE;
        let d = MorphCase::DATIVE;
        let a = MorphCase::ACCUSATIVE;
        let ins = MorphCase::INSTRUMENTAL;
        let p = MorphCase::PREPOSITIONAL;

        // WhereFrom
        for prep in &["ИЗ", "ОТ", "С", "ИЗНУТРИ"] {
            list.push(ControlModelQuestion::new(Some(prep), g, QuestionType::WhereFrom));
        }
        // WhereTo
        list.push(ControlModelQuestion::new(Some("В"),       a, QuestionType::WhereTo));
        list.push(ControlModelQuestion::new(Some("НА"),      a, QuestionType::WhereTo));
        list.push(ControlModelQuestion::new(Some("ПО"),      a, QuestionType::WhereTo));
        list.push(ControlModelQuestion::new(Some("К"),       d, QuestionType::WhereTo));
        list.push(ControlModelQuestion::new(Some("НАВСТРЕЧУ"), d, QuestionType::WhereTo));
        list.push(ControlModelQuestion::new(Some("ДО"),      g, QuestionType::WhereTo));
        // Where (genitive)
        for prep in &["У","ОКОЛО","ВОКРУГ","ВОЗЛЕ","ВБЛИЗИ","МИМО","ПОЗАДИ","ВПЕРЕДИ",
                       "ВГЛУБЬ","ВДОЛЬ","ВНЕ","КРОМЕ","МЕЖДУ","НАПРОТИВ","ПОВЕРХ",
                       "ПОДЛЕ","ПОПЕРЕК","ПОСЕРЕДИНЕ","СВЕРХ","СРЕДИ","СНАРУЖИ","ВНУТРИ"] {
            list.push(ControlModelQuestion::new(Some(prep), g, QuestionType::Where));
        }
        // Where (dative)
        for prep in &["ПАРАЛЛЕЛЬНО"] {
            list.push(ControlModelQuestion::new(Some(prep), d, QuestionType::Where));
        }
        // Where (accusative)
        for prep in &["СКВОЗЬ","ЧЕРЕЗ","ПОД"] {
            list.push(ControlModelQuestion::new(Some(prep), a, QuestionType::Where));
        }
        // Where (instrumental)
        for prep in &["МЕЖДУ","НАД","ПОД","ПЕРЕД","ЗА"] {
            list.push(ControlModelQuestion::new(Some(prep), ins, QuestionType::Where));
        }
        // Where (prepositional)
        for prep in &["В","НА","ПРИ"] {
            list.push(ControlModelQuestion::new(Some(prep), p, QuestionType::Where));
        }
        // When
        list.push(ControlModelQuestion::new(Some("ПРЕЖДЕ"),  g, QuestionType::When));
        list.push(ControlModelQuestion::new(Some("ПОСЛЕ"),   g, QuestionType::When));
        list.push(ControlModelQuestion::new(Some("НАКАНУНЕ"),g, QuestionType::When));
        list.push(ControlModelQuestion::new(Some("СПУСТЯ"),  a, QuestionType::When));
        // Genitive plain
        for prep in &["БЕЗ","ДЛЯ","РАДИ","ИЗЗА","ВВИДУ","ВЗАМЕН","ВМЕСТО","ПРОТИВ",
                       "СВЫШЕ","ВСЛЕДСТВИЕ","ПОМИМО","ПОСРЕДСТВОМ","ПУТЕМ"] {
            list.push(ControlModelQuestion::new(Some(prep), g, QuestionType::Undefined));
        }
        // Dative plain
        for prep in &["ПО","ПОДОБНО","СОГЛАСНО","СООТВЕТСТВЕННО","СОРАЗМЕРНО","ВОПРЕКИ"] {
            list.push(ControlModelQuestion::new(Some(prep), d, QuestionType::Undefined));
        }
        // Accusative plain
        for prep in &["ПРО","О","ЗА","ВКЛЮЧАЯ","С"] {
            list.push(ControlModelQuestion::new(Some(prep), a, QuestionType::Undefined));
        }
        // Instrumental plain
        for prep in &["С"] {
            list.push(ControlModelQuestion::new(Some(prep), ins, QuestionType::Undefined));
        }
        // Prepositional plain
        for prep in &["О","ПО"] {
            list.push(ControlModelQuestion::new(Some(prep), p, QuestionType::Undefined));
        }

        // Bubble sort by (preposition, case rank) — matches C# CompareTo
        let n = list.len();
        for _ in 0..n {
            for j in 0..n-1 {
                if cmp_question(&list[j], &list[j+1]) > 0 {
                    list.swap(j, j+1);
                }
            }
        }

        // Insert the 10 "base" / abstract questions at the front
        list.insert(IDX_BASE_NOM, ControlModelQuestion { is_base: true,     ..ControlModelQuestion::new(None, MorphCase::NOMINATIVE,  QuestionType::Undefined) });
        list.insert(IDX_BASE_GEN, ControlModelQuestion { is_base: true,     ..ControlModelQuestion::new(None, MorphCase::GENITIVE,    QuestionType::Undefined) });
        list.insert(IDX_BASE_ACC, ControlModelQuestion { is_base: true,     ..ControlModelQuestion::new(None, MorphCase::ACCUSATIVE,  QuestionType::Undefined) });
        list.insert(IDX_BASE_INS, ControlModelQuestion { is_base: true,     ..ControlModelQuestion::new(None, MorphCase::INSTRUMENTAL,QuestionType::Undefined) });
        list.insert(IDX_BASE_DAT, ControlModelQuestion { is_base: true,     ..ControlModelQuestion::new(None, MorphCase::DATIVE,      QuestionType::Undefined) });
        list.insert(IDX_TODO,     ControlModelQuestion::new(None, MorphCase::UNDEFINED, QuestionType::WhatToDo));
        list.insert(6, ControlModelQuestion { is_abstract: true, ..ControlModelQuestion::new(None, MorphCase::UNDEFINED, QuestionType::Where)     });
        list.insert(7, ControlModelQuestion { is_abstract: true, ..ControlModelQuestion::new(None, MorphCase::UNDEFINED, QuestionType::WhereTo)   });
        list.insert(8, ControlModelQuestion { is_abstract: true, ..ControlModelQuestion::new(None, MorphCase::UNDEFINED, QuestionType::WhereFrom) });
        list.insert(9, ControlModelQuestion { is_abstract: true, ..ControlModelQuestion::new(None, MorphCase::UNDEFINED, QuestionType::When)      });

        // Assign ids (1-based)
        let mut out = list;
        for (i, it) in out.iter_mut().enumerate() {
            it.id = i + 1;
        }
        out
    })
}

fn cmp_question(a: &ControlModelQuestion, b: &ControlModelQuestion) -> i32 {
    let pa = a.preposition.as_deref().unwrap_or("");
    let pb = b.preposition.as_deref().unwrap_or("");
    let c = pa.cmp(pb);
    if c != std::cmp::Ordering::Equal {
        return if c == std::cmp::Ordering::Less { -1 } else { 1 };
    }
    let ra = case_rank(a.case);
    let rb = case_rank(b.case);
    ra.cmp(&rb) as i32
}

fn case_rank(c: MorphCase) -> i32 {
    if c.is_genitive()      { 1 }
    else if c.is_dative()   { 2 }
    else if c.is_accusative()   { 3 }
    else if c.is_instrumental() { 4 }
    else if c.is_prepositional(){ 5 }
    else                        { 0 }
}

pub fn get_by_id(id: usize) -> Option<&'static ControlModelQuestion> {
    let list = items();
    if id >= 1 && id <= list.len() { Some(&list[id - 1]) } else { None }
}

pub fn find_by_spel(spel: &str) -> Option<&'static ControlModelQuestion> {
    let map = HASH_BY_SPEL.get_or_init(|| {
        let list = items();
        list.iter().enumerate().map(|(i, it)| (it.spelling.clone(), i)).collect()
    });
    map.get(spel).map(|&i| &items()[i])
}

// ── ControlModelItem ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ControlModelItem {
    pub typ:   ControlModelItemType,
    pub word:  Option<String>,
    /// question index (0-based into items()) → role
    pub links: HashMap<usize, SemanticRole>,
    pub nominative_can_be_agent_and_pacient: bool,
    pub ignorable: bool,
}

impl ControlModelItem {
    pub fn new() -> Self {
        ControlModelItem {
            typ: ControlModelItemType::Word,
            word: None,
            links: HashMap::new(),
            nominative_can_be_agent_and_pacient: false,
            ignorable: false,
        }
    }
}

// ── ControlModel ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct ControlModel {
    pub items:    Vec<ControlModelItem>,
    pub pacients: Vec<String>,
}

impl ControlModel {
    pub fn new() -> Self { Self::default() }

    pub fn find_item_by_typ(&self, typ: ControlModelItemType) -> Option<&ControlModelItem> {
        self.items.iter().find(|it| it.typ == typ)
    }

    pub fn deserialize(&mut self, buf: &ByteArrayWrapper, pos: &mut usize) {
        let mut cou = buf.deserialize_short(pos) as i32;
        while cou > 0 {
            cou -= 1;
            let mut it = ControlModelItem::new();
            let b = buf.deserialize_byte(pos);
            if (b & 0x80) != 0 { it.nominative_can_be_agent_and_pacient = true; }
            it.typ = ControlModelItemType::from_u8(b & 0x7F);
            if it.typ == ControlModelItemType::Word {
                it.word = Some(buf.deserialize_string(pos));
            }
            let mut licou = buf.deserialize_short(pos) as i32;
            while licou > 0 {
                licou -= 1;
                let qi = buf.deserialize_byte(pos) as usize; // question index (0-based)
                let role_b = buf.deserialize_byte(pos);
                let role = SemanticRole::from_u8(role_b);
                it.links.insert(qi, role);
            }
            self.items.push(it);
        }
        let mut pcou = buf.deserialize_short(pos) as i32;
        while pcou > 0 {
            pcou -= 1;
            let s = buf.deserialize_string(pos);
            if !s.is_empty() { self.pacients.push(s); }
        }
    }
}
