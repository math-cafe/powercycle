#!/usr/bin/env python3
"""Path variant at M = 11^3 = 1331: degree screen, extremes, then all cases.

Usage: scripts/gerbicz-screen-1331.py   (needs LKH in PATH; about 10 h on 14 cores)
Log of the run behind doc/en/cube-necklaces.md: doc/en/data/gerbicz-screen-1331.log
"""
import itertools, os, random, subprocess, tempfile, time
from multiprocessing import get_context
import gerbicz_paths as paths

k, m = 3, 11
M, h = m**3, (m**3 - 1) // 2
LIMIT = 10
random.seed(1)
CASES = paths.residues(m, k)
SAMPLE = random.sample(CASES, 12) + [(0, h), (1, h), (0, h + M), (1, h + M)]
EXTREMES = [(0, h), (0, h + M), (1, h), (1, h + M)]

def degree_ok(t):
    S, e0, e1 = t
    return t, all(paths.min_degree(*paths.instance(k, m, S, e0, e1, p, r)[:2]) >= 2 for p, r in SAMPLE)

def solve(job):
    S, e0, e1, p, r, cap = job
    nodes, edges, fixed = paths.instance(k, m, S, e0, e1, p, r)
    if paths.min_degree(nodes, edges) < 2:
        return job, False
    with tempfile.TemporaryDirectory() as d:
        with open(f"{d}/g.hcp", "w") as f:
            f.write(f"NAME : g\nTYPE : HCP\nDIMENSION : {len(nodes)}\nEDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION\n")
            f.writelines(f"{a} {b}\n" for a, b in edges); f.write("-1\nFIXED_EDGES_SECTION\n")
            f.writelines(f"{a} {b}\n" for a, b in fixed); f.write("-1\nEOF\n")
        with open(f"{d}/g.par", "w") as f:
            f.write(f"PROBLEM_FILE = g.hcp\nTOUR_FILE = g.tour\nSUBGRADIENT = NO\nOPTIMUM = 0\nSTOP_AT_OPTIMUM = YES\n"
                    f"RUNS = 1000\nTRACE_LEVEL = 0\nMAX_TRIALS = 5000\nTOTAL_TIME_LIMIT = {cap}\n")
        subprocess.run(["LKH", "g.par"], cwd=d, capture_output=True)
        return job, os.path.exists(f"{d}/g.tour") and "Length = 0" in open(f"{d}/g.tour").read()

if __name__ == "__main__":
    start = time.time()
    triples = [t for t in itertools.permutations(range(1, LIMIT + 1), 3)
               if not paths.is_power(t[0] + t[1], k) and not paths.is_power(t[0] + t[2], k)]
    with get_context("fork").Pool(14) as pool:
        survivors = sorted(t for t, ok in pool.imap_unordered(degree_ok, triples) if ok)
        print(f"[{time.time()-start:.0f}s] degree screen: {len(survivors)} of {len(triples)} triples: {survivors}", flush=True)
        jobs = [(*t, p, r, 60) for t in survivors for p, r in EXTREMES]
        passed = {}
        for (S, e0, e1, *_), ok in pool.imap_unordered(solve, jobs):
            passed.setdefault((S, e0, e1), []).append(ok)
        best = sorted(t for t, v in passed.items() if all(v))
        print(f"[{time.time()-start:.0f}s] extremes: {len(best)} triples pass all four: {best}", flush=True)
        for t in best[:3]:
            results = [ok for _, ok in pool.imap_unordered(solve, [(*t, p, r, 60) for p, r in CASES])]
            print(f"[{time.time()-start:.0f}s] S={t[0]} ends {t[1]}/{t[2]}: {sum(results)} of {len(results)} cases", flush=True)
