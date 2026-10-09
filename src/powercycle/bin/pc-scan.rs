use std::env;
use std::error::Error;
use std::fmt;
use std::process::ExitCode;
use std::time::Instant;

use powercycle_core::PowerGraph;
use powercycle_search::preflight::{
    PreflightObstruction, hamiltonian_preflight, quick_degree_obstruction,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-scan: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();

    match parse_arguments(&arguments)? {
        Command::Help => {
            print_help();
            return Ok(());
        }

        Command::Version => {
            println!("pc-scan {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }

        Command::Scan(options) => {
            scan(options)?;
        }
    }

    Ok(())
}

fn scan(options: ScanOptions) -> Result<(), Box<dyn Error>> {
    let start_time = Instant::now();

    eprintln!(
        "pc-scan: scanning G_{}(n) for n={}..{}",
        options.k, options.start, options.end,
    );

    if options.compact {
        eprintln!("pc-scan: output mode compact");
    } else {
        eprintln!("pc-scan: output mode detailed");
    }

    let mut stats = ScanStats::default();
    let mut compact_group = None::<CompactGroup>;

    for n in options.start..=options.end {
        let classification = classify(options.k, n, options.max_graph_n)?;

        stats.scanned += 1;

        if classification.graph_built {
            stats.graphs_built += 1;
        }

        match classification.status {
            ScanStatus::Candidate => {
                stats.candidates += 1;
            }

            ScanStatus::Unchecked => {
                stats.unchecked += 1;
            }

            ScanStatus::Impossible => {
                stats.impossible += 1;

                match classification.source {
                    Some(RejectionSource::QuickDegree) => {
                        stats.quick_degree_rejections += 1;
                    }

                    Some(RejectionSource::FullPreflight) => {
                        stats.full_preflight_rejections += 1;
                    }

                    None => {
                        unreachable!("impossible classification must have a rejection source");
                    }
                }
            }
        }

        if options.compact {
            match compact_group {
                Some(mut group) if group.status == classification.status => {
                    group.end = n;
                    compact_group = Some(group);
                }

                Some(group) => {
                    print_compact_group(group);

                    compact_group = Some(CompactGroup {
                        start: n,
                        end: n,
                        status: classification.status,
                    });
                }

                None => {
                    compact_group = Some(CompactGroup {
                        start: n,
                        end: n,
                        status: classification.status,
                    });
                }
            }
        } else {
            print_detailed(n, &classification);
        }
    }

    if let Some(group) = compact_group {
        print_compact_group(group);
    }

    eprintln!("pc-scan: summary");
    eprintln!("pc-scan:   scanned               : {}", stats.scanned);
    eprintln!("pc-scan:   impossible            : {}", stats.impossible);
    eprintln!(
        "pc-scan:   quick degree rejects  : {}",
        stats.quick_degree_rejections
    );
    eprintln!(
        "pc-scan:   full preflight rejects: {}",
        stats.full_preflight_rejections
    );
    eprintln!("pc-scan:   candidates            : {}", stats.candidates);

    if stats.unchecked > 0 {
        eprintln!("pc-scan:   unchecked             : {}", stats.unchecked);
    }

    eprintln!("pc-scan:   graphs built          : {}", stats.graphs_built);
    eprintln!(
        "pc-scan:   elapsed               : {:.3} s",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

/// Announce a graph construction expected to need at least this much memory.
const LARGE_GRAPH_WARNING_BYTES: u128 = 2_000_000_000;

/// Rough memory cost of `PowerGraph::new(k, n)`, or `None` on overflow.
///
/// Counts the edge list and the CSR incidence list at 16 bytes per edge each,
/// plus one offset per vertex.
fn estimated_graph_bytes(k: u32, n: usize) -> Option<u128> {
    let n = n as u128;
    let mut edges = 0u128;
    let mut base = 2u128;

    loop {
        let power = base.checked_pow(k)?;

        if power > 2 * n {
            break;
        }

        let low = power.saturating_sub(n).max(1);
        let high = ((power - 1) / 2).min(n);

        if low <= high {
            edges += high - low + 1;
        }

        base += 1;
    }

    Some(32 * edges + 8 * n)
}

fn classify(k: u32, n: usize, max_graph_n: usize) -> Result<Classification, Box<dyn Error>> {
    if let Some(obstruction) = quick_degree_obstruction(k, n)? {
        return Ok(Classification {
            status: ScanStatus::Impossible,
            reason: Some(format!(
                "degree<2 vertex={} degree={}",
                obstruction.vertex, obstruction.degree,
            )),
            source: Some(RejectionSource::QuickDegree),
            graph_built: false,
        });
    }

    if n > max_graph_n {
        return Ok(Classification {
            status: ScanStatus::Unchecked,
            reason: Some(if max_graph_n == 0 {
                "graph skipped by --quick-only".to_owned()
            } else {
                format!("graph skipped, n exceeds max-graph-n={max_graph_n}")
            }),
            source: None,
            graph_built: false,
        });
    }

    /*
     * The graph stores each edge twice plus a CSR incidence entry, so it is
     * roughly 32 bytes per edge. At k = 7 the first candidate needs about
     * 36 GB, which is worth announcing before attempting it.
     */
    if let Some(bytes) = estimated_graph_bytes(k, n)
        && bytes >= LARGE_GRAPH_WARNING_BYTES
    {
        eprintln!(
            "pc-scan: warning: building G_{k}({n}) needs about {:.1} GB; \
             use --max-graph-n or --quick-only to skip it",
            bytes as f64 / 1e9
        );
    }

    let graph = PowerGraph::new(k, n)?;

    let report = hamiltonian_preflight(&graph);

    if report.passed() {
        return Ok(Classification {
            status: ScanStatus::Candidate,
            reason: None,
            source: None,
            graph_built: true,
        });
    }

    let obstruction = report
        .obstruction
        .as_ref()
        .expect("failed preflight must contain an obstruction");

    Ok(Classification {
        status: ScanStatus::Impossible,
        reason: Some(concise_obstruction(obstruction)),
        source: Some(RejectionSource::FullPreflight),
        graph_built: true,
    })
}

fn concise_obstruction(obstruction: &PreflightObstruction) -> String {
    match obstruction {
        PreflightObstruction::MinimumDegree { vertex, degree } => {
            format!("degree<2 vertex={vertex} degree={degree}")
        }

        PreflightObstruction::Disconnected => "disconnected".to_owned(),

        PreflightObstruction::ArticulationVertex { vertex } => {
            format!("articulation vertex={vertex}")
        }

        PreflightObstruction::TooManyForcedEdges { vertex, forced } => {
            format!("forced>2 vertex={vertex} forced={forced}")
        }

        PreflightObstruction::TooFewAvailableEdges { vertex, available } => {
            format!("available<2 vertex={vertex} available={available}")
        }

        PreflightObstruction::ForcedSubcycle { start_vertex, size } => {
            format!("forced-subcycle start={start_vertex} size={size}")
        }
    }
}

fn print_detailed(n: usize, classification: &Classification) {
    match &classification.reason {
        Some(reason) => {
            println!("{n}\t{}\t{reason}", classification.status,);
        }

        None => {
            println!("{n}\t{}", classification.status,);
        }
    }
}

fn print_compact_group(group: CompactGroup) {
    if group.start == group.end {
        println!("{}\t{}", group.start, group.status,);
    } else {
        println!("{}..{}\t{}", group.start, group.end, group.status,);
    }
}

fn parse_arguments(arguments: &[String]) -> Result<Command, ScanCliError> {
    if arguments.len() == 1 {
        match arguments[0].as_str() {
            "-h" | "--help" => {
                return Ok(Command::Help);
            }

            "-V" | "--version" => {
                return Ok(Command::Version);
            }

            _ => {}
        }
    }

    let mut compact = false;
    let mut max_graph_n = usize::MAX;
    let mut max_graph_n_seen = false;
    let mut positional = Vec::<&str>::new();
    let mut options_finished = false;

    for argument in arguments {
        if !options_finished {
            match argument.as_str() {
                "--compact" => {
                    if compact {
                        return Err(ScanCliError::DuplicateOption {
                            option: "--compact",
                        });
                    }

                    compact = true;
                    continue;
                }

                "--quick-only" => {
                    if max_graph_n_seen {
                        return Err(ScanCliError::DuplicateOption {
                            option: "--quick-only",
                        });
                    }

                    max_graph_n_seen = true;
                    max_graph_n = 0;
                    continue;
                }

                "--" => {
                    options_finished = true;
                    continue;
                }

                value if value.starts_with("--max-graph-n=") => {
                    if max_graph_n_seen {
                        return Err(ScanCliError::DuplicateOption {
                            option: "--max-graph-n",
                        });
                    }

                    let value = value
                        .strip_prefix("--max-graph-n=")
                        .expect("matched --max-graph-n= prefix must strip");

                    max_graph_n_seen = true;

                    max_graph_n =
                        value
                            .parse::<usize>()
                            .map_err(|_| ScanCliError::InvalidVertexCount {
                                name: "max-graph-n",
                                value: value.to_owned(),
                            })?;

                    continue;
                }

                _ if argument.starts_with('-') => {
                    return Err(ScanCliError::UnknownOption {
                        value: argument.clone(),
                    });
                }

                _ => {}
            }
        }

        positional.push(argument.as_str());
    }

    if positional.len() != 2 && positional.len() != 3 {
        return Err(ScanCliError::Usage);
    }

    let k = parse_k(positional[0])?;

    let start = parse_n(positional[1], "n/start")?;

    let end = if positional.len() == 3 {
        parse_n(positional[2], "end")?
    } else {
        start
    };

    if end < start {
        return Err(ScanCliError::DescendingInterval { start, end });
    }

    Ok(Command::Scan(ScanOptions {
        k,
        start,
        end,
        compact,
        max_graph_n,
    }))
}

fn parse_k(value: &str) -> Result<u32, ScanCliError> {
    let k = value
        .parse::<u32>()
        .map_err(|_| ScanCliError::InvalidExponent {
            value: value.to_owned(),
        })?;

    if k < 2 {
        return Err(ScanCliError::ExponentTooSmall { k });
    }

    Ok(k)
}

fn parse_n(value: &str, name: &'static str) -> Result<usize, ScanCliError> {
    let n = value
        .parse::<usize>()
        .map_err(|_| ScanCliError::InvalidVertexCount {
            name,
            value: value.to_owned(),
        })?;

    if n == 0 {
        return Err(ScanCliError::ZeroVertexCount { name });
    }

    Ok(n)
}

fn print_help() {
    println!(
        "pc-scan {version}\n\
         \n\
         Scan necessary Hamiltonian conditions for power-sum graphs.\n\
         \n\
         Usage:\n\
           pc-scan [OPTIONS] <k> <n>\n\
           pc-scan [OPTIONS] <k> <start> <end>\n\
         \n\
         Output, one tab-separated record per n on stdout:\n\
           IMPOSSIBLE  a necessary Hamiltonian condition failed\n\
           CANDIDATE   no implemented necessary obstruction was found\n\
           UNCHECKED   the graph-free check passed but the full preflight\n\
                       was skipped, so nothing was really decided\n\
         \n\
         Options:\n\
           --compact            group consecutive equal statuses into ranges\n\
           --max-graph-n=N      never build a graph beyond n = N\n\
           --quick-only         never build a graph at all\n\
           -h, --help           show this help\n\
           -V, --version        show version\n\
         \n\
         The graph-free degree check always runs. Only the values it fails to\n\
         reject pay for a graph construction and the full preflight, which is\n\
         what --max-graph-n and --quick-only bound.\n\
         \n\
         CANDIDATE does not prove that a Hamiltonian cycle exists.",
        version = env!("CARGO_PKG_VERSION"),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanStatus {
    Impossible,
    Candidate,
    Unchecked,
}

impl fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Impossible => write!(f, "IMPOSSIBLE"),

            Self::Candidate => write!(f, "CANDIDATE"),

            Self::Unchecked => write!(f, "UNCHECKED"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RejectionSource {
    QuickDegree,
    FullPreflight,
}

#[derive(Debug)]
struct Classification {
    status: ScanStatus,
    reason: Option<String>,
    source: Option<RejectionSource>,
    graph_built: bool,
}

#[derive(Clone, Copy, Debug)]
struct CompactGroup {
    start: usize,
    end: usize,
    status: ScanStatus,
}

#[derive(Clone, Copy, Debug, Default)]
struct ScanStats {
    scanned: usize,
    impossible: usize,
    quick_degree_rejections: usize,
    full_preflight_rejections: usize,
    candidates: usize,
    unchecked: usize,
    graphs_built: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScanOptions {
    k: u32,
    start: usize,
    end: usize,
    compact: bool,

    /// Largest `n` for which a graph may be constructed.
    ///
    /// The graph-free degree check always runs. Beyond this bound the full
    /// preflight is skipped and the verdict is `UNCHECKED` rather than
    /// `CANDIDATE`, because nothing was actually checked.
    max_graph_n: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Scan(ScanOptions),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScanCliError {
    Usage,

    UnknownOption { value: String },

    DuplicateOption { option: &'static str },

    InvalidExponent { value: String },

    ExponentTooSmall { k: u32 },

    InvalidVertexCount { name: &'static str, value: String },

    ZeroVertexCount { name: &'static str },

    DescendingInterval { start: usize, end: usize },
}

impl fmt::Display for ScanCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(
                    f,
                    "usage: pc-scan [--compact] <k> <n> | pc-scan [--compact] <k> <start> <end>"
                )
            }

            Self::UnknownOption { value } => {
                write!(f, "unknown option {value:?}")
            }

            Self::DuplicateOption { option } => {
                write!(f, "option {option} specified more than once")
            }

            Self::InvalidExponent { value } => {
                write!(f, "invalid exponent {value:?}: expected an integer >= 2")
            }

            Self::ExponentTooSmall { k } => {
                write!(f, "invalid exponent k={k}: expected k >= 2")
            }

            Self::InvalidVertexCount { name, value } => {
                write!(f, "invalid {name} {value:?}: expected a positive integer")
            }

            Self::ZeroVertexCount { name } => {
                write!(f, "invalid {name}=0: expected a positive integer")
            }

            Self::DescendingInterval { start, end } => {
                write!(f, "invalid interval {start}..{end}: end must be >= start")
            }
        }
    }
}

impl Error for ScanCliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_n() {
        let arguments = strings(&["5", "246877"]);

        assert_eq!(
            parse_arguments(&arguments,).unwrap(),
            Command::Scan(ScanOptions {
                k: 5,
                start: 246_877,
                end: 246_877,
                compact: false,
                max_graph_n: usize::MAX,
            })
        );
    }

    #[test]
    fn parses_the_graph_construction_guards() {
        assert_eq!(
            parse_arguments(&strings(&["--quick-only", "7", "1", "10"])).unwrap(),
            Command::Scan(ScanOptions {
                k: 7,
                start: 1,
                end: 10,
                compact: false,
                max_graph_n: 0,
            })
        );

        assert_eq!(
            parse_arguments(&strings(&["--max-graph-n=1000", "5", "1", "10"])).unwrap(),
            Command::Scan(ScanOptions {
                k: 5,
                start: 1,
                end: 10,
                compact: false,
                max_graph_n: 1_000,
            })
        );

        assert!(parse_arguments(&strings(&["--max-graph-n=x", "5", "1", "10"])).is_err());

        assert!(
            parse_arguments(&strings(&["--quick-only", "--quick-only", "5", "1", "10"])).is_err()
        );
    }

    #[test]
    fn parses_interval_and_compact_mode() {
        let arguments = strings(&["--compact", "5", "246876", "246878"]);

        assert_eq!(
            parse_arguments(&arguments,).unwrap(),
            Command::Scan(ScanOptions {
                k: 5,
                start: 246_876,
                end: 246_878,
                compact: true,
                max_graph_n: usize::MAX,
            })
        );
    }

    #[test]
    fn rejects_descending_interval() {
        let arguments = strings(&["5", "20", "10"]);

        assert!(matches!(
            parse_arguments(&arguments,),
            Err(ScanCliError::DescendingInterval { start: 20, end: 10 })
        ));
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }
}
