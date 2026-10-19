#!/usr/bin/env python3
"""Check the comb certificate of doc/en/non-hamiltonicity.md, in exact arithmetic.

    scripts/g2-31-comb.py

After the three arithmetic steps of the proof by hand, G_2(31) reduces to five
forced paths, contracted to their ends, and the free vertices 1, 4 and 15.
This script recomputes that reduction from scratch, then checks that the
degree equations, the five forced links, 0 <= x <= 1, one connectivity cut and
one comb inequality admit no common solution: a combination of them with the
multipliers below gives 0 >= 1 (Farkas). Standard library only.
"""
import math
from fractions import Fraction as F

n = 31
square = lambda x: math.isqrt(x) ** 2 == x
E = {frozenset((a, b)) for a in range(1, n + 1) for b in range(a + 1, n + 1) if square(a + b)}

# Steps 1 to 3: degree-two propagation.
forced, forbidden = set(), set()
changed = True
while changed:
    changed = False
    for v in range(1, n + 1):
        avail = [e for e in E - forbidden if v in e]
        if len(avail) == 2 and not set(avail) <= forced:
            forced |= set(avail); changed = True
        if sum(1 for e in forced if v in e) == 2:
            extra = [e for e in avail if e not in forced]
            if extra:
                forbidden |= set(extra); changed = True
assert (len(forced), len(forbidden)) == (23, 6)

# Contract the forced paths to their ends.
nbrs = lambda v: [next(iter(e - {v})) for e in forced if v in e]
links, seen = [], set()
for v in range(1, n + 1):
    if len(nbrs(v)) == 1 and v not in seen:
        prev, cur = None, v
        while True:
            nxt = [u for u in nbrs(cur) if u != prev]
            if not nxt:
                break
            prev, cur = cur, nxt[0]
            seen.add(cur)
        seen.add(v)
        links.append(tuple(sorted((v, cur))))
free = sorted(tuple(sorted(e)) for e in E - forced - forbidden)
edges = sorted(links) + free
nodes = sorted({v for e in edges for v in e})
assert sorted(links) == [(3, 12), (5, 22), (6, 21), (10, 14), (11, 24)]
assert len(nodes) == 13 and len(free) == 15

cross = lambda S: [1 if (a in S) != (b in S) else 0 for a, b in edges]
S = {1, 6, 10, 11, 14, 15, 21, 24}                  # paths C, D, E with 1 and 15
H, teeth = {6, 10, 15, 21}, [{10, 14}, {1, 15}, {3, 4, 6, 12, 21}]
assert len(teeth) % 2 == 1 and all(T & H and T - H for T in teeth)
assert all(not (a & b) for i, a in enumerate(teeth) for b in teeth[i + 1:])
comb = [sum(c) for c in zip(cross(H), *[cross(T) for T in teeth])]

# Rows: (coefficients, right-hand side, kind) with kind "=", ">=" or "<=".
rows = [([1 if v in e else 0 for e in edges], 2, "=") for v in nodes]
rows += [([1 if e == l else 0 for e in edges], 1, "=") for l in sorted(links)]
rows += [(cross(S), 2, ">="), (comb, 3 * len(teeth) + 1, ">=")]
rows += [([1 if e == f else 0 for e in edges], 1, "<=") for f in edges]

degree = {1: F(-1, 2), 3: F(-2), 4: F(1), 5: F(-2), 6: F(1, 2), 10: F(-5, 2), 11: F(3, 2),
          12: F(-1), 14: F(-5, 2), 15: F(1, 2), 21: F(-5, 2), 22: F(1), 24: F(-1, 2)}
link = {(3, 12): F(3), (5, 22): F(1), (6, 21): F(2), (10, 14): F(4), (11, 24): F(-1)}
mult = [degree.get(v, F(0)) for v in nodes] + [link[l] for l in sorted(links)] + [F(1, 2), F(1)]
mult += [-F(1) if e == (1, 15) else F(0) for e in edges]   # upper bound x(1,15) <= 1, as -x >= -1

assert all(m >= 0 for m, (_, _, k) in zip(mult, rows) if k == ">=")
assert all(m <= 0 for m, (_, _, k) in zip(mult, rows) if k == "<=")
# Sum of multiplier * row: every coefficient <= 0 (with x >= 0), right-hand side > 0.
coef = [sum(m * r[0][j] for m, r in zip(mult, rows)) for j in range(len(edges))]
rhs = sum(m * r[1] for m, r in zip(mult, rows))
assert all(c <= 0 for c in coef) and rhs > 0
print(f"certificate checked: a combination with every coefficient <= 0 has right-hand side {rhs} > 0, impossible for x >= 0")
