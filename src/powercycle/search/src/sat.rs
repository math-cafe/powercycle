use std::error::Error;
use std::fmt;

use powercycle_core::{CycleDecomposition, EdgeId, Factor, FactorError, PowerGraph, Vertex};

use crate::preflight::EdgeState;

use rustsat::solvers::{ControlSignal, Solve, SolverResult, Terminate};
use rustsat::types::{Clause, Lit, TernaryVal, Var};
use rustsat_cadical::CaDiCaL;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Statistics describing the base SAT model.
///
/// Version 1 uses exactly one SAT variable per graph edge.
///
/// The only constraints represented here are the selected-degree-equals-two
/// constraints. Additional search cuts are deliberately outside this
/// base-model count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SatModelStats {
    pub variables: usize,
    pub at_least_two_clauses: usize,
    pub at_most_two_clauses: usize,
    pub vertices_below_degree_two: usize,
}

impl SatModelStats {
    /// Returns the total number of clauses in the base model.
    #[must_use]
    pub const fn clause_count(self) -> usize {
        self.at_least_two_clauses + self.at_most_two_clauses
    }
}

/// Statistics for one unbudgeted batch of search cuts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubtourCutStats {
    pub clauses: usize,
    pub literals: usize,
    pub min_width: usize,
    pub max_width: usize,
}

impl SubtourCutStats {
    /// Returns the average clause width.
    #[must_use]
    pub fn average_width(self) -> f64 {
        if self.clauses == 0 {
            0.0
        } else {
            self.literals as f64 / self.clauses as f64
        }
    }
}

/// Per-round budget for cycle-breaking cuts.
///
/// Candidates are ordered by increasing clause width before the budget is
/// applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleBreakingBudget {
    pub max_clauses: usize,
    pub max_literals: usize,
}

impl CycleBreakingBudget {
    /// Creates a cycle-breaking budget.
    #[must_use]
    pub const fn new(max_clauses: usize, max_literals: usize) -> Self {
        Self {
            max_clauses,
            max_literals,
        }
    }
}

impl Default for CycleBreakingBudget {
    fn default() -> Self {
        Self {
            max_clauses: 256,
            max_literals: 50_000,
        }
    }
}

/// Statistics for one budgeted cycle-breaking round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetedCutStats {
    pub candidate_clauses: usize,
    pub candidate_literals: usize,

    pub added_clauses: usize,
    pub added_literals: usize,

    pub skipped_clauses: usize,
    pub skipped_literals: usize,

    pub min_added_width: usize,
    pub max_added_width: usize,
}

impl BudgetedCutStats {
    /// Returns the average width of clauses actually added to the solver.
    #[must_use]
    pub fn average_added_width(self) -> f64 {
        if self.added_clauses == 0 {
            0.0
        } else {
            self.added_literals as f64 / self.added_clauses as f64
        }
    }
}

/// Per-round budget for boundary subtour-elimination cuts.
///
/// Boundary clauses whose width is less than or equal to
/// `priority_max_width` are selected before wider clauses.
///
/// Inside each priority class, clauses are ordered by increasing width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryCutBudget {
    pub max_clauses: usize,
    pub max_literals: usize,
    pub priority_max_width: usize,
}

impl BoundaryCutBudget {
    /// Creates a boundary-cut budget.
    #[must_use]
    pub const fn new(max_clauses: usize, max_literals: usize, priority_max_width: usize) -> Self {
        Self {
            max_clauses,
            max_literals,
            priority_max_width,
        }
    }
}

impl Default for BoundaryCutBudget {
    fn default() -> Self {
        Self {
            max_clauses: 256,
            max_literals: 50_000,
            priority_max_width: 8,
        }
    }
}

/// Statistics for one budgeted boundary-SEC round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetedBoundaryCutStats {
    pub candidate_clauses: usize,
    pub candidate_literals: usize,

    pub priority_candidate_clauses: usize,
    pub priority_candidate_literals: usize,

    pub added_clauses: usize,
    pub added_literals: usize,

    pub priority_added_clauses: usize,
    pub priority_added_literals: usize,

    pub skipped_clauses: usize,
    pub skipped_literals: usize,

    pub min_added_width: usize,
    pub max_added_width: usize,
}

impl BudgetedBoundaryCutStats {
    /// Returns the average width of boundary clauses actually added.
    #[must_use]
    pub fn average_added_width(self) -> f64 {
        if self.added_clauses == 0 {
            0.0
        } else {
            self.added_literals as f64 / self.added_clauses as f64
        }
    }
}

