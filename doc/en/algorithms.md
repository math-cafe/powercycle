# powercycle algorithms

## 1. The object

For integers `k >= 2` and `n >= 1`, the power-sum graph `G_k(n)` has vertex set
`{1, ..., n}` and an undirected edge `{x, y}`, with `x != y`, whenever

```text
x + y = a^k        for some integer a >= 2
```

A Hamiltonian cycle of `G_k(n)` is an arrangement of `1..n` around a circle whose
adjacent sums are all perfect `k`-th powers.

These graphs are extremely sparse, and that sparsity dictates every choice below.
`G_5(246877)` has 374622 edges for 246877 vertices: average degree 3.04, minimum
degree 2, maximum degree 10, and 110244 vertices — 45% — of degree exactly two.
Only twelve fifth powers fall in the usable range, so the graph is a union of
twelve matching-like layers.

Edges are built in a deterministic order — increasing power, then increasing
first endpoint — and the resulting index is the `EdgeId`. Every persistent
artifact depends on that order being stable.

## 2. Necessary conditions

Before any search, `powercycle-search::preflight` tries to prove that no
Hamiltonian cycle can exist. Each check is a genuine necessary condition, so a
failure is a proof of impossibility; passing them all proves nothing.

### 2.1 Graph-free degree check

`quick_degree_obstruction(k, n)` decides the minimum-degree condition **without
constructing the graph**.

For a fixed power `p`, vertex `v` is incident to the edge that `p` generates
exactly when

```text
p - n <= v <= p - 1
```

with the single exception of the self-loop position `v = p / 2`, which is not an
edge. Each power therefore contributes one interval and at most one puncture, so
the degree function over `1..n` is piecewise constant with `O(number of powers)`
breakpoints — about `log_k(2n)` of them. A sweep over those events replaces an
`O(n)` construction.

This matters because the number of relevant powers grows logarithmically while
`n` grows: scanning `n = 1..246876` at `k = 5` takes 0.24 s and builds no graph
at all.

### 2.2 Full preflight

`hamiltonian_preflight` needs the constructed graph and applies, in increasing
order of cost:

1. **Minimum degree.** A Hamiltonian cycle uses exactly two edges at every
   vertex, so any vertex of degree below two is fatal.
2. **Connectivity.**
3. **Articulation vertex.** A Hamiltonian cycle is 2-connected, so any cut vertex
   is fatal.
4. **Degree-two propagation.** A vertex of degree exactly two forces both of its
   edges. A vertex with two forced edges forbids its remaining ones, which can
   create new degree-two vertices, and so on. The cascade fails if some vertex
   ends with more than two forced edges or fewer than two available ones.
5. **Forced subcycle.** If the forced edges close a cycle that does not span all
   vertices, no Hamiltonian cycle contains them.

The propagation is not only a filter. On `G_5(246877)` it determines 198205
forced and 60195 forbidden edges — 69% of all edges — and on `G_6(15748961)`,
47%. That information is currently discarded after the check.

## 3. Relaxation to a 2-factor

Encoding a Hamiltonian cycle directly in SAT needs position variables and does
not scale. Instead the problem is relaxed.

A **2-factor** is a subset of edges giving every vertex degree exactly two. It is
a disjoint union of cycles covering all vertices, and it is a Hamiltonian cycle
exactly when it has **one** cycle.

The SAT model uses one variable per edge, with variable `i` corresponding to
`EdgeId(i)`, and encodes only the degree constraint. For a vertex of degree `d`
with incident literals `e_1..e_d`:

- **at least two**: `d` clauses of width `d - 1`, one per omitted literal — if
  two of the `e_i` were false, some clause omitting a third would be empty;
- **at most two**: `C(d, 3)` clauses `(¬e_i ∨ ¬e_j ∨ ¬e_l)` over all triples.

A vertex of degree below two yields the empty clause, so the formula is
immediately unsatisfiable. On `G_5(246877)` this gives 374622 variables and
1598168 clauses; on `G_6(15748961)`, 26281979 variables and 132356257 clauses.

Solving uses CaDiCaL incrementally through `rustsat`, so cuts accumulate across
rounds without rebuilding the formula. A cooperative terminator lets Ctrl-C
interrupt a solve.

## 4. Subtour elimination

A satisfying model is a 2-factor, generally with many cycles. Two families of cut
are available, both skipping the largest cycle — the **giant** — since forbidding
every cycle including the giant would be wasteful.

**Cycle-breaking cut.** For a non-giant cycle `C`, the clause `¬(all edges of C)`
forbids exactly that cycle. Width is `|C|`. Cheap and weak: it rules out one
configuration.

