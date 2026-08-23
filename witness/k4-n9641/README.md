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

**Lower bound** is `scan.txt`: for every `n` in `[1, 9640]` the graph `G_4(n)`
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

## Checksums

Of the **uncompressed** artifacts:

```text
cycle.txt          c3ea9c7f188333eaea35dfeeca9320bd63889e9d724f634e1fe984595b9da1e5
scan.txt           f1ec6d03fcfd027fe3faebb45358c8ed9a448fa5c4183b5e39839420de83b27f
final.pcdump.json  de15f073d202395161cee3a27dd20e0ebf6fa0b9303db02cbee0db629f9ec211
```

The cycle is exported canonically: rotated so that vertex `1` is first, oriented
so that the smaller neighbour of `1` is second. It begins `1, 15, ...`,
ends at `80`, and closes with `80 + 1 = 81 = 3^4`.

`scan.txt` is the `pc-scan` output for `n = 1 .. 9641`: the first `9640` lines
are `IMPOSSIBLE` with the offending vertex and its degree, and the last line is
the `CANDIDATE` verdict at `9641`.

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

The lower bound is re-derived with:

```console
xz -dc witness/k4-n9641/scan.txt.xz > /tmp/powercycle-scan.txt
cargo run --release --bin pc-scan -- 4 1 9641 | diff - /tmp/powercycle-scan.txt
```

Both verifiers are independent of the powercycle libraries: they use only `k`,
`n` and the candidate witness.

## Licence

The contents of this directory are released under CC0 1.0 Universal. See
[`../LICENSE`](../LICENSE).
