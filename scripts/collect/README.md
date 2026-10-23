# Collecting the cycles found before power-sum-graphs

The two scripts that gathered the Hamiltonian cycles found by this notebook
into the unversioned staging directory `data/witnesses/`, each cycle in
canonical order and checked by both verifiers of this repository. Copied on
2026-10-23 from the unversioned `tmp/`, unchanged; their paths are those of
that time.

- `collect_witnesses.py`: every cycle found under `tmp/` (final LKH tours of
  cost 0, converted cycles, the `k = 3` cycles of
  [`../../witness/k3-n473-800/`](../../witness/k3-n473-800/)); writes
  `inventory.csv`. `collect.log` is its output.
- `inventory-k4-island.csv`, the 359 cycles of the `k = 4` island, was written
  by a command that was not saved.
- `collect_h5.py`: the 1956 cycles of the `k = 5` island archive; writes
  `inventory-h5.csv`.

The SHA-256 lists [`../../doc/en/data/k4-island-cycles.csv`](../../doc/en/data/k4-island-cycles.csv)
and [`../../doc/en/data/k5-island-cycles.csv`](../../doc/en/data/k5-island-cycles.csv)
come from these inventories. They identify the notebook's cycles, not the
witnesses published in power-sum-graphs, which are other cycles of the same
graphs.
