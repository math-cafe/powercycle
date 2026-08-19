//! Persistent data formats for powercycle.
//!
//! This crate defines the versioned JSON dump format shared by the
//! powercycle tools.
//!
//! It depends on `powercycle-core` for mathematical validation, but contains
//! no solver, patching algorithm, command-line interface or logging code.

use std::error::Error;
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use powercycle_core::{CycleDecomposition, EdgeId, Factor, FactorError, GraphError, PowerGraph};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Name stored in every powercycle dump.
pub const DUMP_FORMAT: &str = "powercycle-dump";

/// Current dump schema version.
pub const DUMP_VERSION: u32 = 1;

/// Integrity algorithm used by dump version 1.
pub const INTEGRITY_ALGORITHM: &str = "sha256";

const GRAPH_HASH_DOMAIN: &[u8] = b"powercycle-graph-v1\0";
const STATE_HASH_DOMAIN: &[u8] = b"powercycle-dump-state-v1\0";

/// Metadata identifying the exact power-sum graph used by a dump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphMetadata {
    pub k: u32,
    pub n: usize,
    pub edge_count: usize,
    pub sha256: String,
}

/// Provenance information associated with a dump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub producer: String,
    pub producer_version: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tour: Option<u64>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reason: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_dump_id: Option<String>,
}

impl Provenance {
    /// Creates minimal provenance information.
    #[must_use]
    pub fn new(producer: impl Into<String>, producer_version: impl Into<String>) -> Self {
        Self {
            producer: producer.into(),
            producer_version: producer_version.into(),
            session: None,
            invocation: None,
            tour: None,
            reason: Vec::new(),
            parent_dump_id: None,
        }
    }
}

/// Persistent representation of a factor.
///
/// Edge IDs are required to be strictly increasing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorRecord {
    pub selected_edges: Vec<usize>,
}

impl FactorRecord {
    #[must_use]
    pub fn from_factor(factor: &Factor) -> Self {
        Self {
            selected_edges: factor
                .selected_edges()
                .iter()
                .map(|edge_id| edge_id.index())
                .collect(),
        }
    }
}

/// A factor-preserving transformation recorded in dump history.
///
/// `removed` and `added` contain graph edge IDs and are required to be
/// strictly increasing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRecord {
    pub kind: String,
    pub removed: Vec<usize>,
    pub added: Vec<usize>,
}

impl MoveRecord {
    /// Creates a move record and canonicalizes its edge-ID arrays.
    #[must_use]
    pub fn new<I, J>(kind: impl Into<String>, removed: I, added: J) -> Self
    where
        I: IntoIterator<Item = EdgeId>,
        J: IntoIterator<Item = EdgeId>,
    {
        let mut removed: Vec<_> = removed.into_iter().map(EdgeId::index).collect();

        let mut added: Vec<_> = added.into_iter().map(EdgeId::index).collect();

        removed.sort_unstable();
        added.sort_unstable();

        Self {
            kind: kind.into(),
            removed,
            added,
        }
    }
}

/// Derived summary of the current factor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorSummary {
    pub cycles: usize,
    pub giant_size: usize,
    pub residual_vertices: usize,
    pub hamiltonian: bool,
}

impl FactorSummary {
    fn from_decomposition(decomposition: &CycleDecomposition) -> Self {
        Self {
            cycles: decomposition.cycle_count(),
            giant_size: decomposition.largest_cycle().len(),
            residual_vertices: decomposition.residual_vertex_count(),
            hamiltonian: decomposition.is_hamiltonian(),
        }
    }
}

/// Integrity information for a dump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    pub algorithm: String,
    pub state_sha256: String,
}

/// Versioned persistent representation of a powercycle factor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dump {
    pub format: String,
    pub version: u32,
    pub dump_id: String,
    pub graph: GraphMetadata,
    pub provenance: Provenance,
    pub base_factor: FactorRecord,
    pub current_factor: FactorRecord,
    pub history: Vec<MoveRecord>,
    pub summary: FactorSummary,
    pub integrity: Integrity,
}

