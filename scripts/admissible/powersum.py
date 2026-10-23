"""The admissible set of the power-sum graphs G_k(n) (Theorems A and B).
Exact integer arithmetic; c is always even."""


def Q(k):
    c = 2
    while 2 * (c + 1)**k >= 3 * c**k:
        c += 2
    return c


def qstar(k):
    c = Q(k)
    while (c + 2)**k >= 2 * c**k:
        c += 2
    return c


def S(k, c):
    return (c + 1)**k - c**k // 2


def A(k):                       # least admissible n
    return S(k, Q(k))


def mainland(k):                # start of the mainland
    return S(k, qstar(k))


def N(k):                       # number of islands
    return (qstar(k) - Q(k)) // 2


def islands(k):
    return [(S(k, c), c**k - 1) for c in range(Q(k), qstar(k), 2)]
