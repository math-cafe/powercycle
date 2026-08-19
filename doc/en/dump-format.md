# powercycle dump format

## 1. Purpose

A powercycle dump is the persistent representation of a 2-factor of a
power-sum graph.

Dump files are used as the exchange format between the powercycle tools:

- `pc-search` produces promising factors;
- `pc-analyze` inspects them;
- `pc-patch` transforms them;
- `pc-export` extracts a Hamiltonian-cycle witness from a one-cycle factor.

A dump contains both the current mathematical state and enough provenance
information to describe how that state was obtained.

The dump format is deliberately independent of the SAT solver, patching
algorithms and command-line interfaces.

The file format is JSON and is explicitly versioned.

Current format:

```text
format  = "powercycle-dump"
version = 1
```

The conventional filename suffix is:

```text
.pcdump.json
```

---

## 2. Mathematical context

For integers `k >= 2` and `n >= 1`, the power-sum graph

```text
G_k(n)
```

has vertex set

```text
{1, 2, ..., n}
```

and an undirected edge `{x, y}`, with `x != y`, whenever

```text
x + y = a^k
```

for some integer `a >= 2`.

The graph implementation assigns a deterministic zero-based `EdgeId` to
every graph edge.

A factor stored in a dump is represented by the list of its selected
`EdgeId` values.

A valid dump always stores valid 2-factors: every vertex has selected degree
exactly two.

Therefore every factor is a disjoint union of cycles.

---

## 3. Top-level structure

A version-1 dump has the following structure:

```json
{
  "format": "powercycle-dump",
  "version": 1,
  "dump_id": "...",

  "graph": {
    "k": 5,
    "n": 490463,
    "edge_count": 833318,
    "sha256": "..."
  },

  "provenance": {
    "producer": "pc-search",
    "producer_version": "0.1.0",
    "session": "...",
    "invocation": 1,
    "tour": 767,
    "reason": [
      "patchability-record"
    ],
    "parent_dump_id": null
  },

  "base_factor": {
    "selected_edges": []
  },

  "current_factor": {
    "selected_edges": []
  },

  "history": [],

  "summary": {
    "cycles": 14,
    "giant_size": 489955,
    "residual_vertices": 508,
    "hamiltonian": false
  },

  "integrity": {
    "algorithm": "sha256",
    "state_sha256": "..."
  }
}
```

---

## 4. Header

### `format`

```json
"format": "powercycle-dump"
```

Identifies the file as a powercycle dump.

Version 1 requires this exact string.

### `version`

```json
"version": 1
```

Identifies the dump schema version.

A reader must reject unsupported versions rather than silently interpreting
them as another schema.

### `dump_id`

```json
"dump_id": "..."
```

The dump identifier is the hexadecimal SHA-256 digest of the semantic dump
state defined in section 11.

For version 1:

```text
dump_id == integrity.state_sha256
```

The dump ID is therefore independent of JSON whitespace, indentation and
object formatting.

---

## 5. Graph metadata

The `graph` object identifies the exact graph and, importantly, the exact
`EdgeId` ordering used by the dump.

```json
"graph": {
  "k": 5,
  "n": 490463,
  "edge_count": 833318,
  "sha256": "..."
}
```

### `k`

The exponent of the power-sum graph.

### `n`

The number of vertices.

The vertex set is exactly:

```text
1 .. n
```

### `edge_count`

The number of edges in the reconstructed graph.

### `sha256`

Stable graph identity as defined in section 10.

The graph checksum includes every edge endpoint in deterministic `EdgeId`
order.

Consequently, two implementations that construct the same mathematical edge
set in different orders do **not** silently share the same graph identity.

This is essential because factors are stored as arrays of `EdgeId`.

---

## 6. Provenance

The `provenance` object records how the dump was produced.

```json
"provenance": {
  "producer": "pc-search",
  "producer_version": "0.1.0",
  "session": "20260819-...",
  "invocation": 1,
  "tour": 767,
  "reason": [
    "patchability-record"
  ],
  "parent_dump_id": null
}
```

### Required fields

#### `producer`

Name of the program that created the dump.

Typical values include:

```text
pc-search
pc-patch
```

#### `producer_version`

