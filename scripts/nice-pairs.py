#!/usr/bin/env python3

# Search Gerbicz nice pairs of power-sum paths, the base cases of his induction.
#
# Usage:
#   scripts/nice-pairs.py search <k> <n> [attempts]
#   scripts/nice-pairs.py sweep <k> <lo> <hi> <out.csv> [attempts]
#   scripts/nice-pairs.py cycles <k> <lo> <hi> <out-dir> <out.csv>
#
# A nice pair (R. Gerbicz, 2018, for squares) is a Hamiltonian path seq0 of
# G_k(n) and a Hamiltonian path seq1 of G_k(n+1), both from 1 to a neighbour of
# 1 so that each closes into a cycle, such that every x <= n sits at positions
# of the same parity in both. Positions alternate along a path, so this is one
# 2-colouring of 1..n that both paths respect: every edge they use joins the two
# colours.
#
# `search` takes seq0 from LKH, reads the colouring off it, then asks LKH for
# seq1 in G_k(n+1) restricted to the edges that colouring allows. Degree-two
# propagation discards hopeless colourings before LKH is called. A failure is
# not a proof that no nice pair exists: only colourings read off LKH paths are
# tried. Every pair returned is checked: two Hamiltonian paths, closing sums,
# and the parity condition.
#
# `cycles` finds one Hamiltonian cycle of G_k(n) per n through pc-tsplib and
# LKH, and checks it with the independent Rust verifier.
#
# Requires LKH on PATH and a release build. See doc/en/cube-necklaces.md.

import csv
import multiprocessing
import os
import subprocess
import sys
import tempfile
import time

ROOT = os.path.realpath(os.path.join(os.path.dirname(__file__), ".."))

import os
import subprocess
import sys
import tempfile


def iroot(x, k):
    r = int(round(x ** (1.0 / k)))
    while r**k > x:
        r -= 1
    while (r + 1) ** k <= x:
        r += 1
    return r


