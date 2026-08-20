use std::error::Error;
use std::fmt;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use powercycle_config::Config;

use powercycle_core::{CycleDecomposition, Factor, PowerGraph};

use powercycle_patch::{greedy_three_opt_21, greedy_three_opt_111};

use powercycle_search::{BoundaryCutBudget, Invocation, ProgramInfo, RunContext, SatFactorSolver};

const DEFAULT_MAX_TOURS: u64 = 20;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-search: {error}");

            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let invocation = Invocation::capture()?;

    let problem = parse_problem(&invocation.argv)?;

    let config = Config::load()?;

    let program = ProgramInfo::new("pc-search", env!("CARGO_PKG_VERSION"));

    eprintln!("pc-search: creating run for G_{}({})", problem.k, problem.n);

    let mut run = RunContext::create(&config, problem.k, problem.n, program, invocation)?;

    eprintln!("pc-search: run {}", run.id());

    eprintln!("pc-search: directory {}", run.paths().root.display());

    if let Err(error) = run_search(&mut run, problem) {
        let message = error.to_string();

        if let Err(mark_error) = run.mark_failed(&message) {
            eprintln!(
                "pc-search: additionally failed to mark run as failed: \
                 {mark_error}"
            );
        }

        return Err(error);
    }

    run.mark_completed()?;

    eprintln!("pc-search: completed");

    println!("{}", run.paths().root.display());

    Ok(())
}