/// Counts for one batch of propagated unit clauses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropagatedUnitStats {
    pub forced: usize,
    pub forbidden: usize,
}

impl PropagatedUnitStats {
    /// Returns the number of edge variables fixed by the batch.
    #[must_use]
    pub const fn total(self) -> usize {
        self.forced + self.forbidden
    }
}

/// Incremental SAT solver for 2-factors of a power-sum graph.
///
/// SAT variable `i` corresponds exactly to graph `EdgeId(i)`.
///
/// The initial formula contains only the constraints requiring every vertex
/// to have selected degree exactly two.
///
/// Additional cuts can then be added incrementally between calls to `solve`.
#[derive(Debug)]
pub struct SatFactorSolver {
    solver: CaDiCaL<'static, 'static>,
    k: u32,
    n: usize,
    graph_edge_count: usize,
    stats: SatModelStats,
}

impl SatFactorSolver {
    /// Builds the base degree-2 SAT model for a graph.
    pub fn new(graph: &PowerGraph) -> Result<Self, SatError> {
        let mut solver = CaDiCaL::default();

        if graph.edge_count() > 0 {
            let maximum_index = variable_index(graph.edge_count() - 1)?;

            solver
                .reserve(Var::new(maximum_index))
                .map_err(|error| SatError::Solver {
                    operation: "reserve variables",
                    message: error.to_string(),
                })?;
        }

        let mut model = Self {
            solver,
            k: graph.k(),
            n: graph.vertex_count(),
            graph_edge_count: graph.edge_count(),

            stats: SatModelStats {
                variables: graph.edge_count(),
                ..SatModelStats::default()
            },
        };

        model.add_degree_two_constraints(graph)?;

        Ok(model)
    }

    /// Returns statistics for the base SAT model.
    #[must_use]
    pub const fn stats(&self) -> SatModelStats {
        self.stats
    }

