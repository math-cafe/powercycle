#!/usr/bin/env python3
"""Check Proposition 8.2 of paper 2 (the motif of the end of an island).

    scripts/island-motif-check.py

For every island [S_c, c^k - 1] with k <= 60 (368 islands): at n = c^k - 1,
every partner 2m - x of Proposition 8.2 lies in the range of Proposition 8.1
and the far ends are vertices; at three values of n per island, the pair
count is n - S_c and the far ends of x and 2m - x sum to c^k. Exact integer
arithmetic. Prints the number of islands and of failures.
"""
from fractions import Fraction
def islands(k):
    Q = next(q for q in range(2, 10**6, 2) if Fraction(q + 1, q)**k < Fraction(3, 2))
    c = Q
    while Fraction(c + 2, c)**k > 2:
        yield c, (c + 1)**k - c**k // 2
        c += 2
tot = bad = 0
for k in range(2, 61):
    for c, S in islands(k):
        tot += 1
        D = (c + 1)**k - c**k; m = c**k - S; top = c**k - (c - 1)**k
        n = c**k - 1
        ok = 2*m - (c**k - n) <= top and top + D <= S and m > 0
        # pairs: x in [c^k - n, m), partner 2m - x in (m, 2m - (c^k - n)]
        for n2 in (S, (S + c**k - 1)//2, c**k - 1):
            lo = c**k - n2
            pairs = max(0, m - lo)
            ok &= pairs == n2 - S
            for x in {lo, m - 1} if pairs else set():
                y = 2*m - x
                ok &= (x + D) + (y + D) == c**k and x + D != y + D and max(x, y) <= top
        if not ok:
            bad += 1; print("FAIL", k, c)
print(tot, "islands, k <= 60;", bad, "failures")
