# Brute force I_k from adjacency only (independent of Lemma 2.1):
# incremental degrees, saturated at 2; n admissible iff no vertex has degree <2.
import sys
from theory import *
def brute(k, N):
    pw = [b**k for b in range(1, iroot(2*N+2, k)+2)]
    deg = bytearray(N+2)
    low = 0  # vertices with degree < 2
    adm = []
    for n in range(1, N+1):
        # add vertex n: neighbours u < n with u+n power
        d = 0
        for p in pw:
            if p <= n: continue
            if p >= 2*n: break
            u = p - n
            d += 1
            du = deg[u]
            if du < 2:
                deg[u] = du + 1
                if du + 1 == 2: low -= 1
        d = min(d, 2)
        deg[n] = d
        if d < 2: low += 1
        if low == 0: adm.append(n)
    # compress into intervals
    iv = []
    for n in adm:
        if iv and iv[-1][1] == n-1: iv[-1][1] = n
        else: iv.append([n, n])
    return [tuple(x) for x in iv]

if __name__ == "__main__":
    k, N = int(sys.argv[1]), int(sys.argv[2])
    iv = brute(k, N)
    Q, qs, isl, main = theoremB(k)
    pred = [x for x in isl if x[0] <= N]
    pred = [(a, min(b, N)) for a, b in pred]
    if main <= N: pred.append((main, N))
    print("k=%d N=%d brute=%s" % (k, N, iv))
    print("      theoremB=%s  match=%s" % (pred, iv == pred))