    /// Returns the SAT solver implementation signature.
    #[must_use]
    pub fn solver_signature(&self) -> &'static str {
        self.solver.signature()
    }

    /// Attaches a cooperative interruption flag to CaDiCaL.
    ///
    /// When the flag becomes true, the solver terminator asks CaDiCaL to
    /// return `SolverResult::Interrupted` at the next termination check.
    ///
    /// The flag is shared with the command-line Ctrl-C handler.
    pub fn attach_interrupt_flag(&mut self, interrupt_requested: Arc<AtomicBool>) {
        self.solver.attach_terminator(move || {
            if interrupt_requested.load(Ordering::SeqCst) {
                ControlSignal::Terminate
            } else {
                ControlSignal::Continue
            }
        });
    }

    /// Solves the current SAT formula and returns one valid 2-factor.
    ///
    /// Returns `Ok(None)` if the formula is unsatisfiable.
    pub fn solve(&mut self, graph: &PowerGraph) -> Result<Option<Factor>, SatError> {
        self.ensure_compatible_graph(graph)?;

        let result = self.solver.solve().map_err(|error| SatError::Solver {
            operation: "solve",
            message: error.to_string(),
        })?;

        match result {
            SolverResult::Sat => self.extract_factor(graph).map(Some),
            SolverResult::Unsat => Ok(None),
            SolverResult::Interrupted => Err(SatError::Interrupted),
        }
    }

    /// Adds one clause that excludes exactly the supplied 2-factor.
    ///
    /// Under the base selected-degree-equals-two constraints, every valid
    /// 2-factor contains exactly `n` selected edges. Therefore the disjunction
    /// of the negations of all currently selected edge variables excludes this
    /// factor while leaving every distinct 2-factor admissible.
    ///
    /// Returns the width of the blocking clause.
    pub fn add_factor_blocking_clause(
        &mut self,
        graph: &PowerGraph,
        factor: &Factor,
    ) -> Result<usize, SatError> {
        self.ensure_compatible_graph(graph)?;

        factor
            .validate_two_factor(graph)
            .map_err(SatError::Factor)?;

        let literals = factor
            .selected_edges()
            .iter()
            .copied()
            .map(|edge_id| edge_literal(edge_id, true))
            .collect::<Result<Vec<_>, _>>()?;

        let width = literals.len();
        let clause: Clause = literals.into_iter().collect();

        self.solver
            .add_clause(clause)
            .map_err(|error| SatError::Solver {
                operation: "add factor-blocking clause",
                message: error.to_string(),
            })?;

        Ok(width)
    }

    /// Asserts the edge values derived by degree-two propagation.
    ///
    /// Every state supplied here is a consequence of the base
    /// selected-degree-equals-two constraints, so the set of satisfying
    /// 2-factors is unchanged: the clauses only hand the solver a conclusion
    /// it could reach on its own.
    pub fn add_propagated_units(
        &mut self,
        graph: &PowerGraph,
        edge_states: &[EdgeState],
    ) -> Result<PropagatedUnitStats, SatError> {
        self.ensure_compatible_graph(graph)?;

        if edge_states.len() != self.graph_edge_count {
            return Err(SatError::PropagationLengthMismatch {
                states: edge_states.len(),
                edge_count: self.graph_edge_count,
            });
        }

        let mut stats = PropagatedUnitStats::default();

        for (index, state) in edge_states.iter().enumerate() {
            let negated = match state {
                EdgeState::Unknown => continue,
                EdgeState::Forced => false,
                EdgeState::Forbidden => true,
            };

            let clause: Clause = [edge_literal(EdgeId::from_index(index), negated)?]
                .into_iter()
                .collect();

            self.solver
                .add_clause(clause)
                .map_err(|error| SatError::Solver {
                    operation: "add propagated unit clause",
                    message: error.to_string(),
                })?;

            if negated {
                stats.forbidden += 1;
            } else {
                stats.forced += 1;
            }
        }

        Ok(stats)
    }

    /// Adds one cycle-breaking cut for every cycle except the largest.
    pub fn add_cycle_breaking_cuts(
        &mut self,
        graph: &PowerGraph,
        decomposition: &CycleDecomposition,
    ) -> Result<SubtourCutStats, SatError> {
        self.ensure_compatible_graph(graph)?;

        let candidates = cycle_breaking_candidates(graph, decomposition)?;

        let mut stats = SubtourCutStats::default();

        for candidate in candidates {
            let width = candidate.literals.len();

            let clause: Clause = candidate.literals.into_iter().collect();

            self.solver
                .add_clause(clause)
                .map_err(|error| SatError::Solver {
                    operation: "add cycle-breaking cut",
                    message: error.to_string(),
                })?;

            update_cut_stats(&mut stats, width);
        }

        Ok(stats)
    }

    /// Adds a budgeted set of cycle-breaking cuts.
    ///
    /// Candidates are ordered by increasing cycle length.
    pub fn add_budgeted_cycle_breaking_cuts(
        &mut self,
        graph: &PowerGraph,
        decomposition: &CycleDecomposition,
        budget: CycleBreakingBudget,
    ) -> Result<BudgetedCutStats, SatError> {
        self.ensure_compatible_graph(graph)?;

        let candidates = cycle_breaking_candidates(graph, decomposition)?;

        let mut stats = BudgetedCutStats::default();

        for candidate in &candidates {
            stats.candidate_clauses += 1;
            stats.candidate_literals += candidate.literals.len();
        }

        for candidate in candidates {
            let width = candidate.literals.len();

            let fits_clause_budget = stats.added_clauses < budget.max_clauses;

            let fits_literal_budget =
                width <= budget.max_literals.saturating_sub(stats.added_literals);

            if !fits_clause_budget || !fits_literal_budget {
                stats.skipped_clauses += 1;
                stats.skipped_literals += width;
                continue;
            }

            let clause: Clause = candidate.literals.into_iter().collect();

            self.solver
                .add_clause(clause)
                .map_err(|error| SatError::Solver {
                    operation: "add budgeted cycle-breaking cut",
                    message: error.to_string(),
                })?;

            stats.added_clauses += 1;
            stats.added_literals += width;

            if stats.added_clauses == 1 {
                stats.min_added_width = width;
                stats.max_added_width = width;
            } else {
                stats.min_added_width = stats.min_added_width.min(width);

                stats.max_added_width = stats.max_added_width.max(width);
            }
        }

        debug_assert_eq!(
            stats.candidate_clauses,
            stats.added_clauses + stats.skipped_clauses
        );

        debug_assert_eq!(
            stats.candidate_literals,
            stats.added_literals + stats.skipped_literals
        );

        Ok(stats)
    }

    /// Adds one boundary subtour-elimination cut for every cycle except the
    /// largest.
    ///
    /// For a satellite vertex set `S`, the clause is the disjunction of all
    /// graph-edge variables crossing the boundary of `S`.
    pub fn add_boundary_subtour_cuts(
        &mut self,
        graph: &PowerGraph,
        decomposition: &CycleDecomposition,
    ) -> Result<SubtourCutStats, SatError> {
        self.ensure_compatible_graph(graph)?;

        let candidates = boundary_cut_candidates(graph, decomposition)?;

        let mut stats = SubtourCutStats::default();

        for candidate in candidates {
            let width = candidate.literals.len();

            let clause: Clause = candidate.literals.into_iter().collect();

            self.solver
                .add_clause(clause)
                .map_err(|error| SatError::Solver {
                    operation: "add boundary subtour cut",
                    message: error.to_string(),
                })?;

            update_cut_stats(&mut stats, width);
        }

        Ok(stats)
    }

    /// Adds a budgeted batch of boundary subtour-elimination cuts.
    ///
    /// All current satellite cycles produce candidate boundary clauses.
    ///
    /// Clauses whose width is at most `priority_max_width` are considered
    /// first. Within the priority and non-priority groups, clauses are
    /// ordered by increasing width.
    ///
    /// The selected batch never exceeds either the clause budget or the
    /// literal budget.
    pub fn add_budgeted_boundary_subtour_cuts(
        &mut self,
        graph: &PowerGraph,
        decomposition: &CycleDecomposition,
        budget: BoundaryCutBudget,
    ) -> Result<BudgetedBoundaryCutStats, SatError> {
        self.ensure_compatible_graph(graph)?;

        let mut candidates = boundary_cut_candidates(graph, decomposition)?;

        let mut stats = BudgetedBoundaryCutStats::default();

        for candidate in &candidates {
            let width = candidate.literals.len();

            stats.candidate_clauses += 1;
            stats.candidate_literals += width;

            if width <= budget.priority_max_width {
                stats.priority_candidate_clauses += 1;
                stats.priority_candidate_literals += width;
            }
        }

        candidates.sort_by_key(|candidate| {
            let width = candidate.literals.len();

            let priority_class = if width <= budget.priority_max_width {
                0usize
            } else {
                1usize
            };

            (priority_class, width)
        });

        for candidate in candidates {
            let width = candidate.literals.len();

            let priority = width <= budget.priority_max_width;

            let fits_clause_budget = stats.added_clauses < budget.max_clauses;

            let remaining_literals = budget.max_literals.saturating_sub(stats.added_literals);

            let fits_literal_budget = width <= remaining_literals;

            if !fits_clause_budget || !fits_literal_budget {
                stats.skipped_clauses += 1;
                stats.skipped_literals += width;
                continue;
            }

            let clause: Clause = candidate.literals.into_iter().collect();

            self.solver
                .add_clause(clause)
                .map_err(|error| SatError::Solver {
                    operation: "add budgeted boundary subtour cut",
                    message: error.to_string(),
                })?;

            stats.added_clauses += 1;
            stats.added_literals += width;

            if priority {
                stats.priority_added_clauses += 1;
                stats.priority_added_literals += width;
            }

            if stats.added_clauses == 1 {
                stats.min_added_width = width;
                stats.max_added_width = width;
            } else {
                stats.min_added_width = stats.min_added_width.min(width);

                stats.max_added_width = stats.max_added_width.max(width);
            }
        }

        debug_assert_eq!(
            stats.candidate_clauses,
            stats.added_clauses + stats.skipped_clauses
        );

        debug_assert_eq!(
            stats.candidate_literals,
            stats.added_literals + stats.skipped_literals
        );

        Ok(stats)
    }

    fn add_degree_two_constraints(&mut self, graph: &PowerGraph) -> Result<(), SatError> {
        for number in 1..=graph.vertex_count() {
            let vertex = graph
                .vertex(number)
                .expect("graph vertex number must be valid");

            let incident = graph
                .incident(vertex)
                .expect("graph-owned vertex must have incidence");

            let literals: Vec<Lit> = incident
                .iter()
                .copied()
                .map(|edge_id| edge_literal(edge_id, false))
                .collect::<Result<_, _>>()?;

            self.add_exactly_two(&literals)?;
        }

        Ok(())
    }

    fn add_exactly_two(&mut self, literals: &[Lit]) -> Result<(), SatError> {
        let degree = literals.len();

        if degree < 2 {
            let clause: Clause = std::iter::empty::<Lit>().collect();

            self.add_clause(clause, ClauseKind::AtLeastTwo)?;

            self.stats.vertices_below_degree_two += 1;

            return Ok(());
        }

        for omitted in 0..degree {
            let clause: Clause = literals
                .iter()
                .enumerate()
                .filter_map(|(index, &literal)| (index != omitted).then_some(literal))
                .collect();

            self.add_clause(clause, ClauseKind::AtLeastTwo)?;
        }

        let negative_literals: Vec<Lit> = literals
            .iter()
            .map(|literal| literal.var().neg_lit())
            .collect();

        for first in 0..degree {
            for second in (first + 1)..degree {
                for third in (second + 1)..degree {
                    let clause: Clause = [
                        negative_literals[first],
                        negative_literals[second],
                        negative_literals[third],
                    ]
                    .into_iter()
                    .collect();

                    self.add_clause(clause, ClauseKind::AtMostTwo)?;
                }
            }
        }

        Ok(())
    }

    fn add_clause(&mut self, clause: Clause, kind: ClauseKind) -> Result<(), SatError> {
        self.solver
            .add_clause(clause)
            .map_err(|error| SatError::Solver {
                operation: "add clause",
                message: error.to_string(),
            })?;

        match kind {
            ClauseKind::AtLeastTwo => {
                self.stats.at_least_two_clauses += 1;
            }

            ClauseKind::AtMostTwo => {
                self.stats.at_most_two_clauses += 1;
            }
        }

        Ok(())
    }

    fn extract_factor(&self, graph: &PowerGraph) -> Result<Factor, SatError> {
        let mut selected_edges = Vec::with_capacity(graph.vertex_count());

        let mut dont_care_count = 0usize;

        for index in 0..graph.edge_count() {
            let edge_id = EdgeId::from_index(index);

            let literal = edge_literal(edge_id, false)?;

            let value = self
                .solver
                .lit_val(literal)
                .map_err(|error| SatError::Solver {
                    operation: "read model",
                    message: error.to_string(),
                })?;

            match value {
                TernaryVal::True => {
                    selected_edges.push(edge_id);
                }

                TernaryVal::False => {}

                TernaryVal::DontCare => {
                    dont_care_count += 1;
                }
            }
        }

        let factor = Factor::new(graph, selected_edges).map_err(SatError::Factor)?;

        match factor.validate_two_factor(graph) {
            Ok(()) => Ok(factor),

            Err(source) => Err(SatError::InvalidModel {
                dont_care_count,
                source,
            }),
        }
    }

    fn ensure_compatible_graph(&self, graph: &PowerGraph) -> Result<(), SatError> {
        if self.k == graph.k()
            && self.n == graph.vertex_count()
            && self.graph_edge_count == graph.edge_count()
        {
            return Ok(());
        }

        Err(SatError::GraphMismatch {
            solver_k: self.k,
            solver_n: self.n,
            solver_edge_count: self.graph_edge_count,

            graph_k: graph.k(),
            graph_n: graph.vertex_count(),
            graph_edge_count: graph.edge_count(),
        })
    }
}