**Boundary cut.** For a non-giant cycle `C`, the disjunction of all edges with
exactly one endpoint in `C` forces the solver to cross the boundary. This is the
real subtour-elimination constraint and is far stronger, but its width is the
size of the edge boundary, which can reach tens of thousands.

Both are budgeted per round by `max_clauses` and `max_literals`, defaulting to
256 and 50000, and candidates are sorted by increasing width so narrow clauses —
which propagate better and cost less to keep — are added first. Boundary cuts add
a priority class: clauses of width at most `priority_max_width`, default 8, are
selected ahead of all wider ones.

`add_factor_blocking_clause` excludes exactly one 2-factor, by negating all of
its selected edges. This is sound only because the degree constraints force every
2-factor to have exactly `n` edges.

## 5. Repair

Waiting for SAT alone to converge is hopeless: there are exponentially many
subtours to eliminate. So each raw factor is pushed towards a single cycle
combinatorially.

Vocabulary: the **giant** is the largest cycle, every other cycle is a
**satellite**, and the **residual** is `n` minus the giant's size.

### 5.1 `three-opt-111`

Removes one selected edge from each of three distinct cycles and adds three
edges reconnecting the resulting paths into one. Decreases the cycle count by
exactly two.

### 5.2 `three-opt-21`

Removes two selected edges from the giant and one from a satellite, then adds
three edges. Absorbs that satellite into the giant, decreasing the cycle count by
exactly one.

### 5.3 Alternating repair

Used when no direct `2+1` move exists.

It starts from a *quasi* `2+1`: remove one satellite edge and two giant edges,
add two satellite-to-giant edges. Degrees now balance everywhere except at two
vertices of the giant, which are left with degree one — the two **deficits**.

One deficit is then walked through the giant by alternating:

- add an edge that was **unselected** in the original factor;
- remove an edge that was **selected** in the original factor.

Each such step transfers the deficit to a new vertex. When the moving deficit
becomes adjacent to the fixed one, a final unselected edge closes the repair.
`depth` is the number of transfer steps; depth zero would merely rediscover an
ordinary `2+1` move and is not returned.

Every candidate is validated before being accepted: degree two must hold at every
vertex, the cycle count must drop by exactly one, and the giant must grow by
exactly the size of the targeted satellite. A candidate that closes the degree
deficit but absorbs the wrong thing is rejected and the search continues.

**Completeness.** The search is exhaustive within its move class at the given
depth — it enumerates every satellite, every satellite edge, both orientations,
every pair of satellite-to-giant edges and every pair of giant edges to remove,
then explores all alternating extensions. The class itself is narrower than "all
depth-`d` alternating repairs":

- only one deficit moves; both assignments are tried, so one-sided growth from
  either end is covered, but not simultaneous growth from both;
- the alternating path is vertex-simple — any vertex already touched by the
  partial move is skipped;
- the path may not route through the fixed deficit, which is reachable only by
  the closing edge;
- exactly one satellite is absorbed per move;
- the giant is the only host, so satellite-to-satellite merges are out of scope;
- the first valid move in `EdgeId` order is returned, not the best.

So "no repair at depth 4" means "no single-satellite, single-moving-deficit,
vertex-simple repair at depth at most 4". Cost grows by roughly a factor of the
squared average degree per level — about 11 for these graphs — so raising the
depth by one or two is affordable when a single small satellite is blocking.

### 5.4 Closure loop

`pc-search` alternates greedy `three-opt-111`, greedy `three-opt-21` and
alternating repair until the factor is Hamiltonian or no operator applies. The
repair depth is `ALTERNATING_REPAIR_MAX_DEPTH`, currently a compile-time constant
of 4; `pc-patch` exposes it as `--max-depth`.

The witness histories show how varied the closure can be. `G_5(246877)` closed
with two `1+1+1` moves and nothing else. `G_4(9641)` needed one `2+1` followed by
four alternating repairs at depths 2, 4, 2 and 2. `G_5(490463)` reached two
cycles and then required a single depth-4 repair to absorb a 25-vertex satellite.

## 6. Canonical export

A one-cycle factor is exported as a plain list of vertices, rotated so that
vertex `1` comes first and oriented so that the smaller of the two neighbours of
`1` comes second. The representation is therefore unique for a given cycle, which
is what makes the SHA-256 of the uncompressed file a meaningful identity.

## 7. Limits

The base SAT model is the scaling wall. `G_6(15748961)` builds in 19 s but its
132-million-clause model has not yet yielded a first solution. `G_7(587761723)`
would need roughly 968 million edges — about 36 GB for the graph alone, before
any solver — and is out of reach for this approach.

`pc-scan` inherits a related hazard: it builds the full graph for the first `n`
that survives the graph-free check, which at `k = 7` means attempting that same
36 GB allocation.
