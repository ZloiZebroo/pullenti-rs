// Pullenti Semantic — Semantic analysis subsystem
// Ported from Pullenti C# SDK v4.33

pub mod types;
pub mod core;
pub mod sem_graph;
pub mod sem_document;
pub mod semantic_service;
pub mod analyze_helper;
pub mod internal;

pub use types::{
    SemObjectType, SemFragmentType, SemLinkType, SemFraglinkType,
    SemAttributeType, SemAttribute, SemProcessParams,
};
pub use sem_graph::{SemGraph, SemObject, SemObjectRef, SemLink, SemLinkRef, SemQuantity};
pub use sem_document::{SemDocument, SemBlock, SemBlockRef, SemFragment, SemFragmentRef, SemFraglink};
pub use semantic_service::{initialize, process, VERSION};
