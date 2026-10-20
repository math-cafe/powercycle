# Exact-integer implementation of the draft's claims.
def iroot(n, k):
    """floor(n^(1/k)) exactly."""
    if n < 2: return n
    lo, hi = 1, 1 << (n.bit_length() // k + 1)
    while lo < hi:
        m = (lo + hi + 1) // 2
        if m**k <= n: lo = m
        else: hi = m - 1
    return lo

def phi_lt_32(c, k):  # phi(c) < 3/2
    return 2*(c+1)**k < 3*c**k
def psi_lt_2(c, k):   # psi(c) < 2
    return (c+2)**k < 2*c**k

def Q_of(k):
    c = 2
    while not phi_lt_32(c, k): c += 2
    return c
def qs_of(k):
    c = 2
    while not psi_lt_2(c, k): c += 2
    return c
def S(c, k): return (c+1)**k - c**k//2

def theoremB(k):
    Q, qs = Q_of(k), qs_of(k)
    islands = [(S(c,k), c**k - 1) for c in range(Q, qs, 2)]
    return Q, qs, islands, S(qs, k)
