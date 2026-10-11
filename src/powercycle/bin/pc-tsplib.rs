//! Bridge to LKH-3: export `G_k(n)` as a TSPLIB Hamiltonian cycle problem, and
//! read the tour LKH finds back as a canonical witness.
//!
//! The export runs the preflight first. Edges its degree-two propagation
//! forces are the contracted chains and go to `FIXED_EDGES_SECTION`; edges it
//! forbids are left out of `EDGE_DATA_SECTION`, which LKH takes as the graph.
//!
//! With `--contract`, each chain of forced edges becomes its two ends joined
//! by one fixed edge, so LKH never stores the interior vertices, which have
//! nothing left to choose. The chains go to `<out>.chains`, which `tour` needs
//! to put the interior back. See section 7 of `doc/en/performance.md`.

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use powercycle_core::{PowerGraph, canonical_order, check_hamiltonian_cycle};
use powercycle_search::preflight::{EdgeState, hamiltonian_preflight};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-tsplib: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();

    match parse_arguments(&arguments)? {
        Command::Help => {
            print_help();
            Ok(())
        }

        Command::Export {
            k,
            n,
            output,
            contract,
        } => export(k, n, &output, contract),

        Command::Tour {
            k,
            n,
            input,
            chains,
        } => tour(k, n, &input, chains.as_deref()),
    }
}

fn export(k: u32, n: usize, output: &Path, contract: bool) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    let graph = PowerGraph::new(k, n)?;
    let report = hamiltonian_preflight(&graph);

    if let Some(obstruction) = &report.obstruction {
        return Err(format!("the preflight rules G_{k}({n}) out: {obstruction}").into());
    }

    let states = &report.edge_states;

    if states.len() != graph.edge_count() {
        return Err("the preflight stopped before degree-two propagation".into());
    }

    let count = |wanted: EdgeState| states.iter().filter(|&&state| state == wanted).count();
    let fixed = count(EdgeState::Forced);
    let removed = count(EdgeState::Forbidden);

    eprintln!(
        "pc-tsplib: G_{k}({n}): {} edges, {fixed} fixed, {removed} removed by propagation",
        graph.edge_count()
    );

    let edges = || {
        graph
            .edges()
            .iter()
            .zip(states)
            .map(|(edge, &state)| (edge.first().number(), edge.second().number(), state))
    };

    let mut file = BufWriter::with_capacity(1 << 20, File::create(output)?);

    if contract {
        if report.forced_spanning_cycle {
            return Err(format!(
                "propagation alone forces a Hamiltonian cycle of G_{k}({n}); there is nothing to search"
            )
            .into());
        }

        let forced = edges()
            .filter(|&(_, _, state)| state == EdgeState::Forced)
            .map(|(a, b, _)| (a, b));

        let contraction = Contraction::new(n, forced)?;
        let m = contraction.node_count();

        if m < 3 {
            return Err(format!("contraction leaves {m} nodes, too few for a tour").into());
        }

        // Every edge left to choose joins two nodes: propagation forbids the
        // free edges of a vertex with two forced ones. An edge joining the two
        // ends of one chain would close it early, and is dropped here.
        let mut free = Vec::new();
        let mut closing = 0usize;

        for (a, b, state) in edges() {
            if state != EdgeState::Unknown {
                continue;
            }

            let (Some(x), Some(y)) = (contraction.node(a), contraction.node(b)) else {
                return Err(format!("edge {a}-{b} is free at an interior vertex").into());
            };

            if contraction.partner(x) == Some(y) {
                closing += 1;
            } else {
                free.push((x, y));
            }
        }

        writeln!(file, "NAME : G{k}_{n}_contracted")?;
        writeln!(
            file,
            "COMMENT : power-sum graph G_{k}({n}) with {} chains of forced edges contracted to \
             {m} nodes; tour expands with {}",
            contraction.chain_count(),
            chains_name(output)?
        )?;
        writeln!(file, "TYPE : HCP")?;
        writeln!(file, "DIMENSION : {m}")?;
        writeln!(file, "EDGE_DATA_FORMAT : EDGE_LIST")?;
        writeln!(file, "EDGE_DATA_SECTION")?;

        for &(x, y) in free.iter().chain(&contraction.links) {
            writeln!(file, "{x} {y}")?;
        }

        writeln!(file, "-1")?;
        writeln!(file, "FIXED_EDGES_SECTION")?;

        for &(x, y) in &contraction.links {
            writeln!(file, "{x} {y}")?;
        }

        writeln!(file, "-1")?;
        writeln!(file, "EOF")?;
        file.flush()?;

        let chains = output.with_extension("chains");
        contraction.write_chains(k, &chains)?;

        eprintln!(
            "pc-tsplib: contracted to {m} nodes ({:.1}% of n): {} chains, {} interior vertices \
             removed, {closing} closing edges dropped",
            100.0 * m as f64 / n as f64,
            contraction.chain_count(),
            n - m
        );
        eprintln!("pc-tsplib: wrote {}", chains.display());
    } else {
        writeln!(file, "NAME : G{k}_{n}")?;
        writeln!(
            file,
            "COMMENT : power-sum graph G_{k}({n}); {fixed} edges fixed and {removed} removed \
             by degree-two propagation"
        )?;
        writeln!(file, "TYPE : HCP")?;
        writeln!(file, "DIMENSION : {n}")?;
        writeln!(file, "EDGE_DATA_FORMAT : EDGE_LIST")?;
        writeln!(file, "EDGE_DATA_SECTION")?;

        for (a, b, state) in edges() {
            if state != EdgeState::Forbidden {
                writeln!(file, "{a} {b}")?;
            }
        }

        writeln!(file, "-1")?;
        writeln!(file, "FIXED_EDGES_SECTION")?;

        for (a, b, state) in edges() {
            if state == EdgeState::Forced {
                writeln!(file, "{a} {b}")?;
            }
        }

        writeln!(file, "-1")?;
        writeln!(file, "EOF")?;
        file.flush()?;
    }

    let parameters = output.with_extension("par");
    fs::write(&parameters, parameter_file(output)?)?;

    eprintln!(
        "pc-tsplib: wrote {} and {} in {:.1} s",
        output.display(),
        parameters.display(),
        start.elapsed().as_secs_f64()
    );

    Ok(())
}

