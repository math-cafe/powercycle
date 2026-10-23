# End of the k = 6 island: the tests that found nothing

The tools behind sections 1 to 3 of
[`../../doc/en/end-of-island.md`](../../doc/en/end-of-island.md): searches and
obstruction tests on `G_6(16777215)` run on 2026-10-21 and 2026-10-22, before
the motif of [`../end-of-island/`](../end-of-island/) gave a cycle. None of
them proved or disproved anything; they are kept as the record of what was
tried. Each directory is a separate crate on posum-core and posum-lkh
(power-sum-graphs, expected next to this repository), built with
`cargo build --release`.

- [`hall/`](hall/): `hall K N [OUT]`, Hopcroft-Karp on the bipartite double
  cover, a perfect 2-matching of `G_K(N)` or a Hall set certifying that none
  exists; `fhall K N [OUT]`, the same test with capacities `f` (one free edge
  at a chain end, two elsewhere) on the contracted graph. On `G_6(16777215)`
  both found a perfect matching (21 s and 54 s).
- [`posa/`](posa/): `posa K N TOUR OUT [SECONDS] [SEED]`, Posa
  rotation-extension by random walk from the paths of an LKH tour cut at its
  non-edges; `posa2 K N TOUR OUT [DEPTH] [CAP]`, the rotation sets by
  breadth-first search. The logs are the runs on the 7-tour: one join in
  357 s, then none in 3 h (`k6-posa.log`); no join at depth 40
  (`k6-posa2.log`).
- [`repair/`](repair/): `repair K N TOUR RADIUS OUT.pcdump.json`, keeps the
  tour outside graph balls around its non-edges and lets CaDiCaL choose the
  edges inside. It also depends on the powercycle crates of this repository.
  `perturb-test.sh` damages a tour of `G_4(18193)` and repairs it, to check
  the encoding; it reads the run directory `tmp/runs/k4-n18193`, which is not
  versioned.
- [`defects/`](defects/): `defects K N TOUR...` lists the non-edges of LKH
  tours with the degrees of their ends; `classes K N [TOUR...]` classifies the
  contracted nodes by degree and chain offset and places the non-edges in
  that classification. `report-1.txt` and `classes-report.txt` are their
  output on the `k = 6` tours, `classes-15748961.txt` the same classification
  for `G_6(15748961)`. The other tools of this crate became
  [`../end-of-island/`](../end-of-island/).

Copied on 2026-10-23 from the unversioned `tmp/` of this repository, with the
dependency paths made relative; the code is otherwise unchanged.
