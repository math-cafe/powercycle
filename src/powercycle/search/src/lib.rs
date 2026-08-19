//! Search infrastructure for powercycle.
//!
//! This crate contains the long-running search machinery used by
//! `pc-search`.
//!
//! Solver-specific code is deliberately kept separate from the run/session
//! infrastructure.

mod run;

pub use run::{Invocation, ProgramInfo, RunContext, RunError, RunPaths, RunState, RunStatus};