fn chains_name(output: &Path) -> Result<String, Box<dyn Error>> {
    Ok(output
        .with_extension("chains")
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("output file name must be UTF-8")?
        .to_owned())
}

/// The forced edges of a graph, as disjoint paths, and the node numbering
/// that keeps only their ends and the vertices on no path.
///
/// Node numbers run from 1 in increasing vertex order, so they can be rebuilt
/// from the chains alone: the interior vertices are exactly those listed
/// strictly inside a chain.
struct Contraction {
    /// Node of each vertex, 0 for an interior vertex; index 0 unused.
    node_of: Vec<u32>,

    /// Vertex of each node; index 0 unused.
    vertex_of: Vec<u32>,

    /// The other end of each chain end, 0 for a vertex on no chain.
    partner: Vec<u32>,

    /// One fixed edge per chain, between the nodes of its two ends.
    links: Vec<(u32, u32)>,

    /// The chains with at least one interior vertex, end to end, flattened.
    chain_vertices: Vec<u32>,

    /// Where each such chain starts in `chain_vertices`, plus a final bound.
    chain_starts: Vec<usize>,
}

impl Contraction {
    fn new(n: usize, forced: impl Iterator<Item = (usize, usize)>) -> Result<Self, String> {
        if u32::try_from(n).is_err() {
            return Err(format!(
                "contraction stores vertices as u32; n = {n} is too large"
            ));
        }

        // Up to two forced neighbours per vertex, 0 meaning none.
        let mut neighbours = vec![[0u32; 2]; n + 1];

        for (a, b) in forced {
            for (from, to) in [(a, b), (b, a)] {
                let slot = neighbours[from]
                    .iter_mut()
                    .find(|slot| **slot == 0)
                    .ok_or_else(|| format!("vertex {from} has more than two forced edges"))?;
                *slot = to as u32;
            }
        }

        let degree = |v: usize| neighbours[v].iter().filter(|&&u| u != 0).count();

        let mut node_of = vec![0u32; n + 1];
        let mut vertex_of = vec![0u32];

        for (v, node) in node_of.iter_mut().enumerate().skip(1) {
            if degree(v) < 2 {
                vertex_of.push(v as u32);
                *node = (vertex_of.len() - 1) as u32;
            }
        }

        let mut partner = vec![0u32; vertex_of.len()];
        let mut links = Vec::new();
        let mut chain_vertices = Vec::new();
        let mut chain_starts = Vec::new();
        let mut covered = 0usize;

        for v in 1..=n {
            if degree(v) != 1 || partner[node_of[v] as usize] != 0 {
                continue;
            }

            let mut chain = vec![v as u32];
            let mut previous = 0u32;
            let mut current = v as u32;

            loop {
                let next = neighbours[current as usize]
                    .into_iter()
                    .find(|&u| u != 0 && u != previous);

                let Some(next) = next else { break };

                chain.push(next);
                previous = current;
                current = next;
            }

            let (x, y) = (node_of[v], node_of[current as usize]);
            partner[x as usize] = y;
            partner[y as usize] = x;
            links.push((x, y));
            covered += chain.len();

            if chain.len() > 2 {
                chain_starts.push(chain_vertices.len());
                chain_vertices.extend_from_slice(&chain);
            }
        }

        // Interior vertices outside every chain would sit on a forced cycle,
        // which the preflight reports as an obstruction before this point.
        let on_paths = (1..=n).filter(|&v| degree(v) > 0).count();

        if covered != on_paths {
            return Err("forced edges contain a cycle".to_owned());
        }

        chain_starts.push(chain_vertices.len());

        Ok(Self {
            node_of,
            vertex_of,
            partner,
            links,
            chain_vertices,
            chain_starts,
        })
    }

