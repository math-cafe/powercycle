# powercycle architecture

## 1. Design principles

powercycle follows a Unix-style architecture built around small tools with
clear responsibilities.

The main principles are:

- mathematical structures are independent of solvers and command-line tools;
- persistent dumps are the exchange format between powercycle programs;
- generated working data is separated from versioned source code;
- Hamiltonian witnesses are simpler than internal dumps;
- witness verification is independent of all powercycle libraries;
- all algorithms are generic in `k` and `n`.

The workflow has two entry points.

Deciding whether a given `n` is worth attacking at all:

```text
pc-scan
    ↓
IMPOSSIBLE  (a necessary condition fails)
    or
CANDIDATE   (no implemented obstruction found)
```

Producing and certifying a witness:

```text
pc-search
    ↓
2-factor dump, patched in place towards a single cycle
    ↓
one-cycle dump
    ↓
pc-export
    ↓
Hamiltonian witness
    ↓
independent Rust / Python verification
```

`pc-search` applies the patching operators itself between SAT tours, so the
common path needs no separate patching step. `pc-patch` exists for the case
where a stored dump is to be pushed further, or with a different repair depth,
outside the run that produced it.

## 2. Repository layout

```text
powercycle/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── CITATION.cff
├── LICENSE
├── powercycle.toml
├── powercycle.local.toml.example
├── powercycle-init.fish
├── README.md
├── README.fr.md
│
├── src/
│   ├── powercycle/
│   │   ├── config/
│   │   ├── core/
│   │   ├── format/
│   │   ├── search/
│   │   ├── patch/
│   │   └── bin/
│   │       ├── pc-scan.rs
│   │       ├── pc-search.rs
│   │       ├── pc-patch.rs
│   │       └── pc-export.rs
│   │
│   └── verify/
│       ├── rust/
│       └── python/
│
├── doc/
│   └── en/
│
├── data/
│   ├── runs/
│   ├── dumps/
│   └── certificates/
│
├── witness/
└── target/
```

All source files live under `src/`.

`data/`, Cargo build directories and `powercycle.local.toml` contain local
state and are not versioned.

`witness/` is versioned and reserved for selected reproducible results.

## 3. Crate boundaries

### `powercycle-core`

Pure mathematical in-memory structures.

Current responsibilities include:

- graph vertices and edges;
- deterministic graph construction;
- factors;
- 2-factor validation;
- canonical cycle decomposition.

It contains no solver, CLI, logging, configuration or persistence code.

### `powercycle-config`

Shared configuration and path resolution.

It contains no mathematical, solver or patching logic.

### `powercycle-format`

Persistent exchange formats shared by the powercycle tools.

It depends on `powercycle-core` in order to validate loaded mathematical
states.

Current responsibilities include:

- versioned JSON dump schema;
- graph identity;
- provenance;
- factor records;
- move history;
- derived summaries;
- integrity hashes;
- replay validation.

### `powercycle-search`

Search and 2-factor generation.

It will contain solver-facing logic, long-running search state, checkpointing
and factor scoring.

### `powercycle-patch`

Factor analysis and transformations.

It will contain patchability analysis and the algorithms used to transform
promising 2-factors toward Hamiltonian cycles.

## 4. Executables

### `pc-scan`

Reports, for every `n` in a range, whether an implemented necessary condition
rules `G_k(n)` out. Each `n` is first tested with the graph-free degree check;
only the survivors pay for a graph construction and the full preflight.

```console
pc-scan [--compact] [--max-graph-n=N | --quick-only] <k> <start> <end>
```

`--max-graph-n` and `--quick-only` bound the graph construction. Beyond the
bound the verdict is `UNCHECKED` rather than `CANDIDATE`, because the full
preflight did not run. An unguarded scan never reports `UNCHECKED`, so the
published certificates are unaffected.

A `CANDIDATE` verdict means no implemented obstruction was found. It does not
prove that a Hamiltonian cycle exists.

### `pc-search`

Long-running producer of 2-factor dumps. Alternates SAT tours with budgeted
subtour-elimination cuts, and applies the patching operators to each raw factor
in search of a single cycle.

```console
pc-search [--probe-only | --no-patch] [--propagated-units] [--max-depth=N]
          [--checkpoint-secs=N] [--dump=always|records|never]
          [--on-hamiltonian=stop|continue] <k> <n> [max-tours]
```

`--max-depth` bounds the alternating repair, matching the `pc-patch` flag of
the same name. `--checkpoint-secs` sets how often a long patch phase yields to
be persisted; the default is 600.

`--no-patch` solves, persists the raw factor and stops. Capturing an expensive
SAT factor becomes an unattended operation, which matters when the solve takes
hours and the patching that follows it can take days. Work on the factor then
continues offline with `pc-patch`.

### `pc-patch`

Reads a stored dump, drives the patching operators over it and writes a derived
dump. Useful for resuming work on a saved factor at a different repair depth.

```console
pc-patch [--max-depth=N] <dump.pcdump.json>
```

### `pc-export`

Exports a canonical Hamiltonian-cycle witness on stdout from a one-cycle dump.
Refuses any dump whose factor is not a single cycle.

```console
pc-export <dump.pcdump.json>
```

## 5. Independent verification

