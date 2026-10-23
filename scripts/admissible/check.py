"""Check powersum.py against a direct computation of the minimum degree.

    python3 check.py [KMAX]

For every k from 2 to KMAX (default 6) and every n up to twice the start of
the mainland, n is admissible when every vertex v <= n has degree at least
two in G_k(n), that is, n >= g(v), where g(v) + v is the second power above
v other than 2v. The admissible n found this way must be exactly the islands
and the mainland given by powersum.py. Then powersum.py and powersum.gp
must agree for k up to 200 (the latter run with $GP, default gp).
"""
import subprocess
import sys

import numpy as np

import os

import powersum as ps

HERE = os.path.dirname(os.path.abspath(__file__))


def admissible_up_to(k, top):
    P = np.array([j**k for j in range(1, int(round((2 * top + 1) ** (1 / k))) + 3)], dtype=np.int64)
    v = np.arange(1, top + 1, dtype=np.int64)
    i = np.searchsorted(P, v, side="right")        # first power above v
    p1, p2, p3 = P[i], P[i + 1], P[i + 2]
    second = np.where(p1 == 2 * v, p3, np.where(p2 == 2 * v, p3, p2))
    g = second - v
    run = np.maximum.accumulate(g)
    return np.nonzero(run <= v)[0] + 1             # the admissible n


def main():
    kmax = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    for k in range(2, kmax + 1):
        top = 2 * ps.mainland(k)
        got = admissible_up_to(k, top)
        want = np.concatenate([np.arange(a, b + 1) for a, b in ps.islands(k)]
                              + [np.arange(ps.mainland(k), top + 1)])
        ok = len(got) == len(want) and bool((got == want).all())
        print(f"k = {k}: n <= {top}, {len(got)} admissible, {'agrees' if ok else 'DISAGREES'}")
        assert ok
    lines = "\n".join(f'print(Q({k}), " ", qstar({k}), " ", A({k}), " ", mainland({k}), " ", islands({k}))'
                      for k in range(2, 201))
    gp = os.environ.get("GP", "gp")
    out = subprocess.run([gp, "-q", "-f", os.path.join(HERE, "powersum.gp")], input=lines,
                         capture_output=True, text=True, check=True).stdout.split("\n")
    for k, line in zip(range(2, 201), out):
        isl = "[" + ", ".join(f"[{a}, {b}]" for a, b in ps.islands(k)) + "]"
        assert line == f"{ps.Q(k)} {ps.qstar(k)} {ps.A(k)} {ps.mainland(k)} {isl}", k
    print("powersum.py and powersum.gp agree for k = 2..200")


if __name__ == "__main__":
    main()
