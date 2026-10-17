#!/usr/bin/env python3
"""The least admissible m of G_k(m), and checks of its closed form.

    scripts/least-admissible.py terms K    k and the least admissible m, k = 1..K
    scripts/least-admissible.py check K    the closed form against the direct
                                           computation, and every inequality of
                                           doc/en/least-admissible.md, k = 2..K

Admissible: every vertex of G_k(m) has degree at least two. The direct
computation follows the smallest blocking vertex, which is 1, some a^k or some
a^k/2 (doc/en/islands.md, section 5), and does not use the closed form.
Exact integer arithmetic throughout.
"""

import sys


def iroot(x, k):
    """floor(x^(1/k)), exactly."""
    if x < 2:
        return x
    r = int(round(x ** (1.0 / k))) if x.bit_length() < 1000 else 1 << (x.bit_length() // k)
    while r**k > x:
        r -= 1
    while (r + 1) ** k <= x:
        r += 1
    return r


def degree(v, m, k):
    return iroot(v + m, k) - iroot(v, k) - (1 if iroot(2 * v, k) ** k == 2 * v else 0)


def direct(k):
    """The least admissible m, by the chain of blocking vertices."""
    if k == 1:
        return 3  # G_1(m) is complete
    safe = 1
    while (safe + 3) ** k > 2 * safe**k:
        safe += 1
    candidates = sorted({1} | {a**k for a in range(2, safe + 4)} | {a**k // 2 for a in range(2, safe + 4, 2)})
    m = 1
    while True:
        blocker = next((x for x in candidates if x <= m and degree(x, m, k) < 2), None)
        if blocker is None:
            return m
        base = iroot(blocker, k)
        powers = [b**k for b in range(base + 1, base + 4) if b**k != 2 * blocker]
        m = powers[1] - blocker  # the blocker has degree two from here on


def least_even_q(k):
    q = 2
    while not (2 * (q + 1) ** k < 3 * q**k):
        q += 2
    return q


def closed_form(k):
    q = least_even_q(k)
    return (q + 1) ** k - q**k // 2


def check(k):
    """Every inequality of doc/en/least-admissible.md, sections 4 to 6."""
    Q = least_even_q(k)
    A = (Q + 1) ** k - Q**k // 2
    assert Q >= 6
    assert all(2 * (c + 1) ** k >= 3 * c**k for c in range(2, Q, 2))  # (i)
    assert Q**k >= 2 * (Q - 2) ** k  # (ii)
    assert Q**k < 2 * (Q - 1) ** k  # (iii)
    assert A < Q**k  # (iv)
    assert (Q - 2) ** k <= Q**k // 2 < (Q - 1) ** k  # L_Q in the case c = a + 2
    assert (Q + 2) ** k > 2 * A  # L_{Q+2} above A
    assert (Q + 1) ** k - (Q - 1) ** k <= A  # powers a^k, a <= Q - 1
    assert Q**k - (Q - 2) ** k // 2 <= A  # loop positions L_c, c <= Q - 2


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("terms", "check"):
        sys.exit(__doc__)
    K = int(sys.argv[2])
    if sys.argv[1] == "terms":
        for k in range(1, K + 1):
            print(k, direct(k))
        return
    for k in range(2, K + 1):
        check(k)
        if k <= 100:
            assert direct(k) == closed_form(k), k
    print(f"closed form = direct computation for k = 2..{min(K, 100)}; "
          f"every inequality holds for k = 2..{K}")


if __name__ == "__main__":
    main()
