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

/// Errors related to factors of a power-sum graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorError {
    /// An edge ID does not belong to the underlying graph.
    InvalidEdgeId { edge_id: usize, edge_count: usize },

    /// An edge ID was supplied more than once.
    DuplicateEdgeId { edge_id: usize },

    /// A vertex does not belong to the underlying graph.
    InvalidVertex { vertex: usize, vertex_count: usize },

    /// The factor is being used with a different graph.
    GraphMismatch {
        factor_k: u32,
        factor_n: usize,
        factor_edge_count: usize,
        graph_k: u32,
        graph_n: usize,
        graph_edge_count: usize,
    },

    /// A vertex does not have the selected degree required by a 2-factor.
    InvalidSelectedDegree {
        vertex: usize,
        degree: usize,
        expected: usize,
    },
}

impl fmt::Display for FactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEdgeId {
                edge_id,
                edge_count,
            } => {
                write!(
                    f,
                    "invalid edge ID {edge_id}: graph contains {edge_count} edges"
                )
            }

            Self::DuplicateEdgeId { edge_id } => {
                write!(f, "duplicate edge ID {edge_id}")
            }

            Self::InvalidVertex {
                vertex,
                vertex_count,
            } => {
                write!(
                    f,
                    "invalid vertex {vertex}: expected a vertex in 1..={vertex_count}"
                )
            }

            Self::GraphMismatch {
                factor_k,
                factor_n,
                factor_edge_count,
                graph_k,
                graph_n,
                graph_edge_count,
            } => {
                write!(
                    f,
                    "factor belongs to G_{factor_k}({factor_n}) \
                     with {factor_edge_count} graph edges, but received \
                     G_{graph_k}({graph_n}) with {graph_edge_count} graph edges"
                )
            }

            Self::InvalidSelectedDegree {
                vertex,
                degree,
                expected,
            } => {
                write!(
                    f,
                    "vertex {vertex} has selected degree {degree}, expected {expected}"
                )
            }
        }
    }
}

impl Error for FactorError {}
