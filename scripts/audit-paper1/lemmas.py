from theory import *
from fractions import Fraction as F
import itertools
bad = []
def chk(cond, msg):
    if not cond: bad.append(msg)

# Lemma 2.1 vs adjacency, and Lemma 3.1/3.2 vs definition of p2, small k
def p2_def(v, k):
    cnt = 0; b = 1
    while True:
        p = b**k
        if p > v and p != 2*v:
            cnt += 1
            if cnt == 2: return p
        b += 1
for k in range(2, 6):
    for n in range(1, 400):
        for v in range(1, n+1):
            adj = sum(1 for u in range(1, n+1) if u != v and iroot(u+v, k)**k == u+v)
            f = iroot(v+n, k) - iroot(v, k) - (1 if iroot(2*v, k)**k == 2*v else 0)
            chk(adj == f, ("L2.1", k, n, v))
            chk((adj <= 1) == (n < p2_def(v, k) - v), ("L3.1", k, n, v))
    for v in range(1, 200000 if k > 2 else 20000):
        a = iroot(v, k); g = p2_def(v, k) - v; ga = (a+2)**k - a**k
        c = iroot(2*v, k)
        if c**k == 2*v:
            chk(c >= a+1, ("L3.2 c>=a+1", k, v))
            if c == a+1: chk(g == (c+2)**k - c**k//2, ("L3.2 near", k, v))
            elif c == a+2: chk(g == (c+1)**k - c**k//2, ("L3.2 far", k, v))
            else: chk(g <= ga and v >= a**k, ("L3.2 c>=a+3", k, v))
        else:
            chk(g <= ga, ("L3.2.1", k, v))
print("small-k lemma 2.1/3.1/3.2 failures:", bad[:5], len(bad))

bad = []
for k in range(2, 301):
    Q, qs = Q_of(k), qs_of(k)
    # L3.4.1: phi(q)!=3/2, psi(q)!=2 for q near thresholds
    for q in range(1, qs+5):
        chk(2*(q+1)**k != 3*q**k and (q+2)**k != 2*q**k, ("3.4.1", k, q))
    chk(Q <= qs, ("3.4.2", k))
    chk(Q >= 6, ("3.4.3a", k))
    for c in range(2, Q, 2): chk(2*(c+1)**k > 3*c**k, ("3.4.3b", k, c))
    chk(Q**k > 2*(Q-2)**k, ("3.4.4 psi(Q-2)>2", k))
    chk(Q**k < 2*(Q-1)**k, ("3.4.5 phi(Q-1)<2", k))
    # stronger: phi(Q-1) < (3/2)^{6/5}  <=>  (Q/(Q-1))^{5k} < (3/2)^6
    chk((Q**(5*k))*2**6 < 3**6*(Q-1)**(5*k), ("3.4.5 bound", k))
    # Theorem A induction: c^k/2 near or far for all even 2<=c<=Q; Q^k/2 far
    for c in range(2, Q+1, 2):
        v = c**k//2; a = iroot(v, k)
        chk(c - a in (1, 2), ("A near/far", k, c))
    chk(Q - iroot(Q**k//2, k) == 2, ("A Q far", k))
    chk(S(Q, k) < Q**k, ("S_Q<Q^k", k))
    chk((Q+2)**k//2 > S(Q, k), ("(Q+2)^k/2>S_Q", k))
    chk((Q+1)**k - (Q-1)**k <= S(Q, k), ("powers", k))
    chk(Q**k - (Q-2)**k//2 <= S(Q, k), ("loops", k))
    # L3.5
    for c in range(Q, qs+1, 2):
        v = c**k//2; chk(c - iroot(v, k) == 2, ("3.5.1", k, c))
    for c in range(qs+2, qs+20, 2):
        v = c**k//2; chk(c - iroot(v, k) >= 3, ("3.5.2", k, c))
    # L3.6
    for c in range(Q, qs, 2):
        chk((c+2)**k - c**k - 1 >= c**k, ("3.6.1 nonempty", k, c))
        chk((c+3)**k - (c+1)**k <= S(c+2, k), ("3.6.1 (c+1)^k", k, c))
        # the unstated step: B(c^k) ends by S_{c+2}-1
        chk((c+2)**k - c**k <= S(c+2, k), ("B(c^k) within", k, c))
    for c in range(qs, qs+20):
        chk((c+2)**k <= 2*c**k, ("3.6.2", k, c))
    # Thm B separation
    for c in range(Q, qs+1, 2): chk(S(c, k) < c**k, ("island nonempty", k, c))
    for c in range(Q, qs, 2): chk(S(c+2, k) > c**k, ("gap nonempty", k, c))
print("k=2..300 lemma failures:", bad[:10], len(bad))
