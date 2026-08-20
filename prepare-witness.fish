#!/usr/bin/env fish

# Build the curated versioned witness bundle for G_5(490463).
#
# Usage:
#   ./prepare-witness.fish [cycle.txt] [final.pcdump.json]
#
# Defaults are the files produced by the validated 2026-08-20 run.

set -l project_root (realpath (status dirname))
set -l witness_dir "$project_root/witness/k5-n490463"

set -l cycle_source "cycle.txt"
set -l dump_source "data/runs/1787236600305-p279014-k5-n490463/dumps/t000001-patched-c0001-g490463-r0-fa62a76a36f8.pcdump.json"

if test (count $argv) -ge 1
    set cycle_source $argv[1]
end

if test (count $argv) -ge 2
    set dump_source $argv[2]
end

if test (count $argv) -gt 2
    echo "usage: prepare-witness.fish [cycle.txt] [final.pcdump.json]" >&2
    exit 2
end

if not type -q zstd
    echo "prepare-witness: zstd is required" >&2
    exit 1
end

if not type -q sha256sum
    echo "prepare-witness: sha256sum is required" >&2
    exit 1
end

if not test -f "$cycle_source"
    echo "prepare-witness: missing cycle witness: $cycle_source" >&2
    exit 1
end

if not test -f "$dump_source"
    echo "prepare-witness: missing final dump: $dump_source" >&2
    exit 1
end

mkdir -p "$witness_dir"

set -l expected_cycle_sha "9c9119e5946db4cf3c433e0c90bb6b098afb19f89a2a3fe6f48bfed669cf1efe"
set -l actual_cycle_sha (sha256sum "$cycle_source" | string split " " | head -n 1)

if test "$actual_cycle_sha" != "$expected_cycle_sha"
    echo "prepare-witness: cycle SHA-256 mismatch" >&2
    echo "  expected: $expected_cycle_sha" >&2
    echo "  found   : $actual_cycle_sha" >&2
    exit 1
end

echo "$actual_cycle_sha  cycle.txt" > "$witness_dir/cycle.txt.sha256"

zstd \
    --force \
    --quiet \
    --threads=0 \
    --ultra \
    -19 \
    "$cycle_source" \
    -o "$witness_dir/cycle.txt.zst"

set -l dump_sha (sha256sum "$dump_source" | string split " " | head -n 1)

echo "$dump_sha  final.pcdump.json" > "$witness_dir/final.pcdump.json.sha256"

zstd \
    --force \
    --quiet \
    --threads=0 \
    --ultra \
    -19 \
    "$dump_source" \
    -o "$witness_dir/final.pcdump.json.zst"

echo "Witness bundle prepared:"
echo "  $witness_dir"
echo
echo "Cycle:"
echo "  SHA-256 : $actual_cycle_sha"
echo "  archive : cycle.txt.zst"
echo
echo "Final dump:"
echo "  dump ID : fa62a76a36f8..."
echo "  SHA-256 : $dump_sha"
echo "  archive : final.pcdump.json.zst"
