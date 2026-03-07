/// SemFragment, SemFraglink, SemBlock, SemDocument.
/// Mirrors SemFragment.cs, SemFraglink.cs, SemBlock.cs, SemDocument.cs.

use std::rc::Rc;
use std::cell::RefCell;
use pullenti_ner::token::TokenRef;
use crate::types::{SemFragmentType, SemFraglinkType};
use crate::types::SemObjectType;
use crate::sem_graph::SemGraph;

// ── SemFragment ───────────────────────────────────────────────────────────

pub type SemFragmentRef = Rc<RefCell<SemFragment>>;

pub struct SemFragment {
    pub graph:         SemGraph,
    pub typ:           SemFragmentType,
    pub begin_token:   Option<TokenRef>,
    pub end_token:     Option<TokenRef>,
    pub begin_char:    usize,
    pub end_char:      usize,
    pub is_or:         bool,
    pub rank:          f64,
    pub alt_fragments: Vec<SemFragmentRef>,
    pub tag:           Option<Box<dyn std::any::Any>>,
}

impl SemFragment {
    pub fn new() -> Self {
        SemFragment {
            graph:         SemGraph::new(),
            typ:           SemFragmentType::Undefined,
            begin_token:   None,
            end_token:     None,
            begin_char:    0,
            end_char:      0,
            is_or:         false,
            rank:          0.0,
            alt_fragments: Vec::new(),
            tag:           None,
        }
    }

    /// Return objects with no incoming links (root nodes / predicates)
    pub fn root_objects(&self) -> Vec<crate::sem_graph::SemObjectRef> {
        self.graph.objects.iter()
            .filter(|o| o.borrow().links_to.is_empty())
            .cloned()
            .collect()
    }

    pub fn can_be_error_structure(&self) -> bool {
        let mut cou = 0usize;
        let mut vcou = 0usize;
        for o in &self.graph.objects {
            let ob = o.borrow();
            if ob.links_to.is_empty() {
                if ob.typ == SemObjectType::Verb { vcou += 1; }
                cou += 1;
            }
        }
        cou > 1 && vcou < cou
    }
}

impl std::fmt::Display for SemFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}-{}]", self.begin_char, self.end_char)
    }
}

// ── SemFraglink ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SemFraglink {
    pub typ:      SemFraglinkType,
    pub source:   Option<SemFragmentRef>,
    pub target:   Option<SemFragmentRef>,
    pub question: Option<String>,
}

impl std::fmt::Display for SemFraglink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.typ)
    }
}

// ── SemBlock ──────────────────────────────────────────────────────────────

pub type SemBlockRef = Rc<RefCell<SemBlock>>;

pub struct SemBlock {
    pub graph:     SemGraph,
    pub fragments: Vec<SemFragmentRef>,
    pub links:     Vec<SemFraglink>,
}

impl SemBlock {
    pub fn new() -> Self {
        SemBlock {
            graph:     SemGraph::new(),
            fragments: Vec::new(),
            links:     Vec::new(),
        }
    }

    pub fn begin_char(&self) -> usize {
        self.fragments.first().map_or(0, |f| f.borrow().begin_char)
    }

    pub fn end_char(&self) -> usize {
        self.fragments.last().map_or(0, |f| f.borrow().end_char)
    }

    pub fn add_link(
        &mut self,
        typ:      SemFraglinkType,
        src:      SemFragmentRef,
        tgt:      SemFragmentRef,
        question: Option<String>,
    ) -> &SemFraglink {
        // Dedup
        for li in &self.links {
            if li.typ == typ
                && li.source.as_ref().map_or(false, |s| Rc::ptr_eq(s, &src))
                && li.target.as_ref().map_or(false, |t| Rc::ptr_eq(t, &tgt))
            {
                return self.links.last().unwrap();
            }
        }
        self.links.push(SemFraglink {
            typ,
            source: Some(src),
            target: Some(tgt),
            question,
        });
        self.links.last().unwrap()
    }

    pub fn merge_with(&mut self, other: &mut SemBlock) {
        self.graph.merge_with(&mut other.graph);
        self.fragments.append(&mut other.fragments);
        self.links.append(&mut other.links);
    }
}

// ── SemDocument ───────────────────────────────────────────────────────────

pub struct SemDocument {
    pub graph:  SemGraph,
    pub blocks: Vec<SemBlockRef>,
}

impl SemDocument {
    pub fn new() -> Self {
        SemDocument {
            graph:  SemGraph::new(),
            blocks: Vec::new(),
        }
    }

    pub fn begin_char(&self) -> usize {
        self.blocks.first().map_or(0, |b| b.borrow().begin_char())
    }

    pub fn end_char(&self) -> usize {
        self.blocks.last().map_or(0, |b| b.borrow().end_char())
    }

    pub fn merge_all_blocks(&mut self) {
        if self.blocks.len() < 2 { return; }
        let rest: Vec<_> = self.blocks.drain(1..).collect();
        for blk in rest {
            self.blocks[0].borrow_mut().merge_with(&mut blk.borrow_mut());
        }
    }
}