fn run_search(run: &mut RunContext, problem: Problem) -> Result<(), Box<dyn Error>> {
    report(run, format!("building G_{}({})", problem.k, problem.n))?;

    let graph_start = Instant::now();

    let graph = PowerGraph::new(problem.k, problem.n)?;

    let graph_elapsed = graph_start.elapsed();

    report(run, "graph")?;

    report(run, format!("  vertices       : {}", graph.vertex_count()))?;

    report(run, format!("  edges          : {}", graph.edge_count()))?;

    report(run, format!("  min degree     : {}", graph.min_degree()))?;

    report(run, format!("  max degree     : {}", graph.max_degree()))?;

    report(
        run,
        format!("  average degree : {:.3}", graph.average_degree()),
    )?;

    report(run, format!("  connected      : {}", graph.is_connected()))?;

    report(
        run,
        format!("  build time     : {:.3} s", graph_elapsed.as_secs_f64()),
    )?;

    report(run, "building base SAT model")?;

    let model_start = Instant::now();

    let mut solver = SatFactorSolver::new(&graph)?;

    let model_elapsed = model_start.elapsed();

    let stats = solver.stats();

    report(run, "SAT base model")?;

    report(
        run,
        format!("  solver         : {}", solver.solver_signature()),
    )?;

    report(run, format!("  variables      : {}", stats.variables))?;

    report(
        run,
        format!("  >=2 clauses    : {}", stats.at_least_two_clauses),
    )?;

    report(
        run,
        format!("  <=2 clauses    : {}", stats.at_most_two_clauses),
    )?;

    report(run, format!("  total clauses  : {}", stats.clause_count()))?;

    report(
        run,
        format!("  degree < 2     : {}", stats.vertices_below_degree_two),
    )?;

    report(
        run,
        format!("  build time     : {:.3} s", model_elapsed.as_secs_f64()),
    )?;

    let budget = BoundaryCutBudget::default();

    report(run, "SEC policy")?;

    report(
        run,
        format!("  priority <=    : {}", budget.priority_max_width),
    )?;

    report(run, format!("  max clauses    : {}", budget.max_clauses))?;

    report(run, format!("  max literals   : {}", budget.max_literals))?;

    report(run, format!("  max tours      : {}", problem.max_tours))?;

    report(run, "patchability analysis")?;

    report(
        run,
        "  every SAT tour : alternating greedy 1+1+1 / 2+1 closure",
    )?;

    report(run, "  patch result   : dry-run only")?;

    report(run, "  SEC source     : raw SAT factor")?;

    report(run, "  records        : raw and patched tracked separately")?;

    let mut raw_records = SearchRecords::default();

    let mut patched_records = SearchRecords::default();

    let search_start = Instant::now();

    for tour in 1..=problem.max_tours {
        report(run, format!("solving tour {tour}"))?;

        let solve_start = Instant::now();

        let factor = solver.solve(&graph)?;

        let solve_elapsed = solve_start.elapsed();

        let Some(factor) = factor else {
            report(
                run,
                format!(
                    "SAT model is UNSAT at tour {tour} after {:.3} s",
                    solve_elapsed.as_secs_f64()
                ),
            )?;

            break;
        };

        let raw_decomposition = CycleDecomposition::new(&graph, &factor)?;

        run.set_tour(tour)?;

        report_factor(
            run,
            tour,
            &graph,
            &factor,
            &raw_decomposition,
            solve_elapsed,
        )?;

        update_records(run, "RAW", &mut raw_records, tour, &raw_decomposition)?;

        let patch_start = Instant::now();

        report(run, format!("tour {tour} patch dry-run"))?;

        let patch = patch_to_local_closure(run, &graph, &factor)?;

        let patch_elapsed = patch_start.elapsed();

        let patched_decomposition = CycleDecomposition::new(&graph, &patch.factor)?;

        report(run, format!("tour {tour} patched factor"))?;

        report(run, format!("  passes         : {}", patch.passes))?;

        report(
            run,
            format!("  1+1+1 moves    : {}", patch.one_one_one_moves),
        )?;

        report(run, format!("  2+1 moves      : {}", patch.two_one_moves))?;

        report(run, format!("  total moves    : {}", patch.total_moves()))?;

        report(
            run,
            format!(
                "  cycle reduction: {}",
                raw_decomposition
                    .cycle_count()
                    .saturating_sub(patched_decomposition.cycle_count(),)
            ),
        )?;

        report_decomposition(run, &patched_decomposition)?;

        report(
            run,
            format!("  patch time     : {:.3} s", patch_elapsed.as_secs_f64()),
        )?;

        update_records(
            run,
            "PATCHED",
            &mut patched_records,
            tour,
            &patched_decomposition,
        )?;

        if patched_decomposition.is_hamiltonian() {
            report(run, format!("tour {tour} PATCHED FACTOR IS HAMILTONIAN"))?;

            report(
                run,
                "  note           : dry-run factor is not yet persisted",
            )?;
        }

        if raw_decomposition.is_hamiltonian() {
            report(run, format!("tour {tour} raw SAT factor is Hamiltonian"))?;

            break;
        }

        if tour == problem.max_tours {
            break;
        }

        /*
         * Important experimental invariant:
         *
         * SEC generation still uses the raw SAT factor.
         *
         * The patched factor is used only to measure patchability and
         * establish patched records. This keeps the SAT/SEC dynamics
         * unchanged while we compare the reconstructed search with the
         * historical exploratory implementation.
         */
        report(
            run,
            format!("adding boundary SEC round {tour} from raw SAT factor"),
        )?;

        let cut_start = Instant::now();

        let cut_stats =
            solver.add_budgeted_boundary_subtour_cuts(&graph, &raw_decomposition, budget)?;

        let cut_elapsed = cut_start.elapsed();

        report(run, format!("boundary SEC round {tour}"))?;

        report(
            run,
            format!(
                "  candidates     : {} clauses / {} literals",
                cut_stats.candidate_clauses, cut_stats.candidate_literals
            ),
        )?;

        report(
            run,
            format!(
                "  priority       : {} clauses / {} literals",
                cut_stats.priority_candidate_clauses, cut_stats.priority_candidate_literals
            ),
        )?;

        report(
            run,
            format!(
                "  added          : {} clauses / {} literals",
                cut_stats.added_clauses, cut_stats.added_literals
            ),
        )?;

        report(
            run,
            format!(
                "  added priority : {} clauses / {} literals",
                cut_stats.priority_added_clauses, cut_stats.priority_added_literals
            ),
        )?;

        report(
            run,
            format!(
                "  skipped        : {} clauses / {} literals",
                cut_stats.skipped_clauses, cut_stats.skipped_literals
            ),
        )?;

        report(
            run,
            format!("  min width      : {}", cut_stats.min_added_width),
        )?;

        report(
            run,
            format!("  max width      : {}", cut_stats.max_added_width),
        )?;

        report(
            run,
            format!("  average width  : {:.2}", cut_stats.average_added_width()),
        )?;

        report(
            run,
            format!("  build time     : {:.3} s", cut_elapsed.as_secs_f64()),
        )?;

        if cut_stats.added_clauses == 0 {
            report(run, "no boundary SEC fitted the round budget")?;

            break;
        }
    }

    report(run, "search summary")?;

    report(
        run,
        format!(
            "  elapsed        : {:.3} s",
            search_start.elapsed().as_secs_f64()
        ),
    )?;

    report_record_summary(run, "RAW", &raw_records)?;

    report_record_summary(run, "PATCHED", &patched_records)?;

    Ok(())
}

