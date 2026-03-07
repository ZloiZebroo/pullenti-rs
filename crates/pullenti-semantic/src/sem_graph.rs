/// SemObject, SemLink, SemGraph — core graph types.
/// Mirrors SemObject.cs, SemLink.cs, SemGraph.cs.

use std::rc::Rc;
use std::cell::RefCell;
use pullenti_morph::{MorphGenderFlags, MorphNumber};
use crate::types::{SemObjectType, SemLinkType, SemAttribute};

// ── SemQuantity ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct SemQuantity {
    pub spelling:    String,
    pub begin_char:  usize,
    pub end_char:    usize,
}

impl std::fmt::Display for SemQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.spelling)
    }
}

// ── Forward declarations ───────────────────────────────────────────────────

pub type SemObjectRef = Rc<RefCell<SemObject>>;
pub type SemLinkRef   = Rc<RefCell<SemLink>>;

// ── SemObject ─────────────────────────────────────────────────────────────

pub struct SemObject {
    pub typ:        SemObjectType,
    pub normal:     String,   // normalized text (NormalCase)
    pub normal_full: String,  // NormalFull
    pub attrs:      Vec<SemAttribute>,
    pub quantity:   Option<SemQuantity>,
    pub not:        bool,
    /// Morphological gender (for AnaforHelper matching)
    pub gender:     MorphGenderFlags,
    /// Morphological number (for AnaforHelper matching)
    pub number:     MorphNumber,
    /// Token span in source text
    pub begin_char: usize,
    pub end_char:   usize,
    /// Outgoing links (Source)
    pub links_from: Vec<SemLinkRef>,
    /// Incoming links (Target)
    pub links_to:   Vec<SemLinkRef>,
    /// Arbitrary user tag
    pub tag:        Option<Box<dyn std::any::Any>>,
}

impl SemObject {
    pub fn new() -> Self {
        SemObject {
            typ:        SemObjectType::Undefined,
            normal:     String::new(),
            normal_full: String::new(),
            attrs:      Vec::new(),
            quantity:   None,
            not:        false,
            gender:     MorphGenderFlags::UNDEFINED,
            number:     MorphNumber::UNDEFINED,
            begin_char: 0,
            end_char:   0,
            links_from: Vec::new(),
            links_to:   Vec::new(),
            tag:        None,
        }
    }

    /// Check if normal_full matches the given text (case-insensitive).
    pub fn is_value(&self, text: &str, typ: SemObjectType) -> bool {
        (typ == SemObjectType::Undefined || self.typ == typ)
            && self.normal_full.eq_ignore_ascii_case(text)
    }

    /// Find a SemObject in LinksFrom chain matching text/link_type/obj_type.
    pub fn find_from_object(
        &self,
        text:      &str,
        link_type: SemLinkType,
        obj_type:  SemObjectType,
    ) -> Option<SemObjectRef> {
        for li in &self.links_from {
            let lb = li.borrow();
            if link_type != SemLinkType::Undefined && lb.typ != link_type { continue; }
            let tgt = lb.target.borrow();
            if obj_type != SemObjectType::Undefined && tgt.typ != obj_type { continue; }
            if tgt.normal_full.eq_ignore_ascii_case(text) {
                return Some(lb.target.clone());
            }
        }
        None
    }

    pub fn compare_to(&self, other: &SemObject) -> std::cmp::Ordering {
        self.begin_char.cmp(&other.begin_char)
            .then(self.end_char.cmp(&other.end_char))
    }
}

impl std::fmt::Display for SemObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.not { write!(f, "НЕ ")?; }
        write!(f, "{}", self.normal)
    }
}

// ── SemLink ───────────────────────────────────────────────────────────────

pub struct SemLink {
    pub typ:        SemLinkType,
    pub source:     SemObjectRef,
    pub target:     SemObjectRef,
    pub question:   Option<String>,
    pub preposition: Option<String>,
    pub is_or:      bool,
    pub alt_link:   Option<SemLinkRef>,
}

impl SemLink {
    pub fn new(typ: SemLinkType, src: SemObjectRef, tgt: SemObjectRef) -> SemLinkRef {
        let link = Rc::new(RefCell::new(SemLink {
            typ,
            source:     src.clone(),
            target:     tgt.clone(),
            question:   None,
            preposition: None,
            is_or:      false,
            alt_link:   None,
        }));
        src.borrow_mut().links_from.push(link.clone());
        tgt.borrow_mut().links_to.push(link.clone());
        link
    }
}

impl std::fmt::Display for SemLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.alt_link.is_some() { write!(f, "??? ")?; }
        if self.is_or { write!(f, "OR ")?; }
        write!(f, "{:?}", self.typ)?;
        if let Some(ref q) = self.question { write!(f, " {}?", q)?; }
        write!(f, " {} -> {}", self.source.borrow(), self.target.borrow())
    }
}

// ── SemGraph ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct SemGraph {
    pub objects: Vec<SemObjectRef>,
    pub links:   Vec<SemLinkRef>,
}

impl SemGraph {
    pub fn new() -> Self { Self::default() }

    pub fn add_object(&mut self, obj: SemObject) -> SemObjectRef {
        let r = Rc::new(RefCell::new(obj));
        self.objects.push(r.clone());
        r
    }

    pub fn add_link(
        &mut self,
        typ: SemLinkType,
        src: SemObjectRef,
        tgt: SemObjectRef,
        question: Option<String>,
        is_or: bool,
        prep: Option<String>,
    ) -> Option<SemLinkRef> {
        // Dedup check
        for li in &self.links {
            let lb = li.borrow();
            if lb.typ == typ && Rc::ptr_eq(&lb.source, &src) && Rc::ptr_eq(&lb.target, &tgt) {
                return Some(li.clone());
            }
        }
        let link = SemLink::new(typ, src, tgt);
        {
            let mut lb = link.borrow_mut();
            lb.question   = question;
            lb.is_or      = is_or;
            lb.preposition = prep;
        }
        self.links.push(link.clone());
        Some(link)
    }

    pub fn remove_link(&mut self, li: &SemLinkRef) {
        self.links.retain(|l| !Rc::ptr_eq(l, li));
        let lb = li.borrow();
        lb.source.borrow_mut().links_from.retain(|l| !Rc::ptr_eq(l, li));
        lb.target.borrow_mut().links_to.retain(|l| !Rc::ptr_eq(l, li));
    }

    pub fn remove_object(&mut self, obj: &SemObjectRef) {
        let links_from: Vec<_> = obj.borrow().links_from.clone();
        for li in &links_from {
            let lb = li.borrow();
            lb.target.borrow_mut().links_to.retain(|l| !Rc::ptr_eq(l, li));
            self.links.retain(|l| !Rc::ptr_eq(l, li));
        }
        let links_to: Vec<_> = obj.borrow().links_to.clone();
        for li in &links_to {
            let lb = li.borrow();
            lb.source.borrow_mut().links_from.retain(|l| !Rc::ptr_eq(l, li));
            self.links.retain(|l| !Rc::ptr_eq(l, li));
        }
        self.objects.retain(|o| !Rc::ptr_eq(o, obj));
    }

    pub fn merge_with(&mut self, other: &mut SemGraph) {
        for obj in other.objects.drain(..) {
            if !self.objects.iter().any(|o| Rc::ptr_eq(o, &obj)) {
                self.objects.push(obj);
            }
        }
        for link in other.links.drain(..) {
            if !self.links.iter().any(|l| Rc::ptr_eq(l, &link)) {
                self.links.push(link);
            }
        }
    }
}

impl std::fmt::Display for SemGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}obj {}links", self.objects.len(), self.links.len())
    }
}
