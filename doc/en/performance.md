# What was tried, measured, kept and dropped

This document records optimisation attempts on the search and patch pipeline:
the hypothesis, the measurement, and the verdict — including for the ones that
failed. Four plausible ideas were abandoned because the numbers contradicted
them — one of them implemented twice before the second measurement agreed with
the first — and without a record someone will re-derive each of them.

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
in 57 passes, while every restart cost a fruitless rescan of both direct
phases.

| `G_6(15748961)`, per absorbed satellite | |
|---|---|
| one repair per pass | 52.1 s — of which 9.8 s and 7.4 s of fruitless rescans |
| repair until exhausted | **27.3 s** |

What the change removes is a fixed cost of about seventeen seconds, not a
proportion. A four-minute sample taken just after a batched `2+1` phase gave
1.74×, which was too generous: it compared a fresh state, where the easy
satellites are still available, against a run already deep into the hard ones.
Over hours the ratio settles lower and keeps falling, because the repair itself
grows more expensive as the easy satellites are consumed — 43 s per repair at
the start of one run, 48 s five windows later. A fixed saving against a growing
cost is a shrinking ratio.

### 2.5 Deciding repair candidates by traversal

The search validated every closing candidate by calling
`AlternatingRepairMove::apply`, which rebuilds a `Factor` and sorts its edge
ids. Instrumenting the k = 5 factor showed **188 such calls for 27 repairs**,
seven per repair, and on `G_6` each one sorts 15.7 million edge ids. Roughly
half of a forty-five second repair was spent answering the same question over
and over — the same defect fixed in 2.1 for `three-opt-21`, but buried inside
the depth-first search.

A candidate re-links a handful of vertices, so the outcome is decided by
walking the cycle it would produce: follow the giant, consult a small override
table for the re-linked vertices, and check that the walk covers the giant and
the satellite exactly once.

| | |
|---|---|
| `G_5(490463)` full patch, before | 15.7 / 13.7 / 12.4 s |
| `G_5(490463)` full patch, after | **4.7 / 5.4 / 4.9 s** — 2.8× |
| `G_6` repair, before | 27.3 s per satellite |
| `G_6` repair, after | **16.4 s** per satellite — 1.67× |

The gain is smaller at `k = 6` because the walk follows the giant: 15.7 million
vertices per candidate against 490 thousand at `k = 5`. What was free becomes
visible — but still far below the rebuild it replaces.

A debug assertion compares the traversal against a full apply on every
candidate, and a debug run of the whole 27-repair k = 5 trajectory passes it on
all 188.

Cumulatively, 2.4 and 2.5 take the `G_6` repair phase from 52.1 s to 16.4 s per
absorbed satellite, **3.2×**.

---

### 2.6 The lever was not in the hot path

Everything above is throughput work, and it earned its place: without the
batched `three-opt-21` the `k = 6` factor would still be sitting at eighteen
thousand cycles. But none of it produced the witness.

After all of it, the patch ran about thirty-four hours and stopped at a local
fixed point: seven cycles, 116 vertices outside a giant of 15 748 845. No
alternating repair existed at depth 4, and proving that took 29 seconds.

Raising the depth closed the factor in two minutes.

| repair depth | outcome | time |
|---|---|---|
| 4 | fixed point at 7 cycles | ~34 h of patching |
| 6 | 5 repairs, down to 2 cycles | 78 s |
| 9 | 1 repair, one cycle | 21 s |

Two runs from different fixed points — 7 cycles and 9 — converged on the same
shape at depth 6, one giant and a single satellite of ten vertices, and both
closed at depth 9 on distinct valid cycles.

`ALTERNATING_REPAIR_MAX_DEPTH` was a compile-time constant when this work
started. Exposing it as a flag was logged as a fifteen-minute convenience,
somewhere below the batching in priority. It was the decisive parameter.

The lesson is not that profiling was wasted — it was necessary to reach the
fixed point at all. It is that a search bound is a parameter like any other,
and leaving one hard-coded hides it from exactly the kind of measurement the
rest of this document applies everywhere else.

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

### 3.3 An incrementally maintained factor

A `WorkingFactor` was written — selected edges, per-vertex neighbours, cycle
membership, satellite list, all updated in place, with a full verification at
each checkpoint to catch drift. The aim was to stop rebuilding a `Factor` for
every atomic move.

**Written, then dropped without being integrated.** Measuring first showed that
the seven rebuilds per repair happen *inside the search*, validating candidates,
not when applying the chosen move. An incremental factor would have removed
about eight seconds of a forty-five second repair; the traversal test in 2.5
removed roughly half of it for a fraction of the risk and no new structure.