Verification is intentionally outside the powercycle dependency graph.

```text
src/verify/
├── rust/
└── python/
```

The Rust verifier is its own Cargo project and is excluded from the main
workspace.

The Python verifier uses only the Python standard library.

Neither verifier depends on powercycle libraries or reads powercycle dumps.
A verifier receives only `k`, `n` and a candidate witness, then checks:

1. every integer in `1..=n` occurs exactly once;
2. every consecutive sum is a perfect `k`-th power;
3. the closing sum between the last and first vertices is also a perfect
   `k`-th power.

## 6. Configuration and path resolution

Shared configuration is loaded with the following precedence:

```text
built-in defaults
        ↓
powercycle.toml
        ↓
powercycle.local.toml
        ↓
environment variables
        ↓
command-line overrides
```

The problem parameters `k` and `n` are never global configuration values;
they belong to each command invocation.

By default, `data_dir` is resolved relative to the project root, while
`runs`, `dumps` and `certificates` are resolved relative to the effective
`data_dir`.

If no explicit root is supplied, the configuration loader walks upward from
the current directory until it finds `powercycle.toml`. This allows the tools
to work without requiring `powercycle-init.fish` to have been sourced.

The shared configuration layer recognizes:

```text
POWERCYCLE_ROOT
POWERCYCLE_CONFIG
POWERCYCLE_LOCAL_CONFIG
POWERCYCLE_DATA
```

Programs that write data create their configured working directories
idempotently before use.

## 7. Streams and logging

Unix stream semantics are part of the public behavior of the tools.

`stdout` contains the primary machine-consumable result of a command.

`stderr` contains progress, warnings and diagnostics.

Long-running tools such as `pc-search` additionally maintain persistent run
logs containing, among other metadata:

- start time;
- program and version;
- argument vector;
- reconstructed command;
- working directory;
- effective configuration;
- `k` and `n`;
- progress and solver state.

## 8. Working data

Search runs own the dumps they produce:

```text
data/runs/<run-id>/
├── run.toml
├── search.log
├── status.json
├── checkpoint/
└── dumps/
```

`checkpoint/` holds crash-recovery state, kept deliberately separate from the
curated dumps that `--dump` governs. The raw factor is written there as soon as
a SAT tour returns one, before any patching, and each long patch phase yields on
a timer so its partial progress is persisted too. Only the latest state per
stage is kept, written through a temporary file and renamed, since a truncated
checkpoint is worse than none.

This matters because the costs are wildly asymmetric: the SAT solve took over
four hours on `G_6(15748961)` while the dump for that tour is only written once
patching finishes, which can take far longer still. A checkpoint turns a lost
run into `pc-patch <run>/checkpoint/raw.pcdump.json`.

A run killed rather than interrupted still leaves `status.json` reporting
`running` with no process behind it; nothing reaps those yet.

`data/dumps/` is reserved for imported or manually selected dumps outside a
specific run.

`data/certificates/` contains temporary or unpublished witness exports.

None of `data/` is versioned.

## 9. Witnesses

`witness/` contains selected versioned scientific results.

A witness bundle contains:

```text
witness/k5-n246877/
├── README.md
├── README.fr.md
├── cycle.txt.zst              existence
├── cycle.txt.sha256
├── scan.txt.zst               lower bound, when minimality is claimed
├── scan.txt.sha256
├── final.pcdump.json.zst      provenance
└── final.pcdump.json.sha256
```

The cycle witness is the mathematically essential object. Every artifact is
stored compressed next to the SHA-256 of its **uncompressed** form, so the
checksum identifies the mathematical content rather than a particular
compression.

A bundle that claims a minimal `n` carries a second certificate. `scan.txt` is
the `pc-scan` output over `1 .. n`, which records the obstruction found at every
smaller value. Existence and minimality are then both reproducible from the
bundle alone.

Bundles are built by `prepare-witness.fish`, which verifies any checksum already
present rather than overwriting it silently.

`witness/` is released under CC0; the rest of the repository is dual-licensed
MIT OR Apache-2.0.

## 10. Mathematical results

[`islands.md`](islands.md) characterises `I_k = { n : delta(G_k(n)) >= 2 }`,
the set every lower bound comes from, and gives it for `k = 2` to `12` with the
three independent derivations that agree on it.

[`non-hamiltonicity.md`](non-hamiltonicity.md) covers the other side: the
`f_2` formulation the pipeline actually descends, the complete certificate for
`G_2(31)` — the one known case where the degree argument is not enough — and
which certificate forms scale.

## 11. Performance

Optimisation attempts, their measurements and their verdicts are recorded in
[`performance.md`](performance.md), failures included. Three plausible ideas
were dropped because the numbers contradicted them; without that record each
one invites being re-derived.

## 12. Pre-publication check

`scripts/check.fish` runs formatting, lints, the test suite, the long
cross-checks and a release build, then re-derives every claim in `witness/`:
checksums, both independent verifiers, and a byte-for-byte replay of each
lower-bound certificate.

It is the single command to run before publishing a result.

## 13. Documentation policy

English documentation is authoritative during active development.

The French documentation tree mirrors the English structure, but complete
translations are produced at release or publication milestones rather than
being maintained continuously while the project is evolving.
