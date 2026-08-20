use std::error::Error;
use std::fmt;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use powercycle_config::Config;
use powercycle_core::{CycleDecomposition, Factor, PowerGraph};
use powercycle_format::{Dump, MoveRecord, Provenance};
use powercycle_patch::{find_alternating_repair, greedy_three_opt_21, greedy_three_opt_111};
use powercycle_search::preflight::{PreflightReport, hamiltonian_preflight};
use powercycle_search::{BoundaryCutBudget, Invocation, ProgramInfo, RunContext, SatFactorSolver};

const DEFAULT_MAX_TOURS: u64 = 20;
const ALTERNATING_REPAIR_MAX_DEPTH: usize = 4;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-search: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchOutcome {
    Completed,
    Interrupted,
}

fn install_interrupt_handler(interrupt_requested: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    let signal_count = Arc::new(AtomicUsize::new(0));

    let signal_count_handler = Arc::clone(&signal_count);

    ctrlc::set_handler(move || {
        let previous = signal_count_handler.fetch_add(1, Ordering::SeqCst);

        if previous == 0 {
            interrupt_requested.store(true, Ordering::SeqCst);

            eprintln!(
                "pc-search: Ctrl-C: graceful stop requested; \
                 press Ctrl-C again to force exit"
            );
        } else {
            eprintln!("pc-search: Ctrl-C: forcing immediate exit");

            std::process::exit(130);
        }
    })?;

    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let invocation = Invocation::capture()?;
    let problem = parse_problem(&invocation.argv)?;
    let config = Config::load()?;

    let program = ProgramInfo::new("pc-search", env!("CARGO_PKG_VERSION"));

    let interrupt_requested = Arc::new(AtomicBool::new(false));

    install_interrupt_handler(Arc::clone(&interrupt_requested))?;

    eprintln!("pc-search: creating run for G_{}({})", problem.k, problem.n);

    let mut run = RunContext::create(&config, problem.k, problem.n, program, invocation)?;

    eprintln!("pc-search: run {}", run.id());

    eprintln!("pc-search: directory {}", run.paths().root.display());

    let outcome = match run_search(&mut run, problem, Arc::clone(&interrupt_requested)) {
        Ok(outcome) => outcome,

        Err(error) => {
            let message = error.to_string();

            if let Err(mark_error) = run.mark_failed(&message) {
                eprintln!(
                    "pc-search: additionally failed to mark run as failed: \
                     {mark_error}"
                );
            }

            return Err(error);
        }
    };

    /*
     * RunContext currently exposes completed/failed lifecycle states only.
     * A graceful Ctrl-C is a clean search termination, not a failure, so the
     * run is finalized normally while the log records the interrupted outcome.
     */
    run.mark_completed()?;

    match outcome {
        SearchOutcome::Completed => {
            eprintln!("pc-search: completed");
        }

        SearchOutcome::Interrupted => {
            eprintln!("pc-search: interrupted");
        }
    }

    println!("{}", run.paths().root.display());

    Ok(())
}

