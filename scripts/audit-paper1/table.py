from theory import *
from decimal import Decimal, getcontext
getcontext().prec = 200
table = {
2:(6,6,[],31),3:(8,8,[],473),4:(10,12,[(9641,9999)],18193),
5:(12,14,[(246877,248831)],490463),6:(16,18,[(15748961,16777215)],30039769),
7:(18,20,[(587761723,612220031)],1161088541),
8:(20,24,[(25022859361,25599999999),(50873048513,54875873535)],97550233537),
9:(22,26,[(1197518052567,1207269217791),(2493793495513,2641807540223)],4910845645499),
10:(26,28,[(135307584267961,141167095653375)],272608849952489),
11:(28,32,[(8053755031969893,8293509467471871),(16551126896404831,17714699999999999)],32527708004244833),
12:(30,34,[(521942283788549761,531440999999999999),(1091428762649561473,1152921504606846975)],2186010166210090097)}
for k in range(2,13):
    t = theoremB(k)
    print(k, t[0], t[1], len(t[2]), "table match:", (t[0],t[1],t[2],t[3]) == table[k])
print("A(k), k=2..8:", [S(Q_of(k),k) for k in range(2,9)], [Q_of(k) for k in range(2,9)])

# Independent chain: union of B(v) over all powers a^k and all loop positions c^k/2
# using p2 from the definition, then complement; compare with Theorem B.
def p2_def(v, k):
    b = iroot(v, k) + 1; cnt = 0
    while True:
        p = b**k
        if p != 2*v:
            cnt += 1
            if cnt == 2: return p
        b += 1
def chain(k):
    qs = qs_of(k); M = 2*qs + 10
    ivs = []
    for a in range(1, M+1):
        v = a**k; g = p2_def(v, k) - v
        if g - 1 >= v: ivs.append((v, g-1))
    for c in range(2, M+1, 2):
        v = c**k//2; g = p2_def(v, k) - v
        if g - 1 >= v: ivs.append((v, g-1))
    # sanity: nothing blocked near the top of the range
    top = max(b for a, b in ivs)
    assert top < (M//2)**k
    ivs.sort(); merged = []
    for a, b in ivs:
        if merged and a <= merged[-1][1] + 1: merged[-1][1] = max(merged[-1][1], b)
        else: merged.append([a, b])
    assert merged[0][0] == 1
    adm = [(merged[i][1]+1, merged[i+1][0]-1) for i in range(len(merged)-1)]
    return adm, merged[-1][1] + 1
bad = 0
for k in range(2, 81):
    Q, qs, isl, main = theoremB(k)
    adm, m = chain(k)
    if adm != isl or m != main: bad += 1; print("chain mismatch", k)
print("chain (all powers + all loops, generic p2) vs Theorem B, k=2..80: mismatches", bad)

# Island count claim and floor formula with 200-digit Decimal
cnt = {k: len(theoremB(k)[2]) for k in range(2, 13)}
print("island counts:", cnt)
fl_bad = []
import math
dev = []
C = 1/Decimal(2).ln() - 1/(2*(Decimal(3)/2).ln())
for k in range(2, 2001):
    Q, qs = Q_of(k), qs_of(k)
    t1 = 1/((Decimal(3)/2)**(Decimal(1)/k) - 1)
    t2 = 2/(Decimal(2)**(Decimal(1)/k) - 1)
    N = (qs - Q)//2
    if N != int(t2/2) - int(t1/2): fl_bad.append(k)
    if not (t1 < Q <= t1 + 2 and t2 < qs <= t2 + 2): fl_bad.append(("Qdef", k))
    dev.append(N - C*k)
print("floor formula failures k<=2000:", fl_bad)
print("C =", str(C)[:12], " N(k)-Ck range:", float(min(dev)), float(max(dev)))
