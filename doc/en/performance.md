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
around 1.7. xz on the plain decimal list achieves 10.7 bits per vertex,
measured across the five published witnesses — six times the floor. An
encoding that stores which incident edge was taken would close most of that
gap: the `k = 7` witness would drop from 920 MB to about 126 MB.

**First dropped on a wrong premise.** The reason given here was that decoding
such a witness requires rebuilding the graph, 33 GB at `k = 7`. It does not.
The neighbours of `v` in `G_k(n)` are the values `b^k - v` with
`v < b^k <= v + n` and `b^k != 2v`: a formula, no graph.

The simplest such encoding stores the base itself: starting from `v = 1`, the
next vertex is `b^k - v`, one base per line, `n - 1` lines. Decoding is that one
subtraction, and every sum is a `k`-th power by construction, so a verifier
only has to check that the values are `1..n`, each once, and that the last one
plus 1 is a `k`-th power. Measured on the published witnesses, xz on both:

| k | plain list | bases, text | bases, one byte each |
|---|---|---|---|
| 3 | 852 B | 168 B | 152 B |
| 4 | 10 300 B | 568 B | 484 B |
| 5 | 258 744 B | 7 524 B | 5 740 B |
| 6 | 24 744 912 B | 1 576 820 B | 1 470 388 B |

15.7 times smaller than the plain list at `k = 6`, and up to 45 times at
`k = 5`, far below the `log2(average degree)` floor above: long runs of
repeated bases compress, forced chains alternating between two powers. The
rate depends on the cycle: an LKH cycle of `G_5(246877)` compresses half as
well as the published SAT one. Bases written as text cost only 7 % more than
raw bytes, so the text form keeps the witness human-readable. Relative ranks
among usable bases, the original idea, compress worse than absolute bases.

The plain list keeps its role: twenty lines of any language verify it with no
encoding to understand. The two are complementary, and the bundle format of the
next repository publishes both, as parallel trees.

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
untried lever. Section 7 measures a different route: handing the whole search,
connectivity included, to a dedicated Hamiltonian cycle heuristic.

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

The graph alone does fit, and that much was measured: `pc-scan 7 587761723`
builds it, runs the full preflight and reports `CANDIDATE`, so the first
admissible `n` at `k = 7` survives every implemented obstruction. Run under
`systemd-run --user -P --wait`, which reports the cgroup peaks:

| | estimate | RAM peak | swap peak | wall | CPU |
|---|---|---|---|---|---|
| `pc-scan 7 587761723` | 35.7 GB | 56 GB | 6.9 GB | 7 min 55 | 7 min 52 |

The estimate is the graph term of the floor, which is also what `pc-scan`
prints in its warning. The preflight needs about 1.6 to 1.8 times that,
depending on whether the two peaks coincide, and on a 60 GB machine it only
completes by spilling to swap. The swap costs little time — CPU time is within
three seconds of wall time — but it leaves no headroom. `pc-search` runs the
same preflight before its first tour, so a `k = 7` search on this machine
would pass through this peak before reaching the formula at all.

A guard should refuse when the floor exceeds available memory, since that is
arithmetic rather than prediction, and warn when five times the floor does.
Not implemented.

---

## 7. An external solver: LKH-3

**Hypothesis.** The pipeline solves a 2-factor with SAT, then repairs it into
one cycle. LKH-3, Helsgaun's Lin-Kernighan implementation, searches directly
for a Hamiltonian cycle: given the graph as a TSPLIB `TYPE : HCP` problem, it
looks for a tour of cost 0, every non-edge costing 1. It also accepts edges
that every tour must use. Whether it beats SAT plus repair is a measurement,
not something its reputation settles.

**Setup.** LKH-3.0.14, built from source with its own Makefile, which compiles
with `-O3 -march=native`; the measurements below use its default
`TWO_LEVEL_TREE` unless stated, and the build question is taken up further
down. `G_k(n)` is
exported after the preflight: the edges degree-two propagation forces go to
`FIXED_EDGES_SECTION`, the ones it forbids are left out. `pc-tsplib export`
does this; at `k = 6` it runs in 11 s with a 1.5 GB peak and writes a 552 MB
problem, and `pc-tsplib tour` turns the tour LKH returns into a checked,
canonical `cycle.txt`. Parameters, which `pc-tsplib` writes next to the
problem:

```text
SUBGRADIENT = NO
OPTIMUM = 0
STOP_AT_OPTIMUM = YES
RUNS = 1
OUTPUT_TOUR_FILE = <name>.$.tour
```

