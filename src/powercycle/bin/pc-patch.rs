use std::env;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use powercycle_core::{CycleDecomposition, Factor, PowerGraph};
use powercycle_format::{Dump, MoveRecord, Provenance};
use powercycle_patch::{find_alternating_repair, greedy_three_opt_21, greedy_three_opt_111};

const DEFAULT_MAX_DEPTH: usize = 3;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-patch: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let options = parse_arguments(&arguments)?;

    eprintln!("pc-patch: reading {}", options.input.display());

    let input_dump = Dump::read(&options.input)?;

    let graph = input_dump.rebuild_graph()?;

    let base_factor = input_dump.build_current_factor(&graph)?;

    let initial = CycleDecomposition::new(&graph, &base_factor)?;

    eprintln!("pc-patch: input dump {}", input_dump.dump_id);

    eprintln!("pc-patch: graph G_{}({})", graph.k(), graph.vertex_count());

    report_decomposition("input", &initial);

    eprintln!(
        "pc-patch: alternating repair depth <= {}",
        options.max_depth
    );

    let start = Instant::now();

    let patch = patch_to_local_closure(&graph, &base_factor, options.max_depth)?;

    let elapsed = start.elapsed();

    let final_decomposition = CycleDecomposition::new(&graph, &patch.factor)?;

    eprintln!("pc-patch: result");
    eprintln!("pc-patch:   passes         : {}", patch.passes);
    eprintln!("pc-patch:   1+1+1 moves    : {}", patch.one_one_one_moves);
    eprintln!("pc-patch:   2+1 moves      : {}", patch.two_one_moves);
    eprintln!("pc-patch:   repairs        : {}", patch.alternating_repairs);
    eprintln!("pc-patch:   history records: {}", patch.history.len());
    eprintln!(
        "pc-patch:   cycle reduction: {}",
        initial
            .cycle_count()
            .saturating_sub(final_decomposition.cycle_count(),)
    );

    report_decomposition("patched", &final_decomposition);

    eprintln!(
        "pc-patch:   patch time     : {:.3} s",
        elapsed.as_secs_f64()
    );

    let mut provenance = Provenance::new("pc-patch", env!("CARGO_PKG_VERSION"));

    provenance.session = input_dump.provenance.session.clone();
    provenance.tour = input_dump.provenance.tour;
    provenance.reason = vec![
        "patched".to_owned(),
        format!("alternating-repair-depth-{}", options.max_depth),
    ];
    provenance.parent_dump_id = Some(input_dump.dump_id.clone());

    let output_dump = Dump::new(
        &graph,
        &base_factor,
        &patch.factor,
        provenance,
        patch.history,
    )?;

    let output_path = output_path(&options.input, &output_dump);

    output_dump.write(&output_path)?;

    /*
     * Re-read the file we just wrote. This deliberately verifies the
     * on-disk serialization, graph identity, factor validity, history replay
     * and semantic SHA-256 before exposing the path to another tool.
     */
    let written = Dump::read(&output_path)?;

    if written.dump_id != output_dump.dump_id {
        return Err(PatchCliError::WrittenDumpIdMismatch {
            expected: output_dump.dump_id,
            found: written.dump_id,
        }
        .into());
    }

    eprintln!("pc-patch: wrote {}", output_path.display());

    println!("{}", output_path.display());

    Ok(())
}