#[derive(Debug)]
struct CycleBreakingCandidate {
    literals: Vec<Lit>,
}

#[derive(Debug)]
struct BoundaryCutCandidate {
    literals: Vec<Lit>,
}

#[derive(Clone, Copy, Debug)]
enum ClauseKind {
    AtLeastTwo,
    AtMostTwo,
}

fn cycle_breaking_candidates(
    graph: &PowerGraph,
    decomposition: &CycleDecomposition,
) -> Result<Vec<CycleBreakingCandidate>, SatError> {
    if decomposition.cycle_count() <= 1 {
        return Ok(Vec::new());
    }

    let largest_cycle = decomposition.largest_cycle_index();

    let mut candidates = Vec::with_capacity(decomposition.cycle_count().saturating_sub(1));

    for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
        if cycle_index == largest_cycle {
            continue;
        }

        let edge_ids = cycle_edge_ids(graph, cycle.vertices())?;

        let literals = edge_ids
            .into_iter()
            .map(|edge_id| edge_literal(edge_id, true))
            .collect::<Result<Vec<_>, _>>()?;

        candidates.push(CycleBreakingCandidate { literals });
    }

    candidates.sort_by_key(|candidate| candidate.literals.len());

    Ok(candidates)
}

fn boundary_cut_candidates(
    graph: &PowerGraph,
    decomposition: &CycleDecomposition,
) -> Result<Vec<BoundaryCutCandidate>, SatError> {
    if decomposition.cycle_count() <= 1 {
        return Ok(Vec::new());
    }

    let component_of = decomposition_component_map(graph, decomposition)?;

    let largest_cycle = decomposition.largest_cycle_index();

    let mut candidates = Vec::with_capacity(decomposition.cycle_count().saturating_sub(1));

    for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
        if cycle_index == largest_cycle {
            continue;
        }

        let mut literals = Vec::<Lit>::new();

        for &vertex in cycle.vertices() {
            let incident = graph
                .incident(vertex)
                .expect("decomposition vertex must belong to graph");

            for &edge_id in incident {
                let edge = graph
                    .edge(edge_id)
                    .expect("incident EdgeId must belong to graph");

                let other = edge
                    .other(vertex)
                    .expect("incident edge must contain vertex");

                if component_of[other.index()] != cycle_index {
                    literals.push(edge_literal(edge_id, false)?);
                }
            }
        }

        candidates.push(BoundaryCutCandidate { literals });
    }

    Ok(candidates)
}