Version of the producing program.

### Optional fields

#### `session`

Identifier of the search or processing session.

#### `invocation`

Invocation number inside a session or workflow.

#### `tour`

Search-tour number when relevant.

#### `reason`

Zero or more human-readable machine-generated reasons explaining why the
dump was retained.

Examples may include:

```text
cycle-record
giant-record
patchability-record
hamiltonian
```

The precise vocabulary is not part of the version-1 mathematical semantics.

#### `parent_dump_id`

Dump ID of the immediate parent dump when the current dump was derived from
another dump.

An initial dump normally has no parent.

---

## 7. Factors

Both `base_factor` and `current_factor` use the same representation:

```json
{
  "selected_edges": [
    12,
    18,
    27
  ]
}
```

### Canonical edge sequence

The `selected_edges` array must be strictly increasing.

Therefore it contains:

- no duplicates;
- no invalid ordering;
- only graph edge IDs in the range

```text
0 .. edge_count - 1
```

### `base_factor`

The factor from which the recorded history starts.

For a dump produced directly by `pc-search`, the base factor and current
factor are normally identical and history is empty.

### `current_factor`

The factor represented by the dump after replaying all recorded history.

Both factors must independently be valid 2-factors of the graph.

---

## 8. History

The `history` array records transformations from the base factor to the
current factor.

Example:

```json
"history": [
  {
    "kind": "three-opt-21",
    "removed": [
      184,
      921,
      1725
    ],
    "added": [
      192,
      936,
      1701
    ]
  }
]
```

Each history element contains:

### `kind`

A textual identifier for the transformation.

Examples may include:

```text
three-opt-111
three-opt-21
alternating-repair
```

The version-1 dump format does not attach algorithm-specific semantics to
these names.

The mathematical transformation is completely determined by the `removed`
and `added` arrays.

### `removed`

Graph edges removed from the selected factor.

### `added`

Graph edges added to the selected factor.

Both arrays must be strictly increasing and contain valid graph edge IDs.

---

## 9. History replay invariants

History is replayed starting from `base_factor`.

For every history entry:

1. each removed edge must currently be selected;
2. each added edge must currently be unselected;
3. all edge IDs must exist in the graph;
4. after the complete move, every affected vertex must again have selected
   degree exactly two.

After replaying the complete history, the resulting selected-edge set must
be exactly equal to `current_factor.selected_edges`.

A dump for which replay does not reproduce the current factor is invalid.

This gives the following invariant:

```text
base_factor + history = current_factor
```

where `+ history` means sequential application of every recorded move.

---

## 10. Graph SHA-256

Version 1 uses SHA-256 to identify the graph and its deterministic edge-ID
ordering.

The hash input begins with the domain separator:

```text
powercycle-graph-v1\0
```

It is followed by:

1. `k`;
2. `n`;
3. graph edge count;
4. every graph edge in `EdgeId` order:
   - first endpoint;
   - second endpoint.

Endpoints of every undirected edge are stored canonically:

```text
first < second
```

### Integer encoding

`k` is encoded as an unsigned 32-bit integer in big-endian byte order.

All other integer values are encoded as unsigned 64-bit integers in
big-endian byte order.

Although Rust internally uses `usize` for several graph quantities, the
persistent hash encoding is therefore independent of machine word size.

The resulting digest is stored as 64 lowercase hexadecimal characters.

---

## 11. Dump-state SHA-256

Version 1 also computes a semantic SHA-256 identity for the complete dump
state.

The hash input begins with:

```text
powercycle-dump-state-v1\0
```

It then contains, in a fixed order:

### Header

```text
format
version
```

### Graph

```text
k
n
edge_count
graph.sha256
```

### Provenance

```text
producer
producer_version
session
invocation
tour
reason[]
parent_dump_id
```

### Factors

```text
base_factor.selected_edges[]
current_factor.selected_edges[]
```

### History

For every move:

```text
kind
removed[]
added[]
```

### Derived summary

```text
cycles
giant_size
residual_vertices
hamiltonian
```

Strings and arrays are length-prefixed.

Optional values include an explicit presence marker.

Boolean values are encoded as one byte:

```text
0 = false
1 = true
```