**Measured, one run each** — two for `G_5` — so the timings carry the 25% noise
of section 1 rather than the paired-run standard. The gap at `k = 6` is far
outside it.

| | fixed | forbidden | trials | LKH | SAT + repair |
|---|---|---|---|---|---|
| `G_5(246877)` | 198 205 (53%) | 60 195 | 2 | 4.6 to 4.8 s | 7.1 s |
| `G_6(15748961)` | 9 756 659 (37%) | 2 480 896 | 2 | **41 min** | ~27 h 45 |

At `k = 6`: 25 s of preprocessing, trial 1 ended at cost 3 after 1 411 s —
three non-edges, so three paths covering every vertex — and trial 2 reached
cost 0 at 2 409 s. The cgroup peak was 9.5 GB, on one core, against 9.88 GB for
`pc-search`.

Both cycles pass both independent verifiers, and both **differ from the
published witnesses**: `G_5(246877)` and `G_6(15748961)` each have a second,
independently found Hamiltonian cycle. They are not published. As controls,
LKH finds the unique Hamiltonian cycle of `G_2(32)` instantly, and on
`G_2(31)` it stops at cost 1, as it must.

**The trap.** With LKH's defaults the `G_5` run printed nothing for ten minutes
and was stopped. For `HCP` problems LKH always runs its subgradient ascent,
here `n/2` iterations over sparse 1-trees, to learn node penalties. With every
edge of the graph costing 0 there is nothing to learn. `SUBGRADIENT = NO`
brought the run to 4.8 s.

**`OUTPUT_TOUR_FILE`.** LKH writes `TOUR_FILE` only at the end of a run.
`OUTPUT_TOUR_FILE` is rewritten after every trial that improves the best tour,
`$` in the name standing for its cost, so a run stopped early still leaves its
cost-3 tour behind — the starting point a hybrid with the alternating repair
would need.

**Compiler flags do nothing; the tour structure does.** Seven builds on
`G_5(490463)`, three interleaved runs each, median wall time:

| build | median | |
|---|---|---|
| gcc `-O3 -march=native`, LKH's default | 12.56 s | 1.00× |
| gcc + LTO | 12.48 s | 1.01× |
| gcc + PGO + LTO | 12.35 s | 1.02× |
| clang | 12.79 s | 0.98× |
| clang + LTO | 13.28 s | 0.95× |
| clang + PGO + LTO | 12.52 s | 1.00× |
| gcc + `THREE_LEVEL_TREE` | **8.26 s** | **1.52×** |

The six two-level builds return the same tour byte for byte, so they follow
the same search path and the comparison is of code speed alone: within noise.
The profiles were trained on `G_5(246877)` and `G_5(600000)`, not on the
instance timed. One trap: gcc names its profile files after the build
directory, so a profile-use build in a fresh directory silently finds none.

`THREE_LEVEL_TREE` is LKH's compile-time switch for large instances. It keeps
the tour as a three-level list, so a flip costs about `n^(1/3)` instead of
`n^(1/2)`. It also changes the search path — a different, equally valid tour —
so its gain mixes faster moves with a different trajectory:

| | two-level | three-level |
|---|---|---|
| `G_5(246877)` | 2 trials, 4.6 s | 11 trials, 11.2 to 11.6 s |
| `G_5(490463)` | 12.6 s | 8.3 s |
| `G_5(2000000)`, one run | 278 s | 258 s |
| `G_6(15748961)`, one run | 2 452 s | **719 s** |

At `k = 6` trial 1 ended at cost 2 after 372 s and trial 2 reached cost 0 at
675 s: each trial 3.3 to 3.8 times faster, the advantage growing with `n` as
the theory says. The cycle passes both verifiers and is a third one, distinct
from the published witness and from the two-level run. At `k = 5` the trials
are twice as fast but eleven of them were needed: **the number of trials, two
to eleven so far, is the real unknown in any projection.** The installed LKH is
the three-level build.

`G_5(2000000)` also shows that size is not the whole story: only 5% of its
edges are fixed, against 37% at `k = 6`, and one trial costs 273 s where
`G_6`, eight times larger, needs about 340. The first value of an island is
not a typical instance.

**Memory is still the wall.** About 600 bytes per vertex at `k = 6`, measured
on the two-level build. Linear extrapolation to `n = 587 761 723` gives about
355 GB, before the problem file of roughly 20 GB is even written.

