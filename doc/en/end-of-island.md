# The end of the k = 6 island: G_6(16777215)

`G_6(16777215)`, the last value of the island `[15748961, 16777215]` of
`k = 6`, is the only admissible graph examined so far for which no Hamiltonian
cycle was found. This note records the searches, the tests that found no
obstruction, and the structure that makes the graph hard: the material of
paper 1, section 7.4 (Proposition 7.2), in more detail.

---

## 1. Searches (2026-10-21 and 22)

| search | time | best tour |
|---|---|---|
| `posum search`, seeds 1 then 2 (resumed) | 3 h + 12 h | 7 non-edges |
| `posum search`, seed 3, from scratch | 12 h | 7 non-edges |
| LKH-3.0.14, population of 6, 40 runs of 40 trials, from the 7-tour | 11.5 h | **6** non-edges, every run |
| LKH-3.0.14, `MOVE_TYPE = 3`, backtracking, `PATCHING_A = 2`, `PATCHING_C = 3` | 12 h | 7 non-edges |
| Posa rotation-extension, random walk, from the 7 paths of the best tour | 3 h | one join (6 paths), then none |
| `pc-search --propagated-units --max-depth=9` | running | SAT 2-factor of 341 245 cycles in 29 min; patching slowly |

After propagation and contraction, 11 390 619 nodes are left to LKH, against
8 813 404 for `G_6(15748961)`, which LKH solved in a single trial.

## 2. No obstruction found

Each test below would have proved that no Hamiltonian cycle exists; none did.

- **Hall.** A Hamiltonian cycle is a perfect 2-matching, which exists if and
  only if the bipartite double cover has a perfect matching; if not, a set
  `S` with `|N(S)| < |S|` certifies non-Hamiltonicity. Hopcroft-Karp finds a
  perfect matching (21 s).
- **f-factor.** In the contracted graph every Hamiltonian cycle uses each
  forced chain and exactly `f(x)` free edges at each node: one at a chain end,
  two elsewhere. The same matching test, with capacities `f`, finds a perfect
  f-matching (54 s); a set with `f(S) > f(N(S))` would have been a
  certificate.
- **2-factor respecting the chains.** `pc-search` found one by SAT.
- **Local repair.** Keeping the best tour and letting CaDiCaL choose every
  edge inside a graph ball around its 7 non-edges never gives degree two
  everywhere, even for balls of 120 000 vertices: the tour is committed far
  from its defects. (The tool repairs a deliberately damaged tour of
  `G_4(18193)` at once, so the encoding is not at fault.)

## 3. Where the searches fail

In every best tour, 19 of the 20 non-edges have an end of the same kind: a
contracted node of degree 3 between about 7.4 and 9.4 million, at the end of
a forced chain whose other end is lower by exactly `17^6 - 16^6 = 7360353`.
Independent searches fail on different vertices of this class. Such an end is
rigid: rotating it (Posa) would almost always remove a forced chain.

## 4. The structure

Write `D = 17^6 - 16^6 = 7360353` and `E = 15^6 - D = 4030272`. The powers
that matter are `14^6`, `15^6`, `16^6` and `17^6`. Checked exactly by
[`scripts/end-of-island-bands.py`](../../scripts/end-of-island-bands.py):

| band | vertices | structure |
|---|---|---|
| T | `[15^6, n]`, 5 386 591 vertices | degree 2: neighbours `16^6 - u` and `17^6 - u`, both edges forced |
| B | `[1, E - 1]` | `x` is joined by a forced chain `x - (16^6 - x) - (x + D)` to `h = x + D` |
| low pairs | `[E, 16^6 - 15^6]` | forced chain from `x` to `2 * 16^6 - 17^6 - x = 9416863 - x` |
| M | `[16^6 - 15^6 + 1, D]`, 1 973 762 vertices | free, degree 3: `14^6 - m`, `15^6 - m`, `16^6 - m` |
| H | `[D + 1, 15^6 - 1]` | the ends `h = x + D` of the chains from B |

So each chain `x -> h` is a **unit** `U_x`. Its low end `x` has high degree
when `x` is small; its high end `h` needs one free edge and has two options
(three when `x < 169183`, the third being `14^6 - h`):

- **(a)** the low end `E - x` of the unit `U_{E - x}`;
- **(b)** `9416863 - x`, which is the high end of `U_{2056510 - x}` when
  `x <= 2056509`, and a vertex of M otherwise.

Choosing (a) at both `U_x` and `U_{E - x}` closes a 4-node cycle; for
`x = 1028255`, option (b) is the loop position `16^6 / 2`. The hard core of
the search is essentially these 4 million binary choices under capacity
constraints.

**Why the end of the island.** For every `n` of the island, every vertex of
`[15^6, n]` has degree two (paper 1, Proposition 7.2), and so captures the low
vertex `16^6 - u`. At the start of the island, `n = 15748961`, the low
vertices below `16^6 - n = 1028255`, those of highest degree, escape the
chains and remain free hubs. At the end, `n = 16^6 - 1`, all of them are
captured, down to 1, and each has a single edge left to give. For `k = 5`
the same capture happens at the end of the island, `G_5(248831)` is
Hamiltonian all the same, and the provenance of the island witnesses shows
the number of LKH trials growing from 9.7 on average at the start to 53.8 in
the last quarter.

## 5. Next: a motif

Fix part of the binary choices by a simple rule and let LKH complete the
rest. First motif: option (b) for every unit `x <= 2056509` but the fixed
point, which joins `h_x` to `h_{2056510 - x}`; the units then pair into long
chains whose ends are small vertices of high degree. A cycle found with these
edges imposed is a Hamiltonian cycle of `G_6(16777215)` all the same; if the
motif makes a cycle impossible, other rules can be tried.