    fn node_count(&self) -> usize {
        self.vertex_of.len() - 1
    }

    fn chain_count(&self) -> usize {
        self.links.len()
    }

    fn node(&self, vertex: usize) -> Option<u32> {
        Some(self.node_of[vertex]).filter(|&node| node != 0)
    }

    fn partner(&self, node: u32) -> Option<u32> {
        Some(self.partner[node as usize]).filter(|&other| other != 0)
    }

    fn write_chains(&self, k: u32, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut file = BufWriter::with_capacity(1 << 20, File::create(path)?);
        let long = self.chain_starts.len() - 1;

        writeln!(file, "PC-TSPLIB CHAINS")?;
        writeln!(file, "K : {k}")?;
        writeln!(file, "N : {}", self.node_of.len() - 1)?;
        writeln!(file, "NODES : {}", self.node_count())?;
        writeln!(file, "CHAINS : {long}")?;

        for window in self.chain_starts.windows(2) {
            let chain = &self.chain_vertices[window[0]..window[1]];
            let line = chain.iter().map(u32::to_string).collect::<Vec<_>>();
            writeln!(file, "{}", line.join(" "))?;
        }

        writeln!(file, "EOF")?;
        file.flush()?;

        Ok(())
    }
}

/// The chains file read back: enough to number the nodes and to put each
/// chain's interior between its two ends.
struct Expansion {
    vertex_of: Vec<u32>,

    /// For each node that ends a chain with an interior, that chain plus one.
    chain_of: Vec<u32>,

    chain_vertices: Vec<u32>,
    chain_starts: Vec<usize>,
}

