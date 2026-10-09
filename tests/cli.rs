//! Contract tests for the command-line tools.
//!
//! The published scan certificates are byte-for-byte `pc-scan` output, so its
//! stdout format is part of the repository's promises, not an implementation
//! detail. These tests pin it.

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

/// Guards the lower-bound half of the published k = 3 and k = 4 results.
///
/// If this breaks, `witness/k3-n473/scan.txt` and `witness/k4-n9641/scan.txt`
/// no longer describe what the tool produces.
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
