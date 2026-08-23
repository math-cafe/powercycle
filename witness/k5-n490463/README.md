# Powercycle witness: k = 5, n = 490463

This directory contains a curated Hamiltonian-cycle witness for the power-sum graph
`G_5(490463)`.

## Mathematical statement

The graph has vertex set `1, 2, ..., 490463`. Two distinct vertices `a` and `b`
are adjacent exactly when `a + b` is a perfect fifth power.

The file `cycle.txt.xz` contains a Hamiltonian cycle: after decompression it is a
plain-text file with one vertex per line. Every integer from `1` through `490463`
appears exactly once, and every consecutive pair, including the closing pair from
the last line back to the first, has a sum equal to a perfect fifth power.

This witness proves the existence of a Hamiltonian cycle at `n = 490463`. By
itself, it does **not** prove that `490463` is minimal.

> **This is not the minimal witness for `k = 5`.** The reference result is
> `n_min(5) = 246877`, carried by `witness/k5-n246877/`. The present bundle is
> kept because it remains valid and independently verified, and because it
> records how the search progressed.

## Canonical witness identity

The uncompressed `cycle.txt` has SHA-256:

`9c9119e5946db4cf3c433e0c90bb6b098afb19f89a2a3fe6f48bfed669cf1efe`

The cycle is exported canonically by rotating it so that vertex `1` is first and
choosing the orientation for which the smaller of the two neighbours of `1` is
second.

`cycle.txt.sha256` stores the checksum of the **uncompressed** witness.

## Provenance

The final versioned powercycle dump is stored as
`final.pcdump.json.xz`. Its semantic dump ID is:

`fa62a76a36f8…`

The successful chain for this witness was:

- SAT tour 1 produced a raw 2-factor with 91 cycles, giant size 383558 and
  residual 106905.
- Greedy `three-opt-111` and `three-opt-21`, followed by bounded alternating
  repairs with maximum depth 3, reduced it to two cycles with giant size 490438
  and one satellite of size 25.
- One alternating repair at depth 4 absorbed the final 25-vertex satellite,
  producing one cycle of length 490463.

The final dump is a provenance artifact. The mathematical certificate is the
plain cycle witness itself.

## Independent verification

The repository contains two independent witness verifiers:

- a standalone Rust verifier under `src/verify/rust/`;
- a Python standard-library-only verifier under `src/verify/python/`.

They verify only the witness definition: permutation of `1..n` and the fifth-power
condition on every consecutive sum, including closure. They do not use the SAT
solver, dump format, graph edge IDs, patch history or search implementation.

Typical checks from the repository root are:

```console
xz -dc witness/k5-n490463/cycle.txt.xz > /tmp/powercycle-cycle.txt

sha256sum /tmp/powercycle-cycle.txt

cargo run --release \
  --manifest-path src/verify/rust/Cargo.toml \
  -- 5 490463 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py \
  5 490463 /tmp/powercycle-cycle.txt
```

The SHA-256 reported by the checksum command and by the Python verifier must be
the canonical witness identity given above.

## Files

- `cycle.txt.xz` — compressed canonical Hamiltonian-cycle witness.
- `cycle.txt.sha256` — SHA-256 of the uncompressed `cycle.txt`.
- `final.pcdump.json.xz` — compressed final powercycle dump with provenance.
- `final.pcdump.json.sha256` — SHA-256 of the uncompressed final dump.
- `README.md` — this document.
- `README.fr.md` — French version.
