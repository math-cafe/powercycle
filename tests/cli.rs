//! Contract tests for the command-line tools.
//!
//! The published lower-bound certificates are byte-for-byte `pc-scan --bands`
//! output, so its stdout format is part of the repository's promises, not an
//! implementation detail. These tests pin it.

use std::process::{Command, Output};

fn run(bin: &str, arguments: &[&str]) -> Output {
    Command::new(bin)
        .args(arguments)
        .output()
        .expect("binary under test must be runnable")
}

fn stdout_of(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8")
}

const PC_SCAN: &str = env!("CARGO_BIN_EXE_pc-scan");
const PC_EXPORT: &str = env!("CARGO_BIN_EXE_pc-export");
const PC_TSPLIB: &str = env!("CARGO_BIN_EXE_pc-tsplib");

#[test]
fn scan_emits_one_tab_separated_record_per_n() {
    let output = run(PC_SCAN, &["5", "30", "34"]);

    assert!(output.status.success());

    let lines: Vec<&str> = stdout_of(&output).lines().collect();

    assert_eq!(lines.len(), 5, "one line per n in the range");

    for (offset, line) in lines.iter().enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();

        assert_eq!(fields[0], (30 + offset).to_string(), "first field is n");

        assert!(
            fields[1] == "IMPOSSIBLE" || fields[1] == "CANDIDATE",
            "second field is the verdict, got {:?}",
            fields[1]
        );

        if fields[1] == "IMPOSSIBLE" {
            assert_eq!(fields.len(), 3, "a rejection carries its reason");
            assert!(!fields[2].is_empty());
        } else {
            assert_eq!(fields.len(), 2, "a candidate carries no reason");
        }
    }
}

#[test]
fn scan_compact_mode_groups_consecutive_verdicts() {
    let output = run(PC_SCAN, &["--compact", "3", "1", "480"]);

    assert!(output.status.success());

    assert_eq!(
        stdout_of(&output).lines().collect::<Vec<_>>(),
        vec!["1..472\tIMPOSSIBLE", "473..480\tCANDIDATE"],
    );
}

/// Guards the threshold of the published k = 3 and k = 4 results.
#[test]
fn scan_reproduces_published_thresholds() {
    for (k, threshold, vertex) in [(3u32, 473usize, 256usize), (4, 9_641, 5_000)] {
        let output = run(
            PC_SCAN,
            &[
                &k.to_string(),
                &(threshold - 1).to_string(),
                &threshold.to_string(),
            ],
        );

        assert!(output.status.success());

        let lines: Vec<&str> = stdout_of(&output).lines().collect();

        assert_eq!(
            lines[0],
            format!(
                "{}\tIMPOSSIBLE\tdegree<2 vertex={vertex} degree=1",
                threshold - 1
            ),
            "unexpected obstruction below the k={k} threshold"
        );

        assert_eq!(
            lines[1],
            format!("{threshold}\tCANDIDATE"),
            "k={k} threshold should clear every implemented condition"
        );
    }
}

/// Pins the certificate format: the whole lower bound of `k = 3` in bands.
///
/// If this breaks, `witness/k3-n473/bands.txt` no longer describes what the
/// tool produces.
#[test]
fn scan_bands_mode_is_the_published_certificate() {
    let output = run(PC_SCAN, &["--bands", "--quick-only", "3", "1", "472"]);

    assert!(output.status.success());

    assert_eq!(
        stdout_of(&output).lines().collect::<Vec<_>>(),
        vec![
            "1..6\tIMPOSSIBLE\tdegree<2 vertex=1 degree=0",
            "7..25\tIMPOSSIBLE\tdegree<2 vertex=1 degree=1",
            "26..59\tIMPOSSIBLE\tdegree<2 vertex=4 degree=1",
            "60..97\tIMPOSSIBLE\tdegree<2 vertex=27 degree=1",
            "98..183\tIMPOSSIBLE\tdegree<2 vertex=32 degree=1",
            "184..234\tIMPOSSIBLE\tdegree<2 vertex=108 degree=1",
            "235..295\tIMPOSSIBLE\tdegree<2 vertex=216 degree=1",
            "296..472\tIMPOSSIBLE\tdegree<2 vertex=256 degree=1",
        ],
    );
}

#[test]
fn scan_bands_mode_rejects_compact() {
    let output = run(PC_SCAN, &["--bands", "--compact", "3", "1", "10"]);

    assert!(!output.status.success());
    assert!(stdout_of(&output).is_empty());
}

