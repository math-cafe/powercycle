/* Is G_2(31) 1-tough? For every non-empty set W of vertices, count the
   components c of G - W and check c <= |W| (W = all vertices excepted).
   All 2^31 - 1 subsets, bitmask flood fill, exact. Prints the worst ratio
   c/|W| found and any violation. */
#include <stdio.h>
#include <stdint.h>
#define N 31
static uint32_t adj[N];
static int comps(uint32_t alive) {
    int c = 0;
    while (alive) {
        uint32_t seen = alive & -alive, front = seen;
        while (front) {
            uint32_t next = 0;
            for (uint32_t f = front; f; f &= f - 1) next |= adj[__builtin_ctz(f)];
            next &= alive & ~seen;
            seen |= next; front = next;
        }
        alive &= ~seen; c++;
    }
    return c;
}
int main(void) {
    for (int a = 1; a <= N; a++) for (int b = a + 1; b <= N; b++) {
        int s = a + b, r = 0; while ((r + 1) * (r + 1) <= s) r++;
        if (r * r == s) { adj[a-1] |= 1u << (b-1); adj[b-1] |= 1u << (a-1); }
    }
    uint32_t full = (1u << N) - 1; long viol = 0; int bestc = 0, bestw = 1;
    for (uint32_t W = 1; W < full; W++) {
        int w = __builtin_popcount(W), c = comps(full & ~W);
        if (c > w) { if (viol < 5) printf("violation: W=%08x |W|=%d c=%d\n", W, w, c); viol++; }
        if ((long)c * bestw > (long)bestc * w) { bestc = c; bestw = w; }
    }
    printf("subsets checked: %u, violations: %ld, max c/|W| = %d/%d\n", full - 1, viol, bestc, bestw);
    return viol != 0;
}
