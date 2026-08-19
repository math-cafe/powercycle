use crate::{Factor, FactorError, PowerGraph, Vertex};

/// A cycle in a 2-factor.
///
/// The representation is canonical:
///
/// 1. the smallest vertex is first;
/// 2. of the two possible orientations, the one with the smaller second
///    vertex is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle {
    vertices: Vec<Vertex>,
}

impl Cycle {
    /// Returns the number of vertices in the cycle.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Returns whether the cycle contains no vertices.
    ///
    /// Cycles produced by [`CycleDecomposition`] are never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Returns the vertices in canonical cyclic order.
    #[must_use]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Returns the first vertex of the canonical representation.
    #[must_use]
    pub fn first(&self) -> Vertex {
        self.vertices[0]
    }
}

/// Canonical decomposition of a valid 2-factor into disjoint cycles.
///
/// Cycles are ordered by their smallest vertex. Therefore the decomposition
/// is deterministic for a given factor.
///
/// The largest cycle is identified separately. If several cycles have the
/// same maximum size, the first one in canonical order is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CycleDecomposition {
    vertex_count: usize,
    cycles: Vec<Cycle>,
    largest_cycle_index: usize,
}

impl CycleDecomposition {
    /// Decomposes a valid 2-factor into cycles.
    ///
    /// The factor is validated before decomposition.
    pub fn new(graph: &PowerGraph, factor: &Factor) -> Result<Self, FactorError> {
        factor.validate_two_factor(graph)?;

        let n = graph.vertex_count();

        // For a valid 2-factor every vertex has exactly two selected
        // neighbours. We store zero-based vertex indices here.
        let mut neighbours = vec![[usize::MAX; 2]; n];
        let mut neighbour_counts = vec![0u8; n];

        for &edge_id in factor.selected_edges() {
            let edge = graph
                .edge(edge_id)
                .expect("factor edge IDs were validated at construction");

            let (a, b) = edge.endpoints();

            for (from, to) in [(a, b), (b, a)] {
                let index = from.index();
                let slot = neighbour_counts[index] as usize;

                debug_assert!(slot < 2);

                neighbours[index][slot] = to.index();
                neighbour_counts[index] += 1;
            }
        }

        debug_assert!(neighbour_counts.iter().all(|&count| count == 2));

        let mut visited = vec![false; n];
        let mut cycles = Vec::new();

        // Scanning start vertices in increasing order automatically orders
        // cycles by their smallest vertex.
        for start in 0..n {
            if visited[start] {
                continue;
            }

            let mut vertices = Vec::new();

            // Since start is the smallest unvisited vertex of this component,
            // it is the smallest vertex of the cycle. Choosing its smaller
            // neighbour fixes the canonical orientation.
            let [first_neighbour, second_neighbour] = neighbours[start];

            let mut previous = start;
            let mut current = first_neighbour.min(second_neighbour);

            visited[start] = true;
            vertices.push(Vertex::from_number_unchecked(start + 1));

            while current != start {
                debug_assert!(!visited[current]);

                visited[current] = true;

                vertices.push(Vertex::from_number_unchecked(current + 1));

                let [a, b] = neighbours[current];

                let next = if a == previous {
                    b
                } else {
                    debug_assert_eq!(b, previous);
                    a
                };

                previous = current;
                current = next;
            }

            cycles.push(Cycle { vertices });
        }

        debug_assert!(!cycles.is_empty());

        let mut largest_cycle_index = 0;

        for index in 1..cycles.len() {
            if cycles[index].len() > cycles[largest_cycle_index].len() {
                largest_cycle_index = index;
            }
        }

        Ok(Self {
            vertex_count: n,
            cycles,
            largest_cycle_index,
        })
    }

    /// Returns the number of cycles.
    #[must_use]
    pub fn cycle_count(&self) -> usize {
        self.cycles.len()
    }

    /// Returns all cycles in canonical order.
    #[must_use]
    pub fn cycles(&self) -> &[Cycle] {
        &self.cycles
    }

    /// Returns a cycle by canonical index.
    #[must_use]
    pub fn cycle(&self, index: usize) -> Option<&Cycle> {
        self.cycles.get(index)
    }

    /// Returns the index of the largest cycle.
    ///
    /// If several cycles have the same size, the first cycle in canonical
    /// order is chosen.
    #[must_use]
    pub const fn largest_cycle_index(&self) -> usize {
        self.largest_cycle_index
    }

    /// Returns the largest cycle.
    #[must_use]
    pub fn largest_cycle(&self) -> &Cycle {
        &self.cycles[self.largest_cycle_index]
    }

