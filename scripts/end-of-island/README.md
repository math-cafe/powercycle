# End of the k = 6 island: motif and expansion

The two tools behind [`../../witness/k6-n16777215/`](../../witness/k6-n16777215/),
built on posum-core and posum-lkh (power-sum-graphs, expected next to this
repository). See [`../../doc/en/end-of-island.md`](../../doc/en/end-of-island.md)
for the structure they use.

```console
cargo build --release
target/release/motif problem.hcp motif.hcp
target/release/expand 6 16777215 tour cycle
```

- `motif SRC.hcp OUT.hcp` takes the contracted problem of `G_6(16777215)`
  written by `posum search` and appends to its fixed edges the motif: for
  every `x` from 1 to 1028254, the high ends `x + D` and `2056510 - x + D` of
  two forced chains (`D = 17^6 - 16^6`) are joined. An edge already forced is
  skipped; a forbidden edge would be reported.
- `expand K N TOUR OUT` expands an LKH tour of the contracted `G_K(N)` to a
  Hamiltonian cycle, checks it with posum-core, and writes it in canonical
  order as `OUT.txt` (one vertex per line) and `OUT.bases` (bases encoding).

## Island structure and motifs in general

Three more tools, used for the start of the `k = 7` island
([`../../doc/en/k7-start.md`](../../doc/en/k7-start.md)):

```console
target/release/structure 7 587761723
target/release/zones 7 587761723 24458309 62748517 ...
target/release/impose 7 587761723 18 c,c1mod=2/3 [motif.hcp]
target/release/impose 6 15748961 16 c --expand tour cycle
```

- `structure K N` describes the contracted problem: nodes by kind (chain end
  or free vertex) and free degree, free edges by base, node positions.
- `zones K N CUT...` counts chains and free edges by pair of zones.
- `impose K N C RULE [OUT.hcp]` runs degree-two propagation with imposed
  edges, its own implementation (it gives posum's counts when nothing is
  imposed), contracts, and writes the HCP problem; with `--expand TOUR OUT`
  it turns an LKH tour of that problem into a checked Hamiltonian cycle. The
  rules, for the island parameter `C`: `c` joins every free vertex `m` of
  degree 3 in `[C^K - (C-1)^K + 1, (C-2)^K - 1]` to `C^K - m`; `c1` joins it
  to `(C-1)^K - m` when that is a low chain end; `c1mod=a/b` does the same
  for `m mod b < a` only.

`../island-profile.py K N` gives the intervals on which the usable powers
do not change, from which the zones are taken.
