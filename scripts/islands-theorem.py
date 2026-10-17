#!/usr/bin/env python3
"""Check the closed form of the admissible islands, doc/en/islands-theorem.md.

    scripts/islands-theorem.py K

For k = 2..K, compares the islands given by the theorem with those computed by
scripts/islands-chain.py, which follows the smallest blocking vertex and does
not use the theorem, and prints the number of finite islands. Exact integer
arithmetic throughout.
"""

import os
import subprocess
import sys


def theorem(k):
    """[(start, end or None)], from the theorem."""
    above_t1 = lambda q: 2 * (q + 1) ** k < 3 * q**k  # (1 + 1/q)^k < 3/2
    above_t2 = lambda q: (q + 2) ** k < 2 * q**k  # (1 + 2/q)^k < 2
    q = 2
    while not above_t1(q):
        q += 2
    islands = []
    while not above_t2(q):
        islands.append(((q + 1) ** k - q**k // 2, q**k - 1))
        q += 2
    islands.append(((q + 1) ** k - q**k // 2, None))
    return islands


def chain(k):
    """[(start, end or None)], from scripts/islands-chain.py."""
    script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "islands-chain.py")
    out = subprocess.run([script, str(k), "--ranges"], capture_output=True, text=True, check=True).stdout
    rows = [line.split("\t") for line in out.splitlines()]
    islands = [(int(a), int(b)) for a, b, v in rows if v == "-"]
    return islands[:-1] + [(islands[-1][0], None)]


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    K = int(sys.argv[1])
    counts = []
    for k in range(2, K + 1):
        expected = theorem(k)
        if chain(k) != expected:
            sys.exit(f"k = {k}: the theorem and islands-chain.py disagree")
        counts.append(f"{k}:{len(expected) - 1}")
    print(f"theorem = islands-chain.py for k = 2..{K}")
    print("finite islands, k:count  " + " ".join(counts))


if __name__ == "__main__":
    main()
