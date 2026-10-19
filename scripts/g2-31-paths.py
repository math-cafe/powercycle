#!/usr/bin/env python3
"""Every Hamiltonian path of G_2(31), by exhaustive search, and their ends.

    scripts/g2-31-paths.py

Prints the number of Hamiltonian paths, the number of distinct end pairs, the
end pairs whose sum is a square (none: G_2(31) is not Hamiltonian), and the
paths running from 4 to 17, the two neighbours of 32 in G_2(32).
See doc/en/non-hamiltonicity.md, section 3.
"""
import math
import sys

sys.setrecursionlimit(10000)
n = 31
square = lambda x: math.isqrt(x) ** 2 == x
adj = {v: [u for u in range(1, n + 1) if u != v and square(u + v)] for v in range(1, n + 1)}
paths = []


def extend(path, used):
    last = path[-1]
    if len(path) == n:
        if path[0] < last:  # each path once
            paths.append(tuple(path))
        return
    # every unused vertex needs a usable neighbour: unused, or an end
    for w in adj:
        if w not in used and not any(u not in used or u in (last, path[0]) for u in adj[w]):
            return
    for u in adj[last]:
        if u not in used:
            used.add(u)
            path.append(u)
            extend(path, used)
            path.pop()
            used.discard(u)


for start in range(1, n + 1):
    extend([start], {start})
ends = {(p[0], p[-1]) for p in paths}
print(f"{len(paths)} Hamiltonian paths, {len(ends)} distinct end pairs")
print("end pairs with a square sum:", sorted(e for e in ends if square(sum(e))) or "none")
print("neighbours of 32 in G_2(32):", [u for u in range(1, n + 1) if square(u + 32)])
for p in paths:
    if set((p[0], p[-1])) == {4, 17}:
        print("path from 4 to 17:", ", ".join(map(str, p if p[0] == 4 else p[::-1])))
