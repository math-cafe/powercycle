#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import sys


# A band covers the values m of [start, end] and names one vertex v whose
# degree in G_k(m) is the same, at most one, for every m in the range.
#
# The degree of v <= m counts the k-th powers p with v < p <= v + m, other
# than the loop position p = 2v. As m grows the window only widens, so the
# degree never decreases: equal at both ends means equal throughout. Checking
# the two endpoints therefore settles every m in between.
BAND = re.compile(
    r"(?P<start>[0-9]+)(?:\.\.(?P<end>[0-9]+))?"
    r"\tIMPOSSIBLE\tdegree<2 vertex=(?P<vertex>[0-9]+) degree=(?P<degree>[0-9]+)"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Independently verify a power-sum lower-bound certificate: no "
            "G_k(m) with m < n has minimum degree two. This script uses only "
            "the certificate file and Python's standard library."
        )
    )
    parser.add_argument("k", type=int, help="power exponent, k >= 2")
    parser.add_argument("n", type=int, help="claimed threshold, n >= 2")
    parser.add_argument("bands", type=Path, help="pc-scan --bands output for 1..n-1")
    return parser.parse_args()


def kth_root(value: int, k: int) -> int:
    """floor(value^(1/k)) for value >= 0, in exact integers."""
    low, high = 0, 1

    while high**k <= value:
        high *= 2

    while high - low > 1:
        middle = (low + high) // 2

        if middle**k <= value:
            low = middle
        else:
            high = middle

    return low


def degree(k: int, vertex: int, m: int) -> int:
    """Degree of vertex in G_k(m), for 1 <= vertex <= m."""
    count = kth_root(vertex + m, k) - kth_root(vertex, k)

    # vertex <= m puts the loop position 2v inside the window when it is a power.
    root = kth_root(2 * vertex, k)

    if root**k == 2 * vertex:
        count -= 1

    return count


def read_bands(path: Path) -> tuple[list[tuple[int, int, int, int]], bytes]:
    data = path.read_bytes()

    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as exc:
        raise ValueError(f"certificate is not ASCII: {exc}") from exc

    bands: list[tuple[int, int, int, int]] = []

    for line_number, line in enumerate(text.splitlines(), start=1):
        match = BAND.fullmatch(line)

        if match is None:
            raise ValueError(f"malformed band at line {line_number}: {line!r}")

        start = int(match["start"])
        end = int(match["end"]) if match["end"] is not None else start

        bands.append((start, end, int(match["vertex"]), int(match["degree"])))

    return bands, data


def validate_bands(bands: list[tuple[int, int, int, int]], k: int, n: int) -> None:
    if not bands:
        raise ValueError("empty certificate")

    expected_start = 1

    for line_number, (start, end, vertex, claimed) in enumerate(bands, start=1):
        if start != expected_start:
            raise ValueError(
                f"line {line_number} starts at {start}, expected {expected_start}"
            )

        if end < start:
            raise ValueError(f"line {line_number} has end {end} < start {start}")

        if not 1 <= vertex <= start:
            raise ValueError(
                f"line {line_number}: vertex {vertex} is not in G_{k}({start})"
            )

        if claimed > 1:
            raise ValueError(f"line {line_number}: degree {claimed} is not below two")

        for m in (start, end):
            actual = degree(k, vertex, m)

            if actual != claimed:
                raise ValueError(
                    f"line {line_number}: vertex {vertex} has degree {actual} "
                    f"in G_{k}({m}), certificate claims {claimed}"
                )

        expected_start = end + 1

    if expected_start != n:
        raise ValueError(f"bands end at {expected_start - 1}, expected {n - 1}")


def main() -> int:
    args = parse_args()

    if args.k < 2:
        print(
            f"verify-bands.py: INVALID: k={args.k}, expected k >= 2",
            file=sys.stderr,
        )
        return 1

    if args.n < 2:
        print(
            f"verify-bands.py: INVALID: n={args.n}, expected n >= 2",
            file=sys.stderr,
        )
        return 1

    try:
        bands, data = read_bands(args.bands)
        validate_bands(bands, args.k, args.n)
    except (OSError, ValueError) as exc:
        print(f"verify-bands.py: INVALID: {exc}", file=sys.stderr)
        return 1

    print("VALID LOWER BOUND")
    print(f"k              : {args.k}")
    print(f"n              : {args.n}")
    print(f"bands          : {len(bands)}")
    print(f"covers         : 1..{args.n - 1}")
    print(f"conclusion     : no G_{args.k}(m) with m < {args.n} is Hamiltonian")
    print(f"sha256         : {hashlib.sha256(data).hexdigest()}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