fn decomposition_component_map(
    graph: &PowerGraph,
    decomposition: &CycleDecomposition,
) -> Result<Vec<usize>, SatError> {
    let mut component_of = vec![usize::MAX; graph.vertex_count()];

    for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
        for &vertex in cycle.vertices() {
            let index = vertex.index();

            if index >= component_of.len() {
                return Err(SatError::InvalidDecomposition {
                    message: format!("vertex {} is outside the graph", vertex.number()),
                });
            }

            if component_of[index] != usize::MAX {
                return Err(SatError::InvalidDecomposition {
                    message: format!("vertex {} occurs in more than one cycle", vertex.number()),
                });
            }

            component_of[index] = cycle_index;
        }
    }

    if let Some(index) = component_of
        .iter()
        .position(|&component| component == usize::MAX)
    {
        return Err(SatError::InvalidDecomposition {
            message: format!("vertex {} is missing from the decomposition", index + 1),
        });
    }

    Ok(component_of)
}

fn cycle_edge_ids(graph: &PowerGraph, vertices: &[Vertex]) -> Result<Vec<EdgeId>, SatError> {
    if vertices.len() < 3 {
        return Err(SatError::InvalidDecomposition {
            message: format!("cycle has invalid length {}", vertices.len()),
        });
    }

    let mut edge_ids = Vec::with_capacity(vertices.len());

    for index in 0..vertices.len() {
        let first = vertices[index];

        let second = vertices[(index + 1) % vertices.len()];

        let edge_id =
            graph
                .edge_between(first, second)
                .ok_or_else(|| SatError::InvalidDecomposition {
                    message: format!(
                        "cycle edge {}--{} does not exist in the graph",
                        first.number(),
                        second.number()
                    ),
                })?;

        edge_ids.push(edge_id);
    }

    Ok(edge_ids)
}

