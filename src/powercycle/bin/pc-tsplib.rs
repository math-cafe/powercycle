//! Bridge to LKH-3: export `G_k(n)` as a TSPLIB Hamiltonian cycle problem, and
//! read the tour LKH finds back as a canonical witness.
//!
//! The export runs the preflight first. Edges its degree-two propagation
//! forces are the contracted chains and go to `FIXED_EDGES_SECTION`; edges it
//! forbids are left out of `EDGE_DATA_SECTION`, which LKH takes as the graph.
//! See section 7 of `doc/en/performance.md`.

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
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

        Command::Export { k, n, output } => export(k, n, &output),

        Command::Tour { k, n, input } => tour(k, n, &input),
    }
}

fn export(k: u32, n: usize, output: &Path) -> Result<(), Box<dyn Error>> {
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

    let fixed = states
        .iter()
        .filter(|&&state| state == EdgeState::Forced)
        .count();
    let removed = states
        .iter()
        .filter(|&&state| state == EdgeState::Forbidden)
        .count();

    let mut file = BufWriter::with_capacity(1 << 20, File::create(output)?);

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

    for (edge, &state) in graph.edges().iter().zip(states) {
        if state != EdgeState::Forbidden {
            writeln!(file, "{} {}", edge.first().number(), edge.second().number())?;
        }
    }

    writeln!(file, "-1")?;
    writeln!(file, "FIXED_EDGES_SECTION")?;

    for (edge, &state) in graph.edges().iter().zip(states) {
        if state == EdgeState::Forced {
            writeln!(file, "{} {}", edge.first().number(), edge.second().number())?;
        }
    }

    writeln!(file, "-1")?;
    writeln!(file, "EOF")?;
    file.flush()?;

    let parameters = output.with_extension("par");
    fs::write(&parameters, parameter_file(output)?)?;

    eprintln!(
        "pc-tsplib: G_{k}({n}): {} edges, {fixed} fixed, {removed} removed",
        graph.edge_count()
    );
    eprintln!(
        "pc-tsplib: wrote {} and {} in {:.1} s",
        output.display(),
        parameters.display(),
        start.elapsed().as_secs_f64()
    );

    Ok(())
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

fn tour(k: u32, n: usize, input: &Path) -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(input)
        .map_err(|error| format!("cannot read {}: {error}", input.display()))?;

    let cycle = parse_tour(&text)?;

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
    match arguments {
        [flag] if flag == "-h" || flag == "--help" => Ok(Command::Help),

        [command, k, n, path] => {
            let k = k
                .parse::<u32>()
                .ok()
                .filter(|&k| k >= 2)
                .ok_or_else(|| CliError::InvalidExponent { value: k.clone() })?;

            let n = n
                .parse::<usize>()
                .ok()
                .filter(|&n| n >= 3)
                .ok_or_else(|| CliError::InvalidVertexCount { value: n.clone() })?;

            let path = PathBuf::from(path);

            match command.as_str() {
                "export" => Ok(Command::Export { k, n, output: path }),
                "tour" => Ok(Command::Tour { k, n, input: path }),
                _ => Err(CliError::Usage),
            }
        }

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
           pc-tsplib export <k> <n> <out.hcp>\n\
           pc-tsplib tour <k> <n> <file.tour> > cycle.txt\n\
         \n\
         export  runs the preflight, then writes G_k(n) as a TSPLIB HCP problem\n\
                 with the forced edges fixed and the forbidden ones removed,\n\
                 and an LKH parameter file <out>.par next to it\n\
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
    Export { k: u32, n: usize, output: PathBuf },
    Tour { k: u32, n: usize, input: PathBuf },
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
                "usage: pc-tsplib export <k> <n> <out.hcp> | pc-tsplib tour <k> <n> <file.tour>"
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
            })
        );

        assert_eq!(
            parse_arguments(&strings(&["tour", "2", "32", "g2.tour"])),
            Ok(Command::Tour {
                k: 2,
                n: 32,
                input: PathBuf::from("g2.tour"),
            })
        );

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
}
