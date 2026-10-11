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