#[test]
fn scan_single_argument_form_scans_one_value() {
    let output = run(PC_SCAN, &["5", "246877"]);

    assert!(output.status.success());
    assert_eq!(stdout_of(&output).trim(), "246877\tCANDIDATE");
}

#[test]
fn scan_help_and_version_succeed() {
    for flag in ["-h", "--help"] {
        let output = run(PC_SCAN, &[flag]);
        assert!(output.status.success());
        assert!(stdout_of(&output).contains("Usage:"));
    }

    for flag in ["-V", "--version"] {
        let output = run(PC_SCAN, &[flag]);
        assert!(output.status.success());
        assert!(stdout_of(&output).starts_with("pc-scan "));
    }
}

#[test]
fn scan_rejects_invalid_input() {
    for arguments in [
        vec![],
        vec!["5"],
        vec!["1", "10"],
        vec!["5", "10", "1"],
        vec!["5", "0", "10"],
        vec!["x", "1", "10"],
        vec!["5", "1", "10", "20"],
    ] {
        let output = run(PC_SCAN, &arguments);

        assert!(!output.status.success(), "{arguments:?} should be rejected");

        assert!(
            stdout_of(&output).is_empty(),
            "{arguments:?} must not emit records on stdout"
        );
    }
}

/// The memory guard must not change what an unguarded scan reports.
///
/// The scan certificates in `witness/` were produced without it.
#[test]
fn scan_guard_leaves_default_output_untouched() {
    let plain = run(PC_SCAN, &["5", "246870", "246880"]);
    let guarded = run(PC_SCAN, &["--max-graph-n=1000000", "5", "246870", "246880"]);

    assert!(plain.status.success() && guarded.status.success());
    assert_eq!(stdout_of(&plain), stdout_of(&guarded));

    assert!(
        !stdout_of(&plain).contains("UNCHECKED"),
        "an unguarded scan never reports UNCHECKED"
    );
}

#[test]
fn scan_quick_only_never_builds_a_graph() {
    let output = run(PC_SCAN, &["--quick-only", "5", "246876", "246878"]);

    assert!(output.status.success());

    assert_eq!(
        stdout_of(&output).lines().collect::<Vec<_>>(),
        vec![
            "246876\tIMPOSSIBLE\tdegree<2 vertex=124416 degree=1",
            "246877\tUNCHECKED\tgraph skipped by --quick-only",
            "246878\tUNCHECKED\tgraph skipped by --quick-only",
        ],
    );

    let stderr = std::str::from_utf8(&output.stderr).unwrap();

    assert!(
        stderr.contains("graphs built          : 0"),
        "--quick-only must build no graph"
    );
}

/// The whole point of the guard: k = 7 reaches its first candidate at
/// n = 587761723, where a graph would need roughly 36 GB.
#[test]
fn scan_guard_survives_an_unbuildable_candidate() {
    let output = run(
        PC_SCAN,
        &["--quick-only", "--compact", "7", "587761722", "587761724"],
    );

    assert!(output.status.success());

    assert_eq!(
        stdout_of(&output).lines().collect::<Vec<_>>(),
        vec!["587761722\tIMPOSSIBLE", "587761723..587761724\tUNCHECKED"],
    );
}

#[test]
fn scan_rejects_a_duplicated_guard() {
    let output = run(
        PC_SCAN,
        &["--quick-only", "--max-graph-n=10", "5", "1", "10"],
    );

    assert!(!output.status.success());
}

#[test]
fn export_rejects_a_missing_dump() {
    let output = run(PC_EXPORT, &["/nonexistent/dump.pcdump.json"]);

    assert!(!output.status.success());
    assert!(stdout_of(&output).is_empty());
}

/// A fresh directory for one test, removed by the test itself.
fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pc-tsplib-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory must be creatable");
    dir
}

/// The unique Hamiltonian cycle of `G_2(32)`, in canonical order.
const G2_32_CYCLE: [usize; 32] = [
    1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9, 27, 22,
    14, 2, 23, 26, 10, 15,
];

fn write_tour(path: &std::path::Path, nodes: &[usize]) {
    let mut text = format!("TYPE : TOUR\nDIMENSION : {}\nTOUR_SECTION\n", nodes.len());

    for node in nodes {
        text.push_str(&format!("{node}\n"));
    }

    text.push_str("-1\nEOF\n");
    std::fs::write(path, text).expect("tour file must be writable");
}

