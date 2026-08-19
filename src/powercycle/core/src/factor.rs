use crate::{EdgeId, FactorError, PowerGraph, Vertex};

/// A subset of edges of a specific power-sum graph.
///
/// Edge IDs are stored in increasing order so that the representation is
/// deterministic.
///
/// A `Factor` is not necessarily a 2-factor. Use [`Factor::validate_two_factor`]
/// to verify that every vertex has selected degree exactly 2.
#[derive(Clone, Debug)]
pub struct Factor {
    k: u32,
    n: usize,
    graph_edge_count: usize,
    selected_edges: Vec<EdgeId>,
    selected: Vec<bool>,
}

impl Factor {
    /// Constructs a factor from a collection of graph edge IDs.
    ///
    /// Edge IDs must belong to `graph` and must not contain duplicates.
    pub fn new<I>(graph: &PowerGraph, selected_edges: I) -> Result<Self, FactorError>
    where
        I: IntoIterator<Item = EdgeId>,
    {
        let graph_edge_count = graph.edge_count();
        let mut selected = vec![false; graph_edge_count];
        let mut edges = Vec::new();

        for edge_id in selected_edges {
            let index = edge_id.index();

            if index >= graph_edge_count {
                return Err(FactorError::InvalidEdgeId {
                    edge_id: index,
                    edge_count: graph_edge_count,
                });
            }

            if selected[index] {
                return Err(FactorError::DuplicateEdgeId { edge_id: index });
            }

            selected[index] = true;
            edges.push(edge_id);
        }

        edges.sort_unstable();

        Ok(Self {
            k: graph.k(),
            n: graph.vertex_count(),
            graph_edge_count,
            selected_edges: edges,
            selected,
        })
    }

    /// Returns the exponent of the graph this factor belongs to.
    #[must_use]
    pub const fn k(&self) -> u32 {
        self.k
    }

