# Powercycle witness: k = 6, n = 16777215

A Hamiltonian cycle of the power-sum graph `G_6(16777215)`, the last value of
the `k = 6` island.

## Statement

`G_6(n)` has vertex set `1, ..., n`; two distinct vertices are adjacent when
their sum is a perfect sixth power. For `k = 6` the admissible set is
`[15748961, 16777215] U [30039769, infinity)`. `G_6(15748961)`, the first value
of the island, is Hamiltonian (power-sum-graphs 1.0.0), and so is
`G_6(30039769)`, the first value of the mainland
([`../k6-n30039769/`](../k6-n30039769/)). This cycle settles the other end of
the island, `n = 16^6 - 1`, the hardest graph met so far: see
[`../../doc/en/end-of-island.md`](../../doc/en/end-of-island.md).

## Encoding and identity

The cycle is stored in the bases encoding of power-sum-graphs
(`doc/formats.md`, section 3): `4caf6992.bases.xz` holds the `n - 1` sixth
roots `b_i` with `v_{i+1} = b_i^6 - v_i`, from `v_1 = 1`. The decoded list, one
vertex per line, in canonical order, has SHA-256

`4caf699294398fb56bbb91d8d1bf8447892d62937f7de8911c4f28e6110cea73`

## Provenance

Found on 2026-10-22 with LKH-3.0.14 on an AMD Ryzen 9 7940HS, after
`posum search` and every other search had stalled at 6 or 7 missing edges. The
cycle follows a **motif**: for every `x` from 1 to 1028254, the vertices
`x + 7360353` and `2056510 - x + 7360353`, ends of two forced chains whose sum
is `16^6`, are joined (one of these edges is already forced, so 1028253 are
imposed). `provenance.toml` records each step with the SHA-256 of its result:

1. the contracted problem written by `posum search 6 16777215`
   (power-sum-graphs 1.0.0), 11390619 nodes;
2. the motif imposed as fixed edges by `motif`, in
   [`../../scripts/end-of-island/`](../../scripts/end-of-island/);
3. LKH from scratch with [`lkh/motif-a.par`](lkh/motif-a.par), seed 21: a tour
   2 edges short at the fourth trial, 1120 s;
4. LKH from that tour with [`lkh/motif-r25.par`](lkh/motif-r25.par), seed 25: a
   Hamiltonian cycle at the first trial, 155 s;
5. the tour expanded to a cycle of `G_6(16777215)` by `expand`, in the same
   directory.

The parameter files are those of the runs, with the file names of this
record. A second run from the same tour, with
[`lkh/motif-r26.par`](lkh/motif-r26.par) (seed 26, first trial, 170 s), gave
another cycle, stored as `ae108229.bases.xz`, of identity
`ae108229ea29ec360f47f4d1bbcc9a8194c1ed8b8020a2fc9976bc24ba6fcd63`; it is
accepted by the Rust verifier of power-sum-graphs 1.1.0 (plain list and bases)
and by `scripts/verify-bases.sh`.
Neither cycle was produced by posum alone, so neither is a posum witness.

The first cycle is accepted by both verifiers of power-sum-graphs 1.1.0 (Rust, on the plain list
and on the bases; Python, on the plain list) and by
[`scripts/verify-bases.sh`](../../scripts/verify-bases.sh), which shares no code
with them.

## Verification

```console
scripts/verify-bases.sh 6 16777215 witness/k6-n16777215/4caf6992.bases.xz
```

prints `VALID` and the digest above.
