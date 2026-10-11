# powercycle

Search, transformation, export and independent verification of power-sum
Hamiltonian cycles.

For integers `k >= 2` and `n >= 1`, the **power-sum graph** `G_k(n)` has vertex
set `{1, ..., n}` and joins two distinct vertices `x != y` whenever `x + y` is a
perfect `k`-th power. A Hamiltonian cycle of `G_k(n)` is an arrangement of the
integers `1` through `n` around a circle in which **every adjacent pair sums to a
perfect `k`-th power**. For `k = 2` this is the classical square-sum problem.

`n_min(k)` denotes the least `n` for which such an arrangement exists.

## Results

| k | n_min | closed form | witness |
|---|---|---|---|
| 2 | 32 | — | known result ([A272259](https://oeis.org/A272259)) |
| 3 | **473** | `9^3 - 8^3/2` | [`witness/k3-n473/`](witness/k3-n473/) |
| 4 | **9641** | `11^4 - 10^4/2` | [`witness/k4-n9641/`](witness/k4-n9641/) |
| 5 | **246877** | `13^5 - 12^5/2` | [`witness/k5-n246877/`](witness/k5-n246877/) |
| 6 | **15748961** | `17^6 - 16^6/2` | [`witness/k6-n15748961/`](witness/k6-n15748961/) |
| 7 | `>= 587761723` | `19^7 - 18^7/2` | open |

Each value in bold is a theorem, not a bound: the bundle carries a Hamiltonian
cycle **and** a certificate ruling out every smaller `n`. See
[`witness/README.md`](witness/README.md) for the argument and its limits — in
particular why `k = 2` does not follow the same pattern.

## Verify a result

Nothing here asks you to trust the search. From a fresh clone:

```console
xz -dc witness/k5-n246877/cycle.txt.xz > /tmp/cycle.txt

sha256sum /tmp/cycle.txt
cat witness/k5-n246877/cycle.txt.sha256

cargo run --release --manifest-path src/verify/rust/Cargo.toml \
  -- 5 246877 /tmp/cycle.txt

python src/verify/python/verify-cycle.py 5 246877 /tmp/cycle.txt
```

The two verifiers are deliberately outside the powercycle dependency graph. One
is a standalone Rust crate with **no dependencies at all**; the other uses only
the Python standard library. Each receives `k`, `n` and a candidate list, and
checks that it is a permutation of `1..n` whose consecutive sums, closure
included, are perfect `k`-th powers. Neither reads a dump, an edge ID, a patch
history or anything produced by the solver.

The lower bound is a dozen lines, checked the same way by two verifiers of its
own, or replayed by `pc-scan`:

```console
xz -dc witness/k5-n246877/bands.txt.xz > /tmp/bands.txt
python src/verify/python/verify-bands.py 5 246877 /tmp/bands.txt
cargo run --release --bin pc-scan -- --bands --quick-only 5 1 246876 | diff - /tmp/bands.txt
```

## Tools

| binary | role |
|---|---|
| `pc-scan` | decides, over a range of `n`, which values a necessary condition rules out |
| `pc-search` | SAT-based 2-factor search with subtour elimination and in-place repair |
| `pc-patch` | pushes a stored dump further, outside the run that produced it |
| `pc-export` | extracts a canonical witness from a one-cycle dump |
| `pc-tsplib` | exports `G_k(n)` for the LKH-3 solver and turns its tour into a canonical witness |

Typical use:

```console
cargo build --release

./target/release/pc-scan --compact 5 1 250000     # where to look
./target/release/pc-search 5 246877 10            # search there
./target/release/pc-export <dump>.pcdump.json     # certify
```

At large `k` the first candidate is far too big to build: at `k = 7` its graph
would need about 36 GB. Bound the construction with `--max-graph-n=N` or
`--quick-only`, which report `UNCHECKED` instead of building. An unguarded scan
behaves exactly as before.

`pc-scan` reporting `CANDIDATE` means no implemented obstruction was found. It
does not prove that a Hamiltonian cycle exists.

[`doc/en/runbook.md`](doc/en/runbook.md) is the full checklist, from locating
the first candidate to a verified witness bundle.

## Approach

A Hamiltonian cycle is relaxed to a **2-factor**: one SAT variable per graph
edge, constrained so that every vertex has selected degree exactly two. A model
is then a disjoint union of cycles covering all vertices, and is Hamiltonian
exactly when there is one cycle. Subtour-elimination cuts are added lazily and
under a per-round budget, and each raw factor is pushed towards a single cycle by
combinatorial repair operators — `three-opt-111`, `three-opt-21` and a
depth-bounded alternating repair.

See [`doc/en/algorithms.md`](doc/en/algorithms.md) for the algorithms,
[`doc/en/architecture.md`](doc/en/architecture.md) for the crate layout and
[`doc/en/dump-format.md`](doc/en/dump-format.md) for the exchange format, and
[`doc/en/performance.md`](doc/en/performance.md) for what was tried, measured,
kept and dropped.

The two mathematical results behind the table above have their own documents:
[`doc/en/islands.md`](doc/en/islands.md) for the admissible set that gives every
lower bound, and [`doc/en/non-hamiltonicity.md`](doc/en/non-hamiltonicity.md)
for what a certificate looks like when the degree argument is not enough —
which so far has happened exactly once, at `k = 2`.

## Development

```console
cargo test --workspace              # fast suite
cargo test --release -- --ignored   # long cross-checks, about a minute
./scripts/check.fish                # everything, plus re-verification of
                                    # every committed witness
```

`scripts/check.fish` is the pre-publication gate: formatting, lints, tests,
release build, then for each bundle in `witness/` it re-derives the checksums,
runs both independent verifiers on each cycle and each lower-bound certificate,
and replays the certificate. It then builds a throwaway bundle for `k = 2`,
which is the only way to exercise the bundler itself, and checks
`scripts/islands-chain.py` against `pc-scan`.

## Licence

Source code is dual-licensed [MIT](LICENSE-MIT) OR
[Apache-2.0](LICENSE-APACHE), at your option.

`witness/` is data, not software, and is released under
[CC0 1.0](witness/LICENSE). See [LICENSE](LICENSE).

[Documentation française](README.fr.md)