fn patch_to_local_closure(
    run: &RunContext,
    graph: &PowerGraph,
    factor: &Factor,
) -> Result<PatchClosureResult, Box<dyn Error>> {
    let mut current = Factor::new(graph, factor.selected_edges().to_vec())?;

    let mut pass = 1usize;

    let mut total_111_moves = 0usize;

    let mut total_21_moves = 0usize;

    loop {
        report(run, format!("patch pass {pass}"))?;

        let one_one_one_start = Instant::now();

        let one_one_one = greedy_three_opt_111(graph, &current)?;

        let one_one_one_elapsed = one_one_one_start.elapsed();

        let one_one_one_moves = one_one_one.move_count();

        total_111_moves += one_one_one_moves;

        current = one_one_one.into_factor();

        let after_111 = CycleDecomposition::new(graph, &current)?;

        report(
            run,
            format!(
                "  1+1+1          : {} moves, {} cycles, \
                 giant {}, residual {}, {:.3} s",
                one_one_one_moves,
                after_111.cycle_count(),
                after_111.largest_cycle().len(),
                after_111.residual_vertex_count(),
                one_one_one_elapsed.as_secs_f64()
            ),
        )?;

        if after_111.is_hamiltonian() {
            return Ok(PatchClosureResult {
                factor: current,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
            });
        }

        /*
         * The previous 2+1 phase, if there was one, was exhaustive.
         * If the next 1+1+1 phase cannot move, both operators are now
         * simultaneously blocked.
         */
        if pass > 1 && one_one_one_moves == 0 {
            report(run, "  closure        : fixed point after 1+1+1")?;

            return Ok(PatchClosureResult {
                factor: current,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
            });
        }

        let two_one_start = Instant::now();

        let two_one = greedy_three_opt_21(graph, &current)?;

        let two_one_elapsed = two_one_start.elapsed();

        let two_one_moves = two_one.move_count();

        total_21_moves += two_one_moves;

        current = two_one.into_factor();

        let after_21 = CycleDecomposition::new(graph, &current)?;

        report(
            run,
            format!(
                "  2+1            : {} moves, {} cycles, \
                 giant {}, residual {}, {:.3} s",
                two_one_moves,
                after_21.cycle_count(),
                after_21.largest_cycle().len(),
                after_21.residual_vertex_count(),
                two_one_elapsed.as_secs_f64()
            ),
        )?;

        if after_21.is_hamiltonian() {
            return Ok(PatchClosureResult {
                factor: current,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
            });
        }

        if two_one_moves == 0 {
            report(run, "  closure        : fixed point after 2+1")?;

            return Ok(PatchClosureResult {
                factor: current,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
            });
        }

        pass += 1;
    }
}

fn report_factor(
    run: &RunContext,
    tour: u64,
    graph: &PowerGraph,
    factor: &Factor,
    decomposition: &CycleDecomposition,
    solve_elapsed: Duration,
) -> Result<(), Box<dyn Error>> {
    factor.validate_two_factor(graph)?;

    report(run, format!("tour {tour} raw SAT factor"))?;

    report(run, format!("  selected edges : {}", factor.edge_count()))?;

    report_decomposition(run, decomposition)?;

    report(
        run,
        format!("  SAT solve time : {:.3} s", solve_elapsed.as_secs_f64()),
    )?;

    Ok(())
}

