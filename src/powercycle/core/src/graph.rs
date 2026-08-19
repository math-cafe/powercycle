use std::collections::VecDeque;

use crate::{Edge, EdgeId, GraphError, Vertex};

/// The power-sum graph G_k(n).
///
/// Its vertices are the integers 1..=n. Two distinct vertices x and y are
/// adjacent if and only if x + y is a perfect k-th power.
///
/// Edges are stored deterministically:
///
/// 1. increasing k-th power;
/// 2. increasing first endpoint.
///
/// Incidence lists are stored in CSR-like form.
#[derive(Clone, Debug)]
pub struct PowerGraph {
    k: u32,
    n: usize,
    powers: Vec<usize>,
    edges: Vec<Edge>,
    offsets: Vec<usize>,
    incident_edges: Vec<EdgeId>,
}

impl PowerGraph {
    /// Constructs G_k(n).
    pub fn new(k: u32, n: usize) -> Result<Self, GraphError> {
        if k < 2 {
            return Err(GraphError::InvalidExponent { k });
        }

        if n == 0 {
            return Err(GraphError::InvalidVertexCount { n });
        }

        let max_sum = n
            .checked_mul(2)
            .and_then(|value| value.checked_sub(1))
            .ok_or(GraphError::SizeOverflow)?;

        let powers = perfect_powers(k, max_sum);
        let edges = build_edges(n, &powers);

        let mut degree_counts = vec![0usize; n];

        for edge in &edges {
            let (a, b) = edge.endpoints();

            degree_counts[a.index()] = degree_counts[a.index()]
                .checked_add(1)
                .ok_or(GraphError::SizeOverflow)?;

            degree_counts[b.index()] = degree_counts[b.index()]
                .checked_add(1)
                .ok_or(GraphError::SizeOverflow)?;
        }

        let mut offsets: Vec<usize> =
            Vec::with_capacity(n.checked_add(1).ok_or(GraphError::SizeOverflow)?);

        offsets.push(0);

        for degree in degree_counts {
            let next = offsets
                .last()
                .copied()
                .unwrap()
                .checked_add(degree)
                .ok_or(GraphError::SizeOverflow)?;

            offsets.push(next);
        }

        let incidence_count = edges.len().checked_mul(2).ok_or(GraphError::SizeOverflow)?;

        let mut incident_edges = vec![EdgeId::from_index(0); incidence_count];

        let mut cursors = offsets[..n].to_vec();

        for (edge_index, edge) in edges.iter().copied().enumerate() {
            let edge_id = EdgeId::from_index(edge_index);
            let (a, b) = edge.endpoints();

            for vertex in [a, b] {
                let vertex_index = vertex.index();
                let position = cursors[vertex_index];

                incident_edges[position] = edge_id;
                cursors[vertex_index] += 1;
            }
        }

        Ok(Self {
            k,
            n,
            powers,
            edges,
            offsets,
            incident_edges,
        })
    }

    /// Returns the exponent k.
    #[must_use]
    pub const fn k(&self) -> u32 {
        self.k
    }

