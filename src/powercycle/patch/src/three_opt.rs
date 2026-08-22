use std::error::Error;
use std::fmt;

use powercycle_core::{Cycle, CycleDecomposition, EdgeId, Factor, FactorError, PowerGraph, Vertex};

/// A 3-opt `1+1+1` move.
///
/// The move removes one selected edge from each of three distinct cycles and
/// adds three graph edges connecting the resulting paths into one cycle.
///
/// Applying a valid move therefore decreases the cycle count by exactly two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreeOpt111Move {
    removed: [EdgeId; 3],
    added: [EdgeId; 3],
}

impl ThreeOpt111Move {
    /// Creates a move from explicit graph edge IDs.
    ///
    /// The move is validated when `apply` is called.
    #[must_use]
    pub const fn new(removed: [EdgeId; 3], added: [EdgeId; 3]) -> Self {
        Self { removed, added }
    }

    /// Returns the three selected edges removed by the move.
    #[must_use]
    pub const fn removed(&self) -> &[EdgeId; 3] {
        &self.removed
    }

    /// Returns the three graph edges added by the move.
    #[must_use]
    pub const fn added(&self) -> &[EdgeId; 3] {
        &self.added
    }

    /// Applies the move to a valid 2-factor.
    ///
    /// The result is checked to remain a valid 2-factor and to have exactly
    /// two fewer cycles.
    pub fn apply(&self, graph: &PowerGraph, factor: &Factor) -> Result<Factor, PatchError> {
        factor
            .validate_two_factor(graph)
            .map_err(PatchError::Factor)?;

        if !three_distinct(self.removed) {
            return Err(PatchError::DuplicateRemovedEdge);
        }

        if !three_distinct(self.added) {
            return Err(PatchError::DuplicateAddedEdge);
        }

        for &edge_id in &self.removed {
            if !factor.contains(edge_id) {
                return Err(PatchError::RemovedEdgeNotSelected { edge_id });
            }
        }

        for &edge_id in &self.added {
            if factor.contains(edge_id) {
                return Err(PatchError::AddedEdgeAlreadySelected { edge_id });
            }
        }

        let before =
            CycleDecomposition::new(graph, factor).map_err(|error| PatchError::Decomposition {
                operation: "decompose input factor",
                message: error.to_string(),
            })?;

        if before.cycle_count() < 3 {
            return Err(PatchError::TooFewCycles {
                cycle_count: before.cycle_count(),
            });
        }

        let mut selected = vec![false; graph.edge_count()];

        for &edge_id in factor.selected_edges() {
            selected[edge_id.index()] = true;
        }

        for &edge_id in &self.removed {
            selected[edge_id.index()] = false;
        }

        for &edge_id in &self.added {
            selected[edge_id.index()] = true;
        }

        let selected_edges = selected
            .into_iter()
            .enumerate()
            .filter_map(|(index, is_selected)| is_selected.then_some(EdgeId::from_index(index)))
            .collect::<Vec<_>>();

        let result = Factor::new(graph, selected_edges).map_err(PatchError::Factor)?;

        result
            .validate_two_factor(graph)
            .map_err(PatchError::Factor)?;

        let after =
            CycleDecomposition::new(graph, &result).map_err(|error| PatchError::Decomposition {
                operation: "decompose patched factor",
                message: error.to_string(),
            })?;

        let expected = before.cycle_count().saturating_sub(2);

        if after.cycle_count() != expected {
            return Err(PatchError::InvalidCycleReduction {
                before: before.cycle_count(),
                after: after.cycle_count(),
                expected,
            });
        }

        Ok(result)
    }
}

/// Result of deterministic greedy 3-opt `1+1+1` patching.
pub struct GreedyThreeOpt111Result {
    factor: Factor,
    moves: Vec<ThreeOpt111Move>,
    initial_cycle_count: usize,
    final_cycle_count: usize,
}

impl GreedyThreeOpt111Result {
    /// Returns the patched factor.
    #[must_use]
    pub const fn factor(&self) -> &Factor {
        &self.factor
    }

    /// Consumes the result and returns the patched factor.
    #[must_use]
    pub fn into_factor(self) -> Factor {
        self.factor
    }

    /// Returns all applied moves in application order.
    #[must_use]
    pub fn moves(&self) -> &[ThreeOpt111Move] {
        &self.moves
    }

