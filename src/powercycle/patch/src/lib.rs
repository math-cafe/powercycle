//! Factor patching algorithms for powercycle.
//!
//! This crate contains transformations applied to valid 2-factors.
//! It has no dependency on SAT solving, persistent dump formats or CLI code.

mod alternating_repair;
mod three_opt;
mod three_opt_21;

pub use alternating_repair::{
    AlternatingRepairError, AlternatingRepairMove, GreedyAlternatingRepairResult,
    find_alternating_repair, greedy_alternating_repair,
};

pub use three_opt::{
    GreedyThreeOpt111Result, PatchError, ThreeOpt111Move, find_three_opt_111, greedy_three_opt_111,
};

pub use three_opt_21::{
    GreedyThreeOpt21Result, ThreeOpt21Error, ThreeOpt21Move, find_three_opt_21, greedy_three_opt_21,
};
