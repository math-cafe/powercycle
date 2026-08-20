#!/usr/bin/env fish

# Full pre-publication check.
#
# Usage:
#   ./scripts/check.fish [--quick]
#
# Runs the code checks, then re-derives every published claim from the
# committed witness bundles: checksums, both independent verifiers, and a
# byte-for-byte replay of each lower-bound certificate.
#
# --quick skips the ignored long-running cross-checks in the test suite.

set -l project_root (realpath (status dirname)/..)
cd $project_root

set -l quick 0

for argument in $argv
    switch $argument
        case --quick
            set quick 1
        case '*'
            echo "usage: check.fish [--quick]" >&2
            exit 2
    end
end

set -l failures

function step --argument-names label
    set_color --bold
    echo ""
    echo "==> $label"
    set_color normal
end

function record_failure --argument-names label
    set -g failures $failures $label
    set_color red
    echo "    FAIL: $label"
    set_color normal
end

for tool in cargo zstd sha256sum python3
    if not type -q $tool
        echo "check: $tool is required" >&2
        exit 1
    end
end

step "formatting"
if cargo fmt --all -- --check
    echo "    clean"
else
    record_failure formatting
end

step "clippy"
if cargo clippy --workspace --all-targets --quiet
    echo "    done"
else
    record_failure clippy
end

step "tests"
if cargo test --workspace --all-targets --quiet
    echo "    passed"
else
    record_failure tests
end

if test $quick -eq 0
    step "exhaustive cross-checks"
    if cargo test --release --workspace --quiet -- --ignored
        echo "    passed"
    else
        record_failure "exhaustive cross-checks"
    end
else
    step "exhaustive cross-checks"
    echo "    skipped (--quick)"
end

step "release build"
if cargo build --release --quiet
    and cargo build --release --quiet --manifest-path src/verify/rust/Cargo.toml
    echo "    built"
else
    record_failure "release build"
end

set -l work (mktemp -d)

for bundle in $project_root/witness/k*-n*/
    set -l name (basename $bundle)
    set -l k (string replace -r '^k([0-9]+)-n[0-9]+$' '$1' $name)
    set -l n (string replace -r '^k[0-9]+-n([0-9]+)$' '$1' $name)

    step "witness $name  (k = $k, n = $n)"

    # Every artifact must match the checksum of its uncompressed form.
    set -l artifact_failed 0

    for artifact in cycle.txt scan.txt final.pcdump.json
        if not test -f "$bundle/$artifact.zst"
            continue
        end

        zstd -dcq "$bundle/$artifact.zst" >"$work/$artifact"

        set -l actual (sha256sum "$work/$artifact" | string split " " | head -n 1)
        set -l expected (cat "$bundle/$artifact.sha256" | string split " " | head -n 1)

        if test "$actual" = "$expected"
            echo "    sha256 $artifact: ok"
        else
            record_failure "$name/$artifact checksum"
            set artifact_failed 1
        end
    end

    if test $artifact_failed -eq 1
        continue
    end

    # Independent verification, twice, by tools outside the dependency graph.
    if ./src/verify/rust/target/release/powercycle-verify $k $n "$work/cycle.txt" >/dev/null
        echo "    rust verifier: VALID"
    else
        record_failure "$name rust verifier"
    end

    if python3 src/verify/python/verify-cycle.py $k $n "$work/cycle.txt" >/dev/null
        echo "    python verifier: VALID"
    else
        record_failure "$name python verifier"
    end

    # A bundle claiming minimality must reproduce its lower-bound certificate.
    if test -f "$bundle/scan.txt.zst"
        if ./target/release/pc-scan $k 1 $n 2>/dev/null | diff -q - "$work/scan.txt" >/dev/null
            echo "    lower bound: scan replayed identically"
        else
            record_failure "$name scan replay"
        end
    else
        echo "    lower bound: no certificate in this bundle"
    end
end

rm -rf $work

echo ""

if test (count $failures) -eq 0
    set_color green --bold
    echo "All checks passed."
    set_color normal
    exit 0
end

set_color red --bold
echo "Failed checks:"
set_color normal

for failure in $failures
    echo "  - $failure"
end

exit 1