    /// Returns the number of vertices of the graph this factor belongs to.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.n
    }

    /// Returns the number of edges in the underlying graph.
    #[must_use]
    pub const fn graph_edge_count(&self) -> usize {
        self.graph_edge_count
    }

    /// Returns the number of selected edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.selected_edges.len()
    }

    /// Returns the selected edge IDs in deterministic increasing order.
    #[must_use]
    pub fn selected_edges(&self) -> &[EdgeId] {
        &self.selected_edges
    }

    /// Returns whether an edge ID is selected.
    #[must_use]
    pub fn contains(&self, edge_id: EdgeId) -> bool {
        self.selected.get(edge_id.index()).copied().unwrap_or(false)
    }

    /// Returns the selected degree of a vertex.
    pub fn selected_degree(
        &self,
        graph: &PowerGraph,
        vertex: Vertex,
    ) -> Result<usize, FactorError> {
        self.ensure_compatible_graph(graph)?;

        if !graph.contains_vertex(vertex) {
            return Err(FactorError::InvalidVertex {
                vertex: vertex.number(),
                vertex_count: graph.vertex_count(),
            });
        }

        let incident = graph
            .incident(vertex)
            .expect("graph-owned vertex must have an incidence list");

        Ok(incident
            .iter()
            .copied()
            .filter(|&edge_id| self.contains(edge_id))
            .count())
    }

    /// Verifies that the factor is a 2-factor.
    ///
    /// A valid 2-factor has selected degree exactly 2 at every vertex.
    pub fn validate_two_factor(&self, graph: &PowerGraph) -> Result<(), FactorError> {
        self.ensure_compatible_graph(graph)?;

        let mut degrees = vec![0usize; self.n];

        for &edge_id in &self.selected_edges {
            let edge = graph
                .edge(edge_id)
                .expect("factor edge IDs were validated at construction");

            let (a, b) = edge.endpoints();

            degrees[a.index()] += 1;
            degrees[b.index()] += 1;
        }

        for (index, degree) in degrees.into_iter().enumerate() {
            if degree != 2 {
                return Err(FactorError::InvalidSelectedDegree {
                    vertex: index + 1,
                    degree,
                    expected: 2,
                });
            }
        }

        Ok(())
    }

    fn ensure_compatible_graph(&self, graph: &PowerGraph) -> Result<(), FactorError> {
        if self.k == graph.k()
            && self.n == graph.vertex_count()
            && self.graph_edge_count == graph.edge_count()
        {
            return Ok(());
        }

        Err(FactorError::GraphMismatch {
            factor_k: self.k,
            factor_n: self.n,
            factor_edge_count: self.graph_edge_count,
            graph_k: graph.k(),
            graph_n: graph.vertex_count(),
            graph_edge_count: graph.edge_count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_selected_edges_in_deterministic_order() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let factor = Factor::new(
            &graph,
            [
                EdgeId::from_index(5),
                EdgeId::from_index(1),
                EdgeId::from_index(3),
            ],
        )
        .unwrap();

        assert_eq!(
            factor.selected_edges(),
            &[
                EdgeId::from_index(1),
                EdgeId::from_index(3),
                EdgeId::from_index(5),
            ]
        );

        assert_eq!(factor.edge_count(), 3);
        assert!(factor.contains(EdgeId::from_index(1)));
        assert!(factor.contains(EdgeId::from_index(3)));
        assert!(factor.contains(EdgeId::from_index(5)));
        assert!(!factor.contains(EdgeId::from_index(0)));
    }

    #[test]
    fn rejects_invalid_edge_ids() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let error = Factor::new(&graph, [EdgeId::from_index(graph.edge_count())]).unwrap_err();

        assert_eq!(
            error,
            FactorError::InvalidEdgeId {
                edge_id: graph.edge_count(),
                edge_count: graph.edge_count(),
            }
        );
    }

    #[test]
    fn rejects_duplicate_edge_ids() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let error =
            Factor::new(&graph, [EdgeId::from_index(1), EdgeId::from_index(1)]).unwrap_err();

        assert_eq!(error, FactorError::DuplicateEdgeId { edge_id: 1 });
    }

    #[test]
    fn computes_selected_degree() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let factor = Factor::new(
            &graph,
            [
                EdgeId::from_index(0),
                EdgeId::from_index(1),
                EdgeId::from_index(2),
            ],
        )
        .unwrap();

        let v1 = graph.vertex(1).unwrap();
        let v2 = graph.vertex(2).unwrap();
        let v3 = graph.vertex(3).unwrap();

        assert_eq!(factor.selected_degree(&graph, v1), Ok(2));
        assert_eq!(factor.selected_degree(&graph, v2), Ok(1));
        assert_eq!(factor.selected_degree(&graph, v3), Ok(1));
    }

    #[test]
    fn rejects_use_with_another_graph() {
        let graph = PowerGraph::new(2, 10).unwrap();
        let other_graph = PowerGraph::new(2, 11).unwrap();

        let factor = Factor::new(&graph, [EdgeId::from_index(0)]).unwrap();

        assert!(matches!(
            factor.validate_two_factor(&other_graph),
            Err(FactorError::GraphMismatch { .. })
        ));
    }

    #[test]
    fn validates_known_square_sum_hamiltonian_cycle() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let cycle = [
            1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9,
            27, 22, 14, 2, 23, 26, 10, 15,
        ];

        let mut selected_edges = Vec::with_capacity(cycle.len());

        for index in 0..cycle.len() {
            let next = (index + 1) % cycle.len();

            let a = graph.vertex(cycle[index]).unwrap();
            let b = graph.vertex(cycle[next]).unwrap();

            let edge_id = graph
                .edge_between(a, b)
                .expect("cycle adjacency must be a graph edge");

            selected_edges.push(edge_id);
        }

        let factor = Factor::new(&graph, selected_edges).unwrap();

        assert_eq!(factor.edge_count(), 32);
        assert_eq!(factor.validate_two_factor(&graph), Ok(()));
    }

    #[test]
    fn detects_broken_two_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let cycle = [
            1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9,
            27, 22, 14, 2, 23, 26, 10, 15,
        ];

        let mut selected_edges = Vec::with_capacity(cycle.len());

        for index in 0..cycle.len() {
            let next = (index + 1) % cycle.len();

            let a = graph.vertex(cycle[index]).unwrap();
            let b = graph.vertex(cycle[next]).unwrap();

            selected_edges.push(
                graph
                    .edge_between(a, b)
                    .expect("cycle adjacency must be a graph edge"),
            );
        }

        selected_edges.pop();

        let factor = Factor::new(&graph, selected_edges).unwrap();

        assert!(matches!(
            factor.validate_two_factor(&graph),
            Err(FactorError::InvalidSelectedDegree {
                degree: 1,
                expected: 2,
                ..
            })
        ));
    }
}