impl Expansion {
    fn read(path: &Path, k: u32, n: usize) -> Result<Self, Box<dyn Error>> {
        let file =
            File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
        let mut lines = BufReader::new(file).lines();

        let mut header = |key: &str| -> Result<String, Box<dyn Error>> {
            let line = lines.next().ok_or("chains file ends in its header")??;
            line.strip_prefix(key)
                .map(|value| value.trim().to_owned())
                .ok_or_else(|| format!("chains file: expected {key:?}, found {line:?}").into())
        };

        header("PC-TSPLIB CHAINS")?;

        if header("K :")?.parse::<u32>()? != k || header("N :")?.parse::<usize>()? != n {
            return Err(format!("chains file was not written for G_{k}({n})").into());
        }

        let nodes = header("NODES :")?.parse::<usize>()?;
        let count = header("CHAINS :")?.parse::<usize>()?;

        let mut chain_vertices = Vec::new();
        let mut chain_starts = Vec::with_capacity(count + 1);
        let mut interior = vec![false; n + 1];

        for line in lines {
            let line = line?;

            if line == "EOF" {
                break;
            }

            let chain = line
                .split_whitespace()
                .map(|token| token.parse::<u32>())
                .collect::<Result<Vec<_>, _>>()?;

            if chain.len() < 3 || chain.iter().any(|&v| v == 0 || v as usize > n) {
                return Err(format!("chains file: malformed chain {line:?}").into());
            }

            for &v in &chain[1..chain.len() - 1] {
                interior[v as usize] = true;
            }

            chain_starts.push(chain_vertices.len());
            chain_vertices.extend_from_slice(&chain);
        }

        chain_starts.push(chain_vertices.len());

        if chain_starts.len() - 1 != count {
            return Err("chains file: chain count does not match its header".into());
        }

        let mut vertex_of = vec![0u32];
        let mut node_of = vec![0u32; n + 1];

        for (v, node) in node_of.iter_mut().enumerate().skip(1) {
            if !interior[v] {
                vertex_of.push(v as u32);
                *node = (vertex_of.len() - 1) as u32;
            }
        }

        if vertex_of.len() - 1 != nodes {
            return Err("chains file: node count does not match its chains".into());
        }

        let mut chain_of = vec![0u32; vertex_of.len()];

        for (index, window) in chain_starts.windows(2).enumerate() {
            for v in [chain_vertices[window[0]], chain_vertices[window[1] - 1]] {
                chain_of[node_of[v as usize] as usize] = index as u32 + 1;
            }
        }

        Ok(Self {
            vertex_of,
            chain_of,
            chain_vertices,
            chain_starts,
        })
    }

    /// Maps a tour over nodes back to vertices, each chain's interior put
    /// between its two ends in the direction the tour crosses them.
    fn expand(&self, tour: &[usize]) -> Result<Vec<usize>, String> {
        let m = self.vertex_of.len() - 1;

        if tour.len() != m || tour.iter().any(|&node| node == 0 || node > m) {
            return Err(format!(
                "tour does not visit the {m} nodes of the contracted problem"
            ));
        }

        let mut cycle = Vec::with_capacity(self.chain_vertices.len() + m);

        for (index, &x) in tour.iter().enumerate() {
            let y = tour[(index + 1) % tour.len()];
            let vertex = self.vertex_of[x] as usize;
            cycle.push(vertex);

            let chain = self.chain_of[x];

            if chain == 0 || chain != self.chain_of[y] {
                continue;
            }

            let range = self.chain_starts[chain as usize - 1]..self.chain_starts[chain as usize];
            let body = &self.chain_vertices[range];
            let interior = &body[1..body.len() - 1];

            if body[0] as usize == vertex {
                cycle.extend(interior.iter().map(|&v| v as usize));
            } else {
                cycle.extend(interior.iter().rev().map(|&v| v as usize));
            }
        }

        Ok(cycle)
    }
}