def edges(k, n):
    out = []
    for b in range(2, iroot(2 * n - 1, k) + 1):
        p = b**k
        for a in range(max(1, p - n), (p - 1) // 2 + 1):
            if a < p - a <= n:
                out.append((a, p - a))
    return out


def hopeless(n, edge_list, start, end):
    """Degree-two propagation on the graph plus the fixed edge start-end.
    True when it proves that no Hamiltonian cycle through that edge exists."""
    adj = [set() for _ in range(n + 1)]
    for a, b in edge_list:
        adj[a].add(b)
        adj[b].add(a)
    if end not in adj[start]:
        adj[start].add(end)
        adj[end].add(start)

    forced = [set() for _ in range(n + 1)]
    forced[start].add(end)
    forced[end].add(start)

    def ends_of(v):
        """Walk the forced path through v; return its two ends, or None on a cycle."""
        if len(forced[v]) == 0:
            return v, v, 1
        sides, length = [], 1
        for first in list(forced[v])[:2]:
            prev, cur = v, first
            length += 1
            while len(forced[cur]) == 2:
                nxt = next(u for u in forced[cur] if u != prev)
                if nxt == v:
                    return None
                prev, cur = cur, nxt
                length += 1
            sides.append(cur)
        if len(sides) == 1:
            sides.append(v)
        return sides[0], sides[1], length

    queue = list(range(1, n + 1))
    while queue:
        v = queue.pop()
        if len(adj[v]) < 2 or len(forced[v]) > 2:
            return True
        if len(forced[v]) == 2 and len(adj[v]) > 2:
            for u in adj[v] - forced[v]:
                adj[u].discard(v)
                queue.append(u)
            adj[v] = set(forced[v])
        if len(adj[v]) == 2 and len(forced[v]) < 2:
            for u in adj[v] - forced[v]:
                info = ends_of(v), ends_of(u)
                if None in info:
                    return True
                (a1, b1, l1), (a2, b2, l2) = info
                if {a1, b1} == {a2, b2} and l1 < n:
                    return True  # u and v already ends of one path: closing early
                forced[v].add(u)
                forced[u].add(v)
                queue.extend([u, v])
    return False


def lkh_path(n, edge_list, start, end, seed, workdir, max_trials, time_limit):
    """A Hamiltonian path from start to end, as a cycle through the fixed edge
    start-end, or None if LKH finds no tour of cost 0."""
    # A cycle needs two edges at every vertex; LKH crashes on HCP problems that
    # violate this, and they are hopeless anyway.
    degree = [0] * (n + 1)
    for a, b in edge_list:
        degree[a] += 1
        degree[b] += 1
    if min(degree[1:]) < 2 or hopeless(n, edge_list, start, end):
        return None

    problem = os.path.join(workdir, "p.hcp")
    params = os.path.join(workdir, "p.par")
    tour = os.path.join(workdir, "p.tour")

    with open(problem, "w") as f:
        f.write(f"NAME : p\nTYPE : HCP\nDIMENSION : {n}\nEDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION\n")
        f.writelines(f"{a} {b}\n" for a, b in edge_list)
        f.write(f"-1\nFIXED_EDGES_SECTION\n{start} {end}\n-1\nEOF\n")

    with open(params, "w") as f:
        f.write(
            f"PROBLEM_FILE = {problem}\nTOUR_FILE = {tour}\nSUBGRADIENT = NO\n"
            f"OPTIMUM = 0\nSTOP_AT_OPTIMUM = YES\nRUNS = 1\nTRACE_LEVEL = 0\n"
            f"SEED = {seed}\nMAX_TRIALS = {max_trials}\nTIME_LIMIT = {time_limit}\n"
        )

    if os.path.exists(tour):
        os.remove(tour)

    if subprocess.run(["LKH", params], capture_output=True).returncode != 0:
        return None

    if not os.path.exists(tour):
        return None

    text = open(tour).read()

    if "Length = 0" not in text:
        return None

    nodes = []
    for token in text.split("TOUR_SECTION")[1].split():
        if token == "-1":
            break
        nodes.append(int(token))

    # Rotate to start, oriented so that end comes last.
    i = nodes.index(start)
    cycle = nodes[i:] + nodes[:i]
    if cycle[1] == end:
        cycle = [cycle[0]] + cycle[1:][::-1]
    assert cycle[-1] == end
    return cycle


def check_path(k, n, path, start, end):
    assert sorted(path) == list(range(1, n + 1))
    assert path[0] == start and path[-1] == end
    for a, b in zip(path, path[1:]):
        assert iroot(a + b, k) ** k == a + b
    assert iroot(start + end, k) ** k == start + end


def search(k, n, attempts=20, max_trials=None):
    neighbours_of_1 = [b**k - 1 for b in range(2, iroot(2 * n, k) + 1) if b**k - 1 <= n]
    g0 = edges(k, n)
    g1 = edges(k, n + 1)
    max_trials = max_trials or 200
    time_limit = max(2, n // 500)
    stats = {"lkh": 0, "pruned": 0}

    with tempfile.TemporaryDirectory() as workdir:
        for seed in range(1, attempts + 1):
            for e0 in neighbours_of_1:
                seq0 = lkh_path(n, g0, 1, e0, seed, workdir, max_trials, time_limit)
                if seq0 is None:
                    continue

                colour = {v: p % 2 for p, v in enumerate(seq0)}  # position parity, 0-based

                # n+1 sits at some position of seq1; try both colours for it.
                for c_new in (0, 1):
                    colour[n + 1] = c_new
                    allowed = [(a, b) for a, b in g1 if colour[a] != colour[b]]

                    # seq1 has n+1 vertices, so its last position has parity n % 2
                    # (0-based), and its end must carry that colour.
                    for e1 in neighbours_of_1 + ([n + 1] if iroot(n + 2, k) ** k == n + 2 else []):
                        if e1 > n + 1 or colour[e1] != n % 2:
                            continue
                        if hopeless(n + 1, allowed, 1, e1):
                            stats["pruned"] += 1
                            continue
                        stats["lkh"] += 1
                        seq1 = lkh_path(n + 1, allowed, 1, e1, seed, workdir, max_trials, time_limit)
                        if seq1 is None:
                            continue

                        check_path(k, n, seq0, 1, e0)
                        check_path(k, n + 1, seq1, 1, e1)
                        pos0 = {v: p for p, v in enumerate(seq0)}
                        pos1 = {v: p for p, v in enumerate(seq1)}
                        assert all((pos0[x] - pos1[x]) % 2 == 0 for x in range(1, n + 1))
                        return {"seed": seed, "e0": e0, "e1": e1, "seq0": seq0, "seq1": seq1, **stats}
    return {"none": True, **stats}


def sweep_one(args):
    k, n, attempts = args
    started = time.time()
    result = search(k, n, attempts)
    return (n, 0 if result.get("none") else 1, round(time.time() - started, 2),
            result.get("e0", ""), result.get("e1", ""), result.get("seed", ""),
            result["lkh"], result["pruned"])


def cycle_one(args):
    k, n, out_dir = args
    tsplib = os.path.join(ROOT, "target/release/pc-tsplib")
    verify = os.path.join(ROOT, "src/verify/rust/target/release/powercycle-verify")
    started = time.time()

    with tempfile.TemporaryDirectory() as workdir:
        subprocess.run([tsplib, "export", str(k), str(n), f"{workdir}/g.hcp"],
                       capture_output=True, check=True)
        subprocess.run(["LKH", "g.par"], cwd=workdir, capture_output=True)

        if not os.path.exists(f"{workdir}/g.tour"):
            return (n, 0, round(time.time() - started, 2), "no tour")

        tour = subprocess.run([tsplib, "tour", str(k), str(n), f"{workdir}/g.tour"],
                              capture_output=True, text=True)
        if tour.returncode != 0:
            return (n, 0, round(time.time() - started, 2), tour.stderr.strip()[-80:])

    path = os.path.join(out_dir, f"k{k}-n{n}.txt")
    with open(path, "w") as f:
        f.write(tour.stdout)

    ok = subprocess.run([verify, str(k), str(n), path], capture_output=True).returncode == 0
    return (n, 1 if ok else 0, round(time.time() - started, 2), "verified" if ok else "verifier rejected")


def run_pool(function, jobs, out_csv, header):
    context = multiprocessing.get_context("fork")
    workers = max(1, (os.cpu_count() or 2) - 2)

    with open(out_csv, "w", newline="") as f, context.Pool(workers) as pool:
        writer = csv.writer(f)
        writer.writerow(header)
        for row in pool.imap_unordered(function, jobs):
            writer.writerow(row)
            f.flush()


def usage():
    print(__doc__ or "", file=sys.stderr)
    print("usage: scripts/nice-pairs.py search <k> <n> [attempts] | "
          "sweep <k> <lo> <hi> <out.csv> [attempts] | "
          "cycles <k> <lo> <hi> <out-dir> <out.csv>", file=sys.stderr)
    sys.exit(2)


if __name__ == "__main__":
    args = sys.argv[1:]

    if len(args) in (3, 4) and args[0] == "search":
        k, n = int(args[1]), int(args[2])
        attempts = int(args[3]) if len(args) == 4 else 20
        result = search(k, n, attempts)
        if result.get("none"):
            print(f"k={k} n={n}: no nice pair found in {attempts} attempts")
        else:
            print(f"k={k} n={n}: nice pair, seed {result['seed']}, ends {result['e0']} / {result['e1']}")
    elif len(args) in (5, 6) and args[0] == "sweep":
        k, lo, hi, out = int(args[1]), int(args[2]), int(args[3]), args[4]
        attempts = int(args[5]) if len(args) == 6 else 10
        run_pool(sweep_one, [(k, n, attempts) for n in range(lo, hi + 1)], out,
                 ["n", "found", "seconds", "e0", "e1", "seed", "lkh_runs", "pruned"])
    elif len(args) == 6 and args[0] == "cycles":
        k, lo, hi, out_dir, out = int(args[1]), int(args[2]), int(args[3]), args[4], args[5]
        os.makedirs(out_dir, exist_ok=True)
        run_pool(cycle_one, [(k, n, out_dir) for n in range(lo, hi + 1)], out,
                 ["n", "ok", "seconds", "note"])
    else:
        usage()