    /// Returns the number of vertices n.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.n
    }

    /// Returns the perfect k-th powers that can occur as edge sums.
    #[must_use]
    pub fn powers(&self) -> &[usize] {
        &self.powers
    }

    /// Returns the number of graph edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns all edges in deterministic edge-ID order.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Returns a vertex if `number` belongs to `[1, n]`.
    #[must_use]
    pub fn vertex(&self, number: usize) -> Option<Vertex> {
        if number == 0 || number > self.n {
            None
        } else {
            Some(Vertex::from_number_unchecked(number))
        }
    }

    /// Returns whether a vertex belongs to this graph.
    #[must_use]
    pub fn contains_vertex(&self, vertex: Vertex) -> bool {
        vertex.number() <= self.n
    }

    /// Returns an edge by ID.
    #[must_use]
    pub fn edge(&self, edge_id: EdgeId) -> Option<Edge> {
        self.edges.get(edge_id.index()).copied()
    }

    /// Returns the incident edge IDs of a vertex.
    #[must_use]
    pub fn incident(&self, vertex: Vertex) -> Option<&[EdgeId]> {
        if !self.contains_vertex(vertex) {
            return None;
        }

        let index = vertex.index();
        let start = self.offsets[index];
        let end = self.offsets[index + 1];

        Some(&self.incident_edges[start..end])
    }

    /// Returns the degree of a vertex.
    #[must_use]
    pub fn degree(&self, vertex: Vertex) -> Option<usize> {
        self.incident(vertex).map(<[EdgeId]>::len)
    }

    /// Returns the minimum vertex degree.
    #[must_use]
    pub fn min_degree(&self) -> usize {
        self.offsets
            .windows(2)
            .map(|window| window[1] - window[0])
            .min()
            .expect("a power graph always contains at least one vertex")
    }

    /// Returns the maximum vertex degree.
    #[must_use]
    pub fn max_degree(&self) -> usize {
        self.offsets
            .windows(2)
            .map(|window| window[1] - window[0])
            .max()
            .expect("a power graph always contains at least one vertex")
    }

    /// Returns the average vertex degree.
    #[must_use]
    pub fn average_degree(&self) -> f64 {
        self.incident_edges.len() as f64 / self.n as f64
    }

    /// Returns the opposite endpoint of an edge.
    #[must_use]
    pub fn other(&self, edge_id: EdgeId, vertex: Vertex) -> Option<Vertex> {
        self.edge(edge_id)?.other(vertex)
    }

    /// Finds the edge joining two vertices, if it exists.
    #[must_use]
    pub fn edge_between(&self, a: Vertex, b: Vertex) -> Option<EdgeId> {
        if a == b || !self.contains_vertex(a) || !self.contains_vertex(b) {
            return None;
        }

        let incident_a = self.incident(a)?;
        let incident_b = self.incident(b)?;

        let (source, target) = if incident_a.len() <= incident_b.len() {
            (a, b)
        } else {
            (b, a)
        };

        self.incident(source)?
            .iter()
            .copied()
            .find(|&edge_id| self.other(edge_id, source) == Some(target))
    }

    /// Returns whether two vertices are adjacent.
    #[must_use]
    pub fn has_edge(&self, a: Vertex, b: Vertex) -> bool {
        self.edge_between(a, b).is_some()
    }

    /// Tests whether the graph is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        let mut visited = vec![false; self.n];
        let mut queue = VecDeque::new();

        let start = Vertex::from_number_unchecked(1);
        visited[start.index()] = true;
        queue.push_back(start);

        let mut visited_count = 1usize;

        while let Some(vertex) = queue.pop_front() {
            let incident = self
                .incident(vertex)
                .expect("graph-owned vertex must be valid");

            for &edge_id in incident {
                let other = self
                    .other(edge_id, vertex)
                    .expect("incident edge must contain its vertex");

                if !visited[other.index()] {
                    visited[other.index()] = true;
                    visited_count += 1;
                    queue.push_back(other);
                }
            }
        }

        visited_count == self.n
    }
}

fn perfect_powers(k: u32, max_sum: usize) -> Vec<usize> {
    let mut powers = Vec::new();
    let mut base = 2usize;

    loop {
        match base.checked_pow(k) {
            Some(power) if power <= max_sum => {
                powers.push(power);
            }

            _ => break,
        }

        match base.checked_add(1) {
            Some(next) => base = next,
            None => break,
        }
    }

    powers
}