/// LKH parameters for a problem written to `problem`, with paths relative to
/// its directory, from which LKH is meant to run.
fn parameter_file(problem: &Path) -> Result<String, Box<dyn Error>> {
    let name = |path: &Path| -> Result<String, Box<dyn Error>> {
        Ok(path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("output file name must be UTF-8")?
            .to_owned())
    };

    let problem_name = name(problem)?;
    let tour_name = name(&problem.with_extension("tour"))?;
    let progress_name = name(&problem.with_extension("$.tour"))?;

    // A Hamiltonian cycle is a tour of cost 0, and LKH stops at the first one.
    // Every graph edge costs 0, so the subgradient ascent LKH runs by default,
    // n/2 iterations, has nothing to learn; at k = 5 it turned a 5 s run into
    // one stopped after ten minutes. OUTPUT_TOUR_FILE is rewritten at each
    // improvement, $ standing for the cost, so a stopped run keeps its best
    // tour.
    Ok(format!(
        "PROBLEM_FILE = {problem_name}\n\
         TOUR_FILE = {tour_name}\n\
         OUTPUT_TOUR_FILE = {progress_name}\n\
         SUBGRADIENT = NO\n\
         OPTIMUM = 0\n\
         STOP_AT_OPTIMUM = YES\n\
         RUNS = 1\n\
         TRACE_LEVEL = 1\n"
    ))
}

fn tour(k: u32, n: usize, input: &Path, chains: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(input)
        .map_err(|error| format!("cannot read {}: {error}", input.display()))?;

    let mut cycle = parse_tour(&text)?;

    if let Some(chains) = chains {
        cycle = Expansion::read(chains, k, n)?.expand(&cycle)?;
    }

    check_hamiltonian_cycle(k, n, &cycle)
        .map_err(|error| format!("not a Hamiltonian cycle of G_{k}({n}): {error}"))?;

    let canonical = canonical_order(&cycle)?;

    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());

    for vertex in &canonical {
        writeln!(output, "{vertex}")?;
    }

    output.flush()?;

    eprintln!(
        "pc-tsplib: G_{k}({n}): canonical start {}, second {}, last {}",
        canonical[0],
        canonical[1],
        canonical[canonical.len() - 1]
    );

    Ok(())
}

/// The node sequence of a TSPLIB tour file: the integers after
/// `TOUR_SECTION`, up to the terminating `-1`.
fn parse_tour(text: &str) -> Result<Vec<usize>, TourError> {
    let mut lines = text.lines();

    lines
        .by_ref()
        .find(|line| line.trim() == "TOUR_SECTION")
        .ok_or(TourError::MissingSection)?;

    let mut cycle = Vec::new();

    for token in lines.flat_map(str::split_whitespace) {
        if token == "-1" {
            return Ok(cycle);
        }

        cycle.push(token.parse().map_err(|_| TourError::InvalidNode {
            value: token.to_owned(),
        })?);
    }

    Err(TourError::Unterminated)
}

fn parse_arguments(arguments: &[String]) -> Result<Command, CliError> {
    let parse_k = |value: &String| {
        value
            .parse::<u32>()
            .ok()
            .filter(|&k| k >= 2)
            .ok_or_else(|| CliError::InvalidExponent {
                value: value.clone(),
            })
    };

    let parse_n = |value: &String| {
        value
            .parse::<usize>()
            .ok()
            .filter(|&n| n >= 3)
            .ok_or_else(|| CliError::InvalidVertexCount {
                value: value.clone(),
            })
    };

    let words = arguments.iter().map(String::as_str).collect::<Vec<_>>();

    match words.as_slice() {
        ["-h" | "--help"] => Ok(Command::Help),

        ["export", rest @ ..] => {
            let (contract, rest) = match rest {
                ["--contract", rest @ ..] => (true, rest),
                _ => (false, rest),
            };

            let [k, n, output] = rest else {
                return Err(CliError::Usage);
            };

            Ok(Command::Export {
                k: parse_k(&(*k).to_owned())?,
                n: parse_n(&(*n).to_owned())?,
                output: PathBuf::from(output),
                contract,
            })
        }

        ["tour", k, n, input, chains @ ..] if chains.len() <= 1 => Ok(Command::Tour {
            k: parse_k(&(*k).to_owned())?,
            n: parse_n(&(*n).to_owned())?,
            input: PathBuf::from(input),
            chains: chains.first().map(PathBuf::from),
        }),

        _ => Err(CliError::Usage),
    }
}

