#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import sys


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Independently verify a power-sum Hamiltonian cycle witness. "
            "This script uses only the witness file and Python's standard library."
        )
    )
    parser.add_argument("k", type=int, help="power exponent, k >= 2")
    parser.add_argument("n", type=int, help="vertex count, n >= 3")
    parser.add_argument("cycle", type=Path, help="one integer per line")
    return parser.parse_args()


def read_cycle(path: Path, expected_n: int) -> tuple[list[int], bytes]:
    data = path.read_bytes()

    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as exc:
        raise ValueError(f"witness is not ASCII: {exc}") from exc

    lines = text.splitlines()

    if len(lines) != expected_n:
        raise ValueError(
            f"wrong witness length: expected {expected_n}, found {len(lines)}"
        )

    cycle: list[int] = []

    for line_number, line in enumerate(lines, start=1):
        if not line:
            raise ValueError(f"blank line at line {line_number}")

        if line.strip() != line:
            raise ValueError(
                f"unexpected leading or trailing whitespace at line {line_number}"
            )

        try:
            vertex = int(line, 10)
        except ValueError as exc:
            raise ValueError(
                f"invalid integer {line!r} at line {line_number}"
            ) from exc

        cycle.append(vertex)

    return cycle, data


def validate_permutation(cycle: list[int], n: int) -> None:
    seen = bytearray(n + 1)

    for position, vertex in enumerate(cycle, start=1):
        if vertex < 1 or vertex > n:
            raise ValueError(
                f"vertex {vertex} at position {position} is outside 1..={n}"
            )

        if seen[vertex]:
            raise ValueError(
                f"duplicate vertex {vertex} at position {position}"
            )

        seen[vertex] = 1

    for vertex in range(1, n + 1):
        if not seen[vertex]:
            raise ValueError(f"missing vertex {vertex}")


def integer_kth_powers(k: int, maximum: int) -> set[int]:
    if k < 2:
        raise ValueError(f"invalid exponent k={k}: expected k >= 2")

    powers: set[int] = set()
    base = 1

    while True:
        power = base**k

        if power > maximum:
            break

        powers.add(power)
        base += 1

    return powers


def validate_adjacencies(
    cycle: list[int],
    powers: set[int],
) -> dict[int, int]:
    counts: dict[int, int] = {}

    for index, first in enumerate(cycle):
        second = cycle[(index + 1) % len(cycle)]
        total = first + second

        if total not in powers:
            raise ValueError(
                f"invalid adjacency at position {index + 1}: "
                f"{first} + {second} = {total} is not a perfect kth power"
            )

        counts[total] = counts.get(total, 0) + 1

    return counts


def exact_kth_root(value: int, k: int) -> int:
    low = 1
    high = 2

    while high**k < value:
        high *= 2

    while low <= high:
        middle = (low + high) // 2
        power = middle**k

        if power == value:
            return middle

        if power < value:
            low = middle + 1
        else:
            high = middle - 1

    raise ValueError(f"{value} is not a perfect {k}th power")


def main() -> int:
    args = parse_args()

    if args.k < 2:
        print(
            f"powercycle-verify.py: INVALID: k={args.k}, expected k >= 2",
            file=sys.stderr,
        )
        return 1

    if args.n < 3:
        print(
            f"powercycle-verify.py: INVALID: n={args.n}, expected n >= 3",
            file=sys.stderr,
        )
        return 1

    try:
        cycle, data = read_cycle(args.cycle, args.n)
        validate_permutation(cycle, args.n)

        powers = integer_kth_powers(
            args.k,
            2 * args.n - 1,
        )

        counts = validate_adjacencies(
            cycle,
            powers,
        )
    except (OSError, ValueError) as exc:
        print(
            f"powercycle-verify.py: INVALID: {exc}",
            file=sys.stderr,
        )
        return 1

    sha256 = hashlib.sha256(data).hexdigest()

    canonical = (
        len(cycle) >= 3
        and cycle[0] == 1
        and cycle[1] < cycle[-1]
    )

    print("VALID HAMILTONIAN CYCLE")
    print(f"k              : {args.k}")
    print(f"n              : {args.n}")
    print(f"vertices       : {len(cycle)}")
    print(f"first          : {cycle[0]}")
    print(f"second         : {cycle[1]}")
    print(f"last           : {cycle[-1]}")
    print(f"closing sum    : {cycle[-1] + cycle[0]}")
    print(f"canonical      : {str(canonical).lower()}")
    print(f"sha256         : {sha256}")
    print("power counts   :")

    for total in sorted(counts):
        root = exact_kth_root(total, args.k)
        print(f"  {root}^{args.k:<2} = {total:<8} : {counts[total]}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
