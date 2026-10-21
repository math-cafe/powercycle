# Powercycle witness: k = 6, n = 30039769

A Hamiltonian cycle of the power-sum graph `G_6(30039769)`, the first value of
the `k = 6` mainland.

## Statement

`G_6(n)` has vertex set `1, ..., n`; two distinct vertices are adjacent when
their sum is a perfect sixth power. For `k = 6` the admissible set is
`[15748961, 16777215] U [30039769, infinity)`. `G_6(15748961)` is
Hamiltonian (power-sum-graphs 1.0.0), no `n` from 16777216 to 30039768 is
admissible, and this cycle shows that `G_6(30039769)` is Hamiltonian again.

## Encoding and identity

The cycle has 30 million vertices, so it is stored in the bases encoding of
power-sum-graphs (`doc/formats.md`, section 3): `a7aa7317.bases.xz` holds the
`n - 1` sixth roots `b_i` with `v_{i+1} = b_i^6 - v_i`, from `v_1 = 1`. The
decoded list, one vertex per line, in canonical order, has SHA-256

`a7aa7317ddc6a3d38164b0d0372c0ab862eab57b3132f7a514a0e262491d4eb6`

## Provenance

Found on 2026-10-21 by `posum search 6 30039769` (power-sum-graphs 1.0.0) with
LKH-3.0.14 on an AMD Ryzen 9 7940HS: 23450876 nodes after propagation and
contraction, 3 trials, 4820 s, about 15 GB. `provenance.toml` is posum's
record. Accepted by both verifiers of power-sum-graphs 1.0.0 (Rust and
Python, on the plain list) and by [`scripts/verify-bases.sh`](../../scripts/verify-bases.sh),
which shares no code with them.

## Verification

```console
scripts/verify-bases.sh 6 30039769 witness/k6-n30039769/a7aa7317.bases.xz
```

prints `VALID` and the digest above.
