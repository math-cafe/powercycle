"""Collect the 1956 k = 5 cycles of H5_246877-248831.tar into data/witnesses/,
checked by both independent verifiers. Writes data/witnesses/inventory-h5.csv."""
import csv, hashlib, lzma, os, re, subprocess, sys, tarfile, tempfile
from multiprocessing import get_context

ROOT = "/home/fred/Projects/powercycle"
TAR = f"{ROOT}/H5_246877-248831.tar"
OUT = f"{ROOT}/data/witnesses"
RUST = f"{ROOT}/src/verify/rust/target/release/powercycle-verify"
PYTHON = f"{ROOT}/src/verify/python/verify-cycle.py"

def canonical(cycle):
    i = cycle.index(1); c = cycle[i:] + cycle[:i]
    return [c[0]] + c[1:][::-1] if c[1] > c[-1] else c

def one(name):
    n = int(re.search(r"n(\d+)", name).group(1))
    with tarfile.open(TAR) as t:
        raw = t.extractfile(name).read().decode()
    cycle = canonical(list(map(int, re.findall(r"\d+", raw))))
    text = "".join(f"{v}\n" for v in cycle).encode()
    sha = hashlib.sha256(text).hexdigest()
    fd, scratch = tempfile.mkstemp(suffix=".txt")
    with os.fdopen(fd, "wb") as f: f.write(text)
    rust = subprocess.run([RUST, "5", str(n), scratch], capture_output=True).returncode == 0
    python = subprocess.run([sys.executable, PYTHON, "5", str(n), scratch], capture_output=True).returncode == 0
    os.remove(scratch)
    d = f"{OUT}/k5-n{n}"; os.makedirs(d, exist_ok=True)
    target = f"{d}/{sha[:8]}.txt.xz"
    if not os.path.exists(target):
        with lzma.open(target, "wb", preset=9 | lzma.PRESET_EXTREME) as f: f.write(text)
    return n, len(cycle), sha, int(rust), int(python), name

if __name__ == "__main__":
    with tarfile.open(TAR) as t:
        names = [m.name for m in t.getmembers() if m.name.startswith("cycle-k5-n")]
    with open(f"{OUT}/inventory-h5.csv", "w", newline="") as f, get_context("fork").Pool(12) as pool:
        w = csv.writer(f); w.writerow(["n", "length", "sha256", "rust_verifier", "python_verifier", "source"])
        for row in pool.imap_unordered(one, names, chunksize=4):
            w.writerow(row); f.flush()
    rows = list(csv.DictReader(open(f"{OUT}/inventory-h5.csv")))
    bad = [r for r in rows if r["rust_verifier"] != "1" or r["python_verifier"] != "1" or int(r["length"]) != int(r["n"])]
    print(f"{len(rows)} cycles collected, {len(bad)} failing: {[r['n'] for r in bad][:20]}")
