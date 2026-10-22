# Evidence on the mainland, 2026-10-21

The conjecture of [`islands.md`](islands.md), section 6, says that for
`k >= 3` every admissible `n` is Hamiltonian. The islands of `k = 4` and
`k = 5` were checked value by value for power-sum-graphs 1.0.0. These runs
test the start of the mainland, where the heuristic of paper 1 (section 7.4)
expects the graphs to be hardest.

| k | values | result | data |
|---|---|---|---|
| 3 | every `n` from 801 to 10000 (9200) | all Hamiltonian, 1 trial each | [`data/sweep-k3-801-10000.tsv`](data/sweep-k3-801-10000.tsv) |
| 4 | every `n` from 18194 to 30000 (11807) | all Hamiltonian, 1 trial each, about 1 s | [`data/sweep-k4-18194-30000.tsv`](data/sweep-k4-18194-30000.tsv) |
| 6 | 30039769, start of the mainland | Hamiltonian, 3 trials, 4820 s | [`../../witness/k6-n30039769/`](../../witness/k6-n30039769/) |
| 6 | 16777215, end of the island | Hamiltonian, with a motif imposed; every other search stalled ([`end-of-island.md`](end-of-island.md)) | [`../../witness/k6-n16777215/`](../../witness/k6-n16777215/) |

With the 328 cycles of [`../../witness/k3-n473-800/`](../../witness/k3-n473-800/),
every `n` from 473 to 10000 is Hamiltonian for `k = 3`. With the island bundle
of power-sum-graphs 1.0.0 and [`../../witness/k4-n18193/`](../../witness/k4-n18193/),
every `n` of `[9641, 9999] U [18193, 30000]` is Hamiltonian for `k = 4`.

Each table lists, for every `n`, the SHA-256 identity of the cycle found,
the time and the number of LKH trials; every cycle was checked by posum at the
end of its search. The cycles themselves (about 90 MB in the bases encoding)
are not in Git: they will be published as bundles in power-sum-graphs 1.1.0.
Until then the runs are reproducible with
[`scripts/continent-sweeps.sh`](../../scripts/continent-sweeps.sh) (posum
1.0.0, LKH-3.0.14); a cycle found again may differ, since LKH is randomised,
but any cycle settles its `n`.
