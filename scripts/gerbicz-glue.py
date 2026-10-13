#!/usr/bin/env python3

# Feasibility of R. Gerbicz's glue step for k-th powers, one output at a time.
#
# Usage:
#   scripts/gerbicz-glue.py <k> <m> <e_even> <e_odd> [residue step] [--start S]
#
# Base pair: seq0 of length n0 and seq1 of length n0+1, both from 1 to E(L), an
# end fixed by the parity of the length L, with 1 + E(L) a k-th power so that
# each closes into a cycle. With M = m^k = 2h + 1, copy c (c = -h..h) of the
# base sequence of length L runs from M + c to M*E(L) + (c if L odd else -c).
# The output for N = M*n0 + r must again run from 1 to E(N), through the glue
# integers 1..h and every copy, each junction a k-th power: a Hamiltonian cycle
# of a small graph in which each copy is a chain with two ends, as in
# pc-tsplib --contract, and the edge 1-E(N) is fixed.
#
# Which copies are long is forced: the extra term n0+1 of seq1 sits at a
# position of the parity of n0+1, so its copy value is M*(n0+1) + s*c with s
# fixed, and the long copies are exactly those with s*c <= r - M. Nothing
# depends on n0 beyond its parity, so each (parity, r) is a finite problem. That
# is why Gerbicz's code finds all its glues in about a second.
#
# This model ignores the nice-pair condition between the outputs for N and
# N+1, so a success here is necessary, not sufficient. It reproduces Gerbicz's
# facts for squares: every case feasible at M = 49 with ends 8/3, and blocked
# cases at M = 25. See doc/en/cube-necklaces.md.
#
# --start S makes the base sequences and every output run from S instead of 1:
# copies start at M*S + c, and the output's edge S-E(N) is virtual. With
# S + E not a k-th power this is the path variant, which removes the forbidden
# junction between a copy's own ends; closing into a cycle is then left to a
# top-level glue, not modelled here.

import os
import subprocess
import sys
import tempfile


def iroot(x, k):
    r = int(round(x ** (1.0 / k)))
    while r**k > x:
        r -= 1
    while (r + 1) ** k <= x:
        r += 1
    return r


def instance(k, m, e_even, e_odd, n0_parity, r, start=1):
    """Nodes (label, value), edges and fixed edges, 1-based, or None when the
    output's end is not a glue integer."""
    M = m**k
    h = (M - 1) // 2
    end_of = lambda parity: e_even if parity == 0 else e_odd
    s = 1 if n0_parity == 0 else -1
    e_N = end_of((n0_parity + r) % 2)

    if e_N > h:
        return None

    nodes, fixed = [], []

    for g in range(1, h + 1):
        nodes.append((("glue", g), g))

    for c in range(-h, h + 1):
        long = s * c <= r - M
        parity = (n0_parity + (1 if long else 0)) % 2
        i = len(nodes)
        nodes.append((("start", c, long), M * start + c))
        nodes.append((("end", c, long), M * end_of(parity) + (c if parity == 1 else -c)))
        fixed.append((i + 1, i + 2))

    by_value = {v: i + 1 for i, (_, v) in enumerate(nodes)}
    assert len(by_value) == len(nodes), "node values must be distinct"
    fixed.append((by_value[start], by_value[e_N]))

    top = max(by_value)
    edges = set(tuple(sorted(e)) for e in fixed)

    for i, (label, value) in enumerate(nodes):
        b = iroot(value, k) + 1
        while b**k - value <= top:
            j = by_value.get(b**k - value)
            b += 1
            if j is None or j == i + 1:
                continue
            other = nodes[j - 1][0]
            if label[0] != "glue" and other[0] != "glue" and label[1] == other[1]:
                continue
            edges.add(tuple(sorted((i + 1, j))))

    return nodes, sorted(edges), fixed


def solve(nodes, edges, fixed, workdir):
    n = len(nodes)
    degree = [0] * (n + 1)

    for a, b in edges:
        degree[a] += 1
        degree[b] += 1

    if min(degree[1:]) < 2:
        return "degree<2"

    with open(f"{workdir}/g.hcp", "w") as f:
        f.write(f"NAME : glue\nTYPE : HCP\nDIMENSION : {n}\n"
                "EDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION\n")
        f.writelines(f"{a} {b}\n" for a, b in edges)
        f.write("-1\nFIXED_EDGES_SECTION\n")
        f.writelines(f"{a} {b}\n" for a, b in fixed)
        f.write("-1\nEOF\n")

    with open(f"{workdir}/g.par", "w") as f:
        f.write("PROBLEM_FILE = g.hcp\nTOUR_FILE = g.tour\nSUBGRADIENT = NO\nOPTIMUM = 0\n"
                "STOP_AT_OPTIMUM = YES\nRUNS = 5\nTRACE_LEVEL = 0\nMAX_TRIALS = 2000\nTIME_LIMIT = 20\n")

    if os.path.exists(f"{workdir}/g.tour"):
        os.remove(f"{workdir}/g.tour")

    if subprocess.run(["LKH", "g.par"], cwd=workdir, capture_output=True).returncode != 0:
        return "lkh crashed"

    if not os.path.exists(f"{workdir}/g.tour"):
        return "no tour"

    return "ok" if "Length = 0" in open(f"{workdir}/g.tour").read() else "cost>0"


if __name__ == "__main__":
    args = sys.argv[1:]
    start = 1

    if "--start" in args:
        i = args.index("--start")
        start = int(args[i + 1])
        del args[i:i + 2]

    if len(args) not in (4, 5):
        print("usage: scripts/gerbicz-glue.py <k> <m> <e_even> <e_odd> [residue step] [--start S]",
              file=sys.stderr)
        sys.exit(2)

    k, m, e_even, e_odd = map(int, args[:4])
    step = int(args[4]) if len(args) == 5 else 1
    M = m**k
    h = (M - 1) // 2
    tally = {}

    with tempfile.TemporaryDirectory() as workdir:
        for n0_parity in (0, 1):
            # r and r + 1 both in range: outputs for N and N + 1.
            for r in range(h, h + M + 1, step):
                inst = instance(k, m, e_even, e_odd, n0_parity, r, start)
                verdict = "end not glue" if inst is None else solve(*inst, workdir)
                tally[verdict] = tally.get(verdict, 0) + 1

    print(f"k={k} m={m} M={M} start {start} ends {e_even}/{e_odd}: {tally}")
