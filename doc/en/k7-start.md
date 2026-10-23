# The start of the k = 7 island: structure and motifs (2026-10-23)

`G_7(587761723)` is the least admissible graph for `k = 7`: a Hamiltonian
cycle in it would give `h(7) = 587761723` (paper 1, open question 1). After
propagation and contraction 297 314 988 nodes are left, about 190 GB for LKH,
three times the memory of this machine (58 GB). The end of the `k = 6` island
was settled by imposing a motif read off its structure
([`end-of-island.md`](end-of-island.md)); this note asks what the same
approach gives at the start of the `k = 7` island. The tools are in
[`../../scripts/end-of-island/`](../../scripts/end-of-island/) and
[`../../scripts/island-profile.py`](../../scripts/island-profile.py).

---

## 1. The start of the k = 7 island is a larger start of the k = 6 island

The island of `k = 7` is `[587761723, 612220031]`, with `c = 18`; the
mainland starts at 1161088541, twice as far, and is not needed for `h(7)`.
The start of an island is where the search is easy (paper 1, section 7.4):
`G_6(15748961)` needed a single LKH trial, against two days for
`G_6(16777215)`. The usable-power profile of `G_7(587761723)` has the same
shape as that of `G_6(15748961)`: two bands of degree two,
`[(c-1)^k, n]` and `[(c-2)^k, c^k/2 - 1]`, then a band of free vertices of
degree three. After propagation (`structure`):

| | `G_6(15748961)`, easy | `G_6(16777215)`, hard | `G_7(587761723)` |
|---|---|---|---|
| nodes / n | 56.0 % | 67.9 % | **50.6 %** |
| chain ends with 2 free edges | 38.4 % | 38.8 % | 40.3 % |
| free hubs: the vertices `1` to `c^k - S_c - 1`, of high degree | 11.7 % | **0 %** | 8.2 % |
| free vertices of degree 3 | 24.3 % | 17.3 % | 22.4 % |

Nothing points to a combinatorial difficulty specific to `k = 7`; the
obstacle is size. The `k = 6` motif of the end of the island does not
transfer: it pairs `h_x` and `h_x'` with `x + x' = c^k - 2D = 2(c^k - S_c)`,
and the units exist only for `x >= c^k - n`, so it has about `n - S_c` pairs,
none at the start of an island.

## 2. Two explicit matchings

The zones of `G_7(587761723)` (`zones`), with what the contracted problem
holds in each:

| zone | vertices | after contraction |
|---|---|---|
| Z0 | `[1, 24458308]` | free hubs, below `c^k - n` |
| Z1, Z2 | `[24458309, 105413503]` | low chain ends, 3 to 6 free edges |
| Z4 | `[141903218, 201881359]` | low chain ends, 2 free edges |
| Z5 | `[201881360, 268435455]` | free vertices of degree 3 (bases 16, 17, 18) |
| Z8 | `[343784577, 410338672]` | high chain ends, 2 free edges |
| others | | interior vertices of chains |

Two families of edges stand out:

- **base `c = 18`, Z5 to Z8:** 66 554 094 edges, as many as the vertices of
  Z5 and as the chain ends of Z8. They form a perfect matching, `m` with
  `18^7 - m`.
- **base `c - 1 = 17`, Z5 to Z4:** 59 978 142 edges, one for each chain end
  of Z4, `m` with `17^7 - m`.

The same two matchings exist at the start of the `k = 6` island, where the
witness [`../../witness/k6-n15748961/`](../../witness/k6-n15748961/), found
without any hint by `pc-patch` (SAT and repair), uses 78.9 % of the first.

## 3. Tests at the start of the k = 6 island