impl Dump {
    /// Builds and validates a dump from two valid 2-factors.
    ///
    /// The history must transform `base_factor` exactly into
    /// `current_factor`.
    pub fn new(
        graph: &PowerGraph,
        base_factor: &Factor,
        current_factor: &Factor,
        provenance: Provenance,
        history: Vec<MoveRecord>,
    ) -> Result<Self, FormatError> {
        base_factor.validate_two_factor(graph)?;
        current_factor.validate_two_factor(graph)?;

        let decomposition = CycleDecomposition::new(graph, current_factor)?;

        let metadata = GraphMetadata {
            k: graph.k(),
            n: graph.vertex_count(),
            edge_count: graph.edge_count(),
            sha256: graph_sha256(graph),
        };

        let mut dump = Self {
            format: DUMP_FORMAT.to_owned(),
            version: DUMP_VERSION,
            dump_id: String::new(),
            graph: metadata,
            provenance,
            base_factor: FactorRecord::from_factor(base_factor),
            current_factor: FactorRecord::from_factor(current_factor),
            history,
            summary: FactorSummary::from_decomposition(&decomposition),
            integrity: Integrity {
                algorithm: INTEGRITY_ALGORITHM.to_owned(),
                state_sha256: String::new(),
            },
        };

        dump.validate_semantics(graph)?;

        let state_sha256 = dump.compute_state_sha256();

        dump.dump_id.clone_from(&state_sha256);
        dump.integrity.state_sha256 = state_sha256;

        dump.validate_against(graph)?;

        Ok(dump)
    }

    /// Parses and fully validates a JSON dump.
    ///
    /// The graph is reconstructed from the `(k, n)` metadata contained in
    /// the dump.
    pub fn from_json_str(json: &str) -> Result<Self, FormatError> {
        let dump: Self = serde_json::from_str(json)?;
        dump.validate()?;
        Ok(dump)
    }

