# What was tried, measured, kept and dropped

This document records optimisation attempts on the search and patch pipeline:
the hypothesis, the measurement, and the verdict — including for the ones that
failed. Three plausible ideas were abandoned because the numbers contradicted
them, and without a record someone will re-derive each of them.

Every claim below is a measurement, not an estimate. Where a range is given it
is the spread of repeated runs.

---

## 1. Method

**Paired runs.** Single timings on this workload are unreliable: the same
binary on the same input has varied by 25% between consecutive runs. Every
verdict here rests on at least three alternating measurements, and one attempt
was wrongly judged a regression on a single sample before the paired
measurement reversed it.

**Witnesses as a regression test.** Any change to the operators can alter which
move is chosen and therefore produce a different, equally valid Hamiltonian
cycle. The published witnesses pin this: `k = 3`, `k = 4` and `k = 5` must
re-export byte-identical cycles. That caught nothing, but it is the reason
these changes could be made at all.

**Fixtures.** `pc-search --no-patch` captures a raw SAT factor once; every
patch experiment then runs from that file. On `G_6(15748961)` the solve costs
four hours and the factor is deterministic, so this turns a four-hour feedback
loop into seconds.

---

## 2. Kept

### 2.1 O(1) validity test for `three-opt-21`

The candidate search called `apply` on every candidate purely to test validity,
which sorts the whole edge list and decomposes twice — per candidate, most of
them rejected.

A candidate cuts the giant into two arcs and adds three edges. Whether those
reconnect into a single cycle depends only on how the two removed giant edges
are oriented relative to each other along the giant: consistently oriented, the
satellite path bridges one arc while the third added edge closes the other;
oppositely oriented, that third edge closes one arc on itself and two cycles
remain. Validity is therefore a comparison of two positions.

| | `G_5(490463)` full patch |
|---|---|
| apply per candidate | 23.3 · 20.6 · 18.1 s |
| orientation test | 13.8 · 12.0 · 11.7 s |
| | **1.65×** |

A debug assertion compares the predicate against a full apply on every
candidate; a debug run of the whole 63-move trajectory passes it.

### 2.2 Batched `three-opt-21`

Two moves whose giant vertices are disjoint do not disturb each other's arcs,
so one scan collects a whole independent set and applies it in a single
rebuild. Independence is argued rather than proved, so the batch is checked for
being a valid 2-factor with exactly one fewer cycle per move, and falls back to
a single move when it is not.

| | moves | duration |
|---|---|---|
| `G_5(490463)` phase, unbatched | 63 | 7.4 s |
| `G_5(490463)` phase, batched | 59 | **0.60 s** |
| `G_6(15748961)` phase, unbatched | — | **> 20 h, never finished** |
| `G_6(15748961)` phase, batched | 15 902 | **105 min** |

This is the single largest win in the pipeline. It is what turned `k = 6` from
unreachable into a computation that runs.

The `k = 6` checkpoint log shows the phase in two regimes: about 82 moves per
ten minutes at first — the cost of one rebuild per move, so batches failing
their check and falling back — then a jump to fourteen thousand as the batches
begin to compose.

### 2.3 Batched `three-opt-111`

Same recipe. A move merges three distinct cycles, so two moves compose exactly
when their triples are disjoint. Sharing a cycle does **not** work: the move
depends on traversing each of its three cycles whole, and a second cut destroys
that traversal. Reserving cycles rather than vertices is therefore conservative
but necessary.

| | moves | duration |
|---|---|---|
| `G_6(15748961)` phase, unbatched | 710 | 5343 s |
| `G_6(15748961)` phase, batched | 710 | **3398 s** |
| | | **1.57×** |

Modest, and the cycle-level reservation is why: the giant can serve only one
move per batch and it is the natural merge target, so each scan harvests few
moves while the fixed cost — a decomposition plus a 15.7-million-edge sort —
stays.

### 2.4 Exhausting the alternating repair

The pass loop applied one repair, then restarted from `1+1+1`, because a repair
changes the giant and may expose new direct moves. On `G_6` that happened once
in 57 passes.

| `G_6(15748961)`, per absorbed satellite | |
|---|---|
| one repair per pass | 52.1 s — of which 9.8 s and 7.4 s of fruitless rescans |
| repair until exhausted | **30.0 s** |
| | **1.74×** |

---

## 3. Dropped

### 3.1 Feeding propagated units to the SAT solver

The preflight's degree-two propagation determines 69% of the edges on
`G_5(246877)` and 47% on `G_6(15748961)`. Asserting all of it as unit clauses
looks like free information for the solver.

**It makes no difference.** Three paired runs of the first SAT tour on
`G_5(490463)`:

```text
             without      with
run 1        48.332 s     48.282 s
run 2        49.764 s     50.656 s
run 3        49.193 s     49.139 s
mean         49.096 s     49.359 s     +0.5%
```

