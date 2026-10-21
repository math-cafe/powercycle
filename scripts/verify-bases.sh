#!/bin/sh
# verify-bases.sh K N FILE.bases.xz
#
# A Hamiltonian-cycle check that shares nothing with posum or its verifiers:
# xz, awk, sort, seq, cmp and sha256sum only.
#
# A bases file holds N - 1 bases b_1, ..., b_{N-1}. Decoding from v_1 = 1 by
# v_{i+1} = b_i^K - v_i makes every consecutive sum v_i + v_{i+1} = b_i^K a
# K-th power by construction. What is left to check is:
#   1. the decoded list v_1, ..., v_N is a permutation of 1..N
#      (so all values are distinct vertices of G_K(N));
#   2. the closing sum v_N + 1 is a K-th power;
#   3. the list is canonical: v_2 < v_N.
# The SHA-256 of the decoded list (one value per line, LF) is the witness
# identity of paper 1, Appendix B.
#
# awk computes in doubles: this is exact while every power used stays below
# 2^53, which holds for all published witnesses (the powers are at most 2N).
# Tested with GNU awk.
set -eu
[ $# -eq 3 ] || { echo "usage: $0 K N FILE.bases.xz" >&2; exit 2; }
K=$1 N=$2 F=$3
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT

xz -dc "$F" | awk -v k="$K" 'BEGIN { v = 1; print v } { v = $1^k - v; print v }' > "$T/list"

lines=$(wc -l < "$T/list")
[ "$lines" -eq "$N" ] || { echo "INVALID: $lines values, expected $N"; exit 1; }

sort -n "$T/list" > "$T/sorted"
seq 1 "$N" > "$T/range"
cmp -s "$T/sorted" "$T/range" || { echo "INVALID: not a permutation of 1..$N"; exit 1; }

awk -v k="$K" 'NR == 2 { v2 = $1 } { last = $1 }
  END {
    s = last + 1; r = int(exp(log(s) / k) + 0.5)
    ok = 0
    for (c = r - 1; c <= r + 1; c++) if (c >= 1 && c^k == s) ok = 1
    if (!ok) { print "INVALID: closing sum " s " is not a " k "-th power"; exit 1 }
    if (!(v2 < last)) { print "INVALID: not canonical (v_2 >= v_N)"; exit 1 }
  }' "$T/list"

echo "VALID k=$K n=$N $(sha256sum < "$T/list" | cut -d' ' -f1)"
