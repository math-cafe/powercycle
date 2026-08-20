use std::collections::VecDeque;
use std::fmt;

use powercycle_core::{EdgeId, PowerGraph};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreflightReport {
    pub min_degree: usize,
    pub initial_degree_two_vertices: usize,
    pub connected: Option<bool>,
    pub articulation_checked: bool,
    pub articulation_vertex: Option<usize>,
    pub propagation_checked: bool,
    pub forced_edges: usize,
    pub forbidden_edges: usize,
    pub propagation_updates: usize,
    pub forced_spanning_cycle: bool,
    pub obstruction: Option<PreflightObstruction>,
}

impl PreflightReport {
    #[must_use]
    pub const fn passed(&self) -> bool {
        self.obstruction.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreflightObstruction {
    MinimumDegree { vertex: usize, degree: usize },

    Disconnected,

    ArticulationVertex { vertex: usize },

    TooManyForcedEdges { vertex: usize, forced: usize },

    TooFewAvailableEdges { vertex: usize, available: usize },

    ForcedSubcycle { start_vertex: usize, size: usize },
}

impl fmt::Display for PreflightObstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MinimumDegree { vertex, degree } => {
                write!(
                    f,
                    "vertex {vertex} has graph degree {degree}, below the Hamiltonian minimum 2"
                )
            }

            Self::Disconnected => {
                write!(f, "graph is disconnected")
            }

            Self::ArticulationVertex { vertex } => {
                write!(f, "vertex {vertex} is an articulation vertex")
            }

            Self::TooManyForcedEdges { vertex, forced } => {
                write!(
                    f,
                    "vertex {vertex} has {forced} forced edges, but a Hamiltonian cycle uses exactly 2"
                )
            }

            Self::TooFewAvailableEdges { vertex, available } => {
                write!(
                    f,
                    "vertex {vertex} has only {available} non-forbidden edge(s), below the required 2"
                )
            }

            Self::ForcedSubcycle { start_vertex, size } => {
                write!(
                    f,
                    "forced edges contain a proper cycle of size {size} starting at vertex {start_vertex}"
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeState {
    Unknown,
    Forced,
    Forbidden,
}

#[derive(Clone, Copy, Debug)]
struct DfsFrame {
    vertex: usize,
    next_incident: usize,
}

pub fn hamiltonian_preflight(graph: &PowerGraph) -> PreflightReport {
    let mut report = PreflightReport {
        min_degree: graph.min_degree(),
        initial_degree_two_vertices: 0,
        connected: None,
        articulation_checked: false,
        articulation_vertex: None,
        propagation_checked: false,
        forced_edges: 0,
        forbidden_edges: 0,
        propagation_updates: 0,
        forced_spanning_cycle: false,
        obstruction: None,
    };

    let mut minimum_degree_obstruction = None;

    for number in 1..=graph.vertex_count() {
        let vertex = graph
            .vertex(number)
            .expect("graph vertex number must be valid");

        let degree = graph
            .incident(vertex)
            .expect("graph-owned vertex must have incidence")
            .len();

        if degree == 2 {
            report.initial_degree_two_vertices += 1;
        }

        if degree < 2 && minimum_degree_obstruction.is_none() {
            minimum_degree_obstruction = Some(PreflightObstruction::MinimumDegree {
                vertex: number,
                degree,
            });
        }
    }

    if let Some(obstruction) = minimum_degree_obstruction {
        report.obstruction = Some(obstruction);

        return report;
    }

    let connected = graph.is_connected();

    report.connected = Some(connected);

    if !connected {
        report.obstruction = Some(PreflightObstruction::Disconnected);

        return report;
    }

    report.articulation_checked = true;
    report.articulation_vertex = first_articulation_vertex(graph);

    if let Some(vertex) = report.articulation_vertex {
        report.obstruction = Some(PreflightObstruction::ArticulationVertex { vertex });

        return report;
    }

    report.propagation_checked = true;

    let propagation = propagate_degree_two_constraints(graph);

    report.forced_edges = propagation.forced_edges;

    report.forbidden_edges = propagation.forbidden_edges;

    report.propagation_updates = propagation.updates;

    if let Some(obstruction) = propagation.obstruction {
        report.obstruction = Some(obstruction);

        return report;
    }

    match find_forced_cycle(graph, &propagation.edge_states, &propagation.forced_degree) {
        Some(ForcedCycle::Proper { start_vertex, size }) => {
            report.obstruction = Some(PreflightObstruction::ForcedSubcycle { start_vertex, size });
        }

        Some(ForcedCycle::Spanning) => {
            report.forced_spanning_cycle = true;
        }

        None => {}
    }

    report
}

fn first_articulation_vertex(graph: &PowerGraph) -> Option<usize> {
    let n = graph.vertex_count();

    if n < 3 {
        return None;
    }

    let undiscovered = usize::MAX;

    let mut discovery = vec![undiscovered; n];

    let mut low = vec![0usize; n];

    let mut parent = vec![None::<usize>; n];

    let mut child_count = vec![0usize; n];

    let mut articulation = vec![false; n];

    let mut time = 0usize;
    let mut stack = Vec::<DfsFrame>::new();

    for root in 0..n {
        if discovery[root] != undiscovered {
            continue;
        }

        discovery[root] = time;
        low[root] = time;
        time += 1;

        stack.push(DfsFrame {
            vertex: root,
            next_incident: 0,
        });

        while !stack.is_empty() {
            let frame_index = stack.len() - 1;

            let vertex_index = stack[frame_index].vertex;

            let vertex = graph
                .vertex(vertex_index + 1)
                .expect("DFS vertex must belong to graph");

            let incident = graph
                .incident(vertex)
                .expect("DFS vertex must have incidence");

            let next_incident = stack[frame_index].next_incident;

            if next_incident < incident.len() {
                let edge_id = incident[next_incident];

                stack[frame_index].next_incident += 1;

                let edge = graph
                    .edge(edge_id)
                    .expect("incident EdgeId must belong to graph");

                let neighbor = edge
                    .other(vertex)
                    .expect("incident edge must contain DFS vertex")
                    .index();

                if discovery[neighbor] == undiscovered {
                    parent[neighbor] = Some(vertex_index);

                    child_count[vertex_index] += 1;

                    discovery[neighbor] = time;

                    low[neighbor] = time;

                    time += 1;

                    stack.push(DfsFrame {
                        vertex: neighbor,
                        next_incident: 0,
                    });
                } else if parent[vertex_index] != Some(neighbor) {
                    low[vertex_index] = low[vertex_index].min(discovery[neighbor]);
                }

                continue;
            }

            stack.pop();

            if let Some(parent_index) = parent[vertex_index] {
                low[parent_index] = low[parent_index].min(low[vertex_index]);

                if parent[parent_index].is_some() && low[vertex_index] >= discovery[parent_index] {
                    articulation[parent_index] = true;
                }
            } else if child_count[vertex_index] > 1 {
                articulation[vertex_index] = true;
            }
        }
    }

    articulation
        .iter()
        .position(|&is_articulation| is_articulation)
        .map(|index| index + 1)
}

#[derive(Debug)]
struct PropagationResult {
    edge_states: Vec<EdgeState>,
    forced_degree: Vec<usize>,
    forced_edges: usize,
    forbidden_edges: usize,
    updates: usize,
    obstruction: Option<PreflightObstruction>,
}

fn propagate_degree_two_constraints(graph: &PowerGraph) -> PropagationResult {
    let n = graph.vertex_count();

    let mut edge_states = vec![EdgeState::Unknown; graph.edge_count()];

    let mut forced_degree = vec![0usize; n];

    let mut available_degree = Vec::<usize>::with_capacity(n);

    for number in 1..=n {
        let vertex = graph
            .vertex(number)
            .expect("graph vertex number must be valid");

        let degree = graph
            .incident(vertex)
            .expect("graph-owned vertex must have incidence")
            .len();

        available_degree.push(degree);
    }

    let mut queue = VecDeque::<usize>::with_capacity(n);

    let mut queued = vec![true; n];

    for vertex_index in 0..n {
        queue.push_back(vertex_index);
    }

    let mut forced_edges = 0usize;
    let mut forbidden_edges = 0usize;
    let mut updates = 0usize;

    while let Some(vertex_index) = queue.pop_front() {
        queued[vertex_index] = false;

        if forced_degree[vertex_index] > 2 {
            let forced = forced_degree[vertex_index];
            return PropagationResult {
                edge_states,
                forced_degree,
                forced_edges,
                forbidden_edges,
                updates,
                obstruction: Some(PreflightObstruction::TooManyForcedEdges {
                    vertex: vertex_index + 1,
                    forced,
                }),
            };
        }

        if available_degree[vertex_index] < 2 {
            let available = available_degree[vertex_index];
            return PropagationResult {
                edge_states,
                forced_degree,
                forced_edges,
                forbidden_edges,
                updates,
                obstruction: Some(PreflightObstruction::TooFewAvailableEdges {
                    vertex: vertex_index + 1,
                    available,
                }),
            };
        }

        let vertex = graph
            .vertex(vertex_index + 1)
            .expect("queued vertex must belong to graph");

        let incident = graph
            .incident(vertex)
            .expect("queued vertex must have incidence");

        if forced_degree[vertex_index] == 2 {
            for &edge_id in incident {
                if edge_states[edge_id.index()] != EdgeState::Unknown {
                    continue;
                }

                set_edge_state(
                    graph,
                    edge_id,
                    EdgeState::Forbidden,
                    &mut edge_states,
                    &mut forced_degree,
                    &mut available_degree,
                    &mut queue,
                    &mut queued,
                );

                forbidden_edges += 1;
                updates += 1;
            }
        } else if available_degree[vertex_index] == 2 {
            for &edge_id in incident {
                if edge_states[edge_id.index()] != EdgeState::Unknown {
                    continue;
                }

                set_edge_state(
                    graph,
                    edge_id,
                    EdgeState::Forced,
                    &mut edge_states,
                    &mut forced_degree,
                    &mut available_degree,
                    &mut queue,
                    &mut queued,
                );

                forced_edges += 1;
                updates += 1;
            }
        }
    }

    for vertex_index in 0..n {
        if forced_degree[vertex_index] > 2 {
            let forced = forced_degree[vertex_index];
            return PropagationResult {
                edge_states,
                forced_degree,
                forced_edges,
                forbidden_edges,
                updates,
                obstruction: Some(PreflightObstruction::TooManyForcedEdges {
                    vertex: vertex_index + 1,
                    forced,
                }),
            };
        }

        if available_degree[vertex_index] < 2 {
            let available = available_degree[vertex_index];
            return PropagationResult {
                edge_states,
                forced_degree,
                forced_edges,
                forbidden_edges,
                updates,
                obstruction: Some(PreflightObstruction::TooFewAvailableEdges {
                    vertex: vertex_index + 1,
                    available,
                }),
            };
        }
    }

    PropagationResult {
        edge_states,
        forced_degree,
        forced_edges,
        forbidden_edges,
        updates,
        obstruction: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn set_edge_state(
    graph: &PowerGraph,
    edge_id: EdgeId,
    new_state: EdgeState,
    edge_states: &mut [EdgeState],
    forced_degree: &mut [usize],
    available_degree: &mut [usize],
    queue: &mut VecDeque<usize>,
    queued: &mut [bool],
) {
    let edge_index = edge_id.index();

    debug_assert_eq!(edge_states[edge_index], EdgeState::Unknown,);

    debug_assert_ne!(new_state, EdgeState::Unknown,);

    edge_states[edge_index] = new_state;

    let edge = graph
        .edge(edge_id)
        .expect("state-changing EdgeId must belong to graph");

    let (first, second) = edge.endpoints();

    for vertex_index in [first.index(), second.index()] {
        match new_state {
            EdgeState::Forced => {
                forced_degree[vertex_index] += 1;
            }

            EdgeState::Forbidden => {
                available_degree[vertex_index] -= 1;
            }

            EdgeState::Unknown => {
                unreachable!("edge state transition cannot target Unknown");
            }
        }

        enqueue(vertex_index, queue, queued);
    }
}

fn enqueue(vertex_index: usize, queue: &mut VecDeque<usize>, queued: &mut [bool]) {
    if queued[vertex_index] {
        return;
    }

    queued[vertex_index] = true;

    queue.push_back(vertex_index);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForcedCycle {
    Proper { start_vertex: usize, size: usize },

    Spanning,
}

fn find_forced_cycle(
    graph: &PowerGraph,
    edge_states: &[EdgeState],
    forced_degree: &[usize],
) -> Option<ForcedCycle> {
    let n = graph.vertex_count();

    let mut visited = vec![false; n];

    let mut queue = VecDeque::<usize>::new();

    for start in 0..n {
        if visited[start] || forced_degree[start] == 0 {
            continue;
        }

        let mut size = 0usize;
        let mut all_degree_two = true;

        visited[start] = true;
        queue.push_back(start);

        while let Some(vertex_index) = queue.pop_front() {
            size += 1;

            if forced_degree[vertex_index] != 2 {
                all_degree_two = false;
            }

            let vertex = graph
                .vertex(vertex_index + 1)
                .expect("forced-component vertex must belong to graph");

            let incident = graph
                .incident(vertex)
                .expect("forced-component vertex must have incidence");

            for &edge_id in incident {
                if edge_states[edge_id.index()] != EdgeState::Forced {
                    continue;
                }

                let edge = graph
                    .edge(edge_id)
                    .expect("forced EdgeId must belong to graph");

                let other = edge
                    .other(vertex)
                    .expect("forced incident edge must contain vertex")
                    .index();

                if !visited[other] {
                    visited[other] = true;

                    queue.push_back(other);
                }
            }
        }

        if !all_degree_two {
            continue;
        }

        if size == n {
            return Some(ForcedCycle::Spanning);
        }

        return Some(ForcedCycle::Proper {
            start_vertex: start + 1,
            size,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_graph_with_degree_below_two() {
        let graph = PowerGraph::new(2, 10).unwrap();

        let report = hamiltonian_preflight(&graph);

        assert!(matches!(
            report.obstruction,
            Some(PreflightObstruction::MinimumDegree { .. })
        ));
    }

    #[test]
    fn accepts_known_hamiltonian_square_sum_graph() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let report = hamiltonian_preflight(&graph);

        assert!(
            report.passed(),
            "unexpected preflight obstruction: {:?}",
            report.obstruction
        );

        assert_eq!(report.connected, Some(true,));

        assert!(report.articulation_checked);

        assert!(report.propagation_checked);
    }
}