fn print_help() {
    println!(
        "pc-tsplib {version}\n\
         \n\
         Bridge to LKH-3 for power-sum graphs.\n\
         \n\
         Usage:\n\
           pc-tsplib export [--contract] <k> <n> <out.hcp>\n\
           pc-tsplib tour <k> <n> <file.tour> [<out>.chains] > cycle.txt\n\
         \n\
         export  runs the preflight, then writes G_k(n) as a TSPLIB HCP problem\n\
                 with the forced edges fixed and the forbidden ones removed,\n\
                 and an LKH parameter file <out>.par next to it\n\
         --contract  replaces each chain of forced edges by its two ends\n\
                 joined by a fixed edge, and writes the chains to\n\
                 <out>.chains; tour then needs that file\n\
         tour    checks that an LKH tour is a Hamiltonian cycle of G_k(n) and\n\
                 writes it in canonical witness order, as pc-export does\n\
         \n\
         Run LKH from the directory holding the files: LKH <out>.par",
        version = env!("CARGO_PKG_VERSION"),
    );
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Export {
        k: u32,
        n: usize,
        output: PathBuf,
        contract: bool,
    },
    Tour {
        k: u32,
        n: usize,
        input: PathBuf,
        chains: Option<PathBuf>,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    Usage,
    InvalidExponent { value: String },
    InvalidVertexCount { value: String },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(
                f,
                "usage: pc-tsplib export [--contract] <k> <n> <out.hcp> | \
                 pc-tsplib tour <k> <n> <file.tour> [<out>.chains]"
            ),

            Self::InvalidExponent { value } => {
                write!(f, "invalid exponent {value:?}: expected an integer >= 2")
            }

            Self::InvalidVertexCount { value } => {
                write!(
                    f,
                    "invalid vertex count {value:?}: expected an integer >= 3"
                )
            }
        }
    }
}

impl Error for CliError {}

#[derive(Debug, PartialEq, Eq)]
enum TourError {
    MissingSection,
    InvalidNode { value: String },
    Unterminated,
}

impl fmt::Display for TourError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSection => write!(f, "tour file has no TOUR_SECTION"),
            Self::InvalidNode { value } => write!(f, "invalid node {value:?} in TOUR_SECTION"),
            Self::Unterminated => write!(f, "TOUR_SECTION is not terminated by -1"),
        }
    }
}

impl Error for TourError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_lkh_tour() {
        let text = "NAME : G2_32.0.tour\nCOMMENT : Length = 0\nTYPE : TOUR\n\
                    DIMENSION : 4\nTOUR_SECTION\n3\n1\n4\n2\n-1\nEOF\n";

