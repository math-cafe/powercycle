//! Search infrastructure for powercycle.
//!
//! This crate contains the long-running search machinery used by
//! `pc-search`.
//!
//! Solver-specific code is kept separate from run/session infrastructure.

mod run;
mod sat;

pub use run::{Invocation, ProgramInfo, RunContext, RunError, RunPaths, RunState, RunStatus};

pub use sat::{
    BoundaryCutBudget, BudgetedBoundaryCutStats, BudgetedCutStats, CycleBreakingBudget, SatError,
    SatFactorSolver, SatModelStats, SubtourCutStats,
};