Differences fall on both sides of zero, the mean is slightly worse, and the
factor produced is identical.

The reason is that these units are already implied by the base encoding, and by
*unit propagation* specifically. A vertex of degree two contributes
at-least-two clauses of width `d - 1 = 1`, which are literally the forced
units; the cascade then follows from the at-most-two clauses. CaDiCaL performs
this at decision level zero during preprocessing, so handing it the conclusion
saves nothing.

Kept behind `--propagated-units` so the experiment is not repeated. A solver
without preprocessing could behave otherwise.

### 3.2 Batching the alternating repair

Implemented and measured **twice**, and lost both times.

| | sequential | batched |
|---|---|---|
| `G_5(490463)` full patch | 9.6 · 9.6 · 10.5 s | 18.6 · 22.7 · 19.6 s |
| `G_6`, repairs in 4 min | 10 | 7 |

The second attempt was made deliberately, because the first verdict looked
scale-dependent: at `k = 5` a decomposition costs about ten milliseconds while
at `k = 6` it costs seconds, and there were far fewer satellites left to scan.
The calculus looked inverted. It was not.

The reason is not the cost of a decomposition. A sequential search takes the
first repairable satellite in canonical order and **leaves the rest**. Each
absorption grows the giant, which makes the remaining satellites easier, so
deferring the hard ones is an advantage. A batch insists on trying every
satellite, and proving that a satellite admits no repair costs a full seed
enumeration plus a bounded depth-first search. What the batch saves in rebuilds
it repays in impossibility proofs.

Bounding the scan by a run of consecutive failures helped — 21.0 s to 16.4 s at
`k = 5` — without changing the verdict.

This is the sharpest lesson of the three: the same idea that gave a twelvefold
gain on `three-opt-21` is a loss on the alternating repair, because the two
operators differ in what a *candidate* costs. Collecting candidates is nearly
free when validity is an O(1) test; it is expensive when each one runs a search.

### 3.3 Encoding witnesses by neighbour index

A cycle in these graphs carries about `log2(average degree)` bits per vertex,
around 1.7. zstd on the plain decimal list achieves 22 bits per vertex,
measured across the four published witnesses — thirteen times the floor. An
encoding that stores which incident edge was taken would close most of that
gap: the `k = 7` witness would drop from 1.6 GB to about 126 MB.

**Not done, deliberately.** Decoding such a witness requires rebuilding the
graph, which at `k = 7` is 33 GB. The plain list verifies with twenty lines of
any language and no data structure at all. That self-containedness is what
makes a witness citable, and it is worth more than a factor of thirteen.

---

## 4. Where the limits actually are

Computed analytically at the first admissible `n` for each `k`, without
building anything: the degree histogram of `G_k(n)` is piecewise constant with
`O(number of powers)` breakpoints.

| k | n | edges | graph | SAT clauses | witness (u64) |
|---|---|---|---|---|---|
| 5 | 246 877 | 374 622 | 13 MB | 1.6 M | 1.9 MB |
| 6 | 15 748 961 | 26 281 979 | 922 MB | 132 M | 120 MB |
| 7 | 587 761 723 | 968 075 013 | **33 GB** | 5.0 G | 4.4 GB |
| 8 | 25 022 859 361 | 40 804 654 361 | **1.4 TB** | 218 G | 186 GB |
| 9 | 1 197 518 052 567 | 1 937 542 828 752 | 65 TB | 10.8 T | 8.7 TB |

The graph is consistently about 7.5 times the witness, so **storage is the last
wall, not the first**. The order in practice is:

1. **the patch**, which was the wall at `k = 6` until the work above;
2. **the SAT solve**, 4 h 00 at `k = 6`, out of reach at `k = 7`;
3. **graph memory**, borderline at `k = 7`, impossible at `k = 8`;
4. **witness storage**, only at `k = 9`.

One observation that has not been acted on: finding a 2-factor is polynomial —
it reduces to perfect matching — so the four hours of SAT in tour one solve in
an NP-complete formalism a problem that is in P. NP-hardness only enters with
connectivity. Replacing the first tour with a matching algorithm is the largest
untried lever.

---

## 5. Peak memory

Predictable only in its floor. The static cost — graph plus base formula — is
computable exactly; the solver's learned-clause database is not.

| | floor | observed | ratio |
|---|---|---|---|
| `G_5(246877)` | 0.03 GB | 0.33 GB | 11.2 |
| `G_5(490463)` | 0.07 GB | 0.93 GB | 12.6 |
| `G_6(15748961)` | 2.38 GB | 9.88 GB | 4.2 |

A guard should refuse when the floor exceeds available memory, since that is
arithmetic rather than prediction, and warn when five times the floor does.
Not implemented.
