# Proving a power-sum graph is not Hamiltonian

Every rejection this project has ever produced came from one criterion: some
vertex has degree below two. That argument is elementary, its certificate is a
handful of integers, and it settles every `n` outside the admissible islands
described in [`islands.md`](islands.md).

Exactly one case is known where it is not enough. This document is about that
case, and about what a certificate can look like when the cheap argument runs
out.

---

## 1. The right object: `f_2`

A **2-factor** is a spanning subgraph in which every vertex has degree exactly
two. It is necessarily a disjoint union of cycles, and a Hamiltonian cycle is
precisely a 2-factor with one component. Define

```text
f_2(G) = min over 2-factors F of  (number of cycles in F)
```

Then, as soon as `G` has a 2-factor at all,

```text
G is Hamiltonian   <=>   f_2(G) = 1
```

This is the quantity the pipeline actually manipulates. The SAT model asks only
for degree two everywhere, so its first solution is a 2-factor; the
subtour-elimination cuts and every patch operator then reduce the cycle count.
`three-opt-111` takes `f_2` down by two per move, `three-opt-21` and the
alternating repair by one. The whole patch phase is a descent on `c(F)`, and
what it reports at each step is a **decreasing upper bound on `f_2`**.

Two consequences worth stating plainly.

**The descent never gives a lower bound.** If the patch stops at `r > 1`, we
know `f_2 <= r` and nothing more: some other 2-factor may do better.

**Minimising `c(F)` is NP-hard**, since it contains Hamiltonicity. Finding
*a* 2-factor, by contrast, is polynomial — it reduces to perfect matching.
The reformulation clarifies the problem; it does not make it easier.

---

## 2. The one hard case: `G_2(31)`

`31` is the first element of `I_2`, so the degree argument stops there. The
graph passes every cheap test:

- 31 vertices, 44 edges, degrees `2^10, 3^16, 4^5`;
- minimum degree 2, connected, no articulation vertex;
- no toughness witness `W` with `c(G - W) > |W|` was found for `|W| <= 7`
  (exhaustive at those sizes; larger `W` was not searched);
- Pósa fails at once — the sorted degrees give `d_1 = 2 > 1` but `d_2 = 2`, not
  `> 2` — which proves nothing, the condition being sufficient only;
- Dirac needs `delta >= 15.5`, Ore needs `d(u) + d(v) >= 31`, and since
  `Delta = 4` no pair reaches 8. The Bondy–Chvátal closure adds no edge at all,
  so `cl(G) = G` and that theorem is silent too.

The classical degree and density criteria are all designed for dense graphs.
Here they say nothing, in either direction.

---

## 3. The certificate

The obstruction is visible only in the 2-factors.

Ten vertices have degree exactly two — `16, 17, 18, 25, 26, 27, 28, 29, 30, 31`
— so both of their edges are forced in any 2-factor. Propagating that
(a vertex with two forced edges forbids its others, which creates new degree-two
vertices) gives a fixed point with

```text
23 forced edges,  6 forbidden,  15 still undecided
```

and no proper cycle closed yet.

At vertex 5 the choice becomes binary. Edge `5-31` is already forced, and
`5-20` is dead because 20 is saturated. Exactly two options remain:

**Branch `5-4`.** Propagation completes the 2-factor. Two cycles, of 17 and 14
vertices:

```text
(1, 15, 21, 28, 8, 17, 19, 30, 6, 10, 26, 23, 2, 14, 11, 25, 24)
(3, 13, 12, 4, 5, 31, 18, 7, 29, 20, 16, 9, 27, 22)
```

**Branch `5-11`.** Propagation completes again. Two cycles, of 20 and 11:

```text
(1, 15, 10, 26, 23, 2, 14, 22, 27, 9, 16, 20, 29, 7, 18, 31, 5, 11, 25, 24)
(3, 6, 30, 19, 17, 8, 28, 21, 4, 12, 13)
```

Both branches are complete and neither is a single cycle, so

```text
f_2(G_2(31)) = 2       and       G_2(31) is not Hamiltonian
```

Every consecutive sum above is a perfect square, closure included, and the two
cycles of each branch partition `1..31`.

An independent complete enumeration — branching on every undecided edge rather
than only on vertex 5 — confirms that `G_2(31)` has **exactly two 2-factors**,
both with two cycles. There is nothing else to check.