What such a structure would still buy is visible in the residue: of the sixteen
seconds a repair now costs on `G_6`, about five are a decomposition and four a
final apply. That is another factor of two, for considerably more machinery. The
`FactorView` trait introduced in 2.5 is the seam it would attach to.

The module was deleted rather than left exported and unused: dead public API is
worse than none, and this document records the design.

### 3.4 Encoding witnesses by neighbour index

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

## 5. How the cost grows

End-to-end time to a Hamiltonian cycle at the minimal `n`, one run each,
`--max-depth=9`:

| k | n_min | time | n ratio | time ratio | local exponent |
|---|---|---|---|---|---|
| 2 | 32 | 0.008 s | — | — | — |
| 3 | 473 | 0.003 s | 14.8 | 0.4 | — |
| 4 | 9 641 | 0.035 s | 20.4 | 12 | 0.81 |
| 5 | 246 877 | 7.1 s | 25.6 | 203 | 1.64 |
| 6 | 15 748 961 | ~27 h 45 | 63.8 | 14 052 | 2.30 |

`k = 2` and `k = 3` are below the resolution of the measurement — process
startup dominates, and `k = 3` comes out faster than `k = 2` because its raw
factor closes in one tour where `k = 2` needs a patch. Only `k = 4, 5, 6`
carry information.

**In `n` the cost is roughly quadratic.** Least squares on those three points:

```text
time ~ 3e-10 * n^2.02   seconds
```

which reproduces `k = 5` to within a factor of 3.3. That is the useful shape:
polynomial in `n`, not exponential. For scale, an `n^n` law would put `k = 6`
at `10^113000000` seconds against the 10^5 measured.

**But the exponent is climbing** — 0.81, 1.64, 2.30. Three points cannot say
whether it settles or whether this is the approach to the exponential regime a
SAT solver is entitled to. `n^2` is the trend observed, not a law established.

**The cross-`k` fit is optimistic.** Each row is the *first* admissible `n` of
its island, and island starts are the easiest instances in their island. Within
a single `k` the exponent is worse: `G_5(490463)` costs 51.8 s against 7.1 s
for `G_5(246877)`, so 1.99 times the vertices for 7.3 times the time — an
exponent of 2.89.

**The explosion lives in `k`, not in `n`.** From
[`islands.md`](islands.md), the first admissible `n` is `(m+1)^k - m^k/2` with
`m` the binding self-loop base, and `m` grows like `2k + O(1)`. So

```text
log n_min(k) ~ k log 2k
```

Composing the two gives the real shape of the wall:

```text
log(time) ~ 2 k log 2k
```

Cost polynomial in the instance, instance exponential in `k log k`. Each
increment of `k` has multiplied the total by 10^1 to 10^4 so far, and nothing
about the algorithms explains it: the instance is simply growing that fast.

**Projection.** The same regression puts `k = 7`, at `n = 587 761 723`, near

```text
1.5e8 s  ~  5 years
```

with an uncertainty of at least an order of magnitude in each direction. The
figure is academic: by section 6 the memory floor at `k = 7` is 89 GB against
60 GB of RAM, so memory refuses the instance long before time does.

---

## 6. Peak memory

Predictable only in its floor. The static cost — graph plus base formula — is
computable exactly; the solver's learned-clause database is not.

The graph costs 32 bytes per edge and 8 per vertex, which is the representation
in `core::graph`. Calibrating the formula against the three measured floors
gives 3.56 bytes per literal, consistent with the 4 bytes CaDiCaL stores per
literal, and reproduces all three to two digits:

```text
floor = 32 * edges + 8 * n + 3.56 * literals
```

| | floor | observed | ratio |
|---|---|---|---|
| `G_5(246877)` | 0.03 GB | 0.33 GB | 11.2 |
| `G_5(490463)` | 0.07 GB | 0.93 GB | 12.6 |
| `G_6(15748961)` | 2.38 GB | 9.88 GB | 4.2 |
| `G_7(587761723)` | **89 GB** | — | — |

The `k = 7` row is the arithmetic, not a measurement: 35.7 GB of graph and
53.7 GB of formula over 15.1 billion literals. It is the reason `k = 7` is out
of reach on a 60 GB machine regardless of the time projection in section 5 —
the floor alone exceeds physical memory by half, before a single learned clause.
At the ratio observed on `G_6` the peak would be near 375 GB.

A guard should refuse when the floor exceeds available memory, since that is
arithmetic rather than prediction, and warn when five times the floor does.
Not implemented.