    /// Reads and fully validates a JSON dump from disk.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, FormatError> {
        let json = fs::read_to_string(path)?;
        Self::from_json_str(&json)
    }

    /// Serializes the dump as pretty-printed JSON.
    pub fn to_json_pretty(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Writes pretty-printed JSON followed by a newline.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), FormatError> {
        let mut json = self.to_json_pretty()?;
        json.push('\n');

        fs::write(path, json)?;

        Ok(())
    }

    /// Reconstructs the graph described by the dump.
    pub fn rebuild_graph(&self) -> Result<PowerGraph, FormatError> {
        Ok(PowerGraph::new(self.graph.k, self.graph.n)?)
    }

    /// Reconstructs the base factor against a supplied graph.
    pub fn build_base_factor(&self, graph: &PowerGraph) -> Result<Factor, FormatError> {
        self.factor_from_record(graph, &self.base_factor)
    }

    /// Reconstructs the current factor against a supplied graph.
    pub fn build_current_factor(&self, graph: &PowerGraph) -> Result<Factor, FormatError> {
        self.factor_from_record(graph, &self.current_factor)
    }

    /// Fully validates the dump by reconstructing its graph.
    pub fn validate(&self) -> Result<(), FormatError> {
        self.validate_header()?;

        let graph = self.rebuild_graph()?;
        self.validate_against(&graph)
    }

    /// Fully validates the dump against an already constructed graph.
    pub fn validate_against(&self, graph: &PowerGraph) -> Result<(), FormatError> {
        self.validate_header()?;
        self.validate_graph_metadata(graph)?;
        self.validate_semantics(graph)?;

        if self.integrity.algorithm != INTEGRITY_ALGORITHM {
            return Err(FormatError::UnsupportedIntegrityAlgorithm {
                found: self.integrity.algorithm.clone(),
            });
        }

        let expected = self.compute_state_sha256();

        if self.integrity.state_sha256 != expected {
            return Err(FormatError::IntegrityMismatch {
                expected: expected.clone(),
                found: self.integrity.state_sha256.clone(),
            });
        }

        if self.dump_id != expected {
            return Err(FormatError::DumpIdMismatch {
                expected,
                found: self.dump_id.clone(),
            });
        }

        Ok(())
    }

    fn validate_header(&self) -> Result<(), FormatError> {
        if self.format != DUMP_FORMAT {
            return Err(FormatError::InvalidFormat {
                found: self.format.clone(),
            });
        }

        if self.version != DUMP_VERSION {
            return Err(FormatError::UnsupportedVersion {
                found: self.version,
            });
        }

        Ok(())
    }

    fn validate_graph_metadata(&self, graph: &PowerGraph) -> Result<(), FormatError> {
        if self.graph.k != graph.k() {
            return Err(FormatError::GraphMetadataMismatch {
                field: "k",
                expected: graph.k().to_string(),
                found: self.graph.k.to_string(),
            });
        }

        if self.graph.n != graph.vertex_count() {
            return Err(FormatError::GraphMetadataMismatch {
                field: "n",
                expected: graph.vertex_count().to_string(),
                found: self.graph.n.to_string(),
            });
        }

        if self.graph.edge_count != graph.edge_count() {
            return Err(FormatError::GraphMetadataMismatch {
                field: "edge_count",
                expected: graph.edge_count().to_string(),
                found: self.graph.edge_count.to_string(),
            });
        }

        let expected_hash = graph_sha256(graph);

        if self.graph.sha256 != expected_hash {
            return Err(FormatError::GraphHashMismatch {
                expected: expected_hash,
                found: self.graph.sha256.clone(),
            });
        }

        Ok(())
    }

    fn validate_semantics(&self, graph: &PowerGraph) -> Result<(), FormatError> {
        validate_edge_sequence(
            "base_factor.selected_edges",
            &self.base_factor.selected_edges,
            graph.edge_count(),
        )?;

        validate_edge_sequence(
            "current_factor.selected_edges",
            &self.current_factor.selected_edges,
            graph.edge_count(),
        )?;

        let base_factor = self.factor_from_record(graph, &self.base_factor)?;

        let current_factor = self.factor_from_record(graph, &self.current_factor)?;

        base_factor.validate_two_factor(graph)?;
        current_factor.validate_two_factor(graph)?;

        self.validate_history(graph, &base_factor, &current_factor)?;

        let decomposition = CycleDecomposition::new(graph, &current_factor)?;

        let expected_summary = FactorSummary::from_decomposition(&decomposition);

        if self.summary != expected_summary {
            return Err(FormatError::SummaryMismatch {
                expected: expected_summary,
                found: self.summary.clone(),
            });
        }

        Ok(())
    }

    fn factor_from_record(
        &self,
        graph: &PowerGraph,
        record: &FactorRecord,
    ) -> Result<Factor, FormatError> {
        let edge_ids = record
            .selected_edges
            .iter()
            .copied()
            .map(EdgeId::from_index);

        Ok(Factor::new(graph, edge_ids)?)
    }

    fn validate_history(
        &self,
        graph: &PowerGraph,
        base_factor: &Factor,
        current_factor: &Factor,
    ) -> Result<(), FormatError> {
        let edge_count = graph.edge_count();

        let mut selected = vec![false; edge_count];
        let mut degrees = vec![0usize; graph.vertex_count()];

        for &edge_id in base_factor.selected_edges() {
            selected[edge_id.index()] = true;

            let edge = graph
                .edge(edge_id)
                .expect("validated factor edge must exist");

            let (a, b) = edge.endpoints();

            degrees[a.index()] += 1;
            degrees[b.index()] += 1;
        }

        for (move_index, movement) in self.history.iter().enumerate() {
            validate_edge_sequence(
                &format!("history[{move_index}].removed"),
                &movement.removed,
                edge_count,
            )?;

            validate_edge_sequence(
                &format!("history[{move_index}].added"),
                &movement.added,
                edge_count,
            )?;

            let mut touched_vertices = Vec::new();

            for &index in &movement.removed {
                if !selected[index] {
                    return Err(FormatError::InvalidHistoryOperation {
                        move_index,
                        edge_id: index,
                        operation: "remove",
                        reason: "edge is not selected",
                    });
                }

                let edge_id = EdgeId::from_index(index);
                let edge = graph.edge(edge_id).expect("validated edge ID must exist");

                let (a, b) = edge.endpoints();

                selected[index] = false;

                degrees[a.index()] -= 1;
                degrees[b.index()] -= 1;

                touched_vertices.push(a.index());
                touched_vertices.push(b.index());
            }

            for &index in &movement.added {
                if selected[index] {
                    return Err(FormatError::InvalidHistoryOperation {
                        move_index,
                        edge_id: index,
                        operation: "add",
                        reason: "edge is already selected",
                    });
                }

                let edge_id = EdgeId::from_index(index);
                let edge = graph.edge(edge_id).expect("validated edge ID must exist");

                let (a, b) = edge.endpoints();

                selected[index] = true;

                degrees[a.index()] += 1;
                degrees[b.index()] += 1;

                touched_vertices.push(a.index());
                touched_vertices.push(b.index());
            }

            touched_vertices.sort_unstable();
            touched_vertices.dedup();

            for vertex_index in touched_vertices {
                let degree = degrees[vertex_index];

                if degree != 2 {
                    return Err(FormatError::HistoryBreaksTwoFactor {
                        move_index,
                        vertex: vertex_index + 1,
                        degree,
                    });
                }
            }
        }

        let replayed: Vec<usize> = selected
            .iter()
            .enumerate()
            .filter_map(|(index, &is_selected)| is_selected.then_some(index))
            .collect();

        let expected: Vec<usize> = current_factor
            .selected_edges()
            .iter()
            .map(|edge_id| edge_id.index())
            .collect();

        if replayed != expected {
            return Err(FormatError::HistoryDoesNotMatchCurrentFactor);
        }

        Ok(())
    }

    fn compute_state_sha256(&self) -> String {
        let mut hasher = SemanticHasher::new(STATE_HASH_DOMAIN);

        hasher.string(&self.format);
        hasher.u32(self.version);

        hasher.u32(self.graph.k);
        hasher.usize(self.graph.n);
        hasher.usize(self.graph.edge_count);
        hasher.string(&self.graph.sha256);

        hasher.string(&self.provenance.producer);
        hasher.string(&self.provenance.producer_version);
        hasher.optional_string(self.provenance.session.as_deref());
        hasher.optional_u64(self.provenance.invocation);
        hasher.optional_u64(self.provenance.tour);

        hasher.usize(self.provenance.reason.len());

        for reason in &self.provenance.reason {
            hasher.string(reason);
        }

        hasher.optional_string(self.provenance.parent_dump_id.as_deref());

        hasher.usize_slice(&self.base_factor.selected_edges);
        hasher.usize_slice(&self.current_factor.selected_edges);

        hasher.usize(self.history.len());

        for movement in &self.history {
            hasher.string(&movement.kind);
            hasher.usize_slice(&movement.removed);
            hasher.usize_slice(&movement.added);
        }

        hasher.usize(self.summary.cycles);
        hasher.usize(self.summary.giant_size);
        hasher.usize(self.summary.residual_vertices);
        hasher.boolean(self.summary.hamiltonian);

        hasher.finish()
    }
}

