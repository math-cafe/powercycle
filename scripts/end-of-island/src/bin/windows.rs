//! windows PROBLEM.hcp WORKDIR W ROUNDS PARALLEL SECONDS [TOUR]
//!
//! Memory-bounded search for a Hamiltonian cycle of an HCP problem (edges,
//! with fixed edges that every tour must take). A full tour is built by a
//! Warnsdorff greedy that keeps the fixed edges (or read from TOUR, one node
//! per line or TSPLIB), then repaired window by window: the tour is cut into
//! W segments, and LKH ($POSUM_LKH or LKH on the PATH) looks, in the subgraph
//! of each segment that has a defect, for a Hamiltonian path between its two
//! fixed ends (an HCP problem with a dummy fixed edge between them, started
//! from the segment's current order). Up to PARALLEL segments run at once,
//! each for at most SECONDS. The windows move by a golden-ratio step of the
//! tour each round.
//! Writes WORKDIR/tour-<round>.txt and, when no defect is left,
//! WORKDIR/final.tour in TSPLIB form.
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::Command;

struct G {
    off: Vec<usize>,
    adj: Vec<u32>,
    partner: Vec<u32>,
}

impl G {
    fn nbrs(&self, v: u32) -> &[u32] {
        &self.adj[self.off[v as usize]..self.off[v as usize + 1]]
    }
    fn has(&self, a: u32, b: u32) -> bool {
        self.nbrs(a).binary_search(&b).is_ok()
    }
}

fn read_hcp(path: &str) -> (usize, G) {
    let r = BufReader::with_capacity(1 << 22, File::open(path).unwrap());
    let (mut dim, mut section, mut edges, mut fixed) = (0usize, 0u8, Vec::new(), Vec::new());
    for line in r.lines() {
        let line = line.unwrap();
        let t = line.trim();
        if let Some(d) = t.strip_prefix("DIMENSION") {
            dim = d.trim_start_matches([' ', ':']).trim().parse().unwrap();
        } else if t == "EDGE_DATA_SECTION" {
            section = 1;
        } else if t == "FIXED_EDGES_SECTION" {
            section = 2;
        } else if t == "-1" || t == "EOF" {
            section = 0;
        } else if section > 0 {
            let mut it = t.split_whitespace().map(|x| x.parse::<u32>().unwrap());
            let e = (it.next().unwrap(), it.next().unwrap());
            if section == 1 { edges.push(e) } else { fixed.push(e) }
        }
    }
    let mut deg = vec![0usize; dim + 2];
    for &(a, b) in &edges {
        deg[a as usize] += 1;
        deg[b as usize] += 1;
    }
    // The neighbours of v are adj[off[v]..off[v + 1]].
    let mut off = vec![0usize; dim + 2];
    for v in 1..=dim {
        off[v + 1] = off[v] + deg[v];
    }
    let mut fill = off.clone();
    let mut adj = vec![0u32; off[dim + 1]];
    for &(a, b) in &edges {
        adj[fill[a as usize]] = b;
        fill[a as usize] += 1;
        adj[fill[b as usize]] = a;
        fill[b as usize] += 1;
    }
    for v in 1..=dim {
        adj[off[v]..off[v + 1]].sort_unstable();
    }
    let mut partner = vec![0u32; dim + 1];
    for &(a, b) in &fixed {
        assert!(partner[a as usize] == 0 && partner[b as usize] == 0, "a node on two fixed edges");
        partner[a as usize] = b;
        partner[b as usize] = a;
    }
    (dim, G { off, adj, partner })
}

