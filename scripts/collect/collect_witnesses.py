"""Collect every Hamiltonian cycle found so far into data/witnesses/.

Sources: final LKH tours (cost 0) anywhere under tmp/, converted cycles, the
k = 3 cycles of tmp/gerbicz/cycles-k3, and the published witnesses (listed,
not copied). Each cycle is put in canonical order, checked by both independent
verifiers, deduplicated by the SHA-256 of its canonical text, and stored
xz-compressed with an inventory row. Nothing is deleted.
"""

import csv
import glob
import hashlib
import lzma
import os
import re
import subprocess
import sys
from multiprocessing import get_context

ROOT = "/home/fred/Projects/powercycle"
TMP = f"{ROOT}/tmp"
OUT = f"{ROOT}/data/witnesses"
TSPLIB = f"{ROOT}/target/release/pc-tsplib"
RUST = f"{ROOT}/src/verify/rust/target/release/powercycle-verify"
PYTHON = f"{ROOT}/src/verify/python/verify-cycle.py"


def engine_note(path):
    rel = os.path.relpath(path, TMP)
    if rel.startswith("gerbicz/cycles-k3"):
        return "pc-tsplib export + LKH-3.0.14 three-level, uncontracted"
    if rel.startswith("lkh-c/"):
        return "pc-tsplib export --contract + LKH-3.0.14 three-level"
    if rel.startswith("lkh-bench/train-"):
        return "LKH-3.0.14 two-level, PGO training build, SEED 7"
    if rel.startswith("lkh-bench/"):
        return "LKH-3.0.14 three-level, uncontracted"
    if "3lvl" in rel:
        return "LKH-3.0.14 three-level, uncontracted"
    if rel.startswith("lkh/"):
        return "LKH-3.0.14 two-level, uncontracted"
    return "unknown"


def canonical(cycle):
    i = cycle.index(1)
    c = cycle[i:] + cycle[:i]
    if c[1] > c[-1]:
        c = [c[0]] + c[1:][::-1]
    return c


def k_n_from_tour(path):
    """k and n from the tour's NAME, e.g. G6_15748961 or G6_15748961_contracted."""
    head = open(path).read(400)
    m = re.search(r"NAME : G(\d+)_(\d+)", head)
    return (int(m.group(1)), int(m.group(2))) if m else (None, None)


def from_tour(path):
    text = open(path).read()
    if "Length = 0" not in text:
        return None, "cost > 0"
    k, n = k_n_from_tour(path)
    if k is None:
        return None, "no NAME"
    chains = re.sub(r"\.tour$", ".chains", path)
    cmd = [TSPLIB, "tour", str(k), str(n), path] + ([chains] if "_contracted" in text[:400] else [])
    if "_contracted" in text[:400] and not os.path.exists(chains):
        return None, "contracted tour without its chains file"
    run = subprocess.run(cmd, capture_output=True, text=True)
    if run.returncode != 0:
        return None, run.stderr.strip()[-120:]
    return (k, n, list(map(int, run.stdout.split()))), "ok"


def from_txt(path, k):
    cycle = list(map(int, open(path).read().split()))
    return (k, len(cycle), canonical(cycle)), "ok"


def process(item):
    kind, path, k_hint = item
    try:
        if kind == "tour":
            got, why = from_tour(path)
        elif kind == "xz":
            cycle = list(map(int, lzma.open(path).read().split()))
            got, why = (k_hint, len(cycle), canonical(cycle)), "ok"
        else:
            got, why = from_txt(path, k_hint)
    except Exception as error:  # report, never crash the whole collection
        return item, None, f"error: {error}"
    if got is None:
        return item, None, why

    k, n, cycle = got
    text = "".join(f"{v}\n" for v in cycle).encode()
    sha = hashlib.sha256(text).hexdigest()
    # One scratch file per worker call: the same cycle may arrive from several
    # sources at once.
    import tempfile
    fd, scratch = tempfile.mkstemp(prefix="collect-", suffix=".txt")
    with os.fdopen(fd, "wb") as f:
        f.write(text)
    rust = subprocess.run([RUST, str(k), str(n), scratch], capture_output=True).returncode == 0
    python = subprocess.run([sys.executable, PYTHON, str(k), str(n), scratch], capture_output=True).returncode == 0
    os.remove(scratch)
    return item, (k, n, sha, text, rust, python), "ok"


if __name__ == "__main__":
    items = []
    for path in sorted(glob.glob(f"{TMP}/**/*.tour", recursive=True)):
        if re.search(r"\.\d+\.tour$", path):  # progress files, written at every improvement
            continue
        items.append(("tour", path, None))
    for path, k in [(p, 6) for p in glob.glob(f"{TMP}/lkh*/*g6*cycle*.txt")] + \
                   [(p, 5) for p in glob.glob(f"{TMP}/lkh*/*g5*cycle*.txt") + glob.glob(f"{TMP}/lkh-c/c5*.txt") + glob.glob(f"{TMP}/lkh-bench/c.txt")] + \
                   [(p, 2) for p in glob.glob(f"{TMP}/lkh*/g2-32-cycle.txt")] + \
                   [(p, 3) for p in glob.glob(f"{TMP}/gerbicz/cycles-k3/*.txt")]:
        items.append(("txt", path, k))
    for path in sorted(glob.glob(f"{ROOT}/witness/k*-n*/cycle.txt.xz")):
        items.append(("xz", path, int(re.search(r"/k(\d+)-n", path).group(1))))

    print(f"{len(items)} sources", flush=True)
    seen = {}
    rejected = []
    with get_context("fork").Pool(12) as pool:
        for item, got, why in pool.imap_unordered(process, items):
            if got is None:
                rejected.append((item[1], why))
                continue
            k, n, sha, text, rust, python = got
            entry = seen.setdefault(sha, {"k": k, "n": n, "sources": [], "rust": rust, "python": python})
            entry["sources"].append((item[0], os.path.relpath(item[1], ROOT)))
            published = item[0] == "xz"
            if not published:
                d = f"{OUT}/k{k}-n{n}"
                os.makedirs(d, exist_ok=True)
                target = f"{d}/{sha[:8]}.txt.xz"
                if not os.path.exists(target):
                    with lzma.open(target, "wb", preset=9 | lzma.PRESET_EXTREME) as f:
                        f.write(text)

    with open(f"{OUT}/inventory.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["k", "n", "sha256", "published", "rust_verifier", "python_verifier", "engine", "sources"])
        for sha, e in sorted(seen.items(), key=lambda kv: (kv[1]["k"], kv[1]["n"], kv[0])):
            published = any(kind == "xz" for kind, _ in e["sources"])
            engines = sorted({engine_note(os.path.join(ROOT, p)) for kind, p in e["sources"] if kind != "xz"})
            w.writerow([e["k"], e["n"], sha, int(published), int(e["rust"]), int(e["python"]),
                        "; ".join(engines) if engines else "published witness",
                        " | ".join(p for _, p in sorted(e["sources"]))])

    print(f"{len(seen)} distinct cycles; rejected sources:", flush=True)
    for path, why in rejected:
        print(f"  {os.path.relpath(path, ROOT)}: {why}", flush=True)
