use std::error::Error;
use std::fmt;

use powercycle_core::{CycleDecomposition, EdgeId, Factor, FactorError, PowerGraph, Vertex};

/// An alternating repair that absorbs one satellite into the current giant.
///
/// The repair starts from a quasi 3-opt `2+1` move:
///
/// - one selected satellite edge is removed;
/// - two selected giant edges are removed;
/// - two satellite-to-giant graph edges are added.
///
/// This leaves exactly two degree-one deficits in the giant.
///
/// One deficit is then moved through the giant by alternating:
///
/// - add one originally unselected giant edge;
/// - remove one originally selected giant edge.
///
/// When the moving deficit becomes adjacent to the fixed deficit, one final
/// unselected graph edge closes the repair.
///
/// `depth` is the number of alternating deficit-transfer steps. A depth-zero
/// repair would simply be a direct 3-opt `2+1` move and is intentionally not
/// returned by `find_alternating_repair`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlternatingRepairMove {
    removed: Vec<EdgeId>,
    added: Vec<EdgeId>,
    depth: usize,
    absorbed_vertices: usize,
}

impl AlternatingRepairMove {
    /// Creates an explicit alternating repair move.
    ///
    /// Full validity is checked by `apply`.
    #[must_use]
    pub fn new(
        removed: Vec<EdgeId>,
        added: Vec<EdgeId>,
        depth: usize,
        absorbed_vertices: usize,
    ) -> Self {
        Self {
            removed,
            added,
            depth,
            absorbed_vertices,
        }
    }

    /// Returns the selected edges removed by the repair.
    #[must_use]
    pub fn removed(&self) -> &[EdgeId] {
        &self.removed
    }

    /// Returns the graph edges added by the repair.
    #[must_use]
    pub fn added(&self) -> &[EdgeId] {
        &self.added
    }

    /// Returns the alternating repair depth.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the number of vertices in the absorbed satellite.
    #[must_use]
    pub const fn absorbed_vertices(&self) -> usize {
        self.absorbed_vertices
    }

    /// Applies and validates the repair.
    ///
    /// A valid repair must:
    ///
    /// - preserve degree two at every vertex;
    /// - decrease the cycle count by exactly one;
    /// - increase the largest cycle by exactly the size of the targeted
    ///   satellite.
    pub fn apply(
        &self,
        graph: &PowerGraph,
        factor: &Factor,
    ) -> Result<Factor, AlternatingRepairError> {
        factor
            .validate_two_factor(graph)
            .map_err(AlternatingRepairError::Factor)?;

        if self.depth == 0 {
            return Err(AlternatingRepairError::ZeroDepth);
        }

        if self.absorbed_vertices == 0 {
            return Err(AlternatingRepairError::ZeroAbsorbedVertices);
        }

        if self.removed.len() != self.added.len() {
            return Err(AlternatingRepairError::UnequalToggleCounts {
                removed: self.removed.len(),
                added: self.added.len(),
            });
        }

        if !all_distinct(&self.removed) {
            return Err(AlternatingRepairError::DuplicateRemovedEdge);
        }

        if !all_distinct(&self.added) {
            return Err(AlternatingRepairError::DuplicateAddedEdge);
        }

        for &edge_id in &self.removed {
            if graph.edge(edge_id).is_none() {
                return Err(AlternatingRepairError::InvalidEdgeId { edge_id });
            }

            if !factor.contains(edge_id) {
                return Err(AlternatingRepairError::RemovedEdgeNotSelected { edge_id });
            }
        }

        for &edge_id in &self.added {
            if graph.edge(edge_id).is_none() {
                return Err(AlternatingRepairError::InvalidEdgeId { edge_id });
            }

            if factor.contains(edge_id) {
                return Err(AlternatingRepairError::AddedEdgeAlreadySelected { edge_id });
            }
        }

        let before = CycleDecomposition::new(graph, factor).map_err(|error| {
            AlternatingRepairError::Decomposition {
                operation: "decompose input factor",
                message: error.to_string(),
            }
        })?;

        if before.cycle_count() < 2 {
            return Err(AlternatingRepairError::TooFewCycles {
                cycle_count: before.cycle_count(),
            });
        }

        let giant_before = before.largest_cycle().len();

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

        let result = Factor::new(graph, selected_edges).map_err(AlternatingRepairError::Factor)?;

        result
            .validate_two_factor(graph)
            .map_err(AlternatingRepairError::Factor)?;

        let after = CycleDecomposition::new(graph, &result).map_err(|error| {
            AlternatingRepairError::Decomposition {
                operation: "decompose repaired factor",
                message: error.to_string(),
            }
        })?;

        let expected_cycles = before.cycle_count().saturating_sub(1);

        if after.cycle_count() != expected_cycles {
            return Err(AlternatingRepairError::InvalidCycleReduction {
                before: before.cycle_count(),
                after: after.cycle_count(),
                expected: expected_cycles,
            });
        }

        let expected_giant = giant_before + self.absorbed_vertices;

        let giant_after = after.largest_cycle().len();

        if giant_after != expected_giant {
            return Err(AlternatingRepairError::InvalidGiantGrowth {
                before: giant_before,
                after: giant_after,
                absorbed: self.absorbed_vertices,
                expected: expected_giant,
            });
        }

        Ok(result)
    }
}

