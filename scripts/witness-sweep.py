#!/usr/bin/env python3

# Find, verify and record a Hamiltonian cycle of G_k(n) for every n of a range,
# with full provenance. Resumable: a run stopped at any point continues where
# it stopped.
#
# Usage:
#   scripts/witness-sweep.py <k> <lo> <hi> <out-dir> [options]
#
# Options:
#   --workers N        parallel LKH runs (default: CPU count - 2)
#   --seed S           LKH seed (default: 1)
#   --no-contract      export the plain graph instead of the contracted one
#   --time-limit T     give up on an n after T seconds of LKH (default: none)
#   --nice N           run LKH at this niceness (default: 19, lowest priority)
#
# For each n: pc-tsplib export [--contract], LKH, pc-tsplib tour (which checks
# the cycle and puts it in canonical order), then both independent verifiers.
# Written per n, under <out-dir>/k<k>-n<n>/:
#
#   <sha8>.txt.xz      the canonical cycle, one vertex per line
#   provenance.toml    how it was found and checked
#
# and one row per n in <out-dir>/sweep.csv. An n counts as done when its
# provenance.toml exists and records both verifiers as valid; anything else is
# redone on the next run.
#
# Requires LKH on PATH (scripts in the PΣG repository build the pinned
# LKH-3.0.14) and a release build of this repository.

import argparse
import csv
import datetime
import hashlib
import lzma
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
from multiprocessing import get_context