LKH keeps every vertex, including the interior of the fixed chains, which have
nothing left to choose. At `k = 6` the fixed edges form 2 821 104 chains with
6 935 555 interior vertices; replacing each chain by its two ends joined by a
fixed edge leaves 8 813 406 vertices, 56% of `n`, as an ordinary TSP.
`pc-tsplib export --contract` does this, drops the edges that would close a
chain on itself — the rule propagation lacks, one or two per instance — and
writes the chains to a file that `pc-tsplib tour` uses to put the interior
back. Three-level build, one run each:

| | nodes | uncontracted | contracted |
|---|---|---|---|
| `G_5(246877)` | 35% | 11 trials, 11.2 s | 4 trials, 2.8 s |
| `G_5(490463)` | 63% | 1 trial, 8.3 s | 2 trials, 15.1 s |
| `G_6(15748961)` | 56% | 2 trials, 727 s, **9.9 GB** | 1 trial, 351 s, **5.7 GB** |

Memory falls with the node count, 58% of the uncontracted peak for 56% of the
nodes, and the problem file from 552 MB to 307 MB plus a 106 MB chains file.
Time per trial does not: 327 s contracted against 310 to 375 s uncontracted at
`k = 6`. The contracted graph is denser, and the faster `G_6` run owes its
speed to needing one trial instead of two. The cycle passes both verifiers and
is a fourth, distinct from the three before it.

**Projection for `k = 7`**, arithmetic on two points rather than a measurement.
On the three-level build a trial grows from 0.97 s at `k = 5` to about 340 s at
`k = 6`, as `n^1.41`:

| | one trial | 2 trials | 11 trials | memory |
|---|---|---|---|---|
| two-level | ~3.7 days | ~7 days | — | ~355 GB |
| three-level | ~15 h | ~31 h | ~7 days | ~370 GB |
| three-level, chains contracted | ~15 h | ~31 h | ~7 days | ~215 GB |

The contracted row is corrected by the measurement above: the 7 h per trial
first projected assumed the time would fall with the node count, and it does
not. Its memory scales the 5.7 GB of `G_6` linearly in `n`. The three-level
build peaks at 9.9 GB uncontracted, against 9.5 GB for the two-level one.
LKH moves the `k = 7` bottleneck from time to memory; it does not remove it.

**Measured with posum 1.0.0, 2026-10-19.** `posum search 7 587761723` runs
degree-two propagation, with the chain-closing rule, and the contraction before
its memory guard refuses LKH. On this machine:

| `G_7(587761723)` | value |
|---|---|
| edges | 968 075 013 |
| forced | 393 598 028 (40.7 %) |
| forbidden | 106 523 041 (11.0 %) |
| free | 467 953 944 (48.3 %) |
| chains contracted | 103 151 293 |
| nodes left for LKH | 297 314 988 (50.6 % of `n`) |
| propagation | 138 s, about 17.6 GB by posum's estimate |
| LKH, by the guard's estimate | 190.1 GB, against 39.9 GB available |

No obstruction: propagation completes, so the first admissible `n` at `k = 7`
survives the argument that refutes `G_2(31)` up to its case analysis, as the
conjecture predicts. Against `G_6`, the forced share rises from 37 % to 40.7 %
and the nodes left fall from 56 % to 50.6 % of `n`. The memory estimate, now
from the exact node count — 600 bytes per node for LKH plus 20 bytes per vertex
for the contraction — replaces the 215 GB extrapolated above. Ways forward, none
tried at first: a machine with 256 GB, LKH's partitioning into subproblems, or
fewer bytes per node through LKH's candidate parameters.

**Where LKH's memory goes, measured on `G_6`, 2026-10-19.** On the contracted
`G_6(15748961)`, 8 813 404 nodes, the peak resident memory after preprocessing
and 90 s of the first trial is the same with `MAX_CANDIDATES = 5`, posum's
setting, and with 3: 5 764 MB, 685 bytes per node. For a problem given as an
edge list LKH takes every edge as a candidate — 3.8 per node on average, up to
14 — so the parameter has no effect, and candidates cost only about 75 bytes
per node at 16 bytes each. Most of the rest is LKH's node structure:
`sizeof(Node)` is 528 bytes in LKH-3.0.14, with dozens of fields serving other
problem types — coordinates in double precision, demands, depots, time windows,
colours. Removing them would mean rewriting much of LKH's source, and would
still leave about 450 bytes per node, some 145 GB at `k = 7`. Partitioning does
not help either: LKH allocates the whole node array, about 157 GB at `k = 7`,
before it cuts the problem into subproblems; it saves time, not memory. Left
for `k = 7`: a machine with 192 GB or more, or a decomposition done by posum
itself — Hamiltonian paths with fixed ends found by separate LKH processes and
glued together, in the spirit of Gerbicz's construction.