fn report_decomposition(
    run: &RunContext,
    decomposition: &CycleDecomposition,
) -> Result<(), Box<dyn Error>> {
    let (satellite_count, min_satellite, max_satellite) = satellite_stats(decomposition);

    report(
        run,
        format!("  cycles         : {}", decomposition.cycle_count()),
    )?;

    report(
        run,
        format!("  giant          : {}", decomposition.largest_cycle().len()),
    )?;

    report(run, format!("  satellites     : {}", satellite_count))?;

    report(
        run,
        format!(
            "  residual       : {}",
            decomposition.residual_vertex_count()
        ),
    )?;

    if let (Some(min_satellite), Some(max_satellite)) = (min_satellite, max_satellite) {
        report(
            run,
            format!("  satellite size : {} .. {}", min_satellite, max_satellite),
        )?;
    }

    report(
        run,
        format!("  hamiltonian    : {}", decomposition.is_hamiltonian()),
    )?;

    Ok(())
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

fn update_records(
    run: &RunContext,
    label: &str,
    records: &mut SearchRecords,
    tour: u64,
    decomposition: &CycleDecomposition,
) -> Result<(), Box<dyn Error>> {
    let cycles = decomposition.cycle_count();

    let giant = decomposition.largest_cycle().len();

    let residual = decomposition.residual_vertex_count();

    let candidate = FactorRecord {
        tour,
        cycles,
        giant,
        residual,
    };

    /*
     * Primary record:
     *
     *   1. fewer cycles;
     *   2. then fewer residual vertices.
     *
     * This is intentionally close to the patchability criterion used during
     * the exploratory phase: a factor that closes to fewer cycles is more
     * interesting even when its raw SAT decomposition looks worse.
     */
    let cycle_record = match records.best_cycles {
        None => true,

        Some(current) => {
            cycles < current.cycles || (cycles == current.cycles && residual < current.residual)
        }
    };

    if cycle_record {
        records.best_cycles = Some(candidate);

        report(
            run,
            format!(
                "  NEW {label} CYCLE RECORD: {} cycles, \
                 giant {}, residual {}",
                cycles, giant, residual
            ),
        )?;
    }

    let giant_record = match records.best_giant {
        None => true,

        Some(current) => {
            giant > current.giant || (giant == current.giant && cycles < current.cycles)
        }
    };

    if giant_record {
        records.best_giant = Some(candidate);

        report(
            run,
            format!(
                "  NEW {label} GIANT RECORD: {}, \
                 {} cycles, residual {}",
                giant, cycles, residual
            ),
        )?;
    }

    Ok(())
}

fn report_record_summary(
    run: &RunContext,
    label: &str,
    records: &SearchRecords,
) -> Result<(), Box<dyn Error>> {
    report(run, format!("{label} records"))?;

    if let Some(record) = records.best_cycles {
        report(
            run,
            format!(
                "  best cycles    : {} at tour {} \
                 (giant {}, residual {})",
                record.cycles, record.tour, record.giant, record.residual
            ),
        )?;
    } else {
        report(run, "  best cycles    : none")?;
    }

    if let Some(record) = records.best_giant {
        report(
            run,
            format!(
                "  best giant     : {} at tour {} \
                 ({} cycles, residual {})",
                record.giant, record.tour, record.cycles, record.residual
            ),
        )?;
    } else {
        report(run, "  best giant     : none")?;
    }

    Ok(())
}

fn report(run: &RunContext, message: impl AsRef<str>) -> Result<(), Box<dyn Error>> {
    let message = message.as_ref();

    eprintln!("pc-search: {message}");

    run.log_info(message)?;

    Ok(())
}

#[derive(Debug)]
struct PatchClosureResult {
    factor: Factor,
    passes: usize,
    one_one_one_moves: usize,
    two_one_moves: usize,
}

impl PatchClosureResult {
    #[must_use]
    const fn total_moves(&self) -> usize {
        self.one_one_one_moves + self.two_one_moves
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Problem {
    k: u32,
    n: usize,
    max_tours: u64,
}

#[derive(Clone, Copy, Debug)]
struct FactorRecord {
    tour: u64,
    cycles: usize,
    giant: usize,
    residual: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct SearchRecords {
    best_cycles: Option<FactorRecord>,
    best_giant: Option<FactorRecord>,
}

fn parse_problem(arguments: &[String]) -> Result<Problem, CliError> {
    if arguments.len() < 2 || arguments.len() > 3 {
        return Err(CliError::Usage);
    }

    let k = arguments[0]
        .parse::<u32>()
        .map_err(|_| CliError::InvalidExponent {
            value: arguments[0].clone(),
        })?;

    let n = arguments[1]
        .parse::<usize>()
        .map_err(|_| CliError::InvalidVertexCount {
            value: arguments[1].clone(),
        })?;

    let max_tours = if let Some(value) = arguments.get(2) {
        value
            .parse::<u64>()
            .map_err(|_| CliError::InvalidTourCount {
                value: value.clone(),
            })?
    } else {
        DEFAULT_MAX_TOURS
    };

    if k < 2 {
        return Err(CliError::ExponentTooSmall { k });
    }

    if n == 0 {
        return Err(CliError::VertexCountZero);
    }

    if max_tours == 0 {
        return Err(CliError::TourCountZero);
    }

    Ok(Problem { k, n, max_tours })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CliError {
    Usage,

    InvalidExponent { value: String },

    InvalidVertexCount { value: String },

    InvalidTourCount { value: String },

    ExponentTooSmall { k: u32 },

    VertexCountZero,

    TourCountZero,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(f, "usage: pc-search <k> <n> [max-tours]")
            }

            Self::InvalidExponent { value } => {
                write!(
                    f,
                    "invalid exponent {value:?}: \
                     expected an integer k >= 2"
                )
            }

            Self::InvalidVertexCount { value } => {
                write!(
                    f,
                    "invalid vertex count {value:?}: \
                     expected an integer n >= 1"
                )
            }

            Self::InvalidTourCount { value } => {
                write!(
                    f,
                    "invalid tour count {value:?}: \
                     expected an integer >= 1"
                )
            }

            Self::ExponentTooSmall { k } => {
                write!(
                    f,
                    "invalid exponent k={k}: \
                     expected k >= 2"
                )
            }

            Self::VertexCountZero => {
                write!(
                    f,
                    "invalid vertex count n=0: \
                     expected n >= 1"
                )
            }

            Self::TourCountZero => {
                write!(
                    f,
                    "invalid tour count 0: \
                     expected at least one tour"
                )
            }
        }
    }
}

impl Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_valid_problem_with_default_tours() {
        assert_eq!(
            parse_problem(&args(&["5", "490463",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: DEFAULT_MAX_TOURS,
            })
        );
    }

    #[test]
    fn parses_explicit_tour_count() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "100",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 100,
            })
        );
    }

    #[test]
    fn rejects_missing_arguments() {
        assert_eq!(parse_problem(&args(&["5"],)), Err(CliError::Usage));
    }

    #[test]
    fn rejects_extra_arguments() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "20", "extra",],)),
            Err(CliError::Usage)
        );
    }

    #[test]
    fn rejects_invalid_exponent() {
        assert_eq!(
            parse_problem(&args(&["five", "490463",],)),
            Err(CliError::InvalidExponent {
                value: "five".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["1", "490463",],)),
            Err(CliError::ExponentTooSmall { k: 1 })
        );
    }

    #[test]
    fn rejects_invalid_vertex_count() {
        assert_eq!(
            parse_problem(&args(&["5", "many",],)),
            Err(CliError::InvalidVertexCount {
                value: "many".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["5", "0",],)),
            Err(CliError::VertexCountZero)
        );
    }

    #[test]
    fn rejects_invalid_tour_count() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "many",],)),
            Err(CliError::InvalidTourCount {
                value: "many".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["5", "490463", "0",],)),
            Err(CliError::TourCountZero)
        );
    }
}