/// Computes the stable SHA-256 identity of a graph.
///
/// The hash includes `(k, n)`, the edge count and every edge endpoint in
/// deterministic edge-ID order. Integer values use fixed-width big-endian
/// encoding.
#[must_use]
pub fn graph_sha256(graph: &PowerGraph) -> String {
    let mut hasher = SemanticHasher::new(GRAPH_HASH_DOMAIN);

    hasher.u32(graph.k());
    hasher.usize(graph.vertex_count());
    hasher.usize(graph.edge_count());

    for edge in graph.edges() {
        let (a, b) = edge.endpoints();

        hasher.usize(a.number());
        hasher.usize(b.number());
    }

    hasher.finish()
}

fn validate_edge_sequence(
    context: &str,
    edge_ids: &[usize],
    edge_count: usize,
) -> Result<(), FormatError> {
    let mut previous = None;

    for &edge_id in edge_ids {
        if edge_id >= edge_count {
            return Err(FormatError::InvalidEdgeId {
                context: context.to_owned(),
                edge_id,
                edge_count,
            });
        }

        if let Some(previous) = previous {
            if edge_id <= previous {
                return Err(FormatError::NonCanonicalEdgeSequence {
                    context: context.to_owned(),
                });
            }
        }

        previous = Some(edge_id);
    }

    Ok(())
}

