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
values tested so far agree; [`islands.md`](islands.md) section 6 lists them,
among them every `n` of the finite island of `k = 5`.

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

## 6. The glue, and why it fails as is

**What Gerbicz's code does.** His `squares.c` (2018-01-21, 17 MB, mostly
tables) stores in `data[4*49][73][4]` one glue per residue `r = 24..72`, parity
of `n0` and output (`N` or `N + 1`): a sequence of 73 items, each a glue integer
or a copy `{c, orientation, base sequence}`. A copy term is
`49*seq[k] + c` for odd `k` and `- c` for even `k`. Because every base sequence
starts at `1` and ends at `8` or `3` according to the parity of its length, the
copy ends are `49 + c` and `49*8 ± c` or `49*3 ± c`: they depend on `c` and on
parities, never on `n0`. Each glue is therefore a finite problem, solved once.

**The model.** `scripts/gerbicz-glue.py` builds that finite problem for any
`k`: glue integers as nodes, each copy as a chain with two ends joined by a
fixed edge — the `pc-tsplib --contract` picture — and the edge from `1` to the
output's end fixed; LKH looks for a Hamiltonian cycle. It checks one output at
a time and ignores the nice-pair condition between `N` and `N + 1`, so a
success is necessary, not sufficient. It reproduces Gerbicz's facts for
squares:

| | ends | feasible (parity, residue) cases |
|---|---|---|
| squares, `M = 49` | 8 / 3 | 100 of 100 |
| squares, `M = 25` | 8 / 3 | 18 of 52, the rest blocked |

**Cubes.** With `1 + E` a cube, the possible ends below the glue bound are 7
and 26 at `M = 125`, and 7, 26, 63, 124, 215, 342, 511 at `M = 1331`:

| | ends | result |
|---|---|---|
| `M = 125` | 7 / 26 | 0 of 252: every case has a node with no junction |
| `M = 125` | 26 / 7 | 0 of 252: 192 blocked, 60 without a cycle |
| `M = 1331` | all 42 ordered pairs, 26 residues each | none works for every residue; best 7/26 and 7/215, about half |

At `M = 1331` the failures are not scattered. With ends 7/26 they are the small
residues when `n0` is even and the large ones when it is odd — exactly the
cases where many copies come from the base sequence ending in 7 — and the
blocked nodes are always those copies' ends.

**The obstruction.** A copy runs from `M + c` to `M*E ± c`, and `1 + E` is a
cube by construction, so

```text
start + end of the same copy = M*(1 + E) = m^3 * 8 = (2m)^3        for E = 7
```

The natural partner of a copy's end is its own start, which is forbidden. The
next cube, `(2m + 1)^3`, lies about `3*(2m)^2 = 12 m^2` away, while a window of
copy ends spans only `M = m^3`. Ends in the middle of the window find no other
partner unless `12 m^2 < m^3`, roughly `m > 12`. At `m = 5` this fails
completely; at `m = 11` only the ends near the window's edges have a partner.
For `E = 26`, `M*27 = (3m)^3` and the gap is about `27 m^2`, roughly `m > 27`.
Since the two ends must differ by parity and only 7 has `1 + E = 8`, the other
end forces `m` near 29, `M = 24 389`, and base cases up to about twenty
million: out of reach. These thresholds are orders of magnitude, not exact
bounds.

Squares escape it: `49*9 = 21^2` is the same forbidden junction, but the next
square is only `2*21 + 1 = 43 < 49` away. Squares are dense enough, cubes are
not.

**Ways around:**

1. Recurse on paths whose ends `S, E` do **not** sum to a cube, which removes
   the forbidden junction, and close into a cycle only at the top level, with a
   glue of its own. Only the inner levels need the shape of the base cases.
2. Several base sequences per length with **different ends** — triplets or more
   instead of pairs — to multiply the copy ends and hence the junctions. Pairs
   suffice to cover residues; more sequences might be what diversifying the
   ends needs.
3. Ends chosen by the length modulo 4 or more instead of its parity.
4. A larger `m`, 17 or 23: removes the obstruction for `E = 7` but not for
   `E = 26`, at the cost of base cases up to millions.

**Variant 1 at `M = 125`, measured.** `scripts/gerbicz-glue.py --start S`
models it, with the same limits as above. Over every `S` and pair of ends
tried, none covers all 252 cases:

| start `S` | ends, even / odd length | feasible cases | where it fails |
|---|---|---|---|
| 2 | 1 / 4 | **220 of 252** | a block: `n0` even with `r = 172..187`, `n0` odd with `r = 62..77` |
| 1 | 2 / 3 | 164 of 252 | |
| 4 | 2 / 1 | 110 of 252 | the mixed cases; all four extremes pass |
| 1 | 2 / 4, 4 / 3, 2 / 17 | 58, 52, 34 of 252 | |
| 1 | 3 / 4, 4 / 5 | 2 of 252 | |

The failures stay structured. For `S = 2` with ends 1/4, the 32 failing cases
are exactly those where almost every copy comes from the odd-length base
sequence and so ends in 4, and none is solved by a retry of two minutes each:
an obstruction, not a search limit. With `S = 2` and `E_even = 1`, no
`E_odd <= 62` passes the two cases where it ends every copy. Ends of 1 work
alone because their window, `125 ± c`, sits next to the glue integers; no other
end tried does. A configuration would need two ends that each work when they
dominate and also work mixed, and with 62 glue integers and a parity rule none
was found. The path variant removes the forbidden junction, but not the
shortage of junctions at `M = 125`.

---

## 7. Where this stands

- Nice pairs exist for cubes, almost everywhere from the threshold on, and the
  induction could start at `A = 801`.
- Gerbicz's glue does not carry over unchanged: with closing ends, cubes meet a
  structural obstruction at `M = 125` and at `M = 1331` (section 6). The path
  variant removes that obstruction but, at `M = 125`, covers at most 220 of the
  252 cases. Untried: the path variant at `M = 1331`, ends by the length modulo
  4, several base sequences per length.
- Even a glue covering every case would be a necessary condition only: the
  nice-pair condition between `N` and `N + 1`, and the top-level closing glue,
  are not modelled yet.
- Base cases up to `M * 801`. At the current pace, 7.5 s per value on average
  but up to two minutes near `n = 2000`, a faster search will be needed.

Reproduce:

```console
scripts/nice-pairs.py search 3 1500
scripts/nice-pairs.py sweep 3 473 2000 sweep.csv 10
scripts/nice-pairs.py cycles 3 473 800 cycles/ cycles.csv
scripts/gerbicz-glue.py 2 7 8 3          # squares, M = 49: all feasible
scripts/gerbicz-glue.py 3 5 7 26         # cubes, M = 125: all blocked
scripts/gerbicz-glue.py 3 11 7 26 111    # cubes, M = 1331, every 111th residue
scripts/gerbicz-glue.py 3 5 1 4 --start 2   # path variant, M = 125: 220 of 252
```
