use std::error::Error;
use std::fmt;

use powercycle_core::{Cycle, CycleDecomposition, EdgeId, Factor, FactorError, PowerGraph, Vertex};

/// A 3-opt `2+1` move.
///
/// The move removes:
///
/// - two selected edges from the current largest cycle;
/// - one selected edge from one satellite cycle.
///
/// It then adds three unselected graph edges.
///
/// A valid move merges the largest cycle with that satellite and therefore
/// decreases the total cycle count by exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreeOpt21Move {
    removed: [EdgeId; 3],
    added: [EdgeId; 3],
}

impl ThreeOpt21Move {
    /// Creates a move from explicit graph edge IDs.
    ///
    /// Full 2+1 semantics are validated when `apply` is called.
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

    /// Applies a valid 3-opt `2+1` move.
    ///
    /// Exactly two removed edges must belong to the current largest cycle,
    /// while the third removed edge must belong to one satellite.
    ///
    /// The patched factor is checked to remain a valid 2-factor and to have
    /// exactly one fewer cycle.
    pub fn apply(&self, graph: &PowerGraph, factor: &Factor) -> Result<Factor, ThreeOpt21Error> {
        factor
            .validate_two_factor(graph)
            .map_err(ThreeOpt21Error::Factor)?;

        if !three_distinct(self.removed) {
            return Err(ThreeOpt21Error::DuplicateRemovedEdge);
        }

        if !three_distinct(self.added) {
            return Err(ThreeOpt21Error::DuplicateAddedEdge);
        }

        for &edge_id in &self.removed {
            if graph.edge(edge_id).is_none() {
                return Err(ThreeOpt21Error::InvalidEdgeId { edge_id });
            }

            if !factor.contains(edge_id) {
                return Err(ThreeOpt21Error::RemovedEdgeNotSelected { edge_id });
            }
        }

        for &edge_id in &self.added {
            if graph.edge(edge_id).is_none() {
                return Err(ThreeOpt21Error::InvalidEdgeId { edge_id });
            }

            if factor.contains(edge_id) {
                return Err(ThreeOpt21Error::AddedEdgeAlreadySelected { edge_id });
            }
        }

        let before = CycleDecomposition::new(graph, factor).map_err(|error| {
            ThreeOpt21Error::Decomposition {
                operation: "decompose input factor",
                message: error.to_string(),
            }
        })?;

        if before.cycle_count() < 2 {
            return Err(ThreeOpt21Error::TooFewCycles {
                cycle_count: before.cycle_count(),
            });
        }

        validate_removed_cycle_pattern(graph, &before, self.removed)?;

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

        let result = Factor::new(graph, selected_edges).map_err(ThreeOpt21Error::Factor)?;

        result
            .validate_two_factor(graph)
            .map_err(ThreeOpt21Error::Factor)?;

        let after = CycleDecomposition::new(graph, &result).map_err(|error| {
            ThreeOpt21Error::Decomposition {
                operation: "decompose patched factor",
                message: error.to_string(),
            }
        })?;

        let expected = before.cycle_count().saturating_sub(1);

        if after.cycle_count() != expected {
            return Err(ThreeOpt21Error::InvalidCycleReduction {
                before: before.cycle_count(),
                after: after.cycle_count(),
                expected,
            });
        }

        Ok(result)
    }
}

/// Result of deterministic greedy 3-opt `2+1` patching.
pub struct GreedyThreeOpt21Result {
    factor: Factor,
    moves: Vec<ThreeOpt21Move>,
    initial_cycle_count: usize,
    final_cycle_count: usize,
}