struct SemanticHasher {
    hasher: Sha256,
}

impl SemanticHasher {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain);

        Self { hasher }
    }

    fn boolean(&mut self, value: bool) {
        self.hasher.update([u8::from(value)]);
    }

    fn u32(&mut self, value: u32) {
        self.hasher.update(value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.hasher.update(value.to_be_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn string(&mut self, value: &str) {
        self.usize(value.len());
        self.hasher.update(value.as_bytes());
    }

    fn optional_string(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.boolean(true);
                self.string(value);
            }

            None => self.boolean(false),
        }
    }

    fn optional_u64(&mut self, value: Option<u64>) {
        match value {
            Some(value) => {
                self.boolean(true);
                self.u64(value);
            }

            None => self.boolean(false),
        }
    }

    fn usize_slice(&mut self, values: &[usize]) {
        self.usize(values.len());

        for &value in values {
            self.usize(value);
        }
    }

    fn finish(self) -> String {
        let digest = self.hasher.finalize();

        let mut result = String::with_capacity(digest.len() * 2);

        for byte in digest {
            write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
        }

        result
    }
}

/// Errors related to persistent powercycle data.
#[derive(Debug)]
pub enum FormatError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Graph(GraphError),
    Factor(FactorError),

    InvalidFormat {
        found: String,
    },

    UnsupportedVersion {
        found: u32,
    },

    GraphMetadataMismatch {
        field: &'static str,
        expected: String,
        found: String,
    },

    GraphHashMismatch {
        expected: String,
        found: String,
    },

    InvalidEdgeId {
        context: String,
        edge_id: usize,
        edge_count: usize,
    },

    NonCanonicalEdgeSequence {
        context: String,
    },

    InvalidHistoryOperation {
        move_index: usize,
        edge_id: usize,
        operation: &'static str,
        reason: &'static str,
    },

    HistoryBreaksTwoFactor {
        move_index: usize,
        vertex: usize,
        degree: usize,
    },

    HistoryDoesNotMatchCurrentFactor,

    SummaryMismatch {
        expected: FactorSummary,
        found: FactorSummary,
    },

    UnsupportedIntegrityAlgorithm {
        found: String,
    },

    IntegrityMismatch {
        expected: String,
        found: String,
    },

    DumpIdMismatch {
        expected: String,
        found: String,
    },
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),

            Self::Json(error) => error.fmt(f),

            Self::Graph(error) => error.fmt(f),

            Self::Factor(error) => error.fmt(f),

            Self::InvalidFormat { found } => {
                write!(f, "invalid dump format {found:?}: expected {DUMP_FORMAT:?}")
            }

            Self::UnsupportedVersion { found } => {
                write!(
                    f,
                    "unsupported dump version {found}: expected version {DUMP_VERSION}"
                )
            }

            Self::GraphMetadataMismatch {
                field,
                expected,
                found,
            } => {
                write!(
                    f,
                    "graph metadata mismatch for {field}: expected {expected}, found {found}"
                )
            }

            Self::GraphHashMismatch { expected, found } => {
                write!(
                    f,
                    "graph SHA-256 mismatch: expected {expected}, found {found}"
                )
            }

            Self::InvalidEdgeId {
                context,
                edge_id,
                edge_count,
            } => {
                write!(
                    f,
                    "invalid edge ID {edge_id} in {context}: graph contains {edge_count} edges"
                )
            }

            Self::NonCanonicalEdgeSequence { context } => {
                write!(f, "edge IDs in {context} must be strictly increasing")
            }

            Self::InvalidHistoryOperation {
                move_index,
                edge_id,
                operation,
                reason,
            } => {
                write!(
                    f,
                    "invalid history move {move_index}: cannot {operation} edge {edge_id}: {reason}"
                )
            }

            Self::HistoryBreaksTwoFactor {
                move_index,
                vertex,
                degree,
            } => {
                write!(
                    f,
                    "history move {move_index} breaks the 2-factor at vertex {vertex}: selected degree is {degree}"
                )
            }

            Self::HistoryDoesNotMatchCurrentFactor => {
                write!(f, "replaying history does not reproduce current_factor")
            }

            Self::SummaryMismatch { expected, found } => {
                write!(
                    f,
                    "dump summary mismatch: expected {expected:?}, found {found:?}"
                )
            }

            Self::UnsupportedIntegrityAlgorithm { found } => {
                write!(
                    f,
                    "unsupported integrity algorithm {found:?}: expected {INTEGRITY_ALGORITHM:?}"
                )
            }

            Self::IntegrityMismatch { expected, found } => {
                write!(
                    f,
                    "dump integrity mismatch: expected {expected}, found {found}"
                )
            }

            Self::DumpIdMismatch { expected, found } => {
                write!(f, "dump ID mismatch: expected {expected}, found {found}")
            }
        }
    }
}