    /// Returns the number of vertices outside the largest cycle.
    #[must_use]
    pub fn residual_vertex_count(&self) -> usize {
        self.vertex_count - self.largest_cycle().len()
    }

    /// Returns whether the 2-factor is a Hamiltonian cycle.
    #[must_use]
    pub fn is_hamiltonian(&self) -> bool {
        self.cycles.len() == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EdgeId;

    fn factor_from_cycles(graph: &PowerGraph, cycles: &[&[usize]]) -> Factor {
        let mut selected_edges = Vec::new();

        for cycle in cycles {
            assert!(cycle.len() >= 3);

            for index in 0..cycle.len() {
                let next = (index + 1) % cycle.len();

                let a = graph.vertex(cycle[index]).unwrap();
                let b = graph.vertex(cycle[next]).unwrap();

                let edge_id = graph
                    .edge_between(a, b)
                    .expect("cycle adjacency must be a graph edge");

                selected_edges.push(edge_id);
            }
        }

        Factor::new(graph, selected_edges).unwrap()
    }

    fn vertex_numbers(cycle: &Cycle) -> Vec<usize> {
        cycle
            .vertices()
            .iter()
            .map(|vertex| vertex.number())
            .collect()
    }

    #[test]
    fn decomposes_hamiltonian_cycle() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let cycle = [
            1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9,
            27, 22, 14, 2, 23, 26, 10, 15,
        ];

        let factor = factor_from_cycles(&graph, &[&cycle]);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        assert_eq!(decomposition.cycle_count(), 1);
        assert_eq!(decomposition.largest_cycle().len(), 32);
        assert_eq!(decomposition.residual_vertex_count(), 0);
        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn canonicalizes_cycle_orientation() {
        let graph = PowerGraph::new(2, 32).unwrap();

        // Same Hamiltonian cycle as above, but deliberately rotated and
        // reversed. The factor itself has no orientation.
        let cycle = [
            19, 17, 32, 4, 21, 28, 8, 1, 15, 10, 26, 23, 2, 14, 22, 27, 9, 16, 20, 29, 7, 18, 31,
            5, 11, 25, 24, 12, 13, 3, 6, 30,
        ];

        let factor = factor_from_cycles(&graph, &[&cycle]);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let expected = vec![
            1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9,
            27, 22, 14, 2, 23, 26, 10, 15,
        ];

        assert_eq!(vertex_numbers(decomposition.largest_cycle()), expected);
    }

    #[test]
    fn decomposes_multiple_cycles_deterministically() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let cycle_a = [1, 8, 28, 21, 15];

        let cycle_b = [
            2, 14, 11, 25, 24, 12, 13, 3, 22, 27, 9, 16, 20, 29, 7, 18, 31, 5, 4, 32, 17, 19, 30,
            6, 10, 26, 23,
        ];

        let factor = factor_from_cycles(&graph, &[&cycle_b, &cycle_a]);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        assert_eq!(decomposition.cycle_count(), 2);

        // Cycles are ordered by their smallest vertex, independently of
        // the order in which they were supplied to Factor.
        assert_eq!(vertex_numbers(decomposition.cycle(0).unwrap()), cycle_a);

        assert_eq!(vertex_numbers(decomposition.cycle(1).unwrap()), cycle_b);

        assert_eq!(decomposition.cycle(0).unwrap().first().number(), 1);
        assert_eq!(decomposition.cycle(1).unwrap().first().number(), 2);
    }

    #[test]
    fn identifies_largest_cycle_and_residual() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let small_cycle = [1, 8, 28, 21, 15];

        let large_cycle = [
            2, 14, 11, 25, 24, 12, 13, 3, 22, 27, 9, 16, 20, 29, 7, 18, 31, 5, 4, 32, 17, 19, 30,
            6, 10, 26, 23,
        ];

        let factor = factor_from_cycles(&graph, &[&small_cycle, &large_cycle]);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        assert_eq!(decomposition.largest_cycle_index(), 1);
        assert_eq!(decomposition.largest_cycle().len(), 27);
        assert_eq!(decomposition.residual_vertex_count(), 5);
        assert!(!decomposition.is_hamiltonian());
    }

    #[test]
    fn rejects_non_two_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = Factor::new(&graph, [EdgeId::from_index(0)]).unwrap();

        assert!(matches!(
            CycleDecomposition::new(&graph, &factor),
            Err(FactorError::InvalidSelectedDegree { .. })
        ));
    }
}