    /// Returns the number of applied moves.
    #[must_use]
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// Returns the cycle count before greedy patching.
    #[must_use]
    pub const fn initial_cycle_count(&self) -> usize {
        self.initial_cycle_count
    }

    /// Returns the cycle count after greedy patching.
    #[must_use]
    pub const fn final_cycle_count(&self) -> usize {
        self.final_cycle_count
    }

    /// Returns the total cycle-count reduction.
    #[must_use]
    pub const fn cycle_reduction(&self) -> usize {
        self.initial_cycle_count - self.final_cycle_count
    }
}

/// Searches deterministically for the first valid 3-opt `1+1+1` move.
///
/// The search treats a move as an alternating six-edge configuration:
///
/// - remove a selected edge from cycle 1;
/// - cross to cycle 2 with an unselected graph edge;
/// - remove a selected edge from cycle 2;
/// - cross to cycle 3;
/// - remove a selected edge from cycle 3;
/// - cross back to cycle 1.
///
/// The three touched factor cycles must be distinct.
///
/// This avoids enumerating triples of cycles explicitly and exploits the low
/// average degree of power-sum graphs.
/// State shared by every candidate search over one decomposition.
///
/// Built once per scan: rebuilding it per move is what made the phase cost a
/// full pass over the factor for every atomic move.
struct Scan<'a> {
    graph: &'a PowerGraph,
    factor: &'a Factor,
    cycle_of: Vec<usize>,
    selected_neighbors: Vec<Option<[Vertex; 2]>>,
}

impl<'a> Scan<'a> {
    fn new(graph: &'a PowerGraph, factor: &'a Factor, decomposition: &CycleDecomposition) -> Self {
        let mut cycle_of = vec![usize::MAX; graph.vertex_count()];

        let mut selected_neighbors = vec![None; graph.vertex_count()];

        for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
            let vertices = cycle.vertices();

            for index in 0..vertices.len() {
                let vertex = vertices[index];

                let previous = vertices[if index == 0 {
                    vertices.len() - 1
                } else {
                    index - 1
                }];

                let next = vertices[(index + 1) % vertices.len()];

                cycle_of[vertex.index()] = cycle_index;

                selected_neighbors[vertex.index()] = Some([previous, next]);
            }
        }