The result is stored in:

```text
integrity.state_sha256
```

and must also equal:

```text
dump_id
```

The semantic hash deliberately does not depend on the textual JSON
serialization.

Therefore these changes do not affect a dump ID:

- indentation;
- whitespace;
- line endings used around JSON tokens;
- pretty-printing choices.

Changes to semantic fields do affect the dump ID.

---

## 12. Summary

The `summary` object contains derived information about `current_factor`.

```json
"summary": {
  "cycles": 14,
  "giant_size": 489955,
  "residual_vertices": 508,
  "hamiltonian": false
}
```

### `cycles`

Number of cycles in the current 2-factor.

### `giant_size`

Number of vertices in the largest cycle.

If several cycles have the same maximum size, the canonical first one is
used as the giant cycle.

### `residual_vertices`

Number of vertices outside the giant cycle:

```text
n - giant_size
```

### `hamiltonian`

Equivalent to:

```text
cycles == 1
```

Summary information is redundant by design.

A reader must recompute it from `current_factor` and reject a dump whose
stored summary disagrees with the reconstructed value.

---

## 13. Integrity object

Version 1 uses:

```json
"integrity": {
  "algorithm": "sha256",
  "state_sha256": "..."
}
```

### `algorithm`

Must be:

```text
sha256
```

### `state_sha256`

Must equal the recomputed semantic dump-state hash.

It must also equal `dump_id`.

---

## 14. Full validation procedure

A conforming reader should validate a dump in the following order.

### 1. Parse JSON

The file must be syntactically valid JSON matching the expected schema.

### 2. Validate format header

Require:

```text
format  == "powercycle-dump"
version == 1
```

### 3. Reconstruct the graph

Construct:

```text
G_k(n)
```

from the graph metadata.

### 4. Validate graph metadata

Check:

```text
edge_count
graph.sha256
```

against the reconstructed graph.

### 5. Validate factor edge arrays

For both factors:

- every edge ID exists;
- edge IDs are strictly increasing;
- no duplicates occur.

### 6. Validate both factors

Every vertex must have selected degree exactly two.

### 7. Replay history

Starting from `base_factor`, apply all history moves.

Every move must preserve the 2-factor condition.

### 8. Compare replayed state

The replay result must exactly equal `current_factor`.

### 9. Recompute summary

Decompose `current_factor` into cycles and verify the stored summary.

### 10. Recompute semantic integrity hash

Require:

```text
integrity.algorithm == "sha256"
integrity.state_sha256 == recomputed state hash
dump_id == recomputed state hash
```

Only after all these checks is a dump considered valid.

---

## 15. Initial dumps

A dump produced directly from a search result normally has:

```text
base_factor    == current_factor
history        == []
parent_dump_id == null
```

For example:

```json
{
  "base_factor": {
    "selected_edges": [1, 4, 7]
  },

  "current_factor": {
    "selected_edges": [1, 4, 7]
  },

  "history": []
}
```

---

## 16. Derived dumps

A dump produced by transforming another dump normally has:

```text
parent_dump_id = previous dump_id
```

Its `base_factor` remains the base factor from which its history is replayed.

The history therefore provides a complete transformation path from the
stored base factor to the current factor.

---

## 17. Dump versus witness

A dump is **not** the final mathematical witness distributed for independent
verification.

A dump contains project-specific information such as:

- graph edge IDs;
- provenance;
- transformation history;
- derived metadata.

A Hamiltonian-cycle witness is deliberately much simpler.

It contains only a permutation:

```text
T[0], T[1], ..., T[n-1]
```

of:

```text
1 .. n
```

such that every consecutive sum, including the closing sum,

```text
T[n-1] + T[0]
```

is a perfect `k`-th power.

The independent Rust and Python verification programs operate on the witness,
not on powercycle dumps.

This separation is intentional.

---

## 18. Compatibility policy

Dump format versions are explicit.

A reader must not silently reinterpret an unsupported version.

Future schema changes that alter persistent semantics should increment the
version number.

Version-1 dumps are intended to remain readable by future versions of the
powercycle tools, either directly or through an explicit migration mechanism.

No backward-compatibility requirement exists for experimental dump formats
that predate this specification.
