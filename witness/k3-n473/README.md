# Powercycle witness: k = 3, n = 473

This directory contains the curated **minimal** Hamiltonian-cycle witness for the
power-sum graph `G_3(473)`.

## Mathematical statement

The graph has vertex set `1, 2, ..., 473`. Two distinct vertices `a` and `b` are
adjacent exactly when `a + b` is a perfect power with exponent 3.

`cycle.txt.zst` decompresses to a plain-text file with one vertex per line. Every
integer from `1` through `473` appears exactly once, and every consecutive pair,
including the closing pair from the last line back to the first, sums to a
perfect power with exponent 3.

## Minimality

This bundle carries both halves of

```text
n_min(3) = 473
```

**Existence** is `cycle.txt`.

**Lower bound** is `scan.txt`: for every `n` in `[1, 472]` the graph `G_3(n)`
has a vertex of degree at most one, which no Hamiltonian cycle can use. The
binding vertex just below the threshold is

```text
v = 256 = 8^3 / 2
```

the self-loop position of `8^3`, which stays at degree one until `9^3` enters
its window at

```text
n = 9^3 - 8^3/2 = 729 - 256 = 473
```

The degree condition is necessary and **not** sufficient, so this argument alone
cannot settle `n_min` for every exponent. See
[`../k5-n246877/README.md`](../k5-n246877/README.md) for the general discussion
and for the `k = 2` case, where the analogous bound is `31` while the true
minimum is `32`.

## Checksums

Of the **uncompressed** artifacts:

```text
cycle.txt          2638ac29ca8b0463836d96fac7c1eadda2638ceda07f20200b49db9b341d824f
scan.txt           e8b3af39bd91954eb994811f83ed0c862db53690c680275deaf7a396a530aaf0
final.pcdump.json  366314d4cc03ce77601a76fd4011ef63c79c679316992dafcb205cadb76c27d1
```

The cycle is exported canonically: rotated so that vertex `1` is first, oriented
so that the smaller neighbour of `1` is second. It begins `1, 63, ...`,
ends at `342`, and closes with `342 + 1 = 343 = 7^3`.

`scan.txt` is the `pc-scan` output for `n = 1 .. 473`: the first `472` lines
are `IMPOSSIBLE` with the offending vertex and its degree, and the last line is
the `CANDIDATE` verdict at `473`.

## Provenance

The final dump `final.pcdump.json.zst` has semantic dump ID `bdb9fcc35ded…`.

- SAT tour 1 produced a raw 2-factor with 7 cycles, giant size 204 and residual 269.
- Two greedy `three-opt-111` moves brought it to 3 cycles with giant size 441.
- Two greedy `three-opt-21` moves absorbed the remaining satellites into a single cycle of length 473.

Total search time was 0.001 seconds.

The final dump is a provenance artifact. The mathematical certificate is the
plain cycle witness itself.

## Independent verification

```console
zstd -dc witness/k3-n473/cycle.txt.zst > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 3 473 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 3 473 /tmp/powercycle-cycle.txt
```

The lower bound is re-derived with:

```console
zstd -dc witness/k3-n473/scan.txt.zst > /tmp/powercycle-scan.txt
cargo run --release --bin pc-scan -- 3 1 473 | diff - /tmp/powercycle-scan.txt
```

Both verifiers are independent of the powercycle libraries: they use only `k`,
`n` and the candidate witness.

## Licence

The contents of this directory are released under CC0 1.0 Universal. See
[`../LICENSE`](../LICENSE).