`impose` runs degree-two propagation with the imposed edges (its own
implementation, which gives posum's counts when nothing is imposed),
contracts, and writes the problem for LKH. Rules: `c` imposes the first
matching, `c1` the second, `c1mod=a/b` the second for `m mod b < a` only.
LKH-3.0.14 with posum's parameters, seed 1, at most 2 h:

| motif | nodes | LKH |
|---|---|---|
| none | 8 813 404 | cycle at the first trial, 369 s |
| `c` | 6 670 462 (76 %) | **cycle** at the first trial, 250 s |
| `c` + ½ `c1` | 5 034 239 (57 %) | 3 non-edges after 15 min; stopped |
| `c` + ⅔ `c1` | 4 497 636 (51 %) | 1 non-edge after 10 min; restarted from that tour (seeds 2 and 3), **cycle** at the first trial, 58 s |
| `c` + `c1` | 3 411 221 (39 %) | stalled at 11 non-edges; stopped after 35 min |

The cycles are accepted by both power-sum-graphs verifiers; they are not
kept (`G_6(15748961)` has its witness), their identities are
`89a2b3dc…` (`c`) and `8c9c94eb…` (`c` + ⅔ `c1`, the same cycle from both
seeds). The two known cycles use the second matching for about 70 % of the
vertices of Z5 (70.5 % for the `pc-patch` witness, 65.9 % for the cycle LKH
found with `c` imposed), evenly over the zone: imposing all of it takes away the
small vertices, hubs and low chain ends, that the other 30 % reach by base
16, which is presumably why the full motif stalls.

As at the end of the island, the productive step is to **restart LKH from
the best tour**: the last non-edge was repaired in one trial.

## 4. What the motifs give at k = 7

Measured with `impose`, propagation cascade included. LKH takes 645 bytes per
node (2.2 GB for the 3.41-million-node problem above):

| motif | nodes | LKH memory |
|---|---|---|
| none | 297 314 988 | ~192 GB |
| `c` | 230 760 894 | ~149 GB |
| `c` + ½ `c1` | 170 437 952 | ~110 GB |
| **`c` + ⅔ `c1`** | **150 509 093** (25.6 % of n) | **~97 GB** |
| `c` + `c1` | 110 421 499 | ~71 GB, a motif that stalled at `k = 6` |

With the motif `c` + ⅔ `c1`, checked at `k = 6`, `G_7(587761723)` fits a
machine of 128 GB. On this one, another factor of two is needed.

## 5. Decomposition fails

Two memory-bounded schemes were tried on `G_6(15748961)` (8 813 404 nodes,
no motif), which LKH solves whole in 6 minutes. Both start from a full tour
built without LKH by a Warnsdorff greedy that keeps the chains: 275 845
non-edges, in 3 s.

| method | LKH memory | result |
|---|---|---|
| `windows`: 4 tour segments, each a Hamiltonian path problem between its fixed ends, 15 min | ÷ 4 | 127 587, then 114 712 non-edges; all the gain in the first 5 s |
| `windows`, 20 s per segment, segments moved by a golden-ratio step | ÷ 4 | 54 201 after 42 rounds, the gain per round shrinking |
| `relink`: tour runs far from the defects contracted to links, `r = 3` then `r = 10` | ÷ 6 to 8 | stuck: 142 000 from the greedy, 65 000 from the windows |

The cause is structural. A segment of a tour, taken alone, has no
Hamiltonian path: in a typical segment of 2.2 million nodes, 229 nodes have
no neighbour inside and 12 400 have one, so each forces a defect whatever
LKH does (LKH even crashes on such a segment without a start tour). Edges
join `u` to `p - u` for every power `p`, and the hubs reach every zone: no
order of the vertices keeps the neighbours of a node near it. Relinking does
not make up for it, since the ends of the contracted runs have almost no
free partner left. A decomposition would need a partition in which every
node keeps two neighbours in its part, with a small cut; the zones of
section 2 are joined by tens of millions of edges each.

So for `k = 7` two routes remain: the motif `c` + ⅔ `c1` on a machine of
128 GB, or a solver that uses much less memory per node than LKH on these
sparse problems (645 bytes per node for about 1.4 free edges per node).