impl GreedyThreeOpt21Result {
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
    pub fn moves(&self) -> &[ThreeOpt21Move] {
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

/// Searches deterministically for the first valid 3-opt `2+1` move.
///
/// The search is satellite-first.
///
/// For every selected satellite edge `e--f`, it searches for two unselected
/// edges connecting `e` and `f` to the current largest cycle. It then chooses
/// one selected giant edge incident to each landing vertex and checks whether
/// the two remaining giant endpoints are adjacent in the graph.
///
/// Conceptually:
///
/// satellite edge:
///
/// `e -- f`
///
/// giant edges:
///
/// `a -- b` and `c -- d`
///
/// are replaced by:
///
/// `b -- e`, `f -- d`, and `c -- a`.
///
/// Complete candidates are validated before being returned.
///
/// Searching from satellites is especially efficient once a large giant has
/// formed, because the work is then driven primarily by the residual vertex
/// set rather than by the size of the giant.
/// State shared by every candidate search over one decomposition.
///
/// Building it costs a decomposition and a few vertex-sized arrays, which is
/// why the batch search reuses a single instance across all satellites rather
/// than rebuilding one per move.
struct Scan<'a> {
    graph: &'a PowerGraph,
    factor: &'a Factor,
    largest_cycle_index: usize,
    cycle_of: Vec<usize>,
    selected_neighbors: Vec<Option<[Vertex; 2]>>,
    giant_position: Vec<usize>,
    giant_length: usize,
}

impl<'a> Scan<'a> {
    fn new(graph: &'a PowerGraph, factor: &'a Factor, decomposition: &CycleDecomposition) -> Self {
        let largest_cycle_index = decomposition.largest_cycle_index();

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

        /*
         * Position of every giant vertex along the giant cycle.
         *
         * A candidate removes two giant edges, cutting the giant into two
         * arcs, and adds three. Whether those reconnect everything into one
         * cycle depends only on how the two removed edges are oriented
         * relative to each other: consistently oriented, the satellite path
         * bridges one arc while the third added edge closes the other;
         * oppositely oriented, that third edge closes one arc on itself and
         * two cycles remain.
         *
         * Validity is therefore an O(1) test rather than a full apply.
         */
        let giant_vertices = decomposition.largest_cycle().vertices();

        let mut giant_position = vec![usize::MAX; graph.vertex_count()];

        for (position, vertex) in giant_vertices.iter().enumerate() {
            giant_position[vertex.index()] = position;
        }

        Self {
            graph,
            factor,
            largest_cycle_index,
            cycle_of,
            selected_neighbors,
            giant_position,
            giant_length: giant_vertices.len(),
        }
    }

    fn follows(&self, from: Vertex, to: Vertex) -> bool {
        (self.giant_position[from.index()] + 1) % self.giant_length
            == self.giant_position[to.index()]
    }

