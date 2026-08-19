use std::error::Error;
use std::fmt;

/// Errors that can occur while constructing a power-sum graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// Power-sum graphs are defined here for exponents k >= 2.
    InvalidExponent { k: u32 },

    /// Vertex sets are `[1, n]`, therefore n must be positive.
    InvalidVertexCount { n: usize },

    /// An arithmetic or allocation-related size computation overflowed.
    SizeOverflow,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExponent { k } => {
                write!(f, "invalid exponent k={k}: expected k >= 2")
            }

            Self::InvalidVertexCount { n } => {
                write!(f, "invalid vertex count n={n}: expected n >= 1")
            }

            Self::SizeOverflow => {
                write!(f, "graph size exceeds supported integer range")
            }
        }
    }
}

impl Error for GraphError {}