fn update_cut_stats(stats: &mut SubtourCutStats, width: usize) {
    stats.clauses += 1;
    stats.literals += width;

    if stats.clauses == 1 {
        stats.min_width = width;
        stats.max_width = width;
    } else {
        stats.min_width = stats.min_width.min(width);

        stats.max_width = stats.max_width.max(width);
    }
}

fn variable_index(index: usize) -> Result<u32, SatError> {
    let index = u32::try_from(index).map_err(|_| SatError::TooManyVariables {
        requested_index: index,
        maximum_index: Var::MAX_IDX,
    })?;

    if index > Var::MAX_IDX {
        return Err(SatError::TooManyVariables {
            requested_index: index as usize,
            maximum_index: Var::MAX_IDX,
        });
    }

    Ok(index)
}

fn edge_literal(edge_id: EdgeId, negated: bool) -> Result<Lit, SatError> {
    let index = variable_index(edge_id.index())?;

    Ok(Var::new(index).lit(negated))
}

/// Errors produced by the SAT 2-factor layer.
#[derive(Debug)]
pub enum SatError {
    TooManyVariables {
        requested_index: usize,
        maximum_index: u32,
    },

    GraphMismatch {
        solver_k: u32,
        solver_n: usize,
        solver_edge_count: usize,

        graph_k: u32,
        graph_n: usize,
        graph_edge_count: usize,
    },

    InvalidDecomposition {
        message: String,
    },

    PropagationLengthMismatch {
        states: usize,
        edge_count: usize,
    },

    Solver {
        operation: &'static str,
        message: String,
    },

    Interrupted,

    Factor(FactorError),

    InvalidModel {
        dont_care_count: usize,
        source: FactorError,
    },
}

