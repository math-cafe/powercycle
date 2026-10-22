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
