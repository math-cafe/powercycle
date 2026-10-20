# Powercycle witness: k = 4, n = 18193

This directory contains a Hamiltonian cycle of the power-sum graph
`G_4(18193)`.

## Mathematical statement

`G_4(n)` has vertex set `1, 2, ..., n`; two distinct vertices are adjacent
exactly when their sum is a perfect fourth power. For `k = 4` the admissible
set is `[9641, 9999] U [18193, infinity)`: one island, then the mainland.
`18193` is the first value of the mainland.

Every `n` of the island is Hamiltonian (the island bundle of power-sum-graphs
1.0.0), no `n` from 10000 to 18192 is (some vertex has degree at most one),
and this cycle shows that `G_4(18193)` is Hamiltonian again. So the
Hamiltonicity of `G_4(n)` is not monotone in `n`.

`cycle.txt.xz` decompresses to a plain list of the integers `1..18193`, one
per line, in canonical order: vertex `1` first, then the smaller of its two
neighbours. Every consecutive pair, including the closing pair from the last
line back to the first, sums to a perfect fourth power.

## Identity

The uncompressed `cycle.txt` has SHA-256:

`699280154bcdbeec8325c52ef5f06cf546c5ccc57ac7d4466f3b379b86005d9c`

## Provenance

Found on 2026-10-19 by `posum search 4 18193` (power-sum-graphs 1.0.0,
doi:10.5281/zenodo.23403537) with LKH-3.0.14: one trial, 0.1 s, after
propagation and contraction left 13532 nodes. `provenance.toml` is posum's
record of the run. Checked by both verifiers of power-sum-graphs 1.0.0 (Rust
and Python) and by the verifiers of this repository.

## Verification

From the repository root:

```console
xz -dc witness/k4-n18193/cycle.txt.xz > /tmp/k4-n18193.txt
sha256sum /tmp/k4-n18193.txt          # must match cycle.txt.sha256
cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 4 18193 /tmp/k4-n18193.txt
python src/verify/python/verify-cycle.py 4 18193 /tmp/k4-n18193.txt
```

## Files

- `cycle.txt.xz` — the compressed canonical cycle.
- `cycle.txt.sha256` — SHA-256 of the uncompressed `cycle.txt`.
- `provenance.toml` — posum's provenance record.
- `README.md` — this document.
- `README.fr.md` — French version.