fn build_edges(n: usize, powers: &[usize]) -> Vec<Edge> {
    let mut edges = Vec::new();

    for &power in powers {
        // y = power - x must satisfy y <= n.
        let first_min = power.saturating_sub(n).max(1);

        // x < y avoids loops and duplicate undirected edges.
        let first_max = n.min((power - 1) / 2);

        if first_min > first_max {
            continue;
        }

        for first_number in first_min..=first_max {
            let second_number = power - first_number;

            debug_assert!(first_number < second_number);
            debug_assert!(second_number <= n);

            let first = Vertex::from_number_unchecked(first_number);

            let second = Vertex::from_number_unchecked(second_number);

            let edge = Edge::new(first, second).expect("edge construction excludes loops");

            edges.push(edge);
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint_numbers(edge: Edge) -> (usize, usize) {
        let (a, b) = edge.endpoints();
        (a.number(), b.number())
    }

    #[test]
    fn rejects_invalid_parameters() {
        assert_eq!(
            PowerGraph::new(1, 10).unwrap_err(),
            GraphError::InvalidExponent { k: 1 }
        );

        assert_eq!(
            PowerGraph::new(2, 0).unwrap_err(),
            GraphError::InvalidVertexCount { n: 0 }
        );
    }

    #[test]
    fn builds_small_square_sum_graph() {
        let graph = PowerGraph::new(2, 10).unwrap();

        assert_eq!(graph.k(), 2);
        assert_eq!(graph.vertex_count(), 10);
        assert_eq!(graph.powers(), &[4, 9, 16]);
        assert_eq!(graph.edge_count(), 7);

        let edges: Vec<_> = graph
            .edges()
            .iter()
            .copied()
            .map(endpoint_numbers)
            .collect();

        assert_eq!(
            edges,
            vec![(1, 3), (1, 8), (2, 7), (3, 6), (4, 5), (6, 10), (7, 9),]
        );
    }

    #[test]
    fn edge_ids_are_deterministic() {
        let graph = PowerGraph::new(2, 10).unwrap();

        assert_eq!(
            endpoint_numbers(graph.edge(EdgeId::from_index(0)).unwrap()),
            (1, 3)
        );

        assert_eq!(
            endpoint_numbers(graph.edge(EdgeId::from_index(1)).unwrap()),
            (1, 8)
        );
    }

    #[test]
    fn incidence_and_degree_are_correct() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let v1 = graph.vertex(1).unwrap();
        let v4 = graph.vertex(4).unwrap();

        assert_eq!(graph.degree(v1), Some(2));
        assert_eq!(graph.degree(v4), Some(1));

        assert_eq!(graph.min_degree(), 1);
        assert_eq!(graph.max_degree(), 2);
        assert_eq!(graph.average_degree(), 1.4);
    }

    #[test]
    fn edge_lookup_is_symmetric() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let v1 = graph.vertex(1).unwrap();
        let v8 = graph.vertex(8).unwrap();

        let forward = graph.edge_between(v1, v8);
        let reverse = graph.edge_between(v8, v1);

        assert_eq!(forward, Some(EdgeId::from_index(1)));
        assert_eq!(reverse, forward);
        assert!(graph.has_edge(v1, v8));
    }

    #[test]
    fn disconnected_graph_is_detected() {
        let graph = PowerGraph::new(2, 10).unwrap();

        assert!(!graph.is_connected());
    }

    #[test]
    fn builds_small_cube_sum_graph() {
        let graph = PowerGraph::new(3, 10).unwrap();

        assert_eq!(graph.powers(), &[8]);
        assert_eq!(graph.edge_count(), 3);

        let edges: Vec<_> = graph
            .edges()
            .iter()
            .copied()
            .map(endpoint_numbers)
            .collect();

        assert_eq!(edges, vec![(1, 7), (2, 6), (3, 5)]);
    }

    #[test]
    fn every_edge_sum_is_a_listed_power() {
        let graph = PowerGraph::new(5, 500).unwrap();

        for edge in graph.edges() {
            let (a, b) = edge.endpoints();
            let sum = a.number() + b.number();

            assert!(graph.powers().contains(&sum));
        }
    }

    #[test]
    fn reproduces_known_k5_n490463_graph() {
        let graph = PowerGraph::new(5, 490_463).unwrap();

        assert_eq!(
            graph.powers(),
            &[
                32, 243, 1_024, 3_125, 7_776, 16_807, 32_768, 59_049, 100_000, 161_051, 248_832,
                371_293, 537_824, 759_375,
            ]
        );

        assert_eq!(graph.vertex_count(), 490_463);
        assert_eq!(graph.edge_count(), 833_318);
        assert_eq!(graph.min_degree(), 2);
        assert_eq!(graph.max_degree(), 12);
        assert!(graph.is_connected());

        let expected_average = 2.0 * graph.edge_count() as f64 / graph.vertex_count() as f64;

        assert!((graph.average_degree() - expected_average).abs() < f64::EPSILON);

        let degree_two_count = (1..=graph.vertex_count())
            .filter(|&number| {
                let vertex = graph.vertex(number).expect("number belongs to graph");

                graph.degree(vertex) == Some(2)
            })
            .count();

        assert_eq!(degree_two_count, 139_252);
    }
}
