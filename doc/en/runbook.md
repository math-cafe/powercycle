# Runbook: from a value of k to a published witness

A checklist for the whole chain — locate the first candidate `n`, search it for
a Hamiltonian cycle, and turn the result into a versioned witness bundle.

Every command below was run end to end on `k = 4` while writing this document.

---

## 0. Build

- [ ] `cargo build --release`
- [ ] optionally `source powercycle-init.fish`, which puts `target/release` on
      your `PATH` so the commands below can drop the prefix

---

## 1. Find the first candidate

The set of `n` that a Hamiltonian cycle is even arithmetically allowed to
occupy is described in [`islands.md`](islands.md). `pc-scan` computes it.

- [ ] bracket the islands without building anything:

      ./target/release/pc-scan --quick-only --compact 4 1 20000

      1..9640       IMPOSSIBLE
      9641..9999    UNCHECKED
      10000..18192  IMPOSSIBLE
      18193..20000  UNCHECKED

- [ ] confirm the first island start with the full preflight:

      ./target/release/pc-scan --compact 4 1 20000

      1..9640       IMPOSSIBLE
      9641..9999    CANDIDATE

`--quick-only` reports **`UNCHECKED`**, not `CANDIDATE`: it runs the graph-free
degree test and never builds a graph, so it rejects but never clears. Only a
run that builds the graph can print `CANDIDATE`. Use `--max-graph-n=N` to allow
graphs up to `N` and leave the rest `UNCHECKED`.

`CANDIDATE` means no implemented obstruction was found. It does not prove a
Hamiltonian cycle exists — `G_2(31)` is the standing counterexample, see
[`non-hamiltonicity.md`](non-hamiltonicity.md).

- [ ] **before a large `k`, check the memory floor by hand.** There is no guard
      yet. Section 6 of [`performance.md`](performance.md) gives

          floor = 32 * edges + 8 * n + 3.56 * literals

      2.4 GB at `k = 6`, 89 GB at `k = 7`.

---

## 2. Search the candidate

- [ ] `./target/release/pc-search --on-hamiltonian=stop 4 9641`

The run prints its directory under `data/runs/` on exit. Watch for:

      pc-search: tour 1 PATCHED FACTOR IS HAMILTONIAN

Flags that matter on long runs:

| flag | when |
|---|---|
| `--max-depth=N` | **the first thing to raise if the patch stalls** |
| `--checkpoint-secs=N` | default 600; a crash loses at most that much |
| `--dump=records` | keep only improving dumps instead of every tour |
| `--no-patch` | stop after the raw SAT factor, to patch separately |

On `--max-depth`: at `k = 6` the patch sat at a local fixed point of seven
cycles for about 34 hours at depth 4, and closed in two minutes at depth 9. If
the cycle count stops moving, raise the depth before anything else — see
section 2.6 of [`performance.md`](performance.md).

- [ ] if the search ends short of one cycle, resume from the dump rather than
      restarting:

          ./target/release/pc-patch --max-depth=9 <dump>.pcdump.json

---

## 3. Export the cycle

- [ ] pick the one-cycle dump — its name carries `c0001`:

      ls data/runs/<run-id>/dumps/*patched*c0001*.pcdump.json

- [ ] `./target/release/pc-export <dump>.pcdump.json > cycle.txt`

**The cycle goes to stdout and the log to stderr.** Never write
`> cycle.txt 2>&1`: it prepends three log lines to the witness and every
checksum downstream will fail.

- [ ] sanity check: `wc -l cycle.txt` must equal `n`, and the first line is `1`

The export is canonical — rotated so vertex `1` comes first, oriented so the
smaller neighbour of `1` comes second — so the same cycle always yields the
same bytes.

---

## 4. Produce the lower-bound certificate

- [ ] `./target/release/pc-scan 4 1 9641 > scan.txt`

Plain output, no `--compact`: one line per `n`, each rejection naming the
offending vertex and its degree, then the `CANDIDATE` verdict at the threshold.
This is the half of the result that establishes minimality.

Rejections need no graph, but the threshold line does: the last `n` is built
and preflighted exactly as in step 1. Do not bound it with `--max-graph-n` —
`check.fish` replays `pc-scan <k> 1 <n>` unbounded and compares byte for byte,
so an `UNCHECKED` last line fails verification.

At `k = 7` the run took 28 min: about 8 for the graph, the rest for the
587 761 722 rejections and their 33 GB of output, against 4 min 30 for the same
range with `--compact`. The memory peak is the preflight's, 56 GB
plus 6 GB of swap, see section 6 of [`performance.md`](performance.md).

Skip this step only when `n` is not the first admissible value, in which case
there is no minimality to certify.

---

## 5. Assemble the bundle

- [ ] run the bundler:

          fish scripts/prepare-witness.fish 4 9641 \
              cycle.txt <dump>.pcdump.json scan.txt

It writes `witness/k<k>-n<n>/`, storing each artifact xz-compressed next to
the SHA-256 of its **uncompressed** form.

When a checksum file already exists it must match, so re-running the script on
a published bundle is a verification rather than a silent overwrite. A fresh
search reproduces `cycle.txt` byte for byte but produces a different
`final.pcdump.json`, which carries run-specific provenance — that mismatch is
expected and the script is right to refuse.

- [ ] write `witness/k<k>-n<n>/README.md`, and `README.fr.md` if you want the
      French version. Follow an existing bundle: the mathematical statement, the
      minimality argument, the checksums, the provenance chain with timings, and
      the independent verification commands.

---

## 6. Verify before publishing

- [ ] `fish scripts/check.fish`

Formatting, clippy with warnings denied, the test suite, a release build, then
for every bundle: checksums, both independent verifiers, and a replay of the
scan certificate compared byte for byte. Finally it runs this whole runbook in
miniature on `k = 2`, building a throwaway bundle and removing it, since
verifying committed bundles never exercises the bundler.

- [ ] update the results table in `README.md` and `README.fr.md`
- [ ] update `CITATION.cff` if the result set changed

---

## Where each artifact ends up

| path | versioned | what |
|---|---|---|
| `data/runs/<run-id>/` | no | working state: log, status, dumps, checkpoints |
| `witness/k<k>-n<n>/` | yes | the published bundle |

`data/` is local scratch. Anything that must survive belongs in a bundle.
