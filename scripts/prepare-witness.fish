#!/usr/bin/env fish

# Build a curated versioned witness bundle.
#
# Usage:
#   scripts/prepare-witness.fish <k> <n> <cycle.txt> <final.pcdump.json> [scan.txt]
#
# The bundle is written to witness/k<k>-n<n>/. Each artifact is stored
# compressed, next to the SHA-256 of its uncompressed form.
#
# When a checksum file already exists, the freshly computed SHA-256 must match
# it. Re-running the script on an existing bundle is therefore a verification
# rather than a silent overwrite.
#
# The optional scan.txt is the pc-scan lower-bound certificate, as produced by
#
#   pc-scan <k> 1 <n>

set -l project_root (realpath (status dirname)/..)

if test (count $argv) -lt 4 -o (count $argv) -gt 5
    echo "usage: scripts/prepare-witness.fish <k> <n> <cycle.txt> <final.pcdump.json> [scan.txt]" >&2
    exit 2
end

set -l k $argv[1]
set -l n $argv[2]
set -l cycle_source $argv[3]
set -l dump_source $argv[4]

set -l scan_source ""

if test (count $argv) -ge 5
    set scan_source $argv[5]
end

if not string match -qr '^[0-9]+$' -- $k
    echo "prepare-witness: k must be a positive integer: $k" >&2
    exit 2
end

if not string match -qr '^[0-9]+$' -- $n
    echo "prepare-witness: n must be a positive integer: $n" >&2
    exit 2
end

set -l witness_dir "$project_root/witness/k$k-n$n"

if not type -q zstd
    echo "prepare-witness: zstd is required" >&2
    exit 1
end

if not type -q sha256sum
    echo "prepare-witness: sha256sum is required" >&2
    exit 1
end

# Stores one artifact: verify or record its SHA-256, then compress it.
#
#   $argv[1]  source path
#   $argv[2]  canonical name inside the bundle
function store_artifact --argument-names source name
    set -l witness_dir $argv[3]

    if not test -f "$source"
        echo "prepare-witness: missing artifact: $source" >&2
        return 1
    end

    set -l actual (sha256sum "$source" | string split " " | head -n 1)
    set -l checksum_path "$witness_dir/$name.sha256"

    if test -f "$checksum_path"
        set -l expected (cat "$checksum_path" | string split " " | head -n 1)

        if test "$actual" != "$expected"
            echo "prepare-witness: $name SHA-256 mismatch" >&2
            echo "  expected: $expected" >&2
            echo "  found   : $actual" >&2
            return 1
        end
    end

    echo "$actual  $name" >"$checksum_path"

    zstd \
        --force \
        --quiet \
        --threads=0 \
        --ultra \
        -19 \
        "$source" \
        -o "$witness_dir/$name.zst"

    echo "  $name : $actual"
end

mkdir -p "$witness_dir"

echo "Witness bundle: k = $k, n = $n"
echo "  $witness_dir"
echo

store_artifact "$cycle_source" cycle.txt "$witness_dir"; or exit 1
store_artifact "$dump_source" final.pcdump.json "$witness_dir"; or exit 1

if test -n "$scan_source"
    store_artifact "$scan_source" scan.txt "$witness_dir"; or exit 1
end

echo
echo "Verify the witness with:"
echo "  zstd -dc $witness_dir/cycle.txt.zst > /tmp/powercycle-cycle.txt"
echo "  cargo run --release --manifest-path src/verify/rust/Cargo.toml \\"
echo "    -- $k $n /tmp/powercycle-cycle.txt"
echo "  python src/verify/python/verify-cycle.py $k $n /tmp/powercycle-cycle.txt"