impl Error for FormatError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Graph(error) => Some(error),
            Self::Factor(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for FormatError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for FormatError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<GraphError> for FormatError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}

impl From<FactorError> for FormatError {
    fn from(error: FactorError) -> Self {
        Self::Factor(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hamiltonian_factor(graph: &PowerGraph) -> Factor {
        let cycle = [
            1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9,
            27, 22, 14, 2, 23, 26, 10, 15,
        ];

        factor_from_cycles(graph, &[&cycle])
    }

    fn two_cycle_factor(graph: &PowerGraph) -> Factor {
        let cycle_a = [1, 8, 28, 21, 15];

        let cycle_b = [
            2, 14, 11, 25, 24, 12, 13, 3, 22, 27, 9, 16, 20, 29, 7, 18, 31, 5, 4, 32, 17, 19, 30,
            6, 10, 26, 23,
        ];

        factor_from_cycles(graph, &[&cycle_a, &cycle_b])
    }

    fn factor_from_cycles(graph: &PowerGraph, cycles: &[&[usize]]) -> Factor {
        let mut selected_edges = Vec::new();

        for cycle in cycles {
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
        }

        Factor::new(graph, selected_edges).unwrap()
    }

    fn symmetric_difference_move(base: &Factor, current: &Factor) -> MoveRecord {
        let removed = base
            .selected_edges()
            .iter()
            .copied()
            .filter(|&edge_id| !current.contains(edge_id));

        let added = current
            .selected_edges()
            .iter()
            .copied()
            .filter(|&edge_id| !base.contains(edge_id));

        MoveRecord::new("test-transform", removed, added)
    }

    #[test]
    fn graph_hash_is_deterministic() {
        let graph_a = PowerGraph::new(2, 32).unwrap();
        let graph_b = PowerGraph::new(2, 32).unwrap();
        let graph_c = PowerGraph::new(2, 33).unwrap();

        assert_eq!(graph_sha256(&graph_a), graph_sha256(&graph_b));
        assert_ne!(graph_sha256(&graph_a), graph_sha256(&graph_c));
    }

    #[test]
    fn creates_and_validates_initial_dump() {
        let graph = PowerGraph::new(2, 32).unwrap();
        let factor = hamiltonian_factor(&graph);

        let provenance = Provenance::new("pc-search", "0.1.0");

        let dump = Dump::new(&graph, &factor, &factor, provenance, Vec::new()).unwrap();

        assert_eq!(dump.format, DUMP_FORMAT);
        assert_eq!(dump.version, DUMP_VERSION);
        assert_eq!(dump.graph.k, 2);
        assert_eq!(dump.graph.n, 32);

        assert_eq!(dump.summary.cycles, 1);
        assert_eq!(dump.summary.giant_size, 32);
        assert_eq!(dump.summary.residual_vertices, 0);
        assert!(dump.summary.hamiltonian);

        assert_eq!(dump.dump_id.len(), 64);
        assert_eq!(dump.dump_id, dump.integrity.state_sha256);

        dump.validate().unwrap();
    }

    #[test]
    fn json_round_trip_preserves_dump() {
        let graph = PowerGraph::new(2, 32).unwrap();
        let factor = hamiltonian_factor(&graph);

        let mut provenance = Provenance::new("pc-search", "0.1.0");

        provenance.session = Some("test-session".to_owned());
        provenance.invocation = Some(1);
        provenance.tour = Some(42);
        provenance.reason = vec!["cycle-record".to_owned(), "patchability-record".to_owned()];

        let dump = Dump::new(&graph, &factor, &factor, provenance, Vec::new()).unwrap();

        let json = dump.to_json_pretty().unwrap();
        let restored = Dump::from_json_str(&json).unwrap();

        assert_eq!(restored, dump);
    }

    #[test]
    fn validates_history_replay() {
        let graph = PowerGraph::new(2, 32).unwrap();

        let base = two_cycle_factor(&graph);
        let current = hamiltonian_factor(&graph);

        let movement = symmetric_difference_move(&base, &current);

        let dump = Dump::new(
            &graph,
            &base,
            &current,
            Provenance::new("pc-patch", "0.1.0"),
            vec![movement],
        )
        .unwrap();

        assert_eq!(dump.summary.cycles, 1);
        assert_eq!(dump.history.len(), 1);

        dump.validate().unwrap();
    }

    #[test]
    fn detects_integrity_tampering() {
        let graph = PowerGraph::new(2, 32).unwrap();
        let factor = hamiltonian_factor(&graph);

        let dump = Dump::new(
            &graph,
            &factor,
            &factor,
            Provenance::new("pc-search", "0.1.0"),
            Vec::new(),
        )
        .unwrap();

        let mut value = serde_json::to_value(&dump).unwrap();

        value["provenance"]["producer"] = serde_json::Value::String("tampered".to_owned());

        let json = serde_json::to_string(&value).unwrap();

        assert!(matches!(
            Dump::from_json_str(&json),
            Err(FormatError::IntegrityMismatch { .. })
        ));
    }

    #[test]
    fn detects_graph_hash_tampering() {
        let graph = PowerGraph::new(2, 32).unwrap();
        let factor = hamiltonian_factor(&graph);

        let dump = Dump::new(
            &graph,
            &factor,
            &factor,
            Provenance::new("pc-search", "0.1.0"),
            Vec::new(),
        )
        .unwrap();

        let mut value = serde_json::to_value(&dump).unwrap();

        value["graph"]["sha256"] = serde_json::Value::String("deadbeef".to_owned());

        let json = serde_json::to_string(&value).unwrap();

        assert!(matches!(
            Dump::from_json_str(&json),
            Err(FormatError::GraphHashMismatch { .. })
        ));
    }
}
