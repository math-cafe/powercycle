"""Variant 1: recurse on paths S -> E(parity), with S + E not a k-th power.

Same model as scripts/gerbicz-glue.py, but base sequences run from S (not 1),
copies from M*S + c to M*E(L) +- c, and the output for N must run from S to
E(N): the edge S-E(N) is virtual (added and fixed). Closing into a cycle is left
to a top-level glue, tested separately.
"""

import importlib.util
import itertools
import os
import random
import sys
import tempfile

spec = importlib.util.spec_from_file_location("glue", os.path.join(os.path.dirname(os.path.abspath(__file__)), "gerbicz-glue.py"))
glue = importlib.util.module_from_spec(spec)
spec.loader.exec_module(glue)
iroot = glue.iroot


def is_power(x, k):
    return x > 0 and iroot(x, k) ** k == x


def instance(k, m, S, e_even, e_odd, n0_parity, r, closing=False):
    M = m**k
    h = (M - 1) // 2
    end_of = lambda parity: e_even if parity == 0 else e_odd
    s = 1 if n0_parity == 0 else -1
    e_N = end_of((n0_parity + r) % 2)

    nodes, fixed = [], []
    for g in range(1, h + 1):
        nodes.append((("glue", g), g))
    for c in range(-h, h + 1):
        long = s * c <= r - M
        parity = (n0_parity + (1 if long else 0)) % 2
        i = len(nodes)
        nodes.append((("start", c, long), M * S + c))
        nodes.append((("end", c, long), M * end_of(parity) + (c if parity == 1 else -c)))
        fixed.append((i + 1, i + 2))

    by_value = {v: i + 1 for i, (_, v) in enumerate(nodes)}
    assert len(by_value) == len(nodes)
    if not closing:
        fixed.append((by_value[S], by_value[e_N]))

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


def min_degree(nodes, edges):
    degree = [0] * (len(nodes) + 1)
    for a, b in edges:
        degree[a] += 1
        degree[b] += 1
    return min(degree[1:])


def residues(m, k):
    M = m**k
    h = (M - 1) // 2
    return [(p, r) for p in (0, 1) for r in range(h, h + M + 1)]


if __name__ == "__main__":
    k, m = int(sys.argv[1]), int(sys.argv[2])
    mode = sys.argv[3]  # screen | full
    M = m**k
    h = (M - 1) // 2
    random.seed(1)
    cases = residues(m, k)

    if mode == "screen":
        # Degree-only screen on a sample of cases, for S and both ends in 1..limit.
        limit = int(sys.argv[4])
        sample = random.sample(cases, 12) + [(0, h), (1, h), (0, h + M), (1, h + M)]
        good = []
        for S, e0, e1 in itertools.permutations(range(1, limit + 1), 3):
            if is_power(S + e0, k) or is_power(S + e1, k):
                continue
            if all(min_degree(*instance(k, m, S, e0, e1, p, r)[:2]) >= 2 for p, r in sample):
                good.append((S, e0, e1))
        print(f"k={k} m={m}: {len(good)} triples pass the degree screen out of S, ends in 1..{limit}")
        print(good[:40])
    else:
        S, e0, e1 = map(int, sys.argv[4:7])
        step = int(sys.argv[7]) if len(sys.argv) > 7 else 1
        tally = {}
        with tempfile.TemporaryDirectory() as d:
            for p, r in cases[::step]:
                nodes, edges, fixed = instance(k, m, S, e0, e1, p, r)
                v = glue.solve(nodes, edges, fixed, d)
                tally[v] = tally.get(v, 0) + 1
        print(f"k={k} m={m} S={S} ends {e0}/{e1}: {tally}")