fn greedy(dim: usize, g: &G) -> Vec<u32> {
    let mut seen = vec![false; dim + 1];
    let mut free: Vec<u32> = (0..=dim).map(|v| if v == 0 { 0 } else { g.nbrs(v as u32).len() as u32 }).collect();
    let mut tour = Vec::with_capacity(dim);
    let mut scan = 1usize;
    let visit = |v: u32, seen: &mut Vec<bool>, free: &mut Vec<u32>, tour: &mut Vec<u32>| {
        seen[v as usize] = true;
        tour.push(v);
        for &w in g.nbrs(v) {
            free[w as usize] -= 1;
        }
    };
    while tour.len() < dim {
        // a new path: an unvisited node of least remaining degree among a few
        while seen[scan] { scan += 1; }
        let mut u = scan as u32;
        visit(u, &mut seen, &mut free, &mut tour);
        loop {
            let p = g.partner[u as usize];
            let next = if p != 0 && !seen[p as usize] {
                Some(p)
            } else {
                g.nbrs(u).iter().copied().filter(|&w| !seen[w as usize])
                    .min_by_key(|&w| (free[w as usize], w))
            };
            match next {
                Some(w) => {
                    visit(w, &mut seen, &mut free, &mut tour);
                    u = w;
                }
                None => break,
            }
        }
    }
    tour
}