**LKH-2, measured on `G_6`, 2026-10-19.** LKH-2.0.11, the earlier line with
fewer problem types (archive SHA-256 `95535d19a3051716a17d65833570884603ef24e1cb11821803547b6052b21c0a`),
reads the same HCP problems with fixed edges. Built with the three-level tree,
on the same contracted `G_6(15748961)` and seed 1:

| | LKH-3.0.14 | LKH-2.0.11 |
|---|---|---|
| `sizeof(Node)` | 528 bytes | 384 bytes |
| peak resident memory | 5.8 to 6.0 GB, 685 bytes per node | **4.7 GB, 541 bytes per node** |
| trials | 1 | 1 |
| first trial | 373 s | **327 s** |
| LKH in all | about 6 min 30 | 5 min 51 |

LKH-2 reports a tour of cost 0, not yet expanded and checked by the
verifiers. Memory falls by 21 %, as the smaller node structure predicts; time
is no worse on this single run. At `k = 7` the estimate becomes about 173 GB
instead of 190: still four times this machine, but under 192 GB. LKH-2 stays
an option for posum — a build script for 2.0.11 and `version = "2.0.11"` in the
provenance — worth adopting only for a `k = 7` search on a larger machine.

**Concorde's Chained Lin-Kernighan, tried on `G_6` and discarded, 2026-10-19.**
`linkern`, from Concorde 03.12.19 (archive SHA-256
`c3650a59c8d57e0a00e81c1288b994a99c5aa03e5d96a314834c2d8f9505c724`), handled
25 million geometric cities on a 4 GB machine (Applegate, Cook and Rohe, 2003),
far leaner than LKH, and reads sparse graphs (norm `SPARSE`: pairs not listed
get a huge default length). The contracted `G_6` was given as an edge list,
chain links at cost 0 and free edges at cost 1, so that a tour of length
`nodes - links = 5 992 300` is a Hamiltonian cycle using every chain; that
length was the stop bound.

| starting tour | stopped after | outcome | memory |
|---|---|---|---|
| Quick-Borůvka, the default | 67 min | starting tour never finished | 815 MB at the stop |
| random | 43 min | first Lin-Kernighan descent unfinished | about 2 GB |
| greedy, 3.1 % jumps | 1 h 42 | first descent unfinished | about 2 GB, 225 bytes per node |
| *LKH-3, for comparison* | *about 6 min 30* | *cycle found, one trial* | *685 bytes per node* |

The greedy starting tour, built in 33 s by a Python sweep over the edge list,
follows each chain link first, then a free edge to the unvisited neighbour
with the fewest unvisited neighbours, and jumps to the least unvisited node
only when stuck: 275 844 jumps for 8 813 404 nodes. Even from there `linkern`
did not finish one descent in 1 h 42. Its economy is real — about a third of
LKH's memory — but its search does not repair these sparse instances in
useful time: repairing a jump means finding a sequence of exchanges that lands
only on graph edges, where LKH's guided 5-opt moves succeed; the sparse norm,
which scans a node's neighbour list for every length it is asked, does not
help either. Discarded. The greedy tour remains a cheap start, 3 % of jumps,
for LKH or for a decomposition.

**A finding on the way.** Comparing the exported fixed edges with a hand
contraction of `G_2(32)` showed that degree-two propagation never forbids an
edge joining the two ends of a forced path. It detects the short cycle such an
edge would close only afterwards, as `ForcedSubcycle`. On `G_2(32)` the missing
rule leaves `6–19` allowed, and with it two forced edges, `17–19` and `1–8`, go
undetected. How many edges the rule would add at `k = 5` and `k = 6` has not
been measured. posum 1.0.0 implements the rule: at `G_6(15748961)` it
leaves 8 813 404 nodes after contraction instead of 8 813 406.

**Verdict: open.** Two instances, one run each, and a 40-fold gain where the
pipeline is slowest — 140-fold with the three-level build. `pc-tsplib` makes it reproducible, but `pc-search`
remains the default: LKH is an external program, and it would not change the
`k = 7` picture without a machine several times larger.
