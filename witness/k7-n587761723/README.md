# Powercycle certificate: k = 7, lower bound 587761723

This directory holds the lower-bound half of a result for `k = 7`, and no
witness: no Hamiltonian cycle of `G_7(587761723)` is known yet.

## Statement

`G_7(n)` has vertex set `1, ..., n`; two distinct vertices are adjacent when
their sum is a perfect seventh power. `bands.txt` is a degree certificate for
every `n` from 1 to 587761722: each of its 18 lines gives a range of `n` and
a vertex of degree at most one throughout that range, so no `G_7(n)` with
`3 <= n < 587761723` is Hamiltonian. Hence `h(7) >= 587761723 = 19^7 - 18^7/2`,
the least admissible order for `k = 7`.

The format is that of power-sum-graphs 1.0.0 (`doc/formats.md`, section 4).
That release publishes the certificates for `k = 3..6` in its bundles but not
this one, since a bundle needs a witness.

## Provenance

Produced on 2026-10-20 by `posum bands 7 587761723` (power-sum-graphs 1.0.0,
doi:10.5281/zenodo.23403537), and accepted by both of its verifiers:

```console
python3 verify/python/posum_verify.py bands 7 587761723 bands.txt
verify/rust/target/release/posum-verify bands 7 587761723 bands.txt
```

An independent audit of paper 1 rebuilt the same 18 lines with its own code.

## Files

- `bands.txt` — the degree certificate.
- `bands.txt.sha256` — its SHA-256.
- `README.md` — this document.
- `README.fr.md` — French version.