/// Finds the first deterministic alternating repair with depth at most
/// `max_depth`.
///
/// Depth zero is deliberately excluded because it is exactly the direct
/// 3-opt `2+1` case handled by `find_three_opt_21`.
///
/// The search is satellite-first. This is well suited to the intended
/// late-stage situation where one giant contains almost every vertex and
/// only a few small satellites remain.
/// `stop` is polled once per satellite. Returning `true` aborts the search
/// with [`AlternatingRepairError::Interrupted`], which a caller must not
/// confuse with "no repair exists at this depth".
pub fn find_alternating_repair(
    graph: &PowerGraph,
    factor: &Factor,
    max_depth: usize,
    stop: &dyn Fn() -> bool,
) -> Result<Option<AlternatingRepairMove>, AlternatingRepairError> {
    factor
        .validate_two_factor(graph)
        .map_err(AlternatingRepairError::Factor)?;

    if max_depth == 0 {
        return Ok(None);
    }

    let decomposition = CycleDecomposition::new(graph, factor).map_err(|error| {
        AlternatingRepairError::Decomposition {
            operation: "decompose factor",
            message: error.to_string(),
        }
    })?;

    if decomposition.cycle_count() < 2 {
        return Ok(None);
    }

    let giant_index = decomposition.largest_cycle_index();

    let mut in_giant = vec![false; graph.vertex_count()];

    let mut neighbours = vec![[0u32; 2]; graph.vertex_count()];

    for cycle in decomposition.cycles() {
        let vertices = cycle.vertices();

        for position in 0..vertices.len() {
            let vertex = vertices[position];

            let previous = vertices[if position == 0 {
                vertices.len() - 1
            } else {
                position - 1
            }];

            let next = vertices[(position + 1) % vertices.len()];

            neighbours[vertex.index()] = [previous.index() as u32, next.index() as u32];
        }
    }

    for &vertex in decomposition.largest_cycle().vertices() {
        in_giant[vertex.index()] = true;
    }

    let view = DecomposedFactor {
        factor,
        in_giant: &in_giant,
        neighbours: &neighbours,
        giant_representative: decomposition.largest_cycle().vertices()[0].index() as u32,
        giant_size: decomposition.largest_cycle().len(),
    };

    let search = RepairSearch {
        graph,
        factor: &view,
        max_depth,
    };

    for (cycle_index, satellite) in decomposition.cycles().iter().enumerate() {
        if cycle_index == giant_index {
            continue;
        }

        if stop() {
            return Err(AlternatingRepairError::Interrupted);
        }

        let vertices = satellite.vertices();

        for index in 0..vertices.len() {
            let first = vertices[index];

            let second = vertices[(index + 1) % vertices.len()];

            let satellite_edge = required_edge(graph, first, second)?;

            /*
             * Both satellite-edge orientations are explored because the
             * alternating repair is directional.
             */
            for (satellite_left, satellite_right) in [(first, second), (second, first)] {
                let left_incident = graph
                    .incident(satellite_left)
                    .expect("satellite vertex must belong to graph");

                for &left_cross in left_incident {
                    if search.factor.contains(left_cross) {
                        continue;
                    }

                    let giant_left = other_vertex(graph, left_cross, satellite_left)?;

                    if !search.factor.in_giant(giant_left) {
                        continue;
                    }

                    let right_incident = graph
                        .incident(satellite_right)
                        .expect("satellite vertex must belong to graph");

                    for &right_cross in right_incident {
                        if search.factor.contains(right_cross) {
                            continue;
                        }

                        let giant_right = other_vertex(graph, right_cross, satellite_right)?;

                        if !search.factor.in_giant(giant_right) {
                            continue;
                        }

                        let giant_left_incident = graph
                            .incident(giant_left)
                            .expect("giant vertex must belong to graph");

                        for &removed_left in giant_left_incident {
                            if !search.factor.contains(removed_left) {
                                continue;
                            }

                            let giant_right_incident = graph
                                .incident(giant_right)
                                .expect("giant vertex must belong to graph");

                            for &removed_right in giant_right_incident {
                                if !search.factor.contains(removed_right) {
                                    continue;
                                }

                                let removed = vec![satellite_edge, removed_left, removed_right];

                                if !all_distinct(&removed) {
                                    continue;
                                }

                                let added = vec![left_cross, right_cross];

                                if !all_distinct(&added) {
                                    continue;
                                }

                                let Some((deficit_a, deficit_b)) =
                                    initial_deficits(graph, &removed, &added)
                                else {
                                    continue;
                                };

                                let mut removed = removed;

                                let mut added = added;

                                if let Some(movement) = search.repair_deficit(
                                    deficit_a,
                                    deficit_b,
                                    0,
                                    satellite.len(),
                                    &mut removed,
                                    &mut added,
                                )? {
                                    return Ok(Some(movement));
                                }

                                /*
                                 * The two deficits are not symmetric once
                                 * the repair path is explored, so try the
                                 * opposite moving/fixed assignment too.
                                 */
                                let mut removed = vec![satellite_edge, removed_left, removed_right];

                                let mut added = vec![left_cross, right_cross];

                                if let Some(movement) = search.repair_deficit(
                                    deficit_b,
                                    deficit_a,
                                    0,
                                    satellite.len(),
                                    &mut removed,
                                    &mut added,
                                )? {
                                    return Ok(Some(movement));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Outcome of repairing until no satellite can be absorbed.
#[derive(Clone, Debug)]
pub struct GreedyAlternatingRepairResult {
    factor: Factor,
    moves: Vec<AlternatingRepairMove>,
}

impl GreedyAlternatingRepairResult {
    /// Consumes the result and returns the patched factor.
    #[must_use]
    pub fn into_factor(self) -> Factor {
        self.factor
    }

    /// Returns all applied repairs in application order.
    #[must_use]
    pub fn moves(&self) -> &[AlternatingRepairMove] {
        &self.moves
    }

    /// Returns the number of applied repairs.
    #[must_use]
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// Returns the deepest repair applied, or zero when none was.
    #[must_use]
    pub fn deepest(&self) -> usize {
        self.moves
            .iter()
            .map(AlternatingRepairMove::depth)
            .max()
            .unwrap_or(0)
    }

    /// Returns the total number of vertices absorbed into the giant.
    #[must_use]
    pub fn absorbed_vertices(&self) -> usize {
        self.moves
            .iter()
            .map(AlternatingRepairMove::absorbed_vertices)
            .sum()
    }
}

/// Absorbs satellites by alternating repair until none can be.
///
/// Deliberately sequential: it always takes the first repairable satellite in
/// canonical order and leaves the rest. Each absorption grows the giant, which
/// makes the remaining satellites easier, so deferring the hard ones is an
/// advantage rather than an oversight. Collecting one repair per satellite in
/// a single scan was measured slower twice, at k = 5 and again at k = 6.
pub fn greedy_alternating_repair(
    graph: &PowerGraph,
    factor: &Factor,
    max_depth: usize,
    stop: &dyn Fn() -> bool,
) -> Result<GreedyAlternatingRepairResult, AlternatingRepairError> {
    let mut current = Factor::new(graph, factor.selected_edges().to_vec())
        .map_err(AlternatingRepairError::Factor)?;

    let mut moves = Vec::new();

    while !stop() {
        let found = match find_alternating_repair(graph, &current, max_depth, stop) {
            Ok(found) => found,

            Err(AlternatingRepairError::Interrupted) if !moves.is_empty() => break,

            Err(error) => return Err(error),
        };

        let Some(movement) = found else {
            break;
        };

        current = movement.apply(graph, &current)?;

        moves.push(movement);
    }

    Ok(GreedyAlternatingRepairResult {
        factor: current,
        moves,
    })
}

/// The three things the repair search asks of a 2-factor.
///
/// Keeping the surface this narrow is what lets the same search run over a
/// `Factor` plus a decomposition, or over an incrementally maintained
/// [`WorkingFactor`](crate::WorkingFactor).
pub trait FactorView {
    fn contains(&self, edge_id: EdgeId) -> bool;

    fn in_giant(&self, vertex: Vertex) -> bool;

    /// Whether the candidate would absorb exactly one satellite.
    ///
    /// The search asks this of every closing candidate it reaches, seven
    /// times per repair on average, so how it is answered dominates the cost
    /// of a repair.
    fn absorbs(
        &self,
        graph: &PowerGraph,
        movement: &AlternatingRepairMove,
    ) -> Result<bool, AlternatingRepairError>;
}

/// A `Factor` paired with the giant membership derived from a decomposition.
struct DecomposedFactor<'a> {
    factor: &'a Factor,
    in_giant: &'a [bool],

    /// The two selected neighbours of each vertex, by vertex index.
    neighbours: &'a [[u32; 2]],

    giant_representative: u32,
    giant_size: usize,
}

impl FactorView for DecomposedFactor<'_> {
    fn contains(&self, edge_id: EdgeId) -> bool {
        self.factor.contains(edge_id)
    }

    fn in_giant(&self, vertex: Vertex) -> bool {
        self.in_giant[vertex.index()]
    }

    /// Answers by walking the cycle the move would produce.
    ///
    /// The move re-links a handful of vertices, so the outcome is decided by
    /// following the giant with a small override table for those vertices and
    /// checking that the walk covers the giant and the satellite exactly
    /// once. Nothing is rebuilt, nothing is sorted, nothing is mutated.
    ///
    /// The previous implementation called `AlternatingRepairMove::apply`,
    /// which rebuilds a `Factor` — sorting 15.7 million edge ids on `G_6` —
    /// and the search reaches this point about seven times per repair.
    fn absorbs(
        &self,
        graph: &PowerGraph,
        movement: &AlternatingRepairMove,
    ) -> Result<bool, AlternatingRepairError> {
        let verdict = self.absorbs_by_traversal(graph, movement)?;

        debug_assert_eq!(
            verdict,
            movement.apply(graph, self.factor).is_ok(),
            "traversal test disagrees with a full apply"
        );

        Ok(verdict)
    }
}

impl DecomposedFactor<'_> {
    fn absorbs_by_traversal(
        &self,
        graph: &PowerGraph,
        movement: &AlternatingRepairMove,
    ) -> Result<bool, AlternatingRepairError> {
        let mut touched = Vec::<u32>::with_capacity(16);

        let mut satellite_seed = None;

        for &edge_id in movement.removed().iter().chain(movement.added()) {
            let edge = required_graph_edge(graph, edge_id)?;

            let (first, second) = edge.endpoints();

            for vertex in [first, second] {
                let index = vertex.index() as u32;

                if !touched.contains(&index) {
                    touched.push(index);
                }

                if !self.in_giant[vertex.index()] {
                    satellite_seed = Some(index);
                }
            }
        }

        let Some(seed) = satellite_seed else {
            return Ok(false);
        };

        // Neighbours each touched vertex would have after the move.
        let mut overrides = Vec::<(u32, [u32; 2])>::with_capacity(touched.len());

        for &index in &touched {
            let vertex =
                Vertex::from_number(index as usize + 1).ok_or(AlternatingRepairError::Working {
                    message: format!("vertex {} is outside the graph", index as usize + 1),
                })?;

            let mut found = [0u32; 2];

            let mut count = 0usize;

            for &edge_id in graph
                .incident(vertex)
                .ok_or(AlternatingRepairError::Working {
                    message: format!("vertex {} has no incidence", index as usize + 1),
                })?
            {
                let selected = if movement.removed().contains(&edge_id) {
                    false
                } else if movement.added().contains(&edge_id) {
                    true
                } else {
                    self.factor.contains(edge_id)
                };

                if !selected {
                    continue;
                }

                if count == 2 {
                    return Ok(false);
                }

                found[count] = other_vertex(graph, edge_id, vertex)?.index() as u32;

                count += 1;
            }

            if count != 2 {
                return Ok(false);
            }

            overrides.push((index, found));
        }

        overrides.sort_unstable_by_key(|&(index, _)| index);

        let low = overrides[0].0;
        let high = overrides[overrides.len() - 1].0;

        let neighbours_of = |vertex: u32| -> [u32; 2] {
            if vertex >= low
                && vertex <= high
                && let Ok(position) = overrides.binary_search_by_key(&vertex, |&(v, _)| v)
            {
                return overrides[position].1;
            }

            self.neighbours[vertex as usize]
        };

        let satellite_size = walk_length(seed, |v| self.neighbours[v as usize], usize::MAX);

        let expected = self.giant_size + satellite_size;

        let visited = walk_length(self.giant_representative, neighbours_of, expected);

        Ok(visited == expected)
    }
}

/// Length of the cycle through `start`, stopping early past `limit`.
fn walk_length(start: u32, neighbours: impl Fn(u32) -> [u32; 2], limit: usize) -> usize {
    let mut visited = 0usize;

    let mut current = start;

    let mut previous = u32::MAX;

    loop {
        visited += 1;

        if visited > limit {
            return visited;
        }

        let [first, second] = neighbours(current);

        let next = if first == previous { second } else { first };

        previous = current;
        current = next;

        if current == start {
            return visited;
        }
    }
}

fn required_graph_edge(
    graph: &PowerGraph,
    edge_id: EdgeId,
) -> Result<powercycle_core::Edge, AlternatingRepairError> {
    graph
        .edge(edge_id)
        .ok_or(AlternatingRepairError::InvalidEdgeId { edge_id })
}

struct RepairSearch<'a, F: FactorView> {
    graph: &'a PowerGraph,
    factor: &'a F,
    max_depth: usize,
}

impl<F: FactorView> RepairSearch<'_, F> {
    fn repair_deficit(
        &self,
        moving_deficit: Vertex,
        fixed_deficit: Vertex,
        depth: usize,
        absorbed_vertices: usize,
        removed: &mut Vec<EdgeId>,
        added: &mut Vec<EdgeId>,
    ) -> Result<Option<AlternatingRepairMove>, AlternatingRepairError> {
        /*
         * Depth zero corresponds to the quasi-2+1 state.
         *
         * We deliberately do not close it directly, because that would
         * merely rediscover an ordinary 2+1 move.
         */
        if depth > 0
            && let Some(closing_edge) = self.graph.edge_between(moving_deficit, fixed_deficit)
            && !self.factor.contains(closing_edge)
            && !added.contains(&closing_edge)
        {
            added.push(closing_edge);

            let candidate = AlternatingRepairMove::new(
                removed.clone(),
                added.clone(),
                depth,
                absorbed_vertices,
            );

            let absorbs = self.factor.absorbs(self.graph, &candidate);

            added.pop();

            if absorbs? {
                return Ok(Some(candidate));
            }

            /*
             * The degree repair closes, but its topology does not absorb
             * exactly the intended satellite. Keep searching.
             */
        }

        if depth >= self.max_depth {
            return Ok(None);
        }

        let incident = self
            .graph
            .incident(moving_deficit)
            .expect("giant deficit vertex must belong to graph");

        for &added_edge in incident {
            /*
             * Alternation is defined relative to the original factor:
             * additions must be originally unselected edges.
             */
            if self.factor.contains(added_edge) {
                continue;
            }

            if added.contains(&added_edge) {
                continue;
            }

            let transfer_vertex = other_vertex(self.graph, added_edge, moving_deficit)?;

            if !self.factor.in_giant(transfer_vertex) {
                continue;
            }

            if transfer_vertex == fixed_deficit {
                continue;
            }

            if current_degree(self.graph, transfer_vertex, removed, added) != 2 {
                continue;
            }

            added.push(added_edge);

            let transfer_incident = self
                .graph
                .incident(transfer_vertex)
                .expect("giant transfer vertex must belong to graph");

            for &removed_edge in transfer_incident {
                /*
                 * The removed side of the alternating path must be an
                 * originally selected giant edge.
                 */
                if !self.factor.contains(removed_edge) {
                    continue;
                }

                if removed.contains(&removed_edge) {
                    continue;
                }

                let next_deficit = other_vertex(self.graph, removed_edge, transfer_vertex)?;

                if !self.factor.in_giant(next_deficit) {
                    continue;
                }

                if next_deficit == fixed_deficit {
                    continue;
                }

                if current_degree(self.graph, next_deficit, removed, added) != 2 {
                    continue;
                }

                removed.push(removed_edge);

                if let Some(movement) = self.repair_deficit(
                    next_deficit,
                    fixed_deficit,
                    depth + 1,
                    absorbed_vertices,
                    removed,
                    added,
                )? {
                    return Ok(Some(movement));
                }

                removed.pop();
            }

            added.pop();
        }

        Ok(None)
    }
}

fn initial_deficits(
    graph: &PowerGraph,
    removed: &[EdgeId],
    added: &[EdgeId],
) -> Option<(Vertex, Vertex)> {
    let mut touched = Vec::<Vertex>::new();

    for &edge_id in removed.iter().chain(added.iter()) {
        let edge = graph.edge(edge_id)?;

        push_vertex_once(&mut touched, edge.first());

        push_vertex_once(&mut touched, edge.second());
    }

    let mut deficits = Vec::<Vertex>::new();

    for vertex in touched {
        match current_degree(graph, vertex, removed, added) {
            1 => {
                deficits.push(vertex);
            }

            2 => {}

            _ => {
                return None;
            }
        }
    }

    if deficits.len() != 2 {
        return None;
    }

    Some((deficits[0], deficits[1]))
}

fn current_degree(graph: &PowerGraph, vertex: Vertex, removed: &[EdgeId], added: &[EdgeId]) -> i32 {
    let mut degree = 2i32;

    for &edge_id in removed {
        let edge = graph
            .edge(edge_id)
            .expect("removed edge must belong to graph");

        if edge.other(vertex).is_some() {
            degree -= 1;
        }
    }

    for &edge_id in added {
        let edge = graph
            .edge(edge_id)
            .expect("added edge must belong to graph");

        if edge.other(vertex).is_some() {
            degree += 1;
        }
    }

    degree
}

fn push_vertex_once(vertices: &mut Vec<Vertex>, vertex: Vertex) {
    if !vertices.contains(&vertex) {
        vertices.push(vertex);
    }
}

fn required_edge(
    graph: &PowerGraph,
    first: Vertex,
    second: Vertex,
) -> Result<EdgeId, AlternatingRepairError> {
    graph
        .edge_between(first, second)
        .ok_or(AlternatingRepairError::MissingGraphEdge {
            first: first.number(),
            second: second.number(),
        })
}

fn other_vertex(
    graph: &PowerGraph,
    edge_id: EdgeId,
    vertex: Vertex,
) -> Result<Vertex, AlternatingRepairError> {
    let edge = graph
        .edge(edge_id)
        .ok_or(AlternatingRepairError::InvalidEdgeId { edge_id })?;

    edge.other(vertex)
        .ok_or(AlternatingRepairError::EdgeDoesNotContainVertex {
            edge_id,
            vertex: vertex.number(),
        })
}

fn all_distinct(values: &[EdgeId]) -> bool {
    for left in 0..values.len() {
        for right in (left + 1)..values.len() {
            if values[left] == values[right] {
                return false;
            }
        }
    }

    true
}

/// Errors produced by bounded alternating repair.
#[derive(Debug)]
pub enum AlternatingRepairError {
    Factor(FactorError),

    /// The incremental working form rejected an operation.
    Working {
        message: String,
    },

    /// The search was stopped by its caller, not exhausted.
    Interrupted,

    Decomposition {
        operation: &'static str,
        message: String,
    },

    TooFewCycles {
        cycle_count: usize,
    },

    ZeroDepth,

    ZeroAbsorbedVertices,

    UnequalToggleCounts {
        removed: usize,
        added: usize,
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

    InvalidGiantGrowth {
        before: usize,
        after: usize,
        absorbed: usize,
        expected: usize,
    },
}

impl fmt::Display for AlternatingRepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Factor(error) => {
                write!(f, "invalid factor: {error}")
            }

            Self::Interrupted => {
                write!(f, "alternating repair search interrupted by its caller")
            }

            Self::Working { message } => {
                write!(f, "working factor rejected the operation: {message}")
            }

            Self::Decomposition { operation, message } => {
                write!(f, "cannot {operation}: {message}")
            }

            Self::TooFewCycles { cycle_count } => {
                write!(
                    f,
                    "alternating repair requires at least two cycles, \
                     but factor has {cycle_count}"
                )
            }

            Self::ZeroDepth => {
                write!(f, "alternating repair depth must be at least one")
            }

            Self::ZeroAbsorbedVertices => {
                write!(f, "alternating repair must absorb a non-empty satellite")
            }

            Self::UnequalToggleCounts { removed, added } => {
                write!(
                    f,
                    "alternating repair removes {removed} edges but adds \
                     {added}; the counts must be equal"
                )
            }

            Self::DuplicateRemovedEdge => {
                write!(f, "alternating repair contains duplicate removed edges")
            }

            Self::DuplicateAddedEdge => {
                write!(f, "alternating repair contains duplicate added edges")
            }

            Self::RemovedEdgeNotSelected { edge_id } => {
                write!(
                    f,
                    "alternating repair tries to remove unselected edge {:?}",
                    edge_id
                )
            }

            Self::AddedEdgeAlreadySelected { edge_id } => {
                write!(
                    f,
                    "alternating repair tries to add already selected edge {:?}",
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
                    "alternating repair changed cycle count from {before} \
                     to {after}; expected {expected}"
                )
            }

            Self::InvalidGiantGrowth {
                before,
                after,
                absorbed,
                expected,
            } => {
                write!(
                    f,
                    "alternating repair changed giant size from {before} \
                     to {after} while absorbing {absorbed} vertices; \
                     expected {expected}"
                )
            }
        }
    }
}

impl Error for AlternatingRepairError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Factor(error) => Some(error),

            Self::Decomposition { .. }
            | Self::TooFewCycles { .. }
            | Self::ZeroDepth
            | Self::ZeroAbsorbedVertices
            | Self::UnequalToggleCounts { .. }
            | Self::DuplicateRemovedEdge
            | Self::DuplicateAddedEdge
            | Self::RemovedEdgeNotSelected { .. }
            | Self::AddedEdgeAlreadySelected { .. }
            | Self::InvalidEdgeId { .. }
            | Self::EdgeDoesNotContainVertex { .. }
            | Self::MissingGraphEdge { .. }
            | Self::InvalidCycleReduction { .. }
            | Self::InvalidGiantGrowth { .. }
            | Self::Working { .. }
            | Self::Interrupted => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::find_three_opt_21;

    fn depth_three_fixture(graph: &PowerGraph) -> Factor {
        /*
         * Deterministic G_2(32) 2-factor.
         *
         * Cycle sizes:
         *
         *   29 + 3
         *
         * No direct 2+1 absorption exists. No alternating repair exists at
         * depth 1 or 2. A depth-3 repair absorbs the 3-cycle.
         */
        let selected_indices = [
            0usize, 6, 7, 8, 9, 12, 13, 17, 18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 34,
            35, 36, 38, 39, 40, 41, 42, 43, 44, 45,
        ];

        let selected_edges = selected_indices
            .into_iter()
            .map(EdgeId::from_index)
            .collect::<Vec<_>>();

        let factor = Factor::new(graph, selected_edges).unwrap();

        factor.validate_two_factor(graph).unwrap();

        factor
    }

    fn known_depth_three_move() -> AlternatingRepairMove {
        /*
         * Graph edges in G_2(32):
         *
         * removed:
         *   17 =  6--19
         *    0 =  1--3
         *   19 =  8--17
         *   35 = 15--21
         *    8 =  4--12
         *   12 =  1--24
         *
         * added:
         *    3 =  3--6
         *   37 = 17--19
         *    5 =  1--15
         *   15 =  4--21
         *   32 = 12--24
         *    1 =  1--8
         *
         * The first three removals and first two additions create the
         * quasi-2+1 state. Three alternating transfers are then required
         * before closure.
         */
        AlternatingRepairMove::new(
            vec![
                EdgeId::from_index(17),
                EdgeId::from_index(0),
                EdgeId::from_index(19),
                EdgeId::from_index(35),
                EdgeId::from_index(8),
                EdgeId::from_index(12),
            ],
            vec![
                EdgeId::from_index(3),
                EdgeId::from_index(37),
                EdgeId::from_index(5),
                EdgeId::from_index(15),
                EdgeId::from_index(32),
                EdgeId::from_index(1),
            ],
            3,
            3,
        )
    }

    #[test]
    fn fixture_has_cycles_29_and_3() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut sizes = decomposition
            .cycles()
            .iter()
            .map(|cycle| cycle.len())
            .collect::<Vec<_>>();

        sizes.sort_unstable();

        assert_eq!(sizes, [3, 29,]);
    }

    #[test]
    fn fixture_has_no_direct_two_plus_one_move() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        assert_eq!(find_three_opt_21(&graph, &factor,).unwrap(), None);
    }

    #[test]
    fn fixture_has_no_repair_below_depth_three() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        assert_eq!(
            find_alternating_repair(&graph, &factor, 1, &|| false).unwrap(),
            None
        );

        assert_eq!(
            find_alternating_repair(&graph, &factor, 2, &|| false).unwrap(),
            None
        );
    }

    #[test]
    fn known_depth_three_repair_is_hamiltonian() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        let movement = known_depth_three_move();

        let repaired = movement.apply(&graph, &factor).unwrap();

        let decomposition = CycleDecomposition::new(&graph, &repaired).unwrap();

        assert_eq!(movement.depth(), 3);

        assert_eq!(movement.absorbed_vertices(), 3);

        assert_eq!(decomposition.cycle_count(), 1);

        assert_eq!(decomposition.largest_cycle().len(), 32);

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn finds_depth_three_repair() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        let movement = find_alternating_repair(&graph, &factor, 3, &|| false)
            .unwrap()
            .expect("fixture must contain a depth-3 alternating repair");

        assert_eq!(movement.depth(), 3);

        assert_eq!(movement.absorbed_vertices(), 3);

        let repaired = movement.apply(&graph, &factor).unwrap();

        let decomposition = CycleDecomposition::new(&graph, &repaired).unwrap();

        assert!(decomposition.is_hamiltonian());
    }

    #[test]
    fn search_is_deterministic() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = depth_three_fixture(&graph);

        let first = find_alternating_repair(&graph, &factor, 3, &|| false).unwrap();

        let second = find_alternating_repair(&graph, &factor, 3, &|| false).unwrap();

        assert_eq!(first, second);
    }
}