impl fmt::Display for SatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyVariables {
                requested_index,
                maximum_index,
            } => {
                write!(
                    f,
                    "SAT variable index {requested_index} exceeds \
                     RustSAT maximum index {maximum_index}"
                )
            }

            Self::GraphMismatch {
                solver_k,
                solver_n,
                solver_edge_count,
                graph_k,
                graph_n,
                graph_edge_count,
            } => {
                write!(
                    f,
                    "SAT model belongs to G_{solver_k}({solver_n}) \
                     with {solver_edge_count} edges, but received \
                     G_{graph_k}({graph_n}) with \
                     {graph_edge_count} edges"
                )
            }

            Self::PropagationLengthMismatch { states, edge_count } => {
                write!(
                    f,
                    "propagation covers {states} edges but the graph has {edge_count}"
                )
            }

            Self::InvalidDecomposition { message } => {
                write!(f, "invalid cycle decomposition: {message}")
            }

            Self::Solver { operation, message } => {
                write!(f, "SAT solver error during {operation}: {message}")
            }

            Self::Interrupted => {
                write!(f, "SAT solving was interrupted")
            }

            Self::Factor(error) => {
                write!(f, "cannot construct factor from SAT model: {error}")
            }

            Self::InvalidModel {
                dont_care_count,
                source,
            } => {
                write!(
                    f,
                    "SAT model does not reconstruct a valid 2-factor \
                     ({dont_care_count} don't-care variables): {source}"
                )
            }
        }
    }
}

