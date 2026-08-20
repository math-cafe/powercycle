//! Factor patching algorithms for powercycle.
//!
//! This crate contains transformations applied to valid 2-factors.
//! It has no dependency on SAT solving, persistent dump formats or CLI code.

mod three_opt;
mod three_opt_21;

pub use three_opt::{
    GreedyThreeOpt111Result, PatchError, ThreeOpt111Move, find_three_opt_111, greedy_three_opt_111,
};

pub use three_opt_21::{
    GreedyThreeOpt21Result, ThreeOpt21Error, ThreeOpt21Move, find_three_opt_21, greedy_three_opt_21,
};
