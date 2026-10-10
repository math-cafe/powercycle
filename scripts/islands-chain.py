#!/usr/bin/env python3

# Follow the smallest blocking vertex of G_k(n) as n grows.
#
# Usage:
#   scripts/islands-chain.py <k> [--ranges]
#
# A vertex blocks when its degree is below two. The degree of x in G_k(n),
# for x <= n, counts the k-th powers in the window (x, x + n] other than the
# loop position 2x:
#
#   deg(x) = floor((x+n)^(1/k)) - floor(x^(1/k)) - [2x is a k-th power]
#
# Along x it only drops at a power or at a loop position, so the smallest
# blocking vertex is always 1, some a^k, or some a^k/2. Only those candidates
# are examined, which makes the sweep independent of n. See section 5 of
# doc/en/islands.md.
#
# Each output line is a range of n with the same smallest blocking vertex, or
# an admissible island. With --ranges the output is tab-separated
# `start end vertex`, the vertex being `-` for an island and the final island
# ending at its own start, for comparison with pc-scan:
#
#   pc-scan --quick-only <k> 1 <start of final island>
#
# grouped by blocking vertex. All arithmetic is in exact integers, so the
# sweep goes beyond the u64 limit of pc-scan.

import sys


def usage():
    print("usage: scripts/islands-chain.py <k> [--ranges]", file=sys.stderr)
    sys.exit(2)


args = sys.argv[1:]
ranges = "--ranges" in args
args = [a for a in args if a != "--ranges"]

if len(args) != 1 or not args[0].isdigit() or int(args[0]) < 2:
    usage()

K = int(args[0])


def iroot(value):
    """floor(value^(1/K)), exactly."""
    if value < 1:
        return 0

    root = int(round(value ** (1.0 / K)))

    while root**K > value:
        root -= 1

    while (root + 1) ** K <= value:
        root += 1

    return root


def is_power(value):
    return iroot(value) ** K == value


def degree(x, n):
    return iroot(x + n) - iroot(x) - is_power(2 * x)


def usable_bases(x):
    """Bases of the first two powers above x that are not the loop 2x."""
    bases, base = [], iroot(x) + 1

    while len(bases) < 2:
        if base**K != 2 * x:
            bases.append(base)

        base += 1

    return bases


def kind(x):
    if x == 1:
        return "1"

    if is_power(x):
        return f"{iroot(x)}^{K}"

    return f"{iroot(2 * x)}^{K}/2"


# Beyond the safe base a* = ceil(3 / (2^(1/K) - 1)) three powers always lie in
# (x, 2x], so no vertex past a*^K can block. Section 2 of doc/en/islands.md.
safe_base = 1

while (safe_base + 3) ** K > 2 * safe_base**K:
    safe_base += 1

last_base = safe_base + 3

candidates = sorted(
    {1}
    | {a**K for a in range(2, last_base)}
    | {a**K // 2 for a in range(2, last_base, 2)}
)


def smallest_blocking(n):
    for x in candidates:
        if x > n:
            return None

        if degree(x, n) < 2:
            return x

    return None


def emit(start, end, vertex, text):
    if ranges:
        print(f"{start}\t{end}\t{vertex if vertex is not None else '-'}")
    else:
        print(f"{start:>24} {end:>24}  {text}")


n = 1
island_start = None

while True:
    x = smallest_blocking(n)

    if x is None:
        if island_start is None:
            island_start = n

        # Degrees never drop as n grows, so the next blocker can only be a
        # vertex entering the graph, and the smallest one is a candidate.
        later = [c for c in candidates if c > n]

        if not later:
            break

        n = later[0]
        continue

    if island_start is not None:
        emit(island_start, n - 1, None, "admissible island")
        island_start = None

    first, second = usable_bases(x)
    end = second**K - x - 1

    emit(n, end, x, f"v={x}  {kind(x)}  powers {first}^{K}, {second}^{K}")

    n = end + 1

if ranges:
    emit(island_start, island_start, None, "")
else:
    emit(island_start, "inf", None, "admissible, final")
