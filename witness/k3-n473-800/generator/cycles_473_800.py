"""A Hamiltonian cycle of G_3(n) for every n in 473..800, through the repository's
pc-tsplib (export, LKH, tour check and canonical form) and the independent Rust
verifier."""
import csv, os, subprocess, tempfile, time
from multiprocessing import Pool

ROOT = "/home/fred/Projects/powercycle"
PC = f"{ROOT}/target/release/pc-tsplib"
VERIFY = f"{ROOT}/src/verify/rust/target/release/powercycle-verify"
OUT = f"{ROOT}/tmp/gerbicz/cycles-k3"

def one(n):
    t = time.time()
    with tempfile.TemporaryDirectory() as d:
        subprocess.run([PC, "export", "3", str(n), f"{d}/g.hcp"], capture_output=True, check=True)
        subprocess.run(["LKH", "g.par"], cwd=d, capture_output=True)
        if not os.path.exists(f"{d}/g.tour"):
            return (n, 0, round(time.time() - t, 2), "no tour")
        conv = subprocess.run([PC, "tour", "3", str(n), f"{d}/g.tour"], capture_output=True, text=True)
        if conv.returncode != 0:
            return (n, 0, round(time.time() - t, 2), conv.stderr.strip()[-80:])
        path = f"{OUT}/k3-n{n}.txt"
        open(path, "w").write(conv.stdout)
    ok = subprocess.run([VERIFY, "3", str(n), path], capture_output=True).returncode == 0
    return (n, 1 if ok else 0, round(time.time() - t, 2), "verified" if ok else "verifier rejected")

if __name__ == "__main__":
    os.makedirs(OUT, exist_ok=True)
    with open("cycles-473-800.csv", "w", newline="") as f, Pool(14) as pool:
        w = csv.writer(f); w.writerow(["n", "ok", "seconds", "note"])
        for row in pool.imap_unordered(one, range(473, 801)):
            w.writerow(row); f.flush()