ROOT = os.path.realpath(os.path.join(os.path.dirname(__file__), ".."))
TSPLIB = os.path.join(ROOT, "target/release/pc-tsplib")
RUST = os.path.join(ROOT, "src/verify/rust/target/release/powercycle-verify")
PYTHON = os.path.join(ROOT, "src/verify/python/verify-cycle.py")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def context():
    """What does not change between n: tools, versions, machine."""
    lkh = shutil.which("LKH")
    if lkh is None:
        sys.exit("witness-sweep: LKH is not on PATH")
    for tool in (TSPLIB, RUST):
        if not os.path.exists(tool):
            sys.exit(f"witness-sweep: missing {tool}; build the release binaries first")

    commit = subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(["git", "-C", ROOT, "status", "--porcelain", "--untracked-files=no"],
                           capture_output=True, text=True).stdout.strip() != ""
    tsplib_version = subprocess.run([TSPLIB, "--help"], capture_output=True, text=True).stdout.split("\n")[0]
    cpu = ""
    try:
        cpu = next(l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name"))
    except (OSError, StopIteration):
        pass

    return {
        "lkh_path": lkh,
        "lkh_sha256": sha256_file(lkh),
        "tsplib": tsplib_version,
        "repository": "math-cafe/powercycle",
        "commit": commit + ("-dirty" if dirty else ""),
        "host": platform.node(),
        "cpu": cpu,
        "system": f"{platform.system()} {platform.release()}",
    }


def done(out_dir, k, n):
    path = os.path.join(out_dir, f"k{k}-n{n}", "provenance.toml")
    if not os.path.exists(path):
        return False
    text = open(path).read()
    return 'rust_verifier = "valid"' in text and 'python_verifier = "valid"' in text


def toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    return '"' + str(value).replace("\\", "\\\\").replace('"', '\\"') + '"'


def one(job):
    k, n, out_dir, opts, ctx = job
    record = {"k": k, "n": n, "status": "failed"}
    started = datetime.datetime.now(datetime.timezone.utc)
    record["started"] = started.isoformat(timespec="seconds")

    with tempfile.TemporaryDirectory(prefix=f"sweep-k{k}-n{n}-") as d:
        problem = os.path.join(d, "g.hcp")
        export = [TSPLIB, "export"] + (["--contract"] if opts["contract"] else []) + [str(k), str(n), problem]
        run = subprocess.run(export, capture_output=True, text=True)
        if run.returncode != 0:
            record["error"] = run.stderr.strip()[-200:]
            return record

        par = os.path.join(d, "g.par")
        extra = f"SEED = {opts['seed']}\n"
        if opts["time_limit"]:
            extra += f"TOTAL_TIME_LIMIT = {opts['time_limit']}\n"
        params = open(par).read() + extra
        with open(par, "w") as f:
            f.write(params)

        t0 = time.time()
        lkh = subprocess.run(["nice", "-n", str(opts["nice"]), ctx["lkh_path"], "g.par"],
                             cwd=d, capture_output=True, text=True)
        record["lkh_seconds"] = round(time.time() - t0, 2)
        trials = re.search(r"Trials\.min = (\d+)", lkh.stdout)
        record["lkh_trials"] = int(trials.group(1)) if trials else -1

        tour_file = os.path.join(d, "g.tour")
        if not os.path.exists(tour_file) or "Length = 0" not in open(tour_file).read():
            record["error"] = "no tour of cost 0"
            return record

        tour = [TSPLIB, "tour", str(k), str(n), tour_file] + ([os.path.join(d, "g.chains")] if opts["contract"] else [])
        run = subprocess.run(tour, capture_output=True, text=True)
        if run.returncode != 0:
            record["error"] = run.stderr.strip()[-200:]
            return record

        text = run.stdout.encode()
        sha = hashlib.sha256(text).hexdigest()
        plain = os.path.join(d, "cycle.txt")
        with open(plain, "wb") as f:
            f.write(text)

        rust = subprocess.run([RUST, str(k), str(n), plain], capture_output=True).returncode == 0
        python = subprocess.run([sys.executable, PYTHON, str(k), str(n), plain], capture_output=True).returncode == 0

        target_dir = os.path.join(out_dir, f"k{k}-n{n}")
        os.makedirs(target_dir, exist_ok=True)
        with lzma.open(os.path.join(target_dir, f"{sha[:8]}.txt.xz"), "wb", preset=9 | lzma.PRESET_EXTREME) as f:
            f.write(text)

        finished = datetime.datetime.now(datetime.timezone.utc)
        record.update({
            "status": "ok" if rust and python else "verifier rejected",
            "sha256": sha,
            "file": f"{sha[:8]}.txt.xz",
            "finished": finished.isoformat(timespec="seconds"),
            "wall_seconds": round((finished - started).total_seconds(), 2),
            "rust_verifier": "valid" if rust else "INVALID",
            "python_verifier": "valid" if python else "INVALID",
        })

        with open(os.path.join(target_dir, "provenance.toml"), "w") as f:
            f.write(f"# Hamiltonian cycle of G_{k}({n}), found by witness-sweep.py\n\n")
            for key in ("k", "n", "sha256", "file", "status", "started", "finished", "wall_seconds",
                        "lkh_seconds", "lkh_trials", "rust_verifier", "python_verifier"):
                f.write(f"{key} = {toml_value(record[key])}\n")
            f.write("\n[engine]\n")
            f.write(f'name = "LKH"\nversion = "3.0.14"\n')
            f.write(f"binary_sha256 = {toml_value(ctx['lkh_sha256'])}\n")
            f.write(f"contracted = {toml_value(opts['contract'])}\n")
            f.write(f"seed = {opts['seed']}\n")
            f.write(f"parameters = '''\n{params}'''\n")
            f.write("\n[tools]\n")
            for key in ("tsplib", "repository", "commit"):
                f.write(f"{key} = {toml_value(ctx[key])}\n")
            f.write("\n[machine]\n")
            for key in ("host", "cpu", "system"):
                f.write(f"{key} = {toml_value(ctx[key])}\n")

    return record


def main():
    parser = argparse.ArgumentParser(description="Find and record Hamiltonian cycles of G_k(n) over a range of n.")
    parser.add_argument("k", type=int)
    parser.add_argument("lo", type=int)
    parser.add_argument("hi", type=int)
    parser.add_argument("out_dir")
    parser.add_argument("--workers", type=int, default=max(1, (os.cpu_count() or 2) - 2))
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--no-contract", action="store_true")
    parser.add_argument("--time-limit", type=int, default=0)
    parser.add_argument("--nice", type=int, default=19)
    args = parser.parse_args()

    os.makedirs(args.out_dir, exist_ok=True)
    ctx = context()
    opts = {"contract": not args.no_contract, "seed": args.seed,
            "time_limit": args.time_limit, "nice": args.nice}

    todo = [n for n in range(args.lo, args.hi + 1) if not done(args.out_dir, args.k, n)]
    total = args.hi - args.lo + 1
    print(f"witness-sweep: k={args.k} n={args.lo}..{args.hi}: {total - len(todo)} already done, "
          f"{len(todo)} to do, {args.workers} workers, LKH {ctx['lkh_sha256'][:12]}, commit {ctx['commit'][:12]}",
          flush=True)

    summary = os.path.join(args.out_dir, "sweep.csv")
    new_file = not os.path.exists(summary)
    started = time.time()
    with open(summary, "a", newline="") as f, get_context("fork").Pool(args.workers) as pool:
        writer = csv.writer(f)
        if new_file:
            writer.writerow(["k", "n", "status", "sha256", "lkh_seconds", "lkh_trials", "finished", "error"])
        for i, rec in enumerate(pool.imap_unordered(one, [(args.k, n, args.out_dir, opts, ctx) for n in todo]), 1):
            writer.writerow([rec["k"], rec["n"], rec["status"], rec.get("sha256", ""), rec.get("lkh_seconds", ""),
                             rec.get("lkh_trials", ""), rec.get("finished", ""), rec.get("error", "")])
            f.flush()
            if i % 50 == 0 or i == len(todo):
                print(f"witness-sweep: {i}/{len(todo)} in {time.time() - started:.0f}s", flush=True)

    failed = [n for n in range(args.lo, args.hi + 1) if not done(args.out_dir, args.k, n)]
    print(f"witness-sweep: {total - len(failed)}/{total} done and verified"
          + (f"; not done: {failed[:20]}{' ...' if len(failed) > 20 else ''}" if failed else ""), flush=True)
    sys.exit(0 if not failed else 1)


if __name__ == "__main__":
    main()
