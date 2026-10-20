# Audit of Sections 2 and 3 of paper 1

On 2026-10-20 an independent agent audited the admissible-set part of paper 1,
*Hamiltonian cycles in power-sum graphs*: Theorem A (the least admissible `n`)
and Theorem B (the islands and the mainland). It was given only the text of
those two sections, and was asked to look for errors rather than to confirm.
It found no false statement. It found:

- one gap in the proof of Theorem B: the upper end of the interval blocked by
  `c^k` was never bounded;
- a wrong cross-reference in the same proof;
- several minor points.

All of them are fixed in the draft.

These are its scripts, kept as written. Every decision uses exact integer
arithmetic, and the brute-force scans build degrees from adjacency alone,
without the degree formula of the paper.

| script | checks | result (2026-10-20) |
|---|---|---|
| `theory.py` | the paper's claims as exact-integer functions (`iroot`, `Q`, `q*`, Theorem B); imported by the others | — |
| `brute.py K N` | the admissible set of `G_K(n)` for every `n <= N`, from adjacency, against Theorem B | match for every run, e.g. `brute.py 4 2000000`: `[9641, 9999]`, `[18193, ...)` in 1.5 s |
| `brute7.c` | the same scan in C, for large `N` | `brute7 7 1200000000`: `[587761723, 612220031]`, `[1161088541, ...)` in 11.5 s, as Theorem B says |
| `lemmas.py` | Lemmas 2.1, 3.1, 3.2 against the definitions for small `k`; every inequality of Section 3 for `k = 2..300`, including the bound missing from the original proof of Theorem B | 0 failures, in 26 s |
| `table.py` | Table 1 for `k = 2..12`; `A(k)` for `k = 2..8`; an independent chain (all powers, all loop positions, `p_2` from its definition) against Theorem B for `k = 2..80`; the island counts; the floor formula for the number of islands for `k <= 2000`, with 200-digit decimals | all match, 0 failures, `N(k) - 0.20954 k` in `[-1.22, 0.73]`, in 3.5 min |

Earlier runs of `brute.py` during the audit reached `k = 2` up to `4 * 10^5`,
`k = 3` up to `3 * 10^6`, `k = 5` up to `1.5 * 10^6` and `k = 6` up to
`3.2 * 10^7`, all matching Theorem B.

## Reproduce

From this directory:

```console
python3 brute.py 4 2000000
python3 brute.py 6 32000000          # slow: pure Python, 32 million values
cc -O2 -o /tmp/brute7 brute7.c && /tmp/brute7 7 1200000000
python3 lemmas.py
python3 table.py
```
