# Towards every n for cubes: Gerbicz's induction at k = 3

A research note: what of R. Gerbicz's proof for squares carries over to cubes,
and the first measurements. Nothing here is a result yet; it records where the
question stands.

---

## 1. The statement to aim at

For squares the question is closed. Gerbicz proved in January 2018 that
square-sum Hamiltonian cycles exist for every `n >= 32` and paths for every
`n >= 25` (mersenneforum, thread 22915, post 22). The analogue for cubes would
be

```text
G_3(n) is Hamiltonian   <=>   n >= 473
```

`n >= 473` is necessary: it is `I_3` from [`islands.md`](islands.md), which
has no finite island. More generally the natural conjecture is that, for
`k >= 3`, `G_k(n)` is Hamiltonian exactly when `n` lies in an admissible island;
`k = 2`, where 31 is admissible and fails, would be the exception. All the
values tested so far agree, but they are few per `k`.

---

## 2. Gerbicz's construction for squares

Take a nice pair: a square-sum path `seq0` on `1..n` and a path `seq1` on
`1..n+1`, both from `1` to an endpoint fixed by the parity of the length (`8`
for even, `3` for odd, so each closes into a cycle), such that every `x <= n`
sits at positions of the same parity in both. For each `c` in `-24..24` build
a copy

```text
T(c) = 49*a_1 + c, 49*a_2 - c, 49*a_3 + c, ...
```

from `seq0` or `seq1`. Neighbours sum to `49*(a + b) = (7s)^2`, a square
again. The 49 copies cover every residue mod 49; the integers `1..24`, in no
copy, glue the copies end to end into one path. Choosing which copies use the
longer sequence gives every length `49n + r`, `r = 24..72`, a complete residue
system. With a table of nice pairs for `n = 41..2032`, the recursion covers
every `n >= 41`; smaller `n` come from another table.

The parity condition is what lets copies from `seq0` and `seq1` be mixed: `x`
must carry the same sign in both, or two copies would produce the same integer.
Two refinements matter. 49 is needed for cycles, 25 suffices for paths, 9 gives
nothing. And the modulus must be an odd square, since `c = M/2` would give the
same residue as `-c`.

---

## 3. What carries over to `k`

**The identity, for every `k`:**

```text
(m^k*a + c) + (m^k*b - c) = m^k*(a + b) = m^k*s^k = (m*s)^k
```

Only additive cancellation and the multiplicativity of `k`-th powers are used.
The pair itself is not tied to `k` either: two lengths `n` and `n + 1` give
`M + 1` consecutive totals, a full residue system mod `M` whatever `k` is.

**The modulus.** At a junction an endpoint `m^k*a ± c` meets a glue integer
`g`, and the sum must be a `k`-th power, a condition on `k`-th power residues
mod `m^k`. For cubes, when `m ≡ 2 (mod 3)`, so `m = 5, 11, 17, ...`, cubing is a
bijection on the units mod `m^3`, since 3 does not divide `φ(m^3)`, and every
residue prime to `m` is a cube. For `m = 3` or `m ≡ 1 (mod 3)` only a third
are; mod 27 the cubes are `0, 1, 8, 10, 17, 19, 26`. The natural moduli are
therefore `5^3 = 125` and `11^3 = 1331`:

| | M = 125 | M = 1331 |
|---|---|---|
| copies | 125 | 1331 |
| glue integers | 62 | 665 |
| base interval, from `A = 801` | to about 100 000 | to about 1 066 000 |
| base cases to publish | about `5·10^9` vertices | about `5·10^11`, not publishable |

The base cases are part of the proof, so `M = 125` is the first choice; 1331
only if 125 cannot be glued.

**What grows with `k`.** `M = m^k`, hence the base interval, and the scarcity
of `k`-th powers, hence the difficulty of gluing. From `k = 4` on, the finite
islands must also be settled value by value: 359 values at `k = 4`, 1 955 at
`k = 5`, about a million graphs of 15 million vertices at `k = 6`.

---

## 4. Nice pairs as one colouring

Positions alternate along a path, so every edge of a path joins positions of
different parity. The parity condition therefore says: there is one
2-colouring of `1..n` such that both paths use only edges joining the two
colours. That keeps about half the edges. For squares the degree grows like
`√n` and plenty remain. For cubes a vertex has about `0.35·n^(1/3)` neighbours,
three or four near `n = 500`, so the restricted graph may not have enough left.
That was the risk to measure first.

**Search.** `scripts/nice-pairs.py` takes `seq0` from LKH, reads the colouring
off it, then asks LKH for `seq1` in `G_3(n+1)` restricted to the edges the
colouring allows, for each admissible pair of endpoints. Degree-two propagation
discards most hopeless colourings before LKH runs: at `n = 473` it rejected 308
of them in 0.3 s, where LKH needed 64 s to fail on the same cases. A failure is
not a proof of absence, since only colourings read off LKH paths are tried.
Every pair returned is checked for both paths, both closing sums and the parity
condition.

---

## 5. Measurements, `n = 473 .. 2000`

Every `n` searched with 10 seeds, then the 45 failures again with 100. The
first pass lacked one endpoint, `n + 1` for `seq1` when `n + 2` is a cube; it
was fixed before the second. Raw results:
[`data/nice-pairs-k3-473-2000.csv`](data/nice-pairs-k3-473-2000.csv).

| n | nice pair found |
|---|---|
| 473 – 600 | 121 of 128 |
| 601 – 800 | 199 of 200 |
| 801 – 2000 | **1 200 of 1 200** |

Not found: `475, 476, 481, 482, 483, 488, 499, 800`. They show no residue
pattern mod 2, 3, 4, 6, 7, 9 or 27; the gaps sit at the threshold, where the
graph is sparsest. `800` is the exception, isolated among 199 neighbours that
all have a pair, and resisted 100 seeds; only an exact search could tell a hard
case from a genuine absence. About 3 h of CPU for the first pass, 2 min for the
second.

**Cycles below the induction.** One Hamiltonian cycle of `G_3(n)` for every
`n = 473 .. 800`, found by `pc-tsplib` and LKH, checked by `pc-tsplib tour` and
the independent Rust verifier: 328 of 328, in 3 s. The values the recursion
will not reach are therefore covered directly, and the gaps above do not
matter.

---

## 6. Where this stands

- Nice pairs exist for cubes, almost everywhere from the threshold on, and the
  induction could start at `A = 801`. Triplets or non-consecutive lengths,
  considered to fill missing values, are not needed at `k = 3` on this range.
- The decisive open step is the **glue**: joining 125 copies with 62 small
  integers, under fixed endpoints, so that the result is again a nice pair for
  `N` and `N + 1`. Gerbicz's code found his in about a second for squares mod
  49. Whether one exists for cubes mod 125 decides the method.
- The endpoints are not yet constrained: Gerbicz fixes them by parity, and the
  glue will dictate which ones cubes need, which may thin out the usable pairs.
- Base cases up to about `100 000` at `M = 125`. At the current pace, 7.5 s per
  value on average but up to two minutes near `n = 2000`, a faster search will
  be needed.

Reproduce:

```console
scripts/nice-pairs.py search 3 1500
scripts/nice-pairs.py sweep 3 473 2000 sweep.csv 10
scripts/nice-pairs.py cycles 3 473 800 cycles/ cycles.csv
```
