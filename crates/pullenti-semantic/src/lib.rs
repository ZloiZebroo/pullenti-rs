// Pullenti Semantic — Semantic analysis subsystem
// Ported from Pullenti C# SDK v4.33

#![allow(
    dead_code,
    unused_imports,
    unused_mut,
    unused_variables
)]

pub mod types;
pub mod core;
pub mod sem_graph;
pub mod sem_document;
pub mod semantic_service;
pub mod analyze_helper;
pub mod internal;
mod score_order;

pub use types::{
    SemObjectType, SemFragmentType, SemLinkType, SemFraglinkType,
    SemAttributeType, SemAttribute, SemProcessParams,
};
pub use sem_graph::{SemGraph, SemObject, SemObjectRef, SemLink, SemLinkRef, SemQuantity};
pub use sem_document::{SemDocument, SemBlock, SemBlockRef, SemFragment, SemFragmentRef, SemFraglink};
pub use semantic_service::{initialize, process, VERSION};