        assert_eq!(parse_tour(text), Ok(vec![3, 1, 4, 2]));
    }

    #[test]
    fn rejects_malformed_tours() {
        assert_eq!(
            parse_tour("TYPE : TOUR\n1\n2\n-1\n"),
            Err(TourError::MissingSection)
        );
        assert_eq!(
            parse_tour("TOUR_SECTION\n1\n2\n"),
            Err(TourError::Unterminated)
        );
        assert!(matches!(
            parse_tour("TOUR_SECTION\n1\nx\n-1\n"),
            Err(TourError::InvalidNode { .. })
        ));
    }

    #[test]
    fn parses_both_commands() {
        let strings = |values: &[&str]| values.iter().map(|&v| v.to_owned()).collect::<Vec<_>>();

        assert_eq!(
            parse_arguments(&strings(&["export", "5", "246877", "g5.hcp"])),
            Ok(Command::Export {
                k: 5,
                n: 246_877,
                output: PathBuf::from("g5.hcp"),
                contract: false,
            })
        );

        assert_eq!(
            parse_arguments(&strings(&["tour", "2", "32", "g2.tour"])),
            Ok(Command::Tour {
                k: 2,
                n: 32,
                input: PathBuf::from("g2.tour"),
                chains: None,
            })
        );

        assert_eq!(
            parse_arguments(&strings(&[
                "export",
                "--contract",
                "6",
                "15748961",
                "g6.hcp"
            ])),
            Ok(Command::Export {
                k: 6,
                n: 15_748_961,
                output: PathBuf::from("g6.hcp"),
                contract: true,
            })
        );

        assert_eq!(
            parse_arguments(&strings(&["tour", "2", "32", "g2.tour", "g2.chains"])),
            Ok(Command::Tour {
                k: 2,
                n: 32,
                input: PathBuf::from("g2.tour"),
                chains: Some(PathBuf::from("g2.chains")),
            })
        );

        assert!(parse_arguments(&strings(&["tour", "2", "32", "a", "b", "c"])).is_err());

        assert!(parse_arguments(&strings(&["import", "2", "32", "x"])).is_err());
        assert!(parse_arguments(&strings(&["export", "1", "32", "x"])).is_err());
        assert!(parse_arguments(&strings(&["export", "2", "2", "x"])).is_err());
    }

    #[test]
    fn parameter_file_points_next_to_the_problem() {
        let text = parameter_file(Path::new("/some/dir/g6-15748961.hcp")).unwrap();

        assert!(text.contains("PROBLEM_FILE = g6-15748961.hcp\n"));
        assert!(text.contains("TOUR_FILE = g6-15748961.tour\n"));
        assert!(text.contains("OUTPUT_TOUR_FILE = g6-15748961.$.tour\n"));
        assert!(text.contains("SUBGRADIENT = NO\n"));
    }

    /// Two chains, 1-2-3-4 and 6-7, and a free vertex 5, out of n = 7.
    fn small() -> Contraction {
        Contraction::new(7, [(1, 2), (3, 2), (3, 4), (6, 7)].into_iter()).unwrap()
    }

    #[test]
    fn contraction_keeps_chain_ends_and_free_vertices() {
        let contraction = small();

        // Vertices 2 and 3 are interior; 1, 4, 5, 6, 7 become nodes 1..=5.
        assert_eq!(contraction.vertex_of, vec![0, 1, 4, 5, 6, 7]);
        assert_eq!(contraction.node(2), None);
        assert_eq!(contraction.node(5), Some(3));
        assert_eq!(contraction.links, vec![(1, 2), (4, 5)]);
        assert_eq!(contraction.partner(1), Some(2));
        assert_eq!(contraction.partner(3), None);
        assert_eq!(contraction.chain_vertices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn contraction_rejects_forced_cycles_and_overloaded_vertices() {
        assert!(Contraction::new(3, [(1, 2), (2, 3), (3, 1)].into_iter()).is_err());
        assert!(Contraction::new(4, [(1, 2), (1, 3), (1, 4)].into_iter()).is_err());
    }

    #[test]
    fn expansion_inverts_contraction_in_both_directions() {
        let dir = std::env::temp_dir().join(format!("pc-tsplib-unit-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("small.chains");
        small().write_chains(2, &path).unwrap();

        let expansion = Expansion::read(&path, 2, 7).unwrap();

        // Nodes 1..=5 are vertices 1, 4, 5, 6, 7.
        assert_eq!(
            expansion.expand(&[1, 2, 3, 4, 5]).unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(
            expansion.expand(&[2, 1, 5, 4, 3]).unwrap(),
            vec![4, 3, 2, 1, 7, 6, 5]
        );
        assert!(expansion.expand(&[1, 2, 3]).is_err());
        assert!(Expansion::read(&path, 2, 8).is_err());

        fs::remove_dir_all(dir).unwrap();
    }
}