#[test]
fn tsplib_export_fixes_forced_edges_and_drops_forbidden_ones() {
    let dir = scratch_dir("export");
    let problem = dir.join("g2-32.hcp");

    let output = run(PC_TSPLIB, &["export", "2", "32", problem.to_str().unwrap()]);
    assert!(output.status.success());

    let text = std::fs::read_to_string(&problem).unwrap();
    let section = |name: &str| -> usize {
        text.split(&format!("{name}\n"))
            .nth(1)
            .and_then(|rest| rest.split("-1\n").next())
            .map_or(0, |body| body.lines().count())
    };

    assert!(text.contains("TYPE : HCP\n") && text.contains("DIMENSION : 32\n"));

    // G_2(32) has 46 edges; propagation forces 23 and forbids 4.
    assert_eq!(section("EDGE_DATA_SECTION"), 42);
    assert_eq!(section("FIXED_EDGES_SECTION"), 23);
    assert!(text.ends_with("-1\nEOF\n"));

    let parameters = std::fs::read_to_string(dir.join("g2-32.par")).unwrap();
    assert!(parameters.contains("PROBLEM_FILE = g2-32.hcp\n"));
    assert!(parameters.contains("SUBGRADIENT = NO\n"));

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn tsplib_export_refuses_a_graph_the_preflight_rules_out() {
    let dir = scratch_dir("refuse");
    let problem = dir.join("g2-30.hcp");

    let output = run(PC_TSPLIB, &["export", "2", "30", problem.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(!problem.exists());

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn tsplib_tour_writes_the_canonical_witness() {
    let dir = scratch_dir("tour");
    let tour = dir.join("g2-32.tour");

    // Rotated and reversed, as LKH may return it.
    let mut nodes = G2_32_CYCLE.to_vec();
    nodes.rotate_left(11);
    nodes.reverse();
    write_tour(&tour, &nodes);

    let output = run(PC_TSPLIB, &["tour", "2", "32", tour.to_str().unwrap()]);
    assert!(output.status.success());

    let expected: String = G2_32_CYCLE.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(stdout_of(&output), expected);

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn tsplib_tour_rejects_a_tour_through_a_non_edge() {
    let dir = scratch_dir("non-edge");
    let tour = dir.join("g2-32.tour");

    // A permutation of 1..=32, but 1 + 2 = 3 is not a square.
    write_tour(&tour, &(1..=32).collect::<Vec<_>>());

    let output = run(PC_TSPLIB, &["tour", "2", "32", tour.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(stdout_of(&output).is_empty());

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn tsplib_contracted_export_round_trips_through_its_chains() {
    let dir = scratch_dir("contract");
    let problem = dir.join("g2-32.hcp");

    let output = run(
        PC_TSPLIB,
        &["export", "--contract", "2", "32", problem.to_str().unwrap()],
    );
    assert!(output.status.success());

    let text = std::fs::read_to_string(&problem).unwrap();
    let chains = std::fs::read_to_string(dir.join("g2-32.chains")).unwrap();

    // 23 forced edges in 7 chains leave 16 nodes.
    assert!(text.contains("DIMENSION : 16\n"));
    assert!(chains.starts_with("PC-TSPLIB CHAINS\nK : 2\nN : 32\nNODES : 16\n"));

    // The interior vertices are the ones strictly inside a listed chain;
    // the others are numbered in increasing order.
    let mut interior = [false; 33];

    for line in chains.lines().skip(5).take_while(|line| *line != "EOF") {
        let chain: Vec<usize> = line.split(' ').map(|v| v.parse().unwrap()).collect();

        for &v in &chain[1..chain.len() - 1] {
            interior[v] = true;
        }
    }

    let node_of = |vertex: usize| (1..=vertex).filter(|&v| !interior[v]).count();
    let nodes: Vec<usize> = G2_32_CYCLE
        .iter()
        .filter(|&&v| !interior[v])
        .map(|&v| node_of(v))
        .collect();
    assert_eq!(nodes.len(), 16);

    let tour = dir.join("g2-32.tour");
    write_tour(&tour, &nodes);

    let expanded = run(
        PC_TSPLIB,
        &[
            "tour",
            "2",
            "32",
            tour.to_str().unwrap(),
            dir.join("g2-32.chains").to_str().unwrap(),
        ],
    );
    assert!(expanded.status.success());

    let expected: String = G2_32_CYCLE.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(stdout_of(&expanded), expected);

    // Without the chains, the same tour is 16 vertices short.
    let bare = run(PC_TSPLIB, &["tour", "2", "32", tour.to_str().unwrap()]);
    assert!(!bare.status.success());

    std::fs::remove_dir_all(dir).unwrap();
}
