# Powercycle witnesses: k = 3, every n from 473 to 800

This directory contains one Hamiltonian cycle of the power-sum graph `G_3(n)`
for each of the 328 values `n = 473, ..., 800`.

## Mathematical statement

`G_3(n)` has vertex set `1, 2, ..., n`; two distinct vertices are adjacent
exactly when their sum is a perfect cube. For `k = 3` the admissible set is
`[473, infinity)`: from `n = 473` on, every vertex has degree at least two.
These cycles show that the first 328 admissible values are all Hamiltonian,
as the conjecture of [`doc/en/islands.md`](../../doc/en/islands.md),
section 6, predicts.

Each file `cycles/k3-n<n>.txt.xz` decompresses to a plain list of the
integers `1..n`, one per line, in canonical order: vertex `1` first, then the
smaller of its two neighbours. Every consecutive pair, including the closing
pair from the last line back to the first, sums to a perfect cube.

The cycle for `n = 473` is not the reference witness of
[`../k3-n473/`](../k3-n473/): it is another Hamiltonian cycle of the same
graph, found by the sweep below.

## Identity

`SHA256SUMS` lists the SHA-256 of each **uncompressed** file, which identifies
the cycle whatever the compression.

## Provenance

Found on 2026-10-13 by `tmp/gerbicz/cycles_473_800.py` (not versioned; the
procedure is `scripts/nice-pairs.py cycles 3 473 800`, see
[`doc/en/cube-necklaces.md`](../../doc/en/cube-necklaces.md)): `pc-tsplib
export`, LKH-3, `pc-tsplib tour` for the check and the canonical form, then
the independent Rust verifier of this repository. Checked again on 2026-10-19
by both verifiers of power-sum-graphs 1.0.0 (Rust and Python,
doi:10.5281/zenodo.23403537): 328 of 328 valid.

## Verification

From the repository root:

```console
D=witness/k3-n473-800
for n in $(seq 473 800); do
  xz -dc $D/cycles/k3-n$n.txt.xz > /tmp/k3-n$n.txt
  python src/verify/python/verify-cycle.py 3 $n /tmp/k3-n$n.txt > /dev/null || echo "n = $n FAILED"
done
(cd /tmp && sha256sum -c --quiet $OLDPWD/$D/SHA256SUMS)
```

## Files

- `cycles/k3-n<n>.txt.xz` — the 328 compressed canonical cycles.
- `SHA256SUMS` — SHA-256 of the uncompressed cycles.
- `README.md` — this document.
- `README.fr.md` — French version.
