#!/usr/bin/env fish

# Full pre-publication check.
#
# Usage:
#   ./scripts/check.fish [--quick]
#
# Runs the code checks, then re-derives every published claim from the
# committed witness bundles: checksums, both independent verifiers, and a
# byte-for-byte replay of each lower-bound certificate. Then it builds a
# throwaway bundle, and checks scripts/islands-chain.py against pc-scan.
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

set -g failures

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

for tool in cargo xz sha256sum python3
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
if cargo clippy --workspace --all-targets --quiet -- -D warnings
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
        if not test -f "$bundle/$artifact.xz"
            continue
        end

        xz -dc "$bundle/$artifact.xz" >"$work/$artifact"

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
    if test -f "$bundle/scan.txt.xz"
        if ./target/release/pc-scan $k 1 $n 2>/dev/null | diff -q - "$work/scan.txt" >/dev/null
            echo "    lower bound: scan replayed identically"
        else
            record_failure "$name scan replay"
        end
    else
        echo "    lower bound: no certificate in this bundle"
    end
end

# Verifying published bundles never runs the bundler, so a fault in it is
# invisible here: moving the script once made it write outside witness/ and
# record fresh checksums instead of verifying committed ones. This builds a
# throwaway bundle for the smallest Hamiltonian case and removes it again.
step "bundle construction"

set -l probe_k 2
set -l probe_n 32
set -l probe_bundle "$project_root/witness/k$probe_k-n$probe_n"

if test -e "$probe_bundle"
    echo "    skipped: $probe_bundle exists and will not be touched"
else
    set -l probe_run (./target/release/pc-search --dump=records \
        --on-hamiltonian=stop $probe_k $probe_n 2>/dev/null)

    set -l probe_dumps $probe_run/dumps/*patched*c0001*.pcdump.json

    if test (count $probe_dumps) -eq 0
        record_failure "bundle construction: no one-cycle dump"
    else
        ./target/release/pc-export $probe_dumps[1] \
            >"$work/probe-cycle.txt" 2>/dev/null

        ./target/release/pc-scan $probe_k 1 $probe_n \
            >"$work/probe-scan.txt" 2>/dev/null

        if fish "$project_root/scripts/prepare-witness.fish" \
                $probe_k $probe_n "$work/probe-cycle.txt" \
                $probe_dumps[1] "$work/probe-scan.txt" >/dev/null
            echo "    built"
        else
            record_failure "bundle construction: prepare-witness failed"
        end

        # It must land under witness/, not beside whatever invoked it.
        if test -f "$probe_bundle/cycle.txt.xz"
            echo "    landed in witness/k$probe_k-n$probe_n"
        else
            record_failure "bundle construction: not written to witness/"
        end

        set -l strays (find $project_root -type d -name witness \
            -not -path "$project_root/witness" -not -path '*/target/*')

        if test -z "$strays"
            echo "    no stray witness root"
        else
            record_failure "bundle construction: stray root $strays"
        end

        # A bundle the script accepts must still be a valid witness: an empty
        # cycle.txt has a perfectly good checksum.
        if test -f "$probe_bundle/cycle.txt.xz"
            xz -dc "$probe_bundle/cycle.txt.xz" >"$work/probe-back.txt"

            set -l lines (wc -l <"$work/probe-back.txt" | string trim)

            if test "$lines" -eq $probe_n
                echo "    $probe_n vertices round-tripped"
            else
                record_failure "bundle construction: $lines lines"
            end

            if ./src/verify/rust/target/release/powercycle-verify \
                    $probe_k $probe_n "$work/probe-back.txt" >/dev/null
                echo "    rust verifier: VALID"
            else
                record_failure "bundle construction: rust verifier"
            end

            if python3 src/verify/python/verify-cycle.py \
                    $probe_k $probe_n "$work/probe-back.txt" >/dev/null
                echo "    python verifier: VALID"
            else
                record_failure "bundle construction: python verifier"
            end
        end

        # Re-running on an existing bundle must verify, never overwrite.
        if fish "$project_root/scripts/prepare-witness.fish" \
                $probe_k $probe_n "$work/probe-cycle.txt" \
                $probe_dumps[1] "$work/probe-scan.txt" >/dev/null
            echo "    re-run verifies existing checksums"
        else
            record_failure "bundle construction: not idempotent"
        end
    end

    rm -rf "$probe_bundle"

    if test -n "$probe_run"
        rm -rf "$probe_run"
    end
end

# islands.md section 5 rests on scripts/islands-chain.py, which shares no code
# with pc-scan. Both must name the same smallest blocking vertex for every n up
# to the final island.
step "island chain"

for chain_k in 2 3 4 5
    if not scripts/islands-chain.py $chain_k --ranges >"$work/chain.txt"
        record_failure "island chain k = $chain_k: script failed"
        continue
    end

    set -l final_start (tail -n 1 "$work/chain.txt" | string split \t)[1]

    # Group pc-scan's per-n verdicts into the same start, end, vertex ranges.
    ./target/release/pc-scan --quick-only $chain_k 1 $final_start 2>/dev/null \
        | awk -F '\t' '
            {
                split($3, field, /[= ]/)
                vertex = ($2 == "IMPOSSIBLE") ? field[3] : "-"
                if (vertex != current) {
                    if (NR > 1) print start "\t" NR - 1 "\t" current
                    start = NR
                    current = vertex
                }
            }
            END { print start "\t" NR "\t" current }' >"$work/scan-ranges.txt"

    if diff -q "$work/scan-ranges.txt" "$work/chain.txt" >/dev/null
        echo "    k = $chain_k: agrees on n = 1..$final_start"
    else
        record_failure "island chain k = $chain_k"
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