fn patch_to_local_closure(
    graph: &PowerGraph,
    factor: &Factor,
    max_depth: usize,
) -> Result<PatchResult, Box<dyn Error>> {
    let mut current = Factor::new(graph, factor.selected_edges().to_vec())?;

    let mut history = Vec::<MoveRecord>::new();

    let mut pass = 1usize;
    let mut total_111_moves = 0usize;
    let mut total_21_moves = 0usize;
    let mut alternating_repairs = 0usize;

    loop {
        eprintln!("pc-patch: pass {pass}");

        /*
         * Exhaust all direct 1+1+1 moves.
         */
        let phase_start = Instant::now();

        let before_111 = clone_factor(graph, &current)?;

        let one_one_one = greedy_three_opt_111(graph, &current, &|| false)?;

        let one_one_one_moves = one_one_one.move_count();

        total_111_moves += one_one_one_moves;

        current = one_one_one.into_factor();

        if one_one_one_moves > 0 {
            history.push(difference_record(
                "greedy-three-opt-111",
                &before_111,
                &current,
            ));
        }

        let after_111 = CycleDecomposition::new(graph, &current)?;

        eprintln!(
            "pc-patch:   1+1+1 : {} moves, {} cycles, giant {}, \
             residual {}, {:.3} s",
            one_one_one_moves,
            after_111.cycle_count(),
            after_111.largest_cycle().len(),
            after_111.residual_vertex_count(),
            phase_start.elapsed().as_secs_f64()
        );

        if after_111.is_hamiltonian() {
            eprintln!("pc-patch:   closure: Hamiltonian after 1+1+1");

            return Ok(PatchResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        /*
         * Exhaust all direct 2+1 absorptions.
         */
        let phase_start = Instant::now();

        let before_21 = clone_factor(graph, &current)?;

        let two_one = greedy_three_opt_21(graph, &current, &|| false)?;

        let two_one_moves = two_one.move_count();

        total_21_moves += two_one_moves;

        current = two_one.into_factor();

        if two_one_moves > 0 {
            history.push(difference_record(
                "greedy-three-opt-21",
                &before_21,
                &current,
            ));
        }

        let after_21 = CycleDecomposition::new(graph, &current)?;

        eprintln!(
            "pc-patch:   2+1   : {} moves, {} cycles, giant {}, \
             residual {}, {:.3} s",
            two_one_moves,
            after_21.cycle_count(),
            after_21.largest_cycle().len(),
            after_21.residual_vertex_count(),
            phase_start.elapsed().as_secs_f64()
        );

        if after_21.is_hamiltonian() {
            eprintln!("pc-patch:   closure: Hamiltonian after 2+1");

            return Ok(PatchResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        /*
         * Direct operators are exhausted. Try exactly one bounded
         * alternating repair, then restart the cheap/direct operators.
         */
        let repair_start = Instant::now();

        eprintln!("pc-patch:   repair: searching depth <= {}", max_depth);

        let repair = find_alternating_repair(graph, &current, max_depth, &|| false)?;

        let Some(repair) = repair else {
            eprintln!(
                "pc-patch:   repair: none found, {:.3} s",
                repair_start.elapsed().as_secs_f64()
            );

            eprintln!("pc-patch:   closure: local fixed point");

            return Ok(PatchResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        };

        let depth = repair.depth();
        let absorbed = repair.absorbed_vertices();

        let movement = MoveRecord::new(
            "alternating-repair",
            repair.removed().iter().copied(),
            repair.added().iter().copied(),
        );

        current = repair.apply(graph, &current)?;

        history.push(movement);

        alternating_repairs += 1;

        let after_repair = CycleDecomposition::new(graph, &current)?;

        eprintln!(
            "pc-patch:   repair: FOUND depth {}, absorbed {}, \
             {} cycles, giant {}, residual {}, {:.3} s",
            depth,
            absorbed,
            after_repair.cycle_count(),
            after_repair.largest_cycle().len(),
            after_repair.residual_vertex_count(),
            repair_start.elapsed().as_secs_f64()
        );

        if after_repair.is_hamiltonian() {
            eprintln!("pc-patch:   closure: Hamiltonian after alternating repair");

            return Ok(PatchResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        pass += 1;
    }
}

fn difference_record(kind: &str, before: &Factor, after: &Factor) -> MoveRecord {
    let removed = before
        .selected_edges()
        .iter()
        .copied()
        .filter(|&edge_id| !after.contains(edge_id));

    let added = after
        .selected_edges()
        .iter()
        .copied()
        .filter(|&edge_id| !before.contains(edge_id));

    MoveRecord::new(kind, removed, added)
}

fn clone_factor(graph: &PowerGraph, factor: &Factor) -> Result<Factor, Box<dyn Error>> {
    Ok(Factor::new(graph, factor.selected_edges().to_vec())?)
}

fn output_path(input: &Path, dump: &Dump) -> PathBuf {
    let directory = input.parent().unwrap_or_else(|| Path::new("."));

    let prefix = &dump.dump_id[..12];

    let tour_prefix = match dump.provenance.tour {
        Some(tour) => {
            format!("t{tour:06}-")
        }

        None => String::new(),
    };

    let filename = format!(
        "{tour_prefix}patched-c{:04}-g{}-r{}-{prefix}.pcdump.json",
        dump.summary.cycles, dump.summary.giant_size, dump.summary.residual_vertices,
    );

    directory.join(filename)
}

fn report_decomposition(label: &str, decomposition: &CycleDecomposition) {
    let (satellite_count, min_satellite, max_satellite) = satellite_stats(decomposition);

    eprintln!("pc-patch: {label}");

    eprintln!(
        "pc-patch:   cycles         : {}",
        decomposition.cycle_count()
    );

    eprintln!(
        "pc-patch:   giant          : {}",
        decomposition.largest_cycle().len()
    );

    eprintln!("pc-patch:   satellites     : {}", satellite_count);

    eprintln!(
        "pc-patch:   residual       : {}",
        decomposition.residual_vertex_count()
    );

    if let (Some(minimum), Some(maximum)) = (min_satellite, max_satellite) {
        eprintln!("pc-patch:   satellite size : {} .. {}", minimum, maximum);
    }

    eprintln!(
        "pc-patch:   hamiltonian    : {}",
        decomposition.is_hamiltonian()
    );
}

fn satellite_stats(decomposition: &CycleDecomposition) -> (usize, Option<usize>, Option<usize>) {
    if decomposition.cycle_count() <= 1 {
        return (0, None, None);
    }

    let largest_index = decomposition.largest_cycle_index();

    let mut count = 0usize;
    let mut minimum = usize::MAX;
    let mut maximum = 0usize;

    for (cycle_index, cycle) in decomposition.cycles().iter().enumerate() {
        if cycle_index == largest_index {
            continue;
        }

        let size = cycle.len();

        count += 1;
        minimum = minimum.min(size);
        maximum = maximum.max(size);
    }

    (
        count,
        (count > 0).then_some(minimum),
        (count > 0).then_some(maximum),
    )
}

#[derive(Debug)]
struct PatchResult {
    factor: Factor,
    history: Vec<MoveRecord>,
    passes: usize,
    one_one_one_moves: usize,
    two_one_moves: usize,
    alternating_repairs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Options {
    input: PathBuf,
    max_depth: usize,
}

fn parse_arguments(arguments: &[String]) -> Result<Options, PatchCliError> {
    let mut input = None::<PathBuf>;
    let mut max_depth = DEFAULT_MAX_DEPTH;
    let mut max_depth_seen = false;

    let mut index = 0usize;

    while index < arguments.len() {
        let argument = &arguments[index];

        if let Some(value) = argument.strip_prefix("--max-depth=") {
            if max_depth_seen {
                return Err(PatchCliError::DuplicateOption {
                    option: "--max-depth",
                });
            }

            max_depth = parse_depth(value)?;

            max_depth_seen = true;
            index += 1;
            continue;
        }

        if argument == "--max-depth" {
            if max_depth_seen {
                return Err(PatchCliError::DuplicateOption {
                    option: "--max-depth",
                });
            }

            let Some(value) = arguments.get(index + 1) else {
                return Err(PatchCliError::MissingOptionValue {
                    option: "--max-depth",
                });
            };

            max_depth = parse_depth(value)?;

            max_depth_seen = true;
            index += 2;
            continue;
        }

        if argument.starts_with('-') {
            return Err(PatchCliError::UnknownOption {
                value: argument.clone(),
            });
        }

        if input.is_some() {
            return Err(PatchCliError::Usage);
        }

        input = Some(PathBuf::from(argument));

        index += 1;
    }

    let Some(input) = input else {
        return Err(PatchCliError::Usage);
    };

    Ok(Options { input, max_depth })
}

fn parse_depth(value: &str) -> Result<usize, PatchCliError> {
    let depth = value
        .parse::<usize>()
        .map_err(|_| PatchCliError::InvalidDepth {
            value: value.to_owned(),
        })?;

    if depth == 0 {
        return Err(PatchCliError::DepthZero);
    }

    Ok(depth)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PatchCliError {
    Usage,

    UnknownOption { value: String },

    DuplicateOption { option: &'static str },

    MissingOptionValue { option: &'static str },

    InvalidDepth { value: String },

    DepthZero,

    WrittenDumpIdMismatch { expected: String, found: String },
}

impl fmt::Display for PatchCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(f, "usage: pc-patch [--max-depth=N] <dump.pcdump.json>")
            }

            Self::UnknownOption { value } => {
                write!(
                    f,
                    "unknown option {value:?}; \
                     usage: pc-patch [--max-depth=N] <dump.pcdump.json>"
                )
            }

            Self::DuplicateOption { option } => {
                write!(f, "option {option:?} was specified more than once")
            }

            Self::MissingOptionValue { option } => {
                write!(f, "option {option:?} requires a value")
            }

            Self::InvalidDepth { value } => {
                write!(
                    f,
                    "invalid alternating repair depth {value:?}: \
                     expected an integer >= 1"
                )
            }

            Self::DepthZero => {
                write!(f, "alternating repair depth must be at least one")
            }

            Self::WrittenDumpIdMismatch { expected, found } => {
                write!(
                    f,
                    "written dump ID mismatch: expected {expected}, \
                     found {found}"
                )
            }
        }
    }
}

impl Error for PatchCliError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_input_with_default_depth() {
        assert_eq!(
            parse_arguments(&args(&["candidate.pcdump.json"],)),
            Ok(Options {
                input: PathBuf::from("candidate.pcdump.json"),
                max_depth: 3,
            })
        );
    }

    #[test]
    fn parses_equals_depth_option() {
        assert_eq!(
            parse_arguments(&args(&["--max-depth=5", "candidate.pcdump.json",],)),
            Ok(Options {
                input: PathBuf::from("candidate.pcdump.json"),
                max_depth: 5,
            })
        );
    }

    #[test]
    fn parses_separate_depth_option() {
        assert_eq!(
            parse_arguments(&args(&["candidate.pcdump.json", "--max-depth", "4",],)),
            Ok(Options {
                input: PathBuf::from("candidate.pcdump.json"),
                max_depth: 4,
            })
        );
    }

    #[test]
    fn rejects_zero_depth() {
        assert_eq!(
            parse_arguments(&args(&["--max-depth=0", "candidate.pcdump.json",],)),
            Err(PatchCliError::DepthZero)
        );
    }

    #[test]
    fn rejects_duplicate_depth() {
        assert_eq!(
            parse_arguments(&args(&[
                "--max-depth=3",
                "--max-depth=4",
                "candidate.pcdump.json",
            ],)),
            Err(PatchCliError::DuplicateOption {
                option: "--max-depth",
            })
        );
    }

    #[test]
    fn rejects_missing_input() {
        assert_eq!(
            parse_arguments(&args(&["--max-depth=3",],)),
            Err(PatchCliError::Usage)
        );
    }

    #[test]
    fn rejects_multiple_inputs() {
        assert_eq!(
            parse_arguments(&args(&["first.pcdump.json", "second.pcdump.json",],)),
            Err(PatchCliError::Usage)
        );
    }

    #[test]
    fn rejects_unknown_option() {
        assert_eq!(
            parse_arguments(&args(&["--fast", "candidate.pcdump.json",],)),
            Err(PatchCliError::UnknownOption {
                value: "--fast".to_owned(),
            })
        );
    }
}
