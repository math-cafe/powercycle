//! Core mathematical structures for powercycle.
//!
//! This crate contains no solver, command-line interface, logging,
//! configuration or persistence code.

mod error;
mod graph;
mod types;

pub use error::GraphError;
pub use graph::PowerGraph;
pub use types::{Edge, EdgeId, Vertex};