        Self {
            graph,
            factor,
            cycle_of,
            selected_neighbors,
        }
    }

    /// First move merging `first_cycle` with two others, in deterministic
    /// order.
    ///
    /// `reserved`, when supplied, marks cycles already claimed by moves
    /// chosen earlier in the same batch.
    fn first_move_from(
        &self,
        first_cycle_index: usize,
        first_cycle: &Cycle,
        reserved: Option<&[bool]>,
    ) -> Result<Option<ThreeOpt111Move>, PatchError> {
        let graph = self.graph;
        let factor = self.factor;
        let cycle_of = &self.cycle_of;
        let selected_neighbors = &self.selected_neighbors;

        let claimed = |cycle_index: usize| reserved.is_some_and(|taken| taken[cycle_index]);

        let vertices = first_cycle.vertices();

        for index in 0..vertices.len() {
            let a = vertices[index];

            let b = vertices[(index + 1) % vertices.len()];

            let removed_1 = required_edge(graph, a, b)?;

            let incident_b = graph
                .incident(b)
                .expect("factor vertex must belong to graph");

            for &added_1 in incident_b {
                if factor.contains(added_1) {
                    continue;
                }

                let c = other_vertex(graph, added_1, b)?;

                let second_cycle_index = cycle_of[c.index()];

                if second_cycle_index == first_cycle_index || claimed(second_cycle_index) {
                    continue;
                }

                let neighbors_c = selected_neighbors[c.index()]
                    .expect("factor vertex must have two selected neighbours");

                for d in neighbors_c {
                    let removed_2 = required_edge(graph, c, d)?;

                    let incident_d = graph
                        .incident(d)
                        .expect("factor vertex must belong to graph");

                    for &added_2 in incident_d {
                        if factor.contains(added_2) {
                            continue;
                        }

                        let e = other_vertex(graph, added_2, d)?;

                        let third_cycle_index = cycle_of[e.index()];

                        if third_cycle_index == first_cycle_index
                            || third_cycle_index == second_cycle_index
                            || claimed(third_cycle_index)
                        {
                            continue;
                        }

                        let neighbors_e = selected_neighbors[e.index()]
                            .expect("factor vertex must have two selected neighbours");

                        for f in neighbors_e {
                            let removed_3 = required_edge(graph, e, f)?;

                            let Some(added_3) = graph.edge_between(f, a) else {
                                continue;
                            };

                            if factor.contains(added_3) {
                                continue;
                            }

                            let candidate = ThreeOpt111Move::new(
                                [removed_1, removed_2, removed_3],
                                [added_1, added_2, added_3],
                            );

                            if three_distinct(candidate.removed) && three_distinct(candidate.added)
                            {
                                return Ok(Some(candidate));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

pub fn find_three_opt_111(
    graph: &PowerGraph,
    factor: &Factor,
) -> Result<Option<ThreeOpt111Move>, PatchError> {
    factor
        .validate_two_factor(graph)
        .map_err(PatchError::Factor)?;

    let decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| PatchError::Decomposition {
            operation: "decompose factor",
            message: error.to_string(),
        })?;

    if decomposition.cycle_count() < 3 {
        return Ok(None);
    }

    let scan = Scan::new(graph, factor, &decomposition);

    for (first_cycle_index, first_cycle) in decomposition.cycles().iter().enumerate() {
        if let Some(movement) = scan.first_move_from(first_cycle_index, first_cycle, None)? {
            return Ok(Some(movement));
        }
    }

    Ok(None)
}

/// Finds pairwise independent 1+1+1 moves in a single scan.
///
/// A move merges three distinct cycles, so two moves are independent exactly
/// when their triples are disjoint. Collecting candidates is cheap here -
/// unlike the alternating repair there is no search to run per candidate - so
/// one scan can harvest many and pay a single factor rebuild instead of one
/// per move.
fn find_three_opt_111_batch(
    graph: &PowerGraph,
    factor: &Factor,
    stop: &dyn Fn() -> bool,
) -> Result<Vec<ThreeOpt111Move>, PatchError> {
    let decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| PatchError::Decomposition {
            operation: "decompose factor",
            message: error.to_string(),
        })?;

    if decomposition.cycle_count() < 3 {
        return Ok(Vec::new());
    }

    let scan = Scan::new(graph, factor, &decomposition);

    let mut reserved = vec![false; decomposition.cycle_count()];

    let mut moves = Vec::new();

    for (first_cycle_index, first_cycle) in decomposition.cycles().iter().enumerate() {
        if stop() {
            break;
        }

        if reserved[first_cycle_index] {
            continue;
        }

        let Some(movement) =
            scan.first_move_from(first_cycle_index, first_cycle, Some(&reserved))?
        else {
            continue;
        };

        for &edge_id in movement.removed() {
            let edge = graph
                .edge(edge_id)
                .expect("a move edge must belong to the graph");

            let (vertex, _) = edge.endpoints();

            reserved[scan.cycle_of[vertex.index()]] = true;
        }

        moves.push(movement);
    }

    Ok(moves)
}

/// Repeatedly applies the first deterministic 3-opt `1+1+1` move until no
/// further move exists.
///
/// Every successful move reduces the number of cycles by exactly two, so the
/// process always terminates.
///
/// This first implementation deliberately rebuilds the decomposition after
/// each move. That keeps the implementation simple and independently
/// verifiable. If profiling shows this to be expensive on the large `k = 5`
/// instance, the search state can later be made incremental without changing
/// the public semantics.
/// `stop` is polled between atomic moves. Returning `true` ends the phase
/// early and yields whatever was achieved so far, which lets a caller both
/// honour an interruption and checkpoint a long-running phase.
/// Applies independent moves in a single rebuild.
///
/// Returns `None` when the result is not a valid 2-factor with exactly two
/// fewer cycles per move, which tells the caller to fall back to applying
/// them one at a time.
fn apply_batch(
    graph: &PowerGraph,
    factor: &Factor,
    moves: &[ThreeOpt111Move],
    cycles_before: usize,
) -> Result<Option<Factor>, PatchError> {
    let mut selected = vec![false; graph.edge_count()];

    for &edge_id in factor.selected_edges() {
        selected[edge_id.index()] = true;
    }

    for movement in moves {
        for &edge_id in movement.removed() {
            if !selected[edge_id.index()] {
                return Ok(None);
            }

            selected[edge_id.index()] = false;
        }

        for &edge_id in movement.added() {
            if selected[edge_id.index()] {
                return Ok(None);
            }

            selected[edge_id.index()] = true;
        }
    }

    let edges = selected
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_selected)| is_selected.then_some(EdgeId::from_index(index)))
        .collect::<Vec<_>>();

    let candidate = Factor::new(graph, edges).map_err(PatchError::Factor)?;

    if candidate.validate_two_factor(graph).is_err() {
        return Ok(None);
    }

    let after =
        CycleDecomposition::new(graph, &candidate).map_err(|error| PatchError::Decomposition {
            operation: "decompose batched factor",
            message: error.to_string(),
        })?;

    if cycles_before.saturating_sub(after.cycle_count()) != 2 * moves.len() {
        return Ok(None);
    }

    Ok(Some(candidate))
}

pub fn greedy_three_opt_111(
    graph: &PowerGraph,
    factor: &Factor,
    stop: &dyn Fn() -> bool,
) -> Result<GreedyThreeOpt111Result, PatchError> {
    factor
        .validate_two_factor(graph)
        .map_err(PatchError::Factor)?;

    let initial_decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| PatchError::Decomposition {
            operation: "decompose initial greedy factor",
            message: error.to_string(),
        })?;

    let initial_cycle_count = initial_decomposition.cycle_count();

    let mut current =
        Factor::new(graph, factor.selected_edges().to_vec()).map_err(PatchError::Factor)?;

    let mut cycle_count = initial_cycle_count;

    let mut moves = Vec::<ThreeOpt111Move>::new();

    while !stop() {
        let batch = find_three_opt_111_batch(graph, &current, stop)?;

        if batch.is_empty() {
            break;
        }

        if let Some(next) = apply_batch(graph, &current, &batch, cycle_count)? {
            cycle_count -= 2 * batch.len();
            current = next;
            moves.extend(batch);
            continue;
        }

        /*
         * The batch did not compose. Keep only its first move, which is what
         * an unbatched pass would have chosen anyway, and rescan.
         */
        let movement = batch
            .into_iter()
            .next()
            .expect("a non-empty batch has a first move");

        current = movement.apply(graph, &current)?;

        cycle_count -= 2;

        moves.push(movement);
    }

    let final_decomposition =
        CycleDecomposition::new(graph, &current).map_err(|error| PatchError::Decomposition {
            operation: "decompose final greedy factor",
            message: error.to_string(),
        })?;

    let final_cycle_count = final_decomposition.cycle_count();

    let expected_reduction = moves.len() * 2;

    let actual_reduction = initial_cycle_count.saturating_sub(final_cycle_count);

    if actual_reduction != expected_reduction {
        return Err(PatchError::InvalidGreedyReduction {
            initial: initial_cycle_count,
            final_count: final_cycle_count,
            moves: moves.len(),
            expected_reduction,
            actual_reduction,
        });
    }

    Ok(GreedyThreeOpt111Result {
        factor: current,
        moves,
        initial_cycle_count,
        final_cycle_count,
    })
}

fn required_edge(graph: &PowerGraph, first: Vertex, second: Vertex) -> Result<EdgeId, PatchError> {
    graph
        .edge_between(first, second)
        .ok_or(PatchError::MissingGraphEdge {
            first: first.number(),
            second: second.number(),
        })
}

fn other_vertex(graph: &PowerGraph, edge_id: EdgeId, vertex: Vertex) -> Result<Vertex, PatchError> {
    let edge = graph
        .edge(edge_id)
        .ok_or(PatchError::InvalidEdgeId { edge_id })?;

    edge.other(vertex)
        .ok_or(PatchError::EdgeDoesNotContainVertex {
            edge_id,
            vertex: vertex.number(),
        })
}

fn three_distinct(values: [EdgeId; 3]) -> bool {
    values[0] != values[1] && values[0] != values[2] && values[1] != values[2]
}

/// Errors produced by factor patching.
#[derive(Debug)]
pub enum PatchError {
    Factor(FactorError),

    Decomposition {
        operation: &'static str,
        message: String,
    },

    TooFewCycles {
        cycle_count: usize,
    },

    DuplicateRemovedEdge,

    DuplicateAddedEdge,

    RemovedEdgeNotSelected {
        edge_id: EdgeId,
    },

    AddedEdgeAlreadySelected {
        edge_id: EdgeId,
    },

    InvalidEdgeId {
        edge_id: EdgeId,
    },

    EdgeDoesNotContainVertex {
        edge_id: EdgeId,
        vertex: usize,
    },

    MissingGraphEdge {
        first: usize,
        second: usize,
    },

    InvalidCycleReduction {
        before: usize,
        after: usize,
        expected: usize,
    },

    InvalidGreedyReduction {
        initial: usize,
        final_count: usize,
        moves: usize,
        expected_reduction: usize,
        actual_reduction: usize,
    },
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Factor(error) => {
                write!(f, "invalid factor: {error}")
            }

            Self::Decomposition { operation, message } => {
                write!(f, "cannot {operation}: {message}")
            }

            Self::TooFewCycles { cycle_count } => {
                write!(
                    f,
                    "3-opt 1+1+1 requires at least three cycles, \
                     but factor has {cycle_count}"
                )
            }

            Self::DuplicateRemovedEdge => {
                write!(f, "3-opt 1+1+1 contains duplicate removed edges")
            }

            Self::DuplicateAddedEdge => {
                write!(f, "3-opt 1+1+1 contains duplicate added edges")
            }

            Self::RemovedEdgeNotSelected { edge_id } => {
                write!(
                    f,
                    "3-opt 1+1+1 tries to remove unselected edge {:?}",
                    edge_id
                )
            }

            Self::AddedEdgeAlreadySelected { edge_id } => {
                write!(
                    f,
                    "3-opt 1+1+1 tries to add already selected edge {:?}",
                    edge_id
                )
            }

            Self::InvalidEdgeId { edge_id } => {
                write!(f, "invalid graph edge ID {:?}", edge_id)
            }

            Self::EdgeDoesNotContainVertex { edge_id, vertex } => {
                write!(
                    f,
                    "graph edge {:?} does not contain vertex {vertex}",
                    edge_id
                )
            }

            Self::MissingGraphEdge { first, second } => {
                write!(f, "required graph edge {first}--{second} does not exist")
            }

            Self::InvalidCycleReduction {
                before,
                after,
                expected,
            } => {
                write!(
                    f,
                    "3-opt 1+1+1 changed cycle count from {before} \
                     to {after}; expected {expected}"
                )
            }

            Self::InvalidGreedyReduction {
                initial,
                final_count,
                moves,
                expected_reduction,
                actual_reduction,
            } => {
                write!(
                    f,
                    "greedy 3-opt 1+1+1 changed cycle count from \
                     {initial} to {final_count} using {moves} moves; \
                     expected reduction {expected_reduction}, \
                     observed {actual_reduction}"
                )
            }
        }
    }
}

