#!/usr/bin/env python3
"""Intervals of constant usable-power set in G_k(n).

    scripts/island-profile.py K N

Prints the intervals longer than n/1000 with their degree and the bases b
of the powers b^k a vertex can use, then the degree distribution.
"""

import sys
k, n = int(sys.argv[1]), int(sys.argv[2])
P = [j**k for j in range(2, 200) if j**k <= 2*n]
cuts = {1, n+1}
for p in P:
    for c in (p - n, p, p//2, p//2 + 1, p - n + 1):
        if 1 <= c <= n+1: cuts.add(c)
cuts = sorted(cuts)
def usable(v): return tuple(round(p**(1/k)) for p in P if v < p <= v + n and p != 2*v)
rows = []
for a, b in zip(cuts, cuts[1:]):
    s = usable(a)
    if rows and rows[-1][2] == s and rows[-1][1] == a - 1: rows[-1][1] = b - 1
    else: rows.append([a, b - 1, s])
tot = {}
for a, b, s in rows:
    tot[len(s)] = tot.get(len(s), 0) + b - a + 1
    if b - a + 1 > n // 1000: print(f"{a:>11} {b:>11} {b-a+1:>11}  deg {len(s)}  bases {s}")
print("degree totals:", {d: f"{c} ({100*c/n:.1f}%)" for d, c in sorted(tot.items())})
