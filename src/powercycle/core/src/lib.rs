//! Core mathematical structures for powercycle.
//!
//! This crate contains no solver, command-line interface, logging,
//! configuration or persistence code.

mod cycle;
mod error;
mod factor;
mod graph;
mod types;

pub use cycle::{Cycle, CycleDecomposition};
pub use error::{FactorError, GraphError};
pub use factor::Factor;
pub use graph::PowerGraph;
pub use types::{Edge, EdgeId, Vertex};
