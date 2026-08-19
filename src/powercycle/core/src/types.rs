use std::fmt;

/// A vertex of a power-sum graph.
///
/// Vertices are numbered from 1 to n.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vertex(usize);

impl Vertex {
    /// Creates a vertex from its mathematical number.
    ///
    /// Returns `None` for 0, since vertices are numbered from 1.
    #[must_use]
    pub const fn from_number(number: usize) -> Option<Self> {
        if number == 0 {
            None
        } else {
            Some(Self(number))
        }
    }

    /// Creates a vertex from a known-valid positive number.
    pub(crate) const fn from_number_unchecked(number: usize) -> Self {
        debug_assert!(number > 0);
        Self(number)
    }

    /// Returns the mathematical vertex number in `[1, n]`.
    #[must_use]
    pub const fn number(self) -> usize {
        self.0
    }

    /// Returns the zero-based array index corresponding to this vertex.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 - 1
    }
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Identifier of an edge in the deterministic edge array.
///
/// Edge identifiers are zero-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(usize);

impl EdgeId {
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An undirected edge between two distinct vertices.
///
/// Endpoints are always stored in increasing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Edge {
    first: Vertex,
    second: Vertex,
}

impl Edge {
    /// Creates an undirected edge.
    ///
    /// Returns `None` if both endpoints are identical.
    #[must_use]
    pub fn new(a: Vertex, b: Vertex) -> Option<Self> {
        if a == b {
            return None;
        }

        if a < b {
            Some(Self {
                first: a,
                second: b,
            })
        } else {
            Some(Self {
                first: b,
                second: a,
            })
        }
    }

    #[must_use]
    pub const fn first(self) -> Vertex {
        self.first
    }

    #[must_use]
    pub const fn second(self) -> Vertex {
        self.second
    }

    #[must_use]
    pub const fn endpoints(self) -> (Vertex, Vertex) {
        (self.first, self.second)
    }

    #[must_use]
    pub fn other(self, vertex: Vertex) -> Option<Vertex> {
        if vertex == self.first {
            Some(self.second)
        } else if vertex == self.second {
            Some(self.first)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_numbers_are_one_based() {
        assert_eq!(Vertex::from_number(0), None);

        let vertex = Vertex::from_number(7).unwrap();

        assert_eq!(vertex.number(), 7);
        assert_eq!(vertex.index(), 6);
    }

    #[test]
    fn edges_are_canonical() {
        let a = Vertex::from_number(7).unwrap();
        let b = Vertex::from_number(2).unwrap();

        let edge = Edge::new(a, b).unwrap();

        assert_eq!(edge.first().number(), 2);
        assert_eq!(edge.second().number(), 7);
    }

    #[test]
    fn loops_are_rejected() {
        let vertex = Vertex::from_number(4).unwrap();

        assert_eq!(Edge::new(vertex, vertex), None);
    }
}