fn run_search(
    run: &mut RunContext,
    problem: Problem,
    interrupt_requested: Arc<AtomicBool>,
) -> Result<SearchOutcome, Box<dyn Error>> {
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

    let preflight_start = Instant::now();

    let preflight = hamiltonian_preflight(&graph);

    let preflight_elapsed = preflight_start.elapsed();

    report_preflight(run, &preflight, preflight_elapsed)?;

    if !preflight.passed() {
        report(run, "SAT model skipped")?;

        return Ok(SearchOutcome::Completed);
    }

    if interrupt_requested.load(Ordering::SeqCst) {
        report(run, "graceful stop requested after Hamiltonian preflight")?;

        return Ok(SearchOutcome::Interrupted);
    }

    report(run, "building base SAT model")?;

    let model_start = Instant::now();

    let mut solver = SatFactorSolver::new(&graph)?;

    solver.attach_interrupt_flag(Arc::clone(&interrupt_requested));

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

    report(
        run,
        format!(
            "  max tours      : {}",
            if problem.max_tours == 0 {
                "unlimited".to_owned()
            } else {
                problem.max_tours.to_string()
            }
        ),
    )?;

    report(
        run,
        format!("  on Hamiltonian : {}", problem.hamiltonian_policy),
    )?;

    report(run, "  Ctrl-C         : first graceful, second forced")?;

    report(run, "patchability analysis")?;

    report(run, format!("  mode           : {}", problem.patch_mode))?;

    match problem.patch_mode {
        PatchMode::Full => {
            report(
                run,
                "  every SAT tour : greedy 1+1+1 / 2+1 plus alternating repair",
            )?;

            report(
                run,
                format!("  repair depth   : <= {}", ALTERNATING_REPAIR_MAX_DEPTH),
            )?;
        }

        PatchMode::ProbeOnly => {
            report(run, "  every SAT tour : greedy 1+1+1 / 2+1 closure only")?;

            report(run, "  repair         : disabled")?;
        }
    }

    report(run, format!("  dump policy    : {}", problem.dump_policy))?;

    match (problem.patch_mode, problem.dump_policy) {
        (PatchMode::Full, DumpPolicy::Always) => {
            report(run, "  dumps          : raw and patched factor every tour")?;
        }

        (PatchMode::Full, DumpPolicy::Records) => {
            report(
                run,
                "  dumps          : raw/patched records and Hamiltonian factors",
            )?;
        }

        (PatchMode::Full, DumpPolicy::Never) => {
            report(run, "  dumps          : disabled")?;
        }

        (PatchMode::ProbeOnly, DumpPolicy::Always) => {
            report(run, "  dumps          : raw SAT factor every tour")?;
        }

        (PatchMode::ProbeOnly, DumpPolicy::Records) => {
            report(
                run,
                "  dumps          : raw SAT records and Hamiltonian factors",
            )?;
        }

        (PatchMode::ProbeOnly, DumpPolicy::Never) => {
            report(run, "  dumps          : disabled")?;
        }
    }

    report(run, "  patch result   : dry-run only")?;

    report(run, "  SEC source     : raw SAT factor")?;

    report(run, "  records        : raw and patched tracked separately")?;

    let mut raw_records = SearchRecords::default();
    let mut patched_records = SearchRecords::default();

    let search_start = Instant::now();
    let mut outcome = SearchOutcome::Completed;

    for tour in 1u64.. {
        if interrupt_requested.load(Ordering::SeqCst) {
            report(run, "graceful stop requested before next SAT tour")?;

            outcome = SearchOutcome::Interrupted;
            break;
        }

        report(run, format!("solving tour {tour}"))?;

        let solve_start = Instant::now();

        let factor = match solver.solve(&graph) {
            Ok(factor) => factor,

            Err(_) if interrupt_requested.load(Ordering::SeqCst) => {
                let solve_elapsed = solve_start.elapsed();

                report(
                    run,
                    format!(
                        "SAT solve interrupted at tour {tour} after {:.3} s",
                        solve_elapsed.as_secs_f64()
                    ),
                )?;

                outcome = SearchOutcome::Interrupted;
                break;
            }

            Err(error) => {
                return Err(Box::new(error));
            }
        };

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

        let raw_update = update_records(run, "RAW", &mut raw_records, tour, &raw_decomposition)?;

        /*
         * Probe-only runs persist raw SAT factors only.
         *
         * The effective default is --dump=always, but an explicit
         * --dump=records or --dump=never can reduce or disable persistence.
         */
        if problem.patch_mode == PatchMode::ProbeOnly {
            let raw_hamiltonian = raw_decomposition.is_hamiltonian();

            let save_raw = match problem.dump_policy {
                DumpPolicy::Always => true,

                DumpPolicy::Records => raw_update.any() || raw_hamiltonian,

                DumpPolicy::Never => false,
            };

            if save_raw {
                let mut reasons = vec!["probe-only".to_owned(), "raw-sat-factor".to_owned()];

                match problem.dump_policy {
                    DumpPolicy::Always => {
                        reasons.push("dump-always".to_owned());
                    }

                    DumpPolicy::Records => {
                        raw_update.append_reasons("raw", &mut reasons);

                        if raw_hamiltonian {
                            reasons.push("hamiltonian".to_owned());
                        }
                    }

                    DumpPolicy::Never => {
                        unreachable!("never policy cannot request a raw dump");
                    }
                }

                save_raw_dump(run, &graph, &factor, &raw_decomposition, tour, reasons)?;
            }
        }

        let patch_start = Instant::now();

        report(run, format!("tour {tour} patch dry-run"))?;

        let patch = patch_to_local_closure(run, &graph, &factor, problem.patch_mode)?;

        let patch_elapsed = patch_start.elapsed();

        let patched_decomposition = CycleDecomposition::new(&graph, &patch.factor)?;

        report(run, format!("tour {tour} patched factor"))?;

        report(run, format!("  passes         : {}", patch.passes))?;

        report(
            run,
            format!("  1+1+1 moves    : {}", patch.one_one_one_moves),
        )?;

        report(run, format!("  2+1 moves      : {}", patch.two_one_moves))?;

        report(
            run,
            format!("  repairs        : {}", patch.alternating_repairs),
        )?;

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

        let patched_update = update_records(
            run,
            "PATCHED",
            &mut patched_records,
            tour,
            &patched_decomposition,
        )?;

        if problem.patch_mode == PatchMode::Full {
            let patched_hamiltonian = patched_decomposition.is_hamiltonian();

            let save_patched = match problem.dump_policy {
                DumpPolicy::Always => true,

                DumpPolicy::Records => patched_update.any() || patched_hamiltonian,

                DumpPolicy::Never => false,
            };

            let save_raw = match problem.dump_policy {
                DumpPolicy::Always => true,

                DumpPolicy::Records => raw_update.any() || save_patched,

                DumpPolicy::Never => false,
            };

            let mut raw_dump_id = None;

            if save_raw {
                let mut reasons = vec!["raw-sat-factor".to_owned()];

                match problem.dump_policy {
                    DumpPolicy::Always => {
                        reasons.push("dump-always".to_owned());
                    }

                    DumpPolicy::Records => {
                        raw_update.append_reasons("raw", &mut reasons);

                        if save_patched {
                            reasons.push("parent-of-patched-record".to_owned());
                        }

                        if patched_hamiltonian {
                            reasons.push("parent-of-hamiltonian".to_owned());
                        }
                    }

                    DumpPolicy::Never => {
                        unreachable!("never policy cannot request a raw dump");
                    }
                }

                raw_dump_id = Some(save_raw_dump(
                    run,
                    &graph,
                    &factor,
                    &raw_decomposition,
                    tour,
                    reasons,
                )?);
            }

            if save_patched {
                let mut reasons = Vec::new();

                match problem.dump_policy {
                    DumpPolicy::Always => {
                        reasons.push("dump-always".to_owned());
                    }

                    DumpPolicy::Records => {
                        patched_update.append_reasons("patched", &mut reasons);

                        if patched_hamiltonian {
                            reasons.push("hamiltonian".to_owned());
                        }
                    }

                    DumpPolicy::Never => {
                        unreachable!("never policy cannot request a patched dump");
                    }
                }

                let parent_dump_id = raw_dump_id.expect("patched dump must have a raw parent dump");

                save_patched_dump(
                    run,
                    &graph,
                    &factor,
                    &patch,
                    &patched_decomposition,
                    tour,
                    parent_dump_id,
                    reasons,
                )?;
            }
        }

        let patched_hamiltonian = patched_decomposition.is_hamiltonian();
        let raw_hamiltonian = raw_decomposition.is_hamiltonian();

        if patched_hamiltonian {
            report(run, format!("tour {tour} PATCHED FACTOR IS HAMILTONIAN"))?;

            if problem.patch_mode == PatchMode::ProbeOnly {
                report(
                    run,
                    "  note           : probe-only persists raw SAT factors only",
                )?;
            }
        }

        if raw_hamiltonian {
            report(run, format!("tour {tour} raw SAT factor is Hamiltonian"))?;
        }

        /*
         * If Ctrl-C arrived after CaDiCaL returned a model, finish the
         * current tour first: patching, record updates and requested dumps
         * above are all completed before stopping. SEC generation belongs to
         * the next SAT tour, so it is intentionally skipped here.
         */
        if interrupt_requested.load(Ordering::SeqCst) {
            report(
                run,
                format!("graceful stop requested after completing tour {tour}"),
            )?;

            outcome = SearchOutcome::Interrupted;
            break;
        }

        if (patched_hamiltonian || raw_hamiltonian)
            && problem.hamiltonian_policy == HamiltonianPolicy::Stop
        {
            report(run, "Hamiltonian policy: stop")?;

            break;
        }

        if problem.max_tours != 0 && tour >= problem.max_tours {
            break;
        }

        /*
         * A raw Hamiltonian factor has no satellite cycle, so there is no
         * boundary SEC to add. In continue mode we exclude exactly that raw
         * 2-factor and ask CaDiCaL for another model.
         *
         * Because every admissible model is a degree-2 factor with exactly n
         * selected edges, one clause containing the negation of every selected
         * edge excludes exactly the current factor.
         */
        if raw_hamiltonian {
            let block_start = Instant::now();

            let width = solver.add_factor_blocking_clause(&graph, &factor)?;

            report(run, format!("factor blocking round {tour}"))?;

            report(
                run,
                format!(
                    "  blocked RAW Hamiltonian with one clause of width {}",
                    width
                ),
            )?;

            report(
                run,
                format!(
                    "  build time     : {:.3} s",
                    block_start.elapsed().as_secs_f64()
                ),
            )?;

            continue;
        }

        /*
         * Important experimental invariant:
         *
         * SEC generation still uses the raw SAT factor.
         *
         * The patched factor is used only to measure patchability and
         * establish patched records. This preserves the current SAT/SEC
         * dynamics while we reconstruct the historical exploratory search.
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

        if interrupt_requested.load(Ordering::SeqCst) {
            report(
                run,
                format!("graceful stop requested after SEC round {tour}"),
            )?;

            outcome = SearchOutcome::Interrupted;
            break;
        }

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

    report(
        run,
        format!(
            "  outcome        : {}",
            match outcome {
                SearchOutcome::Completed => "completed",
                SearchOutcome::Interrupted => "interrupted",
            }
        ),
    )?;

    Ok(outcome)
}

fn save_raw_dump(
    run: &RunContext,
    graph: &PowerGraph,
    factor: &Factor,
    decomposition: &CycleDecomposition,
    tour: u64,
    reasons: Vec<String>,
) -> Result<String, Box<dyn Error>> {
    let mut provenance = Provenance::new("pc-search", env!("CARGO_PKG_VERSION"));

    provenance.session = Some(run.id().to_owned());

    provenance.tour = Some(tour);

    provenance.reason = reasons;

    let dump = Dump::new(graph, factor, factor, provenance, Vec::new())?;

    let dump_id = dump.dump_id.clone();
    let dump_id_prefix = &dump_id[..12];

    let filename = format!(
        "t{tour:06}-raw-c{:04}-g{}-r{}-{dump_id_prefix}.pcdump.json",
        decomposition.cycle_count(),
        decomposition.largest_cycle().len(),
        decomposition.residual_vertex_count(),
    );

    let path = run.paths().root.join("dumps").join(filename);

    dump.write(&path)?;

    report(run, format!("  raw dump       : {}", path.display()))?;

    Ok(dump_id)
}

fn save_patched_dump(
    run: &RunContext,
    graph: &PowerGraph,
    base_factor: &Factor,
    patch: &PatchClosureResult,
    decomposition: &CycleDecomposition,
    tour: u64,
    parent_dump_id: String,
    reasons: Vec<String>,
) -> Result<String, Box<dyn Error>> {
    let mut provenance = Provenance::new("pc-search", env!("CARGO_PKG_VERSION"));

    provenance.session = Some(run.id().to_owned());

    provenance.tour = Some(tour);

    provenance.reason = reasons;

    provenance.parent_dump_id = Some(parent_dump_id);

    let dump = Dump::new(
        graph,
        base_factor,
        &patch.factor,
        provenance,
        patch.history.clone(),
    )?;

    let dump_id = dump.dump_id.clone();
    let dump_id_prefix = &dump_id[..12];

    let filename = format!(
        "t{tour:06}-patched-c{:04}-g{}-r{}-{dump_id_prefix}.pcdump.json",
        decomposition.cycle_count(),
        decomposition.largest_cycle().len(),
        decomposition.residual_vertex_count(),
    );

    let path = run.paths().root.join("dumps").join(filename);

    dump.write(&path)?;

    report(run, format!("  patched dump   : {}", path.display()))?;

    Ok(dump_id)
}

fn patch_to_local_closure(
    run: &RunContext,
    graph: &PowerGraph,
    factor: &Factor,
    patch_mode: PatchMode,
) -> Result<PatchClosureResult, Box<dyn Error>> {
    let mut current = Factor::new(graph, factor.selected_edges().to_vec())?;

    let mut history = Vec::<MoveRecord>::new();

    let mut pass = 1usize;
    let mut total_111_moves = 0usize;
    let mut total_21_moves = 0usize;
    let mut alternating_repairs = 0usize;

    loop {
        report(run, format!("patch pass {pass}"))?;

        /*
         * Phase 1: exhaust all direct 1+1+1 moves.
         *
         * The greedy implementation may perform several atomic moves.
         * For persistence we record the whole valid factor-to-factor
         * transformation as one replayable batch.
         */
        let before_111 = Factor::new(graph, current.selected_edges().to_vec())?;

        let one_one_one_start = Instant::now();

        let one_one_one = greedy_three_opt_111(graph, &current)?;

        let one_one_one_elapsed = one_one_one_start.elapsed();

        let one_one_one_moves = one_one_one.move_count();

        total_111_moves += one_one_one_moves;

        current = one_one_one.into_factor();

        if one_one_one_moves > 0 {
            history.push(symmetric_difference_move(
                "three-opt-111-batch",
                &before_111,
                &current,
            ));
        }

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
            report(run, "  closure        : Hamiltonian after 1+1+1")?;

            return Ok(PatchClosureResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        /*
         * Phase 2: exhaust all direct 2+1 absorptions.
         *
         * As above, persistence records the complete greedy phase as one
         * factor-preserving batch.
         */
        let before_21 = Factor::new(graph, current.selected_edges().to_vec())?;

        let two_one_start = Instant::now();

        let two_one = greedy_three_opt_21(graph, &current)?;

        let two_one_elapsed = two_one_start.elapsed();

        let two_one_moves = two_one.move_count();

        total_21_moves += two_one_moves;

        current = two_one.into_factor();

        if two_one_moves > 0 {
            history.push(symmetric_difference_move(
                "three-opt-21-batch",
                &before_21,
                &current,
            ));
        }

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
            report(run, "  closure        : Hamiltonian after 2+1")?;

            return Ok(PatchClosureResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        /*
         * Probe-only mode deliberately stops before alternating repair.
         *
         * We still allow another direct pass if the current pass actually
         * changed the factor, because an exhaustive 2+1 phase may expose a
         * new 1+1+1 opportunity.
         */
        if patch_mode == PatchMode::ProbeOnly {
            if one_one_one_moves == 0 && two_one_moves == 0 {
                report(
                    run,
                    "  closure        : probe fixed point for 1+1+1 and 2+1",
                )?;

                return Ok(PatchClosureResult {
                    factor: current,
                    history,
                    passes: pass,
                    one_one_one_moves: total_111_moves,
                    two_one_moves: total_21_moves,
                    alternating_repairs,
                });
            }

            pass += 1;
            continue;
        }

        /*
         * Full mode:
         *
         * Both direct operators are now exhausted. Try one bounded
         * alternating repair. If one satellite can be absorbed, restart
         * from 1+1+1 because the repair changes the giant topology and may
         * create new direct moves.
         */
        report(
            run,
            format!(
                "  repair         : searching depth <= {}",
                ALTERNATING_REPAIR_MAX_DEPTH
            ),
        )?;

        let repair_start = Instant::now();

        let repair = find_alternating_repair(graph, &current, ALTERNATING_REPAIR_MAX_DEPTH)?;

        let repair_search_elapsed = repair_start.elapsed();

        let Some(repair) = repair else {
            report(
                run,
                format!(
                    "  repair         : none found, {:.3} s",
                    repair_search_elapsed.as_secs_f64()
                ),
            )?;

            report(
                run,
                "  closure        : fixed point for 1+1+1, 2+1 and repair",
            )?;

            return Ok(PatchClosureResult {
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
        let removed_count = repair.removed().len();
        let added_count = repair.added().len();

        let before_repair = Factor::new(graph, current.selected_edges().to_vec())?;

        current = repair.apply(graph, &current)?;

        history.push(symmetric_difference_move(
            format!("alternating-repair-depth-{depth}"),
            &before_repair,
            &current,
        ));

        alternating_repairs += 1;

        let after_repair = CycleDecomposition::new(graph, &current)?;

        report(
            run,
            format!(
                "  repair         : FOUND depth {}, absorbed {}, \
                 toggle {}-{}, {} cycles, giant {}, residual {}, {:.3} s",
                depth,
                absorbed,
                removed_count,
                added_count,
                after_repair.cycle_count(),
                after_repair.largest_cycle().len(),
                after_repair.residual_vertex_count(),
                repair_search_elapsed.as_secs_f64()
            ),
        )?;

        if after_repair.is_hamiltonian() {
            report(
                run,
                "  closure        : Hamiltonian after alternating repair",
            )?;

            return Ok(PatchClosureResult {
                factor: current,
                history,
                passes: pass,
                one_one_one_moves: total_111_moves,
                two_one_moves: total_21_moves,
                alternating_repairs,
            });
        }

        /*
         * A repair succeeded. Restart the cheap/direct operators before
         * attempting another repair.
         */
        pass += 1;
    }
}

fn symmetric_difference_move(
    kind: impl Into<String>,
    before: &Factor,
    after: &Factor,
) -> MoveRecord {
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

fn report_preflight(
    run: &RunContext,
    preflight: &PreflightReport,
    elapsed: Duration,
) -> Result<(), Box<dyn Error>> {
    report(run, "Hamiltonian preflight")?;

    report(
        run,
        format!("  min degree       : {}", preflight.min_degree),
    )?;

    report(
        run,
        format!(
            "  initial deg-2    : {}",
            preflight.initial_degree_two_vertices
        ),
    )?;

    report(
        run,
        format!(
            "  connected        : {}",
            match preflight.connected {
                Some(true) => "true",
                Some(false) => "false",
                None => "not checked",
            }
        ),
    )?;

    report(
        run,
        format!(
            "  articulation     : {}",
            if preflight.articulation_checked {
                match preflight.articulation_vertex {
                    Some(vertex) => {
                        format!("vertex {vertex}")
                    }

                    None => "none".to_owned(),
                }
            } else {
                "not checked".to_owned()
            }
        ),
    )?;

    if preflight.propagation_checked {
        report(
            run,
            format!("  forced edges     : {}", preflight.forced_edges),
        )?;

        report(
            run,
            format!("  forbidden edges  : {}", preflight.forbidden_edges),
        )?;

        report(
            run,
            format!(
                "  propagation      : {} edge updates",
                preflight.propagation_updates
            ),
        )?;

        report(
            run,
            format!("  forced spanning  : {}", preflight.forced_spanning_cycle),
        )?;
    } else {
        report(run, "  propagation      : not run")?;
    }

    report(
        run,
        format!(
            "  result           : {}",
            if preflight.passed() {
                "PASS"
            } else {
                "IMPOSSIBLE"
            }
        ),
    )?;

    if let Some(obstruction) = &preflight.obstruction {
        report(run, format!("  obstruction      : {obstruction}"))?;
    }

    report(
        run,
        format!("  preflight time   : {:.3} s", elapsed.as_secs_f64()),
    )?;

    Ok(())
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
) -> Result<RecordUpdate, Box<dyn Error>> {
    let cycles = decomposition.cycle_count();
    let giant = decomposition.largest_cycle().len();
    let residual = decomposition.residual_vertex_count();

    let candidate = FactorRecord {
        tour,
        cycles,
        giant,
        residual,
    };

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

    Ok(RecordUpdate {
        cycle_record,
        giant_record,
    })
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
    history: Vec<MoveRecord>,
    passes: usize,
    one_one_one_moves: usize,
    two_one_moves: usize,
    alternating_repairs: usize,
}

impl PatchClosureResult {
    #[must_use]
    const fn total_moves(&self) -> usize {
        self.one_one_one_moves + self.two_one_moves + self.alternating_repairs
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatchMode {
    Full,
    ProbeOnly,
}

impl fmt::Display for PatchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "full"),

            Self::ProbeOnly => write!(f, "probe-only"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HamiltonianPolicy {
    Stop,
    Continue,
}

impl HamiltonianPolicy {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "stop" => Ok(Self::Stop),

            "continue" => Ok(Self::Continue),

            _ => Err(CliError::InvalidHamiltonianPolicy {
                value: value.to_owned(),
            }),
        }
    }
}

impl fmt::Display for HamiltonianPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stop => write!(f, "stop"),

            Self::Continue => write!(f, "continue"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DumpPolicy {
    Always,
    Records,
    Never,
}

impl DumpPolicy {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "always" => Ok(Self::Always),

            "records" => Ok(Self::Records),

            "never" => Ok(Self::Never),

            _ => Err(CliError::InvalidDumpPolicy {
                value: value.to_owned(),
            }),
        }
    }
}

impl fmt::Display for DumpPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always => write!(f, "always"),

            Self::Records => write!(f, "records"),

            Self::Never => write!(f, "never"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Problem {
    k: u32,
    n: usize,
    max_tours: u64,
    patch_mode: PatchMode,
    dump_policy: DumpPolicy,
    hamiltonian_policy: HamiltonianPolicy,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RecordUpdate {
    cycle_record: bool,
    giant_record: bool,
}

impl RecordUpdate {
    #[must_use]
    const fn any(self) -> bool {
        self.cycle_record || self.giant_record
    }

    fn append_reasons(self, prefix: &str, reasons: &mut Vec<String>) {
        if self.cycle_record {
            reasons.push(format!("{prefix}-cycle-record"));
        }

        if self.giant_record {
            reasons.push(format!("{prefix}-giant-record"));
        }
    }
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
    let mut positional = Vec::<&str>::new();

    let mut patch_mode = PatchMode::Full;
    let mut probe_only_seen = false;
    let mut dump_policy = None::<DumpPolicy>;
    let mut hamiltonian_policy = None::<HamiltonianPolicy>;

    for argument in arguments {
        match argument.as_str() {
            "--probe-only" => {
                if probe_only_seen {
                    return Err(CliError::DuplicateOption {
                        option: "--probe-only",
                    });
                }

                probe_only_seen = true;
                patch_mode = PatchMode::ProbeOnly;
            }

            value if value.starts_with("--dump=") => {
                if dump_policy.is_some() {
                    return Err(CliError::DuplicateOption { option: "--dump" });
                }

                let value = value
                    .strip_prefix("--dump=")
                    .expect("matched --dump= prefix must strip");

                dump_policy = Some(DumpPolicy::parse(value)?);
            }

            value if value.starts_with("--on-hamiltonian=") => {
                if hamiltonian_policy.is_some() {
                    return Err(CliError::DuplicateOption {
                        option: "--on-hamiltonian",
                    });
                }

                let value = value
                    .strip_prefix("--on-hamiltonian=")
                    .expect("matched --on-hamiltonian= prefix must strip");

                hamiltonian_policy = Some(HamiltonianPolicy::parse(value)?);
            }

            value if value.starts_with('-') => {
                return Err(CliError::UnknownOption {
                    value: value.to_owned(),
                });
            }

            value => {
                positional.push(value);
            }
        }
    }

    if positional.len() < 2 || positional.len() > 3 {
        return Err(CliError::Usage);
    }

    let k = positional[0]
        .parse::<u32>()
        .map_err(|_| CliError::InvalidExponent {
            value: positional[0].to_owned(),
        })?;

    let n = positional[1]
        .parse::<usize>()
        .map_err(|_| CliError::InvalidVertexCount {
            value: positional[1].to_owned(),
        })?;

    let max_tours = if let Some(value) = positional.get(2) {
        value
            .parse::<u64>()
            .map_err(|_| CliError::InvalidTourCount {
                value: (*value).to_owned(),
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

    let dump_policy = dump_policy.unwrap_or(match patch_mode {
        PatchMode::Full => DumpPolicy::Records,
        PatchMode::ProbeOnly => DumpPolicy::Always,
    });

    let hamiltonian_policy = hamiltonian_policy.unwrap_or(HamiltonianPolicy::Stop);

    Ok(Problem {
        k,
        n,
        max_tours,
        patch_mode,
        dump_policy,
        hamiltonian_policy,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CliError {
    Usage,

    UnknownOption { value: String },

    DuplicateOption { option: &'static str },

    InvalidExponent { value: String },

    InvalidVertexCount { value: String },

    InvalidTourCount { value: String },

    InvalidDumpPolicy { value: String },

    InvalidHamiltonianPolicy { value: String },

    ExponentTooSmall { k: u32 },

    VertexCountZero,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(
                    f,
                    "usage: pc-search [--probe-only] [--dump=always|records|never] [--on-hamiltonian=stop|continue] <k> <n> [max-tours]"
                )
            }

            Self::UnknownOption { value } => {
                write!(
                    f,
                    "unknown option {value:?}; \
                     usage: pc-search [--probe-only] [--dump=always|records|never] [--on-hamiltonian=stop|continue] <k> <n> [max-tours]"
                )
            }

            Self::DuplicateOption { option } => {
                write!(f, "option {option:?} was specified more than once")
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
                     expected an integer >= 0 (0 means unlimited)"
                )
            }

            Self::InvalidDumpPolicy { value } => {
                write!(
                    f,
                    "invalid dump policy {value:?}: \
                     expected always, records or never"
                )
            }

            Self::InvalidHamiltonianPolicy { value } => {
                write!(
                    f,
                    "invalid Hamiltonian policy {value:?}: \
                     expected stop or continue"
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
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Records,
                hamiltonian_policy: HamiltonianPolicy::Stop,
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
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Records,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_probe_only_before_problem() {
        assert_eq!(
            parse_problem(&args(&["--probe-only", "5", "490463", "100",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 100,
                patch_mode: PatchMode::ProbeOnly,
                dump_policy: DumpPolicy::Always,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_probe_only_after_problem() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "100", "--probe-only",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 100,
                patch_mode: PatchMode::ProbeOnly,
                dump_policy: DumpPolicy::Always,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_explicit_dump_policy_in_full_mode() {
        assert_eq!(
            parse_problem(&args(&["--dump=always", "5", "490463", "100",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 100,
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Always,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_explicit_dump_policy_in_probe_mode() {
        assert_eq!(
            parse_problem(&args(&["--probe-only", "--dump=records", "5", "490463",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: DEFAULT_MAX_TOURS,
                patch_mode: PatchMode::ProbeOnly,
                dump_policy: DumpPolicy::Records,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_dump_never() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "--dump=never",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: DEFAULT_MAX_TOURS,
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Never,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }

    #[test]
    fn parses_continue_on_hamiltonian() {
        assert_eq!(
            parse_problem(&args(&["--on-hamiltonian=continue", "5", "490463", "0",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 0,
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Records,
                hamiltonian_policy: HamiltonianPolicy::Continue,
            })
        );
    }

    #[test]
    fn rejects_duplicate_hamiltonian_policy() {
        assert_eq!(
            parse_problem(&args(&[
                "--on-hamiltonian=stop",
                "--on-hamiltonian=continue",
                "5",
                "490463",
            ],)),
            Err(CliError::DuplicateOption {
                option: "--on-hamiltonian",
            })
        );
    }

    #[test]
    fn rejects_invalid_hamiltonian_policy() {
        assert_eq!(
            parse_problem(&args(&["--on-hamiltonian=forever", "5", "490463",],)),
            Err(CliError::InvalidHamiltonianPolicy {
                value: "forever".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_duplicate_dump_policy() {
        assert_eq!(
            parse_problem(&args(&["--dump=always", "--dump=records", "5", "490463",],)),
            Err(CliError::DuplicateOption { option: "--dump" })
        );
    }

    #[test]
    fn rejects_invalid_dump_policy() {
        assert_eq!(
            parse_problem(&args(&["--dump=sometimes", "5", "490463",],)),
            Err(CliError::InvalidDumpPolicy {
                value: "sometimes".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_duplicate_probe_only() {
        assert_eq!(
            parse_problem(&args(&["--probe-only", "5", "490463", "--probe-only",],)),
            Err(CliError::DuplicateOption {
                option: "--probe-only",
            })
        );
    }

    #[test]
    fn rejects_unknown_option() {
        assert_eq!(
            parse_problem(&args(&["--fast", "5", "490463",],)),
            Err(CliError::UnknownOption {
                value: "--fast".to_owned(),
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
    fn parses_unlimited_tour_count_and_rejects_non_numeric_count() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "many",],)),
            Err(CliError::InvalidTourCount {
                value: "many".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["5", "490463", "0",],)),
            Ok(Problem {
                k: 5,
                n: 490_463,
                max_tours: 0,
                patch_mode: PatchMode::Full,
                dump_policy: DumpPolicy::Records,
                hamiltonian_policy: HamiltonianPolicy::Stop,
            })
        );
    }
}
