# Powercycle witness: k = 3, n = 473

This directory contains the curated **minimal** Hamiltonian-cycle witness for the
power-sum graph `G_3(473)`.

## Mathematical statement

The graph has vertex set `1, 2, ..., 473`. Two distinct vertices `a` and `b` are
adjacent exactly when `a + b` is a perfect power with exponent 3.

`cycle.txt.xz` decompresses to a plain-text file with one vertex per line. Every
integer from `1` through `473` appears exactly once, and every consecutive pair,
including the closing pair from the last line back to the first, sums to a
perfect power with exponent 3.

## Minimality

This bundle carries both halves of

```text
n_min(3) = 473
```

**Existence** is `cycle.txt`.

**Lower bound** is `bands.txt`: for every `n` in `[1, 472]` the graph `G_3(n)`
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

## Prior work

`n_min(3) = 473` is not new. Giovanni Resta found the cubic loop of length 473
(Carlos Rivera, [Puzzle 311](http://www.primepuzzles.net/puzzles/puzz_311.htm): Sum to a cube), and the value is a(3) of OEIS [A115418](https://oeis.org/A115418),
submitted by Roberto Tauraso in 2006. This bundle contributes an independently
found and verified cycle, and the lower-bound certificate, for which no
published counterpart has been found.

## Checksums

Of the **uncompressed** artifacts:

```text
cycle.txt          2638ac29ca8b0463836d96fac7c1eadda2638ceda07f20200b49db9b341d824f
bands.txt          e9717e68480df786ee177f7ff4b50263953457f6beb8c6c48fc3afcc7f5ae729
final.pcdump.json  366314d4cc03ce77601a76fd4011ef63c79c679316992dafcb205cadb76c27d1
```

The cycle is exported canonically: rotated so that vertex `1` is first, oriented
so that the smaller neighbour of `1` is second. It begins `1, 63, ...`,
ends at `342`, and closes with `342 + 1 = 343 = 7^3`.

`bands.txt` is the `pc-scan --bands` output for `n = 1 .. 472`: 8 lines,
each a range of `n` with the vertex of degree at most one and its degree. The
degree of a vertex never decreases as `n` grows, since its window only widens,
so a band that holds at both ends holds throughout; the verifiers check each
band at its two endpoints. Up to version 0.1.0 the bundle carried `scan.txt`
instead, one line per `n`, still available at the tag `witness-k3-n473`.

## Provenance

The final dump `final.pcdump.json.xz` has semantic dump ID `bdb9fcc35ded…`.

- SAT tour 1 produced a raw 2-factor with 7 cycles, giant size 204 and residual 269.
- Two greedy `three-opt-111` moves brought it to 3 cycles with giant size 441.
- Two greedy `three-opt-21` moves absorbed the remaining satellites into a single cycle of length 473.

Total search time was 0.001 seconds.

The final dump is a provenance artifact. The mathematical certificate is the
plain cycle witness itself.

## Independent verification

```console
xz -dc witness/k3-n473/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 3 473 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 3 473 /tmp/powercycle-cycle.txt
```

The lower bound is checked, and replayed, with:

```console
xz -dc witness/k3-n473/bands.txt.xz > /tmp/powercycle-bands.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- 3 473 /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py 3 473 /tmp/powercycle-bands.txt

cargo run --release --bin pc-scan -- --bands --quick-only 3 1 472 \
  | diff - /tmp/powercycle-bands.txt
```

The verifiers are independent of the powercycle libraries: they use only `k`,
`n` and the file they check.

## Licence

The contents of this directory are released under CC0 1.0 Universal. See
[`../LICENSE`](../LICENSE).
