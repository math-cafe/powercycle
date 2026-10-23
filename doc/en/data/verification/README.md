# Full checks of the power-sum-graphs 1.1.0 witnesses

Two complete checks of the 13 bundles of power-sum-graphs 1.1.0 (23 659
witnesses), whose logs were first kept outside any repository.

- [`posum-check-1.1.log`](posum-check-1.1.log), 2026-10-21: in
  `power-sum-graphs`, `target/release/posum check --jobs 14`, then
  `echo "exit $?"`. Every bundle checked by both verifiers on both trees;
  13 bundles, 0 problems, exit 0.
- [`verify-bases-all.log`](verify-bases-all.log), 2026-10-22: in
  `power-sum-graphs/witness/bases`, every `*.bases.xz` given to
  [`../../../../scripts/verify-bases.sh`](../../../../scripts/verify-bases.sh)
  (which shares no code with posum) through
  [`verify-bases-one.sh`](verify-bases-one.sh), which reads `k` and `n` from
  the path and prints `OK` when the verdict and the identity prefix match:

  ```sh
  find . -name '*.bases.xz' | sed 's|^\./||' \
    | xargs -P 10 -n 1 verify-bases-one.sh | sort | uniq -c
  ```

  Result: `23659 OK`, no other line.