impl Error for SatError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Factor(error) => Some(error),

            Self::InvalidModel { source, .. } => Some(source),

            Self::TooManyVariables { .. }
            | Self::GraphMismatch { .. }
            | Self::InvalidDecomposition { .. }
            | Self::PropagationLengthMismatch { .. }
            | Self::Solver { .. }
            | Self::Interrupted => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use powercycle_core::{CycleDecomposition, Factor};

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
    fn uses_one_variable_per_graph_edge() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let model = SatFactorSolver::new(&graph).unwrap();

        assert_eq!(model.stats().variables, graph.edge_count());

        assert_eq!(model.stats().variables, 7);
    }

    #[test]
    fn builds_expected_small_degree_constraints() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let model = SatFactorSolver::new(&graph).unwrap();

        let stats = model.stats();

        assert_eq!(stats.variables, 7);

        assert_eq!(stats.at_least_two_clauses, 14);

        assert_eq!(stats.at_most_two_clauses, 0);

        assert_eq!(stats.vertices_below_degree_two, 6);

        assert_eq!(stats.clause_count(), 14);
    }

    #[test]
    fn detects_graph_without_two_factor_as_unsat() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        assert!(solver.solve(&graph).unwrap().is_none());
    }

    /// Cross-validates degree-two propagation against the solver.
    ///
    /// The propagation claims each forced edge belongs to every 2-factor and
    /// each forbidden edge to none. An unconstrained model is an independent
    /// oracle for that claim.
    #[test]
    fn propagation_agrees_with_an_unconstrained_solution() {
        for (k, n) in [(2u32, 32usize), (2, 40), (3, 473), (5, 246_877)] {
            let graph = PowerGraph::new(k, n).unwrap();

            let report = crate::preflight::hamiltonian_preflight(&graph);

            assert!(report.passed(), "G_{k}({n}) should clear the preflight");

            let mut solver = SatFactorSolver::new(&graph).unwrap();

            let factor = solver
                .solve(&graph)
                .unwrap()
                .expect("the graph must admit a 2-factor");

            for (index, state) in report.edge_states.iter().enumerate() {
                let edge_id = EdgeId::from_index(index);

                match state {
                    EdgeState::Forced => assert!(
                        factor.contains(edge_id),
                        "G_{k}({n}): forced edge {index} missing from a 2-factor"
                    ),

                    EdgeState::Forbidden => assert!(
                        !factor.contains(edge_id),
                        "G_{k}({n}): forbidden edge {index} used by a 2-factor"
                    ),

                    EdgeState::Unknown => {}
                }
            }
        }
    }

    #[test]
    fn propagated_units_keep_the_model_satisfiable() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let report = crate::preflight::hamiltonian_preflight(&graph);

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let stats = solver
            .add_propagated_units(&graph, &report.edge_states)
            .unwrap();

        assert_eq!(stats.forced, report.forced_edges);
        assert_eq!(stats.forbidden, report.forbidden_edges);
        assert_eq!(stats.total(), stats.forced + stats.forbidden);
        assert!(stats.total() > 0);

        let factor = solver
            .solve(&graph)
            .unwrap()
            .expect("the units are implied, so the model stays satisfiable");

        factor.validate_two_factor(&graph).unwrap();
        assert_eq!(factor.edge_count(), graph.vertex_count());
    }

    #[test]
    fn propagated_units_reject_a_mismatched_length() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let error = solver
            .add_propagated_units(&graph, &[EdgeState::Unknown])
            .expect_err("a short state vector must be rejected");

        assert!(matches!(
            error,
            SatError::PropagationLengthMismatch { states: 1, edge_count }
                if edge_count == graph.edge_count()
        ));
    }

    #[test]
    fn solves_square_sum_n32_two_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let factor = solver
            .solve(&graph)
            .unwrap()
            .expect("G_2(32) must admit a 2-factor");

        factor.validate_two_factor(&graph).unwrap();

        assert_eq!(factor.edge_count(), graph.vertex_count());
    }

    #[test]
    fn budgeted_boundary_cut_recognizes_priority_frontier() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let stats = solver
            .add_budgeted_boundary_subtour_cuts(
                &graph,
                &decomposition,
                BoundaryCutBudget::new(256, 50_000, 8),
            )
            .unwrap();

        assert_eq!(stats.candidate_clauses, 1);

        assert_eq!(stats.candidate_literals, 5);

        assert_eq!(stats.priority_candidate_clauses, 1);

        assert_eq!(stats.priority_candidate_literals, 5);

        assert_eq!(stats.added_clauses, 1);

        assert_eq!(stats.added_literals, 5);

        assert_eq!(stats.priority_added_clauses, 1);

        assert_eq!(stats.priority_added_literals, 5);

        assert_eq!(stats.skipped_clauses, 0);

        assert_eq!(stats.min_added_width, 5);

        assert_eq!(stats.max_added_width, 5);
    }

    #[test]
    fn budgeted_boundary_cut_respects_literal_budget() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let stats = solver
            .add_budgeted_boundary_subtour_cuts(
                &graph,
                &decomposition,
                BoundaryCutBudget::new(256, 4, 8),
            )
            .unwrap();

        assert_eq!(stats.candidate_clauses, 1);

        assert_eq!(stats.priority_candidate_clauses, 1);

        assert_eq!(stats.added_clauses, 0);

        assert_eq!(stats.added_literals, 0);

        assert_eq!(stats.skipped_clauses, 1);

        assert_eq!(stats.skipped_literals, 5);
    }

    #[test]
    fn boundary_and_cycle_breaking_cut_have_expected_width() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let factor = two_cycle_factor(&graph);

        let decomposition = CycleDecomposition::new(&graph, &factor).unwrap();

        let mut boundary_solver = SatFactorSolver::new(&graph).unwrap();

        let boundary = boundary_solver
            .add_boundary_subtour_cuts(&graph, &decomposition)
            .unwrap();

        assert_eq!(boundary.clauses, 1);

        assert_eq!(boundary.literals, 5);

        let mut cycle_solver = SatFactorSolver::new(&graph).unwrap();

        let cycle = cycle_solver
            .add_cycle_breaking_cuts(&graph, &decomposition)
            .unwrap();

        assert_eq!(cycle.clauses, 1);

        assert_eq!(cycle.literals, 5);
    }

    #[test]
    fn factor_blocking_clause_excludes_the_current_factor() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        let first = solver
            .solve(&graph)
            .unwrap()
            .expect("G_2(32) must admit a 2-factor");

        let width = solver.add_factor_blocking_clause(&graph, &first).unwrap();

        assert_eq!(width, first.edge_count());

        if let Some(second) = solver.solve(&graph).unwrap() {
            assert_ne!(
                second.selected_edges(),
                first.selected_edges(),
                "the blocked 2-factor must not be returned again"
            );
        }
    }

    #[test]
    fn rejects_another_graph_at_solve_time() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let other_graph = PowerGraph::new(2, 33).unwrap();

        let mut solver = SatFactorSolver::new(&graph).unwrap();

        assert!(matches!(
            solver.solve(&other_graph),
            Err(SatError::GraphMismatch { .. })
        ));
    }

    #[test]
    fn solver_has_a_signature() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let solver = SatFactorSolver::new(&graph).unwrap();

        assert!(!solver.solver_signature().is_empty());
    }
}
