# Powercycle witness: k = 6, n = 15748961

This directory contains the curated **minimal** Hamiltonian-cycle witness for the
power-sum graph `G_6(15748961)`.

## Mathematical statement

The graph has vertex set `1, 2, ..., 15748961`. Two distinct vertices `a` and
`b` are adjacent exactly when `a + b` is a perfect sixth power.

`cycle.txt.xz` decompresses to a plain-text file with one vertex per line.
Every integer from `1` through `15748961` appears exactly once, and every
consecutive pair, including the closing pair from the last line back to the
first, sums to a perfect sixth power.

## Minimality

This bundle carries both halves of

```text
n_min(6) = 15748961
```

**Existence** is `cycle.txt`.

**Lower bound** is `bands.txt`: for every `n` in `[1, 15748960]` the graph
`G_6(n)` has a vertex of degree at most one, which no Hamiltonian cycle can
use. The binding vertex just below the threshold is

```text
v = 8388608 = 16^6 / 2
```

the self-loop position of `16^6`: it lies inside the interval that power
generates but cannot use it, so it stays at degree one until `17^6` enters its
window, at

```text
n = 17^6 - 16^6/2 = 24137569 - 8388608 = 15748961
```

The degree condition is necessary and **not** sufficient — see
[`../k5-n246877/README.md`](../k5-n246877/README.md) for the `k = 2` case,
where the analogous bound is `31` while the true minimum is `32`. Here the two
halves meet, because a witness exists at the first `n` the bound allows.

## Checksums

Of the **uncompressed** artifacts:

```text
cycle.txt          5f2b4192388caf362098311b1363163bfb78b554fb2ae8109608f773f50066c5
bands.txt          015b5be7015d2f6d1756a02068fa0202b5472442c0e3ca09d52a3935ed93b06e
final.pcdump.json  df6402b4d5ebd5a68ba59a2d4d6edfae99efa83c6f46a9fc6da0c97613bf0402
```

The cycle is exported canonically: rotated so vertex `1` is first, oriented so
the smaller neighbour of `1` is second. It begins `1, 63, ...`, ends at
`531440`, and closes with `531440 + 1 = 531441 = 9^6`.

`bands.txt` is the `pc-scan --bands` output for `n = 1 .. 15748960`: 16 lines,
each a range of `n` with the offending vertex and its degree. The degree of a
vertex never decreases as `n` grows, since its window only widens, so a band
that holds at both ends holds throughout; the verifiers check each band at its
two endpoints. Every rejection in the range comes from the graph-free degree
check, so reproducing it builds no graph and takes about 6 seconds. Up to
version 0.1.0 the bundle carried `scan.txt` instead, one line per `n`, still
available at the tag `witness-k6-n15748961`.

## Provenance

The final dump `final.pcdump.json.xz` has semantic dump ID `34a7b80245ee…`.

The chain that produced this witness, with its timings:

- **SAT tour 1**, 3 h 59 (14 332 s), gave a raw 2-factor with 18 391 cycles,
  giant size 9 571 535 and residual 6 177 426.
- **Greedy `three-opt-111`**, 710 moves in 3 413 s, brought it to 16 971 cycles.
- **Greedy `three-opt-21`**, 15 902 moves in 6 326 s, to 1 069 cycles.
- **1 056 alternating repairs at depth ≤ 4**, about 21 hours, to 7 cycles with
  giant size 15 748 845 and 6 satellites totalling 116 vertices. There the
  search reached a local fixed point: no repair exists at that depth.
- **5 repairs at depth ≤ 6**, 78 s, to 2 cycles — one satellite of 10 vertices
  left.
- **1 repair at depth ≤ 9**, 21 s, closing the cycle.

Depth was the decisive parameter. Thirty-four hours of patching stalled at
depth 4; raising it closed the factor in two minutes.

The last satellite to resist is worth recording. Ten vertices, with an exact
two-fold symmetry:

```text
425729 —12⁶— 2560255 —16⁶— 14216961 —17⁶— 9920608 —16⁶— 6856608 —14⁶—
672928 —12⁶— 2313056 —16⁶— 14464160 —17⁶— 9673409 —16⁶— 7103807 —14⁶— 425729
```

The power sequence `12⁶, 16⁶, 17⁶, 16⁶, 14⁶` repeats twice. Two of its
vertices, `14216961` and `14464160`, have degree exactly two in the graph, so
both of their edges are forced in every 2-factor. That rigidity is why breaking
this cycle needed an alternating path of length nine.

The final dump is a provenance artifact. The mathematical certificate is the
plain cycle witness itself.

## Independent verification

```console
xz -dc witness/k6-n15748961/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 6 15748961 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 6 15748961 /tmp/powercycle-cycle.txt
```

The lower bound is checked, and replayed, with:

```console
xz -dc witness/k6-n15748961/bands.txt.xz > /tmp/powercycle-bands.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- 6 15748961 /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py 6 15748961 /tmp/powercycle-bands.txt

cargo run --release --bin pc-scan -- --bands --quick-only 6 1 15748960 \
  | diff - /tmp/powercycle-bands.txt
```

The verifiers are independent of the powercycle libraries: they use only `k`,
`n` and the file they check.

## Files

- `cycle.txt.xz` — compressed canonical Hamiltonian-cycle witness (24 MB).
- `cycle.txt.sha256` — SHA-256 of the uncompressed `cycle.txt`.
- `bands.txt.xz` — compressed `pc-scan --bands` lower-bound certificate (328 bytes).
- `bands.txt.sha256` — SHA-256 of the uncompressed `bands.txt`.
- `final.pcdump.json.xz` — compressed final dump with provenance (11 MB).
- `final.pcdump.json.sha256` — SHA-256 of the uncompressed final dump.
- `README.md` — this document.
- `README.fr.md` — French version.

A Hamiltonian cycle is close to incompressible — about 12 bits per vertex here
against an information-theoretic floor near 1.7 — so `cycle.txt.xz` dominates
this bundle. An encoding by neighbour index would be far smaller but would need
the graph, 922 MB at this size, to decode. The plain list verifies with twenty
lines and no data structure, which is worth more than the space.

## Licence

The contents of this directory are released under CC0 1.0 Universal. See
[`../LICENSE`](../LICENSE).
