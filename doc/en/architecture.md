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

The intended workflow is:

```text
pc-search
    ↓
promising 2-factor dump
    ↓
pc-analyze
    ↓
pc-patch
    ↓
new dump
    ↓
...
    ↓
one-cycle factor
    ↓
pc-export
    ↓
Hamiltonian witness
    ↓
independent Rust / Python verification
```

## 2. Repository layout

```text
powercycle/
├── Cargo.toml
├── Cargo.lock
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
│   │       ├── powercycle.rs
│   │       ├── pc-search.rs
│   │       ├── pc-analyze.rs
│   │       ├── pc-patch.rs
│   │       └── pc-export.rs
│   │
│   └── verify/
│       ├── rust/
│       └── python/
│
├── doc/
│   ├── en/
│   └── fr/
│
├── bin/
├── data/
│   ├── runs/
│   ├── dumps/
│   └── certificates/
│
├── witness/
└── target/
```

All source files live under `src/`.

`bin/`, `data/`, Cargo build directories and `powercycle.local.toml` contain
local state and are not versioned.

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

### `pc-search`

Long-running producer of promising 2-factor dumps.

### `pc-analyze`

Read-only inspection and analysis of a dump.

### `pc-patch`

Transforms a dump and writes a derived dump.

### `pc-export`

Exports a Hamiltonian-cycle witness from a one-cycle dump.

### `powercycle`

Optional convenience façade exposing the same functionality through
subcommands.

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

`data/dumps/` is reserved for imported or manually selected dumps outside a
specific run.

`data/certificates/` contains temporary or unpublished witness exports.

None of `data/` is versioned.

## 9. Witnesses

`witness/` contains selected versioned scientific results.

A witness bundle may eventually contain:

```text
witness/k5-n490463/
├── README.md
├── README.fr.md
├── cycle.txt.zst
├── cycle.txt.sha256
├── final.pcdump.json.zst
└── final.pcdump.json.sha256
```

The cycle witness is the mathematically essential object.

The final dump provides additional provenance and reproducibility material.

## 10. Documentation policy

English documentation is authoritative during active development.

The French documentation tree mirrors the English structure, but complete
translations are produced at release or publication milestones rather than
being maintained continuously while the project is evolving.