`a(2) = 32` follows: the degree bound allows 31, the enumeration rules it out,
and a witness exists at 32. None of this is new as a fact: `32` is the classical
answer to the square-sum problem, and Bert Dobbelaere noted in OEIS
[A071984](https://oeis.org/A071984) (2018) both the degree argument for
`n <= 30` and that `n = 31` can be ruled out by hand. What this section adds is
the argument written as a certificate: two branches, each closing on two
cycles.

### A proof by hand, every step arithmetic

The certificate above can be checked without a computer. Each step asks only
whether a sum is a square; the squares available are `4, 9, 16, 25, 36, 49`,
since sums stay below `2 * 31 = 62`.

**Step 1: the top vertices have degree two.** For `v` in `25..31`, the window
`(v, v + 31]` lies inside `(25, 62]`, which holds only 36 and 49; so `v` is
adjacent to exactly `36 - v` and `49 - v`. For 16 and 17 the window holds only
25 and 36. For 18 it holds 25, 36 and 49, but `36 = 18 + 18` is the excluded
loop. These ten vertices force twenty edges, two each:

```text
16: 9, 20     17: 8, 19     18: 7, 31     25: 11, 24    26: 10, 23
27: 9, 22     28: 8, 21     29: 7, 20     30: 6, 19     31: 5, 18
```

**Step 2: saturation.** Vertex 7 now has two forced edges (18, 29), so its
edges to 2 and 9 are forbidden; likewise 8 (17, 28) loses 1, 19 (17, 30) loses
6, and 20 (16, 29) loses 5.

**Step 3: two more forced vertices.** Vertex 2, adjacent to 7, 14 and 23, has
lost 7: its edges to 14 and 23 are forced. Then 23 (26, 2) is saturated and
loses 13, and 13, adjacent to 3, 12 and 23, keeps exactly 3 and 12.

**The forced structure.** The forced edges form five disjoint paths, and three
vertices are still free:

```text
B: 5-31-18-7-29-20-16-9-27-22      C: 6-30-19-17-8-28-21
D: 10-26-23-2-14      E: 11-25-24      A: 3-13-12      free: 1, 4, 15
```

Every interior vertex is settled. Each path end needs exactly one more edge,
each free vertex two, and the edges still possible are:

```text
5: 4, 11      22: 3, 14     6: 3, 10      21: 4, 15     10: 6, 15
14: 11, 22    11: 5, 14     24: 1, 12     3: 1, 6, 22   12: 4, 24
1: 3, 15, 24      4: 5, 12, 21      15: 1, 10, 21
```

No path can close on itself: the sums of its two ends, `27, 27, 24, 35, 15`,
are not squares.

**Step 4: two cases, at vertex 5.**

- *5 joins 11.* Then 14 must join 22, and 3 cannot use 22. Vertex 4 has lost 5,
  so it joins both 12 and 21. Then 24 must join 1, and 15, having lost 21,
  joins both 1 and 10. Vertex 1 is full (24, 15), so 3 joins 6. Everything is
  decided, and the edges close **two cycles**: paths B, D, E with 15 and 1 (20
  vertices), and paths A, C with 4 (11 vertices).
- *5 joins 4.* Then 11 must join 14, 14 is done, 22 must join 3, and 4 needs
  one more edge, to 12 or to 21.
  - *4 joins 21.* Then 12 must join 24, and 1 is left with a single possible
    neighbour, 15, since 3 and 24 are taken: impossible.
  - *4 joins 12.* Then 21 must join 15, 24 must join 1, 1 must join 15 for its
    second edge, 15 is full, and 10 joins 6. Again **two cycles**: paths B, A
    with 4 (14 vertices), and the rest (17 vertices).

Every case ends in two cycles or in a contradiction, so `G_2(31)` has no
Hamiltonian cycle. ∎

**A remark on what the steps do.** An edge of sum `s` maps `v` to `s - v`, a
reflection; two in a row make a translation, as `49 - (36 - v) = v + 13`. The
forced path B is such a product of reflections, `5 -> 31 -> 18 -> 7 -> ...`,
alternating translations of `+13`, `+11`, `-13`, `+11`. A Hamiltonian cycle is a
product of reflections that visits every number once and returns to its start;
the proof shows that at `n = 31` the reflections available cannot be glued into
one such product. No purely local invariant — a count, a congruence, the total
of the edge sums, always `2 (1 + ... + 31) = 992` — can show this, since
`G_2(31)` has 2-factors and they satisfy every such invariant: the argument has
to use, somewhere, that a Hamiltonian cycle is connected.

### Seen through Hamiltonian paths, and why 32 is enough

The same fact reads more plainly through paths. An exhaustive search
([`scripts/g2-31-paths.py`](../../scripts/g2-31-paths.py), a few seconds)
finds **349 Hamiltonian paths** in `G_2(31)`, with 172 distinct pairs of ends,
and **no pair of ends sums to a square**: every way of visiting `1..31` once
ends on two numbers that cannot be joined. That is non-Hamiltonicity again,
stated as an obstruction on the ends.

Adding 32 changes no edge among `1..31`, since adjacency depends only on the
sum. The new vertex has exactly two neighbours, `4` (`32 + 4 = 36`) and `17`
(`32 + 17 = 49`), so a Hamiltonian cycle of `G_2(32)` is exactly a Hamiltonian
path of `G_2(31)` from 4 to 17, closed through 32. Among the 349 paths, **exactly
one runs from 4 to 17**:

```text
4, 21, 28, 8, 1, 15, 10, 26, 23, 2, 14, 22, 27, 9, 16, 20, 29, 7, 18, 31,
5, 11, 25, 24, 12, 13, 3, 6, 30, 19, 17
```

So `G_2(32)` has exactly one Hamiltonian cycle, the published witness, and the
search explains why it is unique. In terms of the forced edges of section 3: in
`G_2(31)`, vertex 17 has degree two, so the chain `6-30-19-17-8-28-21` is
forced; 32 gives 17 a third neighbour, the chain breaks between 17 and 8, and
the edge `8-1`, forbidden in `G_2(31)`, becomes usable — it is in the cycle.

---

## 4. A tempting argument that is circular

A graph is *pseudo 2-factor isomorphic* when the number of cycles has the same
parity in all its 2-factors. Both 2-factors of `G_2(31)` have two cycles, hence
even parity; a Hamiltonian cycle would be one cycle, odd; therefore no
Hamiltonian cycle.

**This adds nothing.** Establishing the parity requires knowing all the
2-factors, and that knowledge already settles the question. The notion is
useful in the literature for graph classes where the property can be proved
structurally — cubic bipartite graphs and the like — but power-sum graphs are
not cubic: degrees run 2 to 4 here and 2 to 14 at `k = 6`. With a sample of two
2-factors, "the parity is constant" is true by vacuity.

---

## 5. What scales, and what does not

| certificate | scope | size | checkable by |
|---|---|---|---|
| degree obstruction, per interval | every rejection so far | `O(k)` integers | hand, or thirty lines |
| enumeration of all 2-factors | tiny residues, as at `n = 31` | two branches | hand |
| UNSAT proof (DRAT/LRAT) | small `n` | MB to GB | `drat-trim`, `cake_lpr` |
| structural (articulation, toughness) | never yet observed | `O(n)` | needs the graph |

The first row covers everything the project has actually needed. The second is
this document. The third is available — `rustsat-cadical` exposes a proof
tracer, and `pc-search` refutes `G_2(31)` by SAT in three tours and seven
milliseconds — but the proofs grow with the instance, and at `k = 6` the model
has 132 million clauses.

The fourth row has never fired. Across `k = 2, 3, 4, 5`, every value below the
threshold was rejected by the degree check alone: the full preflight —
connectivity, articulation vertices, degree-two propagation, forced subcycles —
has never rejected a single `n`.

That is worth keeping in mind before adding more structural filters. It also
means the negative half of every published result rests on one short
arithmetic argument, which is why that argument has three independent
derivations behind it.


### Does the argument of section 3 carry over to `k >= 3`?

Its engine is general: vertices near `n` see roughly the powers in
`(n, 2n]`, and where only two lie there, they have degree two and force their
edges. Measured:

| graph | powers in `(n, 2n]` | vertices of degree two |
|---|---|---|
| `G_2(31)` | 36, 49 | 10 (32 %) |
| `G_2(100)` | 121, 144, 169, 196 | 0 |
| `G_3(473)` | 512, 729 | 172 (36 %) |
| `G_3(800)` | 1000, 1331 | 92 (12 %) |
| `G_3(5000)` | 5832, 6859, 8000, 9261 | 0 |
| `G_4(9641)` | `10^4`, `11^4` | 3 986 (41 %) |
| `G_5(246877)` | `12^5`, `13^5` | 110 244 (45 %) |

Two powers in `(n, 2n]` is exactly the situation at the start of an island —
the loop position `q^k/2` waits for `(q + 1)^k`, see
[`islands-theorem.md`](islands-theorem.md) — and there the forced structure is
large, larger as `k` grows. Further into an island more powers enter the
window and the degree-two vertices disappear.

But the argument proves non-Hamiltonicity only when the residual case analysis
closes every branch on two cycles or more. After propagation and contraction,
`G_2(31)` leaves 13 nodes and two branches; `G_3(473)` leaves 210 nodes, 67
chains and 279 free edges, and LKH finds a cycle in it. For `k >= 3` there is
nothing to prove this way: no admissible `n` has ever failed, and the
conjecture of [`islands.md`](islands.md), section 6, says none does.

**A heuristic for the exception at `k = 2`, not a proof.** The share left free
by propagation is comparable — 13 of 31 vertices remain as nodes at `k = 2`,
210 of 473 at `k = 3`, about 42 to 44 % — but the absolute size is not. A
residual of 13 nodes has few configurations, and none of them may close into
one cycle, as at `n = 31`. A residual of hundreds or millions of nodes has so
many that a complete dead end becomes implausible. `k = 2` is the only exponent
whose first island starts that low, at 31; from `k = 3` on it starts at 473 or
more. If that reading is right, `G_2(31)` is the only admissible
non-Hamiltonian power-sum graph, which is the conjecture.

Read the other way, the forced paths are explicit products of reflections that
cover a large share of the graph at the start of every island. They could serve
to **build** cycles rather than refute them: the induction of
[`cube-necklaces.md`](cube-necklaces.md) for `k = 3`, or a proof that the
residual puzzle of a whole finite island always has a solution.