fn defects(g: &G, t: &[u32]) -> usize {
    (0..t.len()).filter(|&i| !g.has(t[i], t[(i + 1) % t.len()])).count()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (w, rounds, par, secs): (usize, usize, usize, u64) =
        (a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap(), a[6].parse().unwrap());
    let work = std::path::PathBuf::from(&a[2]);
    std::fs::create_dir_all(&work).unwrap();
    let lkh = std::env::var("POSUM_LKH").unwrap_or_else(|_| "LKH".into());
    let t0 = std::time::Instant::now();
    let (dim, g) = read_hcp(&a[1]);
    eprintln!("read {dim} nodes, {} edge ends, {:.0} s", g.adj.len(), t0.elapsed().as_secs_f64());
    let mut tour: Vec<u32> = if a.len() > 7 {
        let text = std::fs::read_to_string(&a[7]).unwrap();
        let body = text.split("TOUR_SECTION").last().unwrap();
        body.split_whitespace().map(|x| x.parse::<i64>().unwrap()).take_while(|&x| x != -1).map(|x| x as u32).collect()
    } else {
        greedy(dim, &g)
    };
    assert_eq!(tour.len(), dim);
    eprintln!("start: {} defects, {:.0} s", defects(&g, &tour), t0.elapsed().as_secs_f64());
    let seg = dim / w;
    let mut local = vec![0u32; dim + 1];
    for round in 0..rounds {
        let shift = ((round as f64 + 1.0) * 0.618_033_988_7).fract() * dim as f64;
        let shift = shift as usize % dim;
        tour.rotate_left(shift);
        // Windows [i*seg, (i+1)*seg), the last one taking the remainder.
        let bounds: Vec<(usize, usize)> = (0..w).map(|i| (i * seg, if i + 1 == w { dim } else { (i + 1) * seg })).collect();
        let todo: Vec<usize> = (0..w)
            .filter(|&i| {
                let (lo, hi) = bounds[i];
                (lo..hi - 1).any(|j| !g.has(tour[j], tour[j + 1]))
            })
            .collect();
        for chunk in todo.chunks(par) {
            let mut kids = Vec::new();
            for &i in chunk {
                let (lo, hi) = bounds[i];
                let s = hi - lo;
                for (j, &v) in tour[lo..hi].iter().enumerate() {
                    local[v as usize] = j as u32 + 1;
                }
                let inside = |v: u32| {
                    let l = local[v as usize] as usize;
                    l >= 1 && l <= s && tour[lo + l - 1] == v
                };
                let dir = work.join(format!("w{i}"));
                std::fs::create_dir_all(&dir).unwrap();
                let mut f = BufWriter::with_capacity(1 << 22, File::create(dir.join("sub.hcp")).unwrap());
                writeln!(f, "NAME : window\nTYPE : HCP\nDIMENSION : {s}\nEDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION").unwrap();
                let mut fixed = vec![(1u32, s as u32)];
                for (j, &v) in tour[lo..hi].iter().enumerate() {
                    let lv = j as u32 + 1;
                    for &u in g.nbrs(v) {
                        if inside(u) && local[u as usize] > lv {
                            writeln!(f, "{lv} {}", local[u as usize]).unwrap();
                        }
                    }
                    let p = g.partner[v as usize];
                    if p != 0 && inside(p) && local[p as usize] > lv {
                        fixed.push((lv, local[p as usize]));
                    }
                }
                if !g.has(tour[lo], tour[hi - 1]) {
                    writeln!(f, "1 {s}").unwrap();
                }
                writeln!(f, "-1\nFIXED_EDGES_SECTION").unwrap();
                for (x, y) in &fixed {
                    writeln!(f, "{x} {y}").unwrap();
                }
                writeln!(f, "-1\nEOF").unwrap();
                f.flush().unwrap();
                let mut it = BufWriter::new(File::create(dir.join("init.tour")).unwrap());
                writeln!(it, "TOUR_SECTION").unwrap();
                for j in 1..=s {
                    writeln!(it, "{j}").unwrap();
                }
                writeln!(it, "-1\nEOF").unwrap();
                it.flush().unwrap();
                std::fs::write(
                    dir.join("sub.par"),
                    format!("PROBLEM_FILE = sub.hcp\nINITIAL_TOUR_FILE = init.tour\nTOUR_FILE = out.tour\nSUBGRADIENT = NO\nOPTIMUM = 0\nSTOP_AT_OPTIMUM = YES\nRUNS = 1\nSEED = {}\nTOTAL_TIME_LIMIT = {secs}\nTRACE_LEVEL = 1\n", round * w + i + 1),
                ).unwrap();
                let _ = std::fs::remove_file(dir.join("out.tour"));
                let log = File::create(dir.join("lkh.log")).unwrap();
                kids.push((i, Command::new(&lkh).arg("sub.par").current_dir(&dir).stdout(log).spawn().unwrap()));
            }
            for (i, mut kid) in kids {
                kid.wait().unwrap();
                let (lo, hi) = bounds[i];
                let s = hi - lo;
                let dir = work.join(format!("w{i}"));
                let Ok(text) = std::fs::read_to_string(dir.join("out.tour")) else { continue };
                let body = text.split("TOUR_SECTION").last().unwrap();
                let mut sub: Vec<usize> = body.split_whitespace().map(|x| x.parse::<i64>().unwrap())
                    .take_while(|&x| x != -1).map(|x| x as usize).collect();
                assert_eq!(sub.len(), s);
                let k = sub.iter().position(|&x| x == 1).unwrap();
                sub.rotate_left(k);
                if sub[1] == s {
                    sub[1..].reverse();
                }
                assert_eq!(sub[s - 1], s, "the dummy edge is kept");
                let old: Vec<u32> = tour[lo..hi].to_vec();
                for (j, &l) in sub.iter().enumerate() {
                    tour[lo + j] = old[l - 1];
                }
            }
        }
        let d = defects(&g, &tour);
        eprintln!("round {round}: {} windows searched, {d} defects, {:.0} s", todo.len(), t0.elapsed().as_secs_f64());
        let mut f = BufWriter::new(File::create(work.join(format!("tour-{round}.txt"))).unwrap());
        for &v in &tour {
            writeln!(f, "{v}").unwrap();
        }
        f.flush().unwrap();
        if d == 0 {
            let mut f = BufWriter::new(File::create(work.join("final.tour")).unwrap());
            writeln!(f, "TOUR_SECTION").unwrap();
            for &v in &tour {
                writeln!(f, "{v}").unwrap();
            }
            writeln!(f, "-1\nEOF").unwrap();
            f.flush().unwrap();
            eprintln!("Hamiltonian cycle of the problem: {}", work.join("final.tour").display());
            return;
        }
    }
}