    /// First valid move absorbing `satellite`, in deterministic order.
    ///
    /// `reserved`, when supplied, marks giant vertices already claimed by
    /// moves chosen earlier in the same batch.
    fn first_move_for(
        &self,
        satellite: &Cycle,
        reserved: Option<&[bool]>,
    ) -> Result<Option<ThreeOpt21Move>, ThreeOpt21Error> {
        let graph = self.graph;
        let factor = self.factor;

        let claimed = |vertex: Vertex| reserved.is_some_and(|taken| taken[vertex.index()]);

        let vertices = satellite.vertices();

        for index in 0..vertices.len() {
            let e = vertices[index];

            let f = vertices[(index + 1) % vertices.len()];

            let removed_satellite = required_edge(graph, e, f)?;

            for (satellite_left, satellite_right) in [(e, f), (f, e)] {
                let incident_left = graph
                    .incident(satellite_left)
                    .expect("satellite vertex must belong to graph");

                for &added_left in incident_left {
                    if factor.contains(added_left) {
                        continue;
                    }

                    let giant_left = other_vertex(graph, added_left, satellite_left)?;

                    if self.cycle_of[giant_left.index()] != self.largest_cycle_index
                        || claimed(giant_left)
                    {
                        continue;
                    }

                    let giant_left_neighbors = self.selected_neighbors[giant_left.index()]
                        .expect("giant vertex must have two selected neighbours");

                    let incident_right = graph
                        .incident(satellite_right)
                        .expect("satellite vertex must belong to graph");

                    for &added_right in incident_right {
                        if factor.contains(added_right) {
                            continue;
                        }

                        let giant_right = other_vertex(graph, added_right, satellite_right)?;

                        if self.cycle_of[giant_right.index()] != self.largest_cycle_index
                            || claimed(giant_right)
                        {
                            continue;
                        }

                        if giant_left == giant_right {
                            continue;
                        }

                        let giant_right_neighbors = self.selected_neighbors[giant_right.index()]
                            .expect("giant vertex must have two selected neighbours");

                        for giant_left_other in giant_left_neighbors {
                            if claimed(giant_left_other) {
                                continue;
                            }

                            let removed_left = required_edge(graph, giant_left, giant_left_other)?;

                            for giant_right_other in giant_right_neighbors {
                                if claimed(giant_right_other) {
                                    continue;
                                }

                                let removed_right =
                                    required_edge(graph, giant_right, giant_right_other)?;

                                if !three_distinct([removed_left, removed_right, removed_satellite])
                                {
                                    continue;
                                }

                                let Some(added_giant) =
                                    graph.edge_between(giant_left_other, giant_right_other)
                                else {
                                    continue;
                                };

                                if factor.contains(added_giant) {
                                    continue;
                                }

                                if !three_distinct([added_left, added_right, added_giant]) {
                                    continue;
                                }

                                if self.follows(giant_left, giant_left_other)
                                    != self.follows(giant_right, giant_right_other)
                                {
                                    // Opposite orientations: the toggle keeps
                                    // degree two but leaves two cycles.
                                    continue;
                                }

                                let candidate = ThreeOpt21Move::new(
                                    [removed_left, removed_right, removed_satellite],
                                    [added_left, added_right, added_giant],
                                );

                                debug_assert!(
                                    reserved.is_some() || candidate.apply(graph, factor).is_ok(),
                                    "orientation test disagrees with a full apply"
                                );

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

pub fn find_three_opt_21(
    graph: &PowerGraph,
    factor: &Factor,
) -> Result<Option<ThreeOpt21Move>, ThreeOpt21Error> {
    factor
        .validate_two_factor(graph)
        .map_err(ThreeOpt21Error::Factor)?;

    let decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| ThreeOpt21Error::Decomposition {
            operation: "decompose factor",
            message: error.to_string(),
        })?;

    if decomposition.cycle_count() < 2 {
        return Ok(None);
    }

    let scan = Scan::new(graph, factor, &decomposition);

    for (satellite_index, satellite) in decomposition.cycles().iter().enumerate() {
        if satellite_index == scan.largest_cycle_index {
            continue;
        }

        if let Some(movement) = scan.first_move_for(satellite, None)? {
            return Ok(Some(movement));
        }
    }

    Ok(None)
}

/// Finds pairwise independent 2+1 moves in a single scan.
///
/// Two moves are independent when the giant vertices they touch are disjoint,
/// so neither disturbs the arcs the other relies on. Applying such a set costs
/// one factor rebuild instead of one per move, which is the difference between
/// minutes and days once the giant has tens of millions of edges.
fn find_three_opt_21_batch(
    graph: &PowerGraph,
    factor: &Factor,
    stop: &dyn Fn() -> bool,
) -> Result<Vec<ThreeOpt21Move>, ThreeOpt21Error> {
    let decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| ThreeOpt21Error::Decomposition {
            operation: "decompose factor",
            message: error.to_string(),
        })?;

    if decomposition.cycle_count() < 2 {
        return Ok(Vec::new());
    }

    let scan = Scan::new(graph, factor, &decomposition);

    let mut reserved = vec![false; graph.vertex_count()];

    let mut moves = Vec::new();

    for (satellite_index, satellite) in decomposition.cycles().iter().enumerate() {
        if satellite_index == scan.largest_cycle_index {
            continue;
        }

        if stop() {
            break;
        }

        let Some(movement) = scan.first_move_for(satellite, Some(&reserved))? else {
            continue;
        };

        for &edge_id in movement.removed() {
            let edge = graph
                .edge(edge_id)
                .expect("a move edge must belong to the graph");

            let (first, second) = edge.endpoints();

            reserved[first.index()] = true;
            reserved[second.index()] = true;
        }

        moves.push(movement);
    }

    Ok(moves)
}

/// Repeatedly absorbs satellites into the current largest cycle using
/// deterministic 3-opt `2+1` moves.
///
/// Each successful move reduces the total cycle count by exactly one and
/// merges one complete satellite into the current largest cycle.
///
/// The process terminates when no further direct 2+1 absorption is found.
/// `stop` is polled between atomic moves, as in [`greedy_three_opt_111`].
/// Applies independent moves in a single rebuild.
///
/// Returns `None` when the result is not a valid 2-factor with exactly one
/// fewer cycle per move, which tells the caller to fall back to applying them
/// one at a time. Independence is argued, not proved, so the outcome is
/// checked rather than trusted.
fn apply_batch(
    graph: &PowerGraph,
    factor: &Factor,
    moves: &[ThreeOpt21Move],
    cycles_before: usize,
) -> Result<Option<Factor>, ThreeOpt21Error> {
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

    let candidate = Factor::new(graph, edges).map_err(ThreeOpt21Error::Factor)?;

    if candidate.validate_two_factor(graph).is_err() {
        return Ok(None);
    }

    let after = CycleDecomposition::new(graph, &candidate).map_err(|error| {
        ThreeOpt21Error::Decomposition {
            operation: "decompose batched factor",
            message: error.to_string(),
        }
    })?;

    if cycles_before.saturating_sub(after.cycle_count()) != moves.len() {
        return Ok(None);
    }

    Ok(Some(candidate))
}

pub fn greedy_three_opt_21(
    graph: &PowerGraph,
    factor: &Factor,
    stop: &dyn Fn() -> bool,
) -> Result<GreedyThreeOpt21Result, ThreeOpt21Error> {
    factor
        .validate_two_factor(graph)
        .map_err(ThreeOpt21Error::Factor)?;

    let initial_decomposition =
        CycleDecomposition::new(graph, factor).map_err(|error| ThreeOpt21Error::Decomposition {
            operation: "decompose initial greedy factor",
            message: error.to_string(),
        })?;

    let initial_cycle_count = initial_decomposition.cycle_count();

    let mut current =
        Factor::new(graph, factor.selected_edges().to_vec()).map_err(ThreeOpt21Error::Factor)?;

    let mut cycle_count = initial_cycle_count;

    let mut moves = Vec::<ThreeOpt21Move>::new();

    while !stop() {
        let batch = find_three_opt_21_batch(graph, &current, stop)?;

        if batch.is_empty() {
            break;
        }

        if let Some(next) = apply_batch(graph, &current, &batch, cycle_count)? {
            cycle_count -= batch.len();
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

        cycle_count -= 1;

        moves.push(movement);
    }

    let final_decomposition = CycleDecomposition::new(graph, &current).map_err(|error| {
        ThreeOpt21Error::Decomposition {
            operation: "decompose final greedy factor",
            message: error.to_string(),
        }
    })?;

    let final_cycle_count = final_decomposition.cycle_count();

    let expected_reduction = moves.len();

    let actual_reduction = initial_cycle_count.saturating_sub(final_cycle_count);

    if actual_reduction != expected_reduction {
        return Err(ThreeOpt21Error::InvalidGreedyReduction {
            initial: initial_cycle_count,
            final_count: final_cycle_count,
            moves: moves.len(),
            expected_reduction,
            actual_reduction,
        });
    }

    Ok(GreedyThreeOpt21Result {
        factor: current,
        moves,
        initial_cycle_count,
        final_cycle_count,
    })
}

fn validate_removed_cycle_pattern(
    graph: &PowerGraph,
    decomposition: &CycleDecomposition,
    removed: [EdgeId; 3],
) -> Result<(), ThreeOpt21Error> {
    let mut cycle_of = vec![usize::MAX; graph.vertex_count()];

    for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
        for &vertex in cycle.vertices() {
            cycle_of[vertex.index()] = cycle_index;
        }
    }

    let mut removed_cycles = [usize::MAX; 3];

    for (position, edge_id) in removed.into_iter().enumerate() {
        let edge = graph
            .edge(edge_id)
            .ok_or(ThreeOpt21Error::InvalidEdgeId { edge_id })?;

        let first_cycle = cycle_of[edge.first().index()];

        let second_cycle = cycle_of[edge.second().index()];

        if first_cycle != second_cycle {
            return Err(ThreeOpt21Error::RemovedEdgeCrossesCycles {
                edge_id,
                first_cycle,
                second_cycle,
            });
        }

        removed_cycles[position] = first_cycle;
    }

    let largest_cycle_index = decomposition.largest_cycle_index();

    if !is_two_plus_one_pattern(removed_cycles, largest_cycle_index) {
        return Err(ThreeOpt21Error::InvalidRemovedCyclePattern {
            largest_cycle_index,
            removed_cycle_indices: removed_cycles,
        });
    }

    Ok(())
}

fn is_two_plus_one_pattern(cycle_indices: [usize; 3], largest_cycle_index: usize) -> bool {
    let giant_count = cycle_indices
        .iter()
        .filter(|&&cycle_index| cycle_index == largest_cycle_index)
        .count();

    if giant_count != 2 {
        return false;
    }

    cycle_indices
        .iter()
        .filter(|&&cycle_index| cycle_index != largest_cycle_index)
        .count()
        == 1
}

fn required_edge(
    graph: &PowerGraph,
    first: Vertex,
    second: Vertex,
) -> Result<EdgeId, ThreeOpt21Error> {
    graph
        .edge_between(first, second)
        .ok_or(ThreeOpt21Error::MissingGraphEdge {
            first: first.number(),
            second: second.number(),
        })
}

fn other_vertex(
    graph: &PowerGraph,
    edge_id: EdgeId,
    vertex: Vertex,
) -> Result<Vertex, ThreeOpt21Error> {
    let edge = graph
        .edge(edge_id)
        .ok_or(ThreeOpt21Error::InvalidEdgeId { edge_id })?;

    edge.other(vertex)
        .ok_or(ThreeOpt21Error::EdgeDoesNotContainVertex {
            edge_id,
            vertex: vertex.number(),
        })
}

fn three_distinct(values: [EdgeId; 3]) -> bool {
    values[0] != values[1] && values[0] != values[2] && values[1] != values[2]
}

/// Errors produced by 3-opt `2+1` patching.
#[derive(Debug)]
pub enum ThreeOpt21Error {
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

    RemovedEdgeCrossesCycles {
        edge_id: EdgeId,
        first_cycle: usize,
        second_cycle: usize,
    },

    InvalidRemovedCyclePattern {
        largest_cycle_index: usize,
        removed_cycle_indices: [usize; 3],
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

impl fmt::Display for ThreeOpt21Error {
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
                    "3-opt 2+1 requires at least two cycles, \
                     but factor has {cycle_count}"
                )
            }

            Self::DuplicateRemovedEdge => {
                write!(f, "3-opt 2+1 contains duplicate removed edges")
            }

            Self::DuplicateAddedEdge => {
                write!(f, "3-opt 2+1 contains duplicate added edges")
            }

            Self::RemovedEdgeNotSelected { edge_id } => {
                write!(f, "3-opt 2+1 tries to remove unselected edge {:?}", edge_id)
            }

            Self::AddedEdgeAlreadySelected { edge_id } => {
                write!(
                    f,
                    "3-opt 2+1 tries to add already selected edge {:?}",
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

            Self::RemovedEdgeCrossesCycles {
                edge_id,
                first_cycle,
                second_cycle,
            } => {
                write!(
                    f,
                    "selected edge {:?} unexpectedly joins factor cycles \
                     {first_cycle} and {second_cycle}",
                    edge_id
                )
            }

            Self::InvalidRemovedCyclePattern {
                largest_cycle_index,
                removed_cycle_indices,
            } => {
                write!(
                    f,
                    "3-opt 2+1 must remove two edges from largest cycle \
                     {largest_cycle_index} and one edge from one satellite; \
                     removed edges belong to cycles {:?}",
                    removed_cycle_indices
                )
            }

            Self::InvalidCycleReduction {
                before,
                after,
                expected,
            } => {
                write!(
                    f,
                    "3-opt 2+1 changed cycle count from {before} \
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
                    "greedy 3-opt 2+1 changed cycle count from \
                     {initial} to {final_count} using {moves} moves; \
                     expected reduction {expected_reduction}, \
                     observed {actual_reduction}"
                )
            }
        }
    }
}

impl Error for ThreeOpt21Error {
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
            | Self::RemovedEdgeCrossesCycles { .. }
            | Self::InvalidRemovedCyclePattern { .. }
            | Self::InvalidCycleReduction { .. }
            | Self::InvalidGreedyReduction { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_cycle_factor(graph: &PowerGraph) -> Factor {
        let selected_indices = [
            1usize, 3, 6, 7, 8, 9, 12, 13, 18, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 34, 35,
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

    fn known_move() -> ThreeOpt21Move {
        ThreeOpt21Move::new(
            [
                EdgeId::from_index(35),
                EdgeId::from_index(12),
                EdgeId::from_index(8),
            ],
            [
                EdgeId::from_index(15),
                EdgeId::from_index(32),
                EdgeId::from_index(5),
            ],
        )
    }

    #[test]
    fn fixture_has_two_cycles_23_and_9() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut sizes = decomposition
            .cycles()
            .iter()
            .map(|cycle| cycle.len())
            .collect::<Vec<_>>();

        sizes.sort_unstable();

        assert_eq!(sizes, [9, 23,]);

        assert_eq!(decomposition.largest_cycle().len(), 23);
    }

    #[test]
    fn known_three_opt_21_merges_two_cycles() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let movement = known_move();

        let patched = movement.apply(&graph, &factor).unwrap();

        patched.validate_two_factor(&graph).unwrap();

        assert_eq!(patched.edge_count(), 32);

        let decomposition = CycleDecomposition::new(&graph, &patched).unwrap();

        assert_eq!(decomposition.cycle_count(), 1);

        assert_eq!(decomposition.largest_cycle().len(), 32);

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn finds_three_opt_21_move() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let movement = find_three_opt_21(&graph, &factor)
            .unwrap()
            .expect("fixture must contain a 3-opt 2+1 move");

        let patched = movement.apply(&graph, &factor).unwrap();

        let decomposition = CycleDecomposition::new(&graph, &patched).unwrap();

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn greedy_three_opt_21_merges_fixture_completely() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let result = greedy_three_opt_21(&graph, &factor, &|| false).unwrap();

        assert_eq!(result.initial_cycle_count(), 2);

        assert_eq!(result.move_count(), 1);

        assert_eq!(result.cycle_reduction(), 1);

        assert_eq!(result.final_cycle_count(), 1);

        let decomposition = CycleDecomposition::new(&graph, result.factor()).unwrap();

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn greedy_three_opt_21_is_noop_on_hamiltonian_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let hamiltonian = known_move().apply(&graph, &factor).unwrap();

        let result = greedy_three_opt_21(&graph, &hamiltonian, &|| false).unwrap();

        assert_eq!(result.initial_cycle_count(), 1);

        assert_eq!(result.final_cycle_count(), 1);

        assert_eq!(result.move_count(), 0);

        assert_eq!(result.cycle_reduction(), 0);
    }

    #[test]
    fn search_is_deterministic() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let first = find_three_opt_21(&graph, &factor).unwrap();

        let second = find_three_opt_21(&graph, &factor).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn returns_none_for_hamiltonian_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let hamiltonian = known_move().apply(&graph, &factor).unwrap();

        assert_eq!(find_three_opt_21(&graph, &hamiltonian,).unwrap(), None);
    }
}
