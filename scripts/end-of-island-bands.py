#!/usr/bin/env python3
"""Check the band structure of G_6(16777215), doc/en/end-of-island.md.

    scripts/end-of-island-bands.py [SAMPLES]

Exact integer arithmetic. For each band, the bounds and SAMPLES random
vertices (default 20000) are checked against the claimed neighbours.
"""
import random
import sys

n = 16**6 - 1
P = [j**6 for j in range(1, 18)]
p14, p15, p16, p17 = (j**6 for j in (14, 15, 16, 17))
D = p17 - p16          # 7360353
E = p15 - D            # 4030272
m_lo = p16 - p15 + 1   # 5386592


def nbrs(v):
    return sorted(p - v for p in P if v < p <= v + n and p != 2 * v)


def check(name, lo, hi, pred, samples, rnd):
    xs = sorted({lo, hi} | {rnd.randint(lo, hi) for _ in range(samples)})
    bad = [x for x in xs if not pred(x)]
    print(f"{name}: {len(xs)} vertices checked, {len(bad)} exceptions {bad[:3]}")
    return not bad


def main():
    samples = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
    rnd = random.Random(1)
    ok = all([
        check("T = [15^6, n], degree 2, neighbours 16^6 - u and 17^6 - u", p15, n,
              lambda u: nbrs(u) == sorted([p16 - u, p17 - u]), samples, rnd),
        check("B = [1, E - 1]: chain x - (16^6 - x) - (x + D), x + D below 15^6", 1, E - 1,
              lambda x: p16 - x >= p15 and x + D < p15, samples, rnd),
        check("low pairs [E, 16^6 - 15^6]: chain to 2*16^6 - 17^6 - x", E, p16 - p15,
              lambda x: x + D >= p15 and p16 - (x + D) < p15 and E <= p16 - (x + D) <= p16 - p15,
              samples, rnd),
        check("H = [D + 1, 15^6 - 1]: neighbours among 14^6, 15^6, 16^6, 17^6 minus h", D + 1, p15 - 1,
              lambda h: set(nbrs(h)) == {x for x in (p14 - h, p15 - h, p16 - h, p17 - h) if 1 <= x <= n and x != h},
              samples, rnd),
        check("M = [16^6 - 15^6 + 1, D], degree 3: 14^6 - m, 15^6 - m, 16^6 - m", m_lo, D,
              lambda m: nbrs(m) == sorted([p14 - m, p15 - m, p16 - m]), samples, rnd),
    ])
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