impl Error for PatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Factor(error) => Some(error),

            Self::Decomposition { .. }
            | Self::TooFewCycles { .. }
            | Self::DuplicateRemovedEdge
            | Self::DuplicateAddedEdge
            | Self::RemovedEdgeNotSelected { .. }
            | Self::AddedEdgeAlreadySelected { .. }
            | Self::InvalidEdgeId { .. }
            | Self::EdgeDoesNotContainVertex { .. }
            | Self::MissingGraphEdge { .. }
            | Self::InvalidCycleReduction { .. }
            | Self::InvalidGreedyReduction { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(graph: &PowerGraph, number: usize) -> Vertex {
        graph.vertex(number).unwrap()
    }

    fn edge_id(graph: &PowerGraph, first: usize, second: usize) -> EdgeId {
        graph
            .edge_between(vertex(graph, first), vertex(graph, second))
            .unwrap()
    }

    fn factor_from_edges(graph: &PowerGraph, edges: &[(usize, usize)]) -> Factor {
        let selected_edges = edges
            .iter()
            .map(|&(first, second)| edge_id(graph, first, second))
            .collect::<Vec<_>>();

        let factor = Factor::new(graph, selected_edges).unwrap();

        factor.validate_two_factor(graph).unwrap();

        factor
    }

    fn three_cycle_factor(graph: &PowerGraph) -> Factor {
        factor_from_edges(
            graph,
            &[
                (1, 15),
                (1, 24),
                (2, 23),
                (2, 34),
                (3, 22),
                (3, 33),
                (4, 21),
                (4, 32),
                (5, 20),
                (5, 31),
                (6, 10),
                (6, 19),
                (7, 18),
                (7, 29),
                (8, 17),
                (8, 28),
                (9, 27),
                (9, 40),
                (10, 39),
                (11, 25),
                (11, 38),
                (12, 13),
                (12, 37),
                (13, 36),
                (14, 22),
                (14, 35),
                (15, 21),
                (16, 20),
                (16, 33),
                (17, 32),
                (18, 31),
                (19, 30),
                (23, 26),
                (24, 40),
                (25, 39),
                (26, 38),
                (27, 37),
                (28, 36),
                (29, 35),
                (30, 34),
            ],
        )
    }

    fn two_cycle_factor(graph: &PowerGraph) -> Factor {
        let selected_indices = [
            1usize, 4, 5, 6, 7, 10, 13, 14, 18, 20, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 35,
            36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        ];

        let selected_edges = selected_indices
            .into_iter()
            .map(EdgeId::from_index)
            .collect::<Vec<_>>();

        let factor = Factor::new(graph, selected_edges).unwrap();

        factor.validate_two_factor(graph).unwrap();

        factor
    }

    #[test]
    fn fixture_has_three_cycles() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut sizes = decomposition
            .cycles()
            .iter()
            .map(|cycle| cycle.len())
            .collect::<Vec<_>>();

        sizes.sort_unstable();

        assert_eq!(sizes, [12, 12, 16,]);
    }

    #[test]
    fn known_three_opt_111_merges_three_cycles() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let movement = ThreeOpt111Move::new(
            [
                edge_id(&graph, 1, 15),
                edge_id(&graph, 2, 34),
                edge_id(&graph, 14, 35),
            ],
            [
                edge_id(&graph, 15, 34),
                edge_id(&graph, 2, 14),
                edge_id(&graph, 1, 35),
            ],
        );

        let patched = movement.apply(&graph, &factor).unwrap();

        patched.validate_two_factor(&graph).unwrap();

        assert_eq!(patched.edge_count(), 40);

        let decomposition = CycleDecomposition::new(&graph, &patched).unwrap();

        assert_eq!(decomposition.cycle_count(), 1);

        assert_eq!(decomposition.largest_cycle().len(), 40);

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn finds_three_opt_111_move() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let movement = find_three_opt_111(&graph, &factor)
            .unwrap()
            .expect("fixture must contain a 3-opt 1+1+1 move");

        let patched = movement.apply(&graph, &factor).unwrap();

        let decomposition = CycleDecomposition::new(&graph, &patched).unwrap();

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn greedy_three_opt_111_merges_fixture_completely() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let result = greedy_three_opt_111(&graph, &factor, &|| false).unwrap();

        assert_eq!(result.initial_cycle_count(), 3);

        assert_eq!(result.move_count(), 1);

        assert_eq!(result.cycle_reduction(), 2);

        assert_eq!(result.final_cycle_count(), 1);

        let decomposition = CycleDecomposition::new(&graph, result.factor()).unwrap();

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn greedy_three_opt_111_is_noop_when_no_move_is_possible() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let result = greedy_three_opt_111(&graph, &factor, &|| false).unwrap();

        assert_eq!(result.initial_cycle_count(), 2);

        assert_eq!(result.final_cycle_count(), 2);

        assert_eq!(result.move_count(), 0);

        assert_eq!(result.cycle_reduction(), 0);

        assert_eq!(result.factor().selected_edges(), factor.selected_edges());
    }

    #[test]
    fn search_is_deterministic() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let first = find_three_opt_111(&graph, &factor).unwrap();

        let second = find_three_opt_111(&graph, &factor).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn returns_none_with_fewer_than_three_cycles() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        assert_eq!(decomposition.cycle_count(), 2);

        assert_eq!(find_three_opt_111(&graph, &factor,).unwrap(), None);
    }

    #[test]
    fn rejects_removing_unselected_edge() {
        let graph = PowerGraph::new(2, 40).unwrap();

        let factor = three_cycle_factor(&graph);

        let movement = ThreeOpt111Move::new(
            [
                edge_id(&graph, 15, 34),
                edge_id(&graph, 2, 34),
                edge_id(&graph, 14, 35),
            ],
            [
                edge_id(&graph, 15, 34),
                edge_id(&graph, 2, 14),
                edge_id(&graph, 1, 35),
            ],
        );

        assert!(matches!(
            movement.apply(&graph, &factor,),
            Err(PatchError::RemovedEdgeNotSelected { .. })
        ));
    }
}
