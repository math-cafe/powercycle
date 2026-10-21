# Checking Gerbicz's square-sum program

R. Gerbicz's proof that `G_2(n)` is Hamiltonian for every `n >= 32`
(mersenneforum.org, thread 17331, post #22, 2018) comes with a C program,
`squares.c`, that builds the cycle by the recursion `n -> 49n + r`. Its glue
tables are in the program. In 2024 a reader of the thread (Y. Lee, posts #48
and #49) reported entries of the table for the step `n -> 49n + 24`, `n` even,
that did not seem to give square sums, for example `{3,0,0}` followed by
`{-5,-1,0}`, and `{13,0,0}` followed by `{5,-1,0}`. No reply was posted.

These scripts run the program and check its output with independent code.
The program is not redistributed here; it is linked from post #22.

## Results (2026-10-21)

| range | checked | bad |
|---|---|---|
| every `n` from 32 to 62000 | 61969 | 0 |
| every `N = 49m + 24`, `m` even, `2000 < N <= 10^6` (the questioned step) | 10183 | 0 |
| ten values `n = 1000003 + 999983 j` up to `10^7` | 10 | 0 |

Every output is a Hamiltonian cycle of `G_2(n)`: a permutation of `1..n`,
consecutive sums and the closing sum all perfect squares (`chk.c`, exact
64-bit integers).

## Why the entries looked wrong

An entry `{c, ty, e}` is a single number `c` when `ty = 0`; otherwise it is a
copy of the base sequence `e`, scaled by 49, forward (`ty = 1`) or reversed
(`ty = -1`), whose term at original position `k` is `49 x + c` for odd `k`
and `49 x - c` for even `k` (`get_term`, the lines `if(k&1)return ret+c;
else return ret-c;`). In a reversed copy of an even-length base sequence the
first term comes from an even position, so `{-5,-1,0}` starts with
`49 * 8 + 5 = 397`, not `49 * 8 - 5`. The program indeed places
`3, 397` and `13, 387` (both sums 400) in its cycle for `N = 2082`. Reading
the sign without the parity gives `3 + 387 = 390` and `13 + 397 = 410`, the
two counterexamples reported.

## Reproduce

Download `squares.c` from the link in post #22 into this directory, then, from here:

```console
gcc -O2 -o gsq squares.c -lm
gcc -O2 -o chk chk.c -lm
./check-range.sh ./gsq 32 62000 1
./check-lee.sh ./gsq
./check-range.sh ./gsq 1000003 10000000 999983
```
