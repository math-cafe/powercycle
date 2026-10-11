# Powercycle witness: k = 4, n = 9641

This directory contains the curated **minimal** Hamiltonian-cycle witness for the
power-sum graph `G_4(9641)`.

## Mathematical statement

The graph has vertex set `1, 2, ..., 9641`. Two distinct vertices `a` and `b` are
adjacent exactly when `a + b` is a perfect power with exponent 4.

`cycle.txt.xz` decompresses to a plain-text file with one vertex per line. Every
integer from `1` through `9641` appears exactly once, and every consecutive pair,
including the closing pair from the last line back to the first, sums to a
perfect power with exponent 4.

## Minimality

This bundle carries both halves of

```text
n_min(4) = 9641
```

**Existence** is `cycle.txt`.

**Lower bound** is `bands.txt`: for every `n` in `[1, 9640]` the graph `G_4(n)`
has a vertex of degree at most one, which no Hamiltonian cycle can use. The
binding vertex just below the threshold is

```text
v = 5000 = 10^4 / 2
```

the self-loop position of `10^4`, which stays at degree one until `11^4` enters
its window at

```text
n = 11^4 - 10^4/2 = 14641 - 5000 = 9641
```

The degree condition is necessary and **not** sufficient, so this argument alone
cannot settle `n_min` for every exponent. See
[`../k5-n246877/README.md`](../k5-n246877/README.md) for the general discussion
and for the `k = 2` case, where the analogous bound is `31` while the true
minimum is `32`.

## Prior work

`n_min(4) = 9641` is not new: it is a(4) of OEIS [A115418](https://oeis.org/A115418), submitted by Roberto
Tauraso in 2006. This bundle contributes an independently found and verified
cycle, and the lower-bound certificate, for which no published counterpart has
been found.

## Checksums

Of the **uncompressed** artifacts:

```text
cycle.txt          c3ea9c7f188333eaea35dfeeca9320bd63889e9d724f634e1fe984595b9da1e5
bands.txt          de031e95b2cdca66c40880616f2cda572365f254a38365868c57073880a6a27d
final.pcdump.json  de15f073d202395161cee3a27dd20e0ebf6fa0b9303db02cbee0db629f9ec211
```

The cycle is exported canonically: rotated so that vertex `1` is first, oriented
so that the smaller neighbour of `1` is second. It begins `1, 15, ...`,
ends at `80`, and closes with `80 + 1 = 81 = 3^4`.

`bands.txt` is the `pc-scan --bands` output for `n = 1 .. 9640`: 10 lines,
each a range of `n` with the vertex of degree at most one and its degree. The
degree of a vertex never decreases as `n` grows, since its window only widens,
so a band that holds at both ends holds throughout; the verifiers check each
band at its two endpoints. Up to version 0.1.0 the bundle carried `scan.txt`
instead, one line per `n`, still available at the tag `witness-k4-n9641`.

## Provenance

The final dump `final.pcdump.json.xz` has semantic dump ID `940ef6637add…`.

- SAT tour 1 produced a raw 2-factor with 6 cycles, giant size 5023 and residual 4618.
- One greedy `three-opt-21` move raised the giant to 9157.
- Four bounded alternating repairs, at depths 2, 4, 2 and 2, absorbed satellites of 212, 204, 34 and 34 vertices in turn, producing one cycle of length 9641.

Total search time was 0.021 seconds.

The final dump is a provenance artifact. The mathematical certificate is the
plain cycle witness itself.

## Independent verification

```console
xz -dc witness/k4-n9641/cycle.txt.xz > /tmp/powercycle-cycle.txt
sha256sum /tmp/powercycle-cycle.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 4 9641 /tmp/powercycle-cycle.txt

python src/verify/python/verify-cycle.py 4 9641 /tmp/powercycle-cycle.txt
```

The lower bound is checked, and replayed, with:

```console
xz -dc witness/k4-n9641/bands.txt.xz > /tmp/powercycle-bands.txt

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  --bin powercycle-verify-bands -- 4 9641 /tmp/powercycle-bands.txt

python src/verify/python/verify-bands.py 4 9641 /tmp/powercycle-bands.txt

cargo run --release --bin pc-scan -- --bands --quick-only 4 1 9640 \
  | diff - /tmp/powercycle-bands.txt
```

The verifiers are independent of the powercycle libraries: they use only `k`,
`n` and the file they check.

## Licence

The contents of this directory are released under CC0 1.0 Universal. See
[`../LICENSE`](../LICENSE).
