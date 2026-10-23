//! relink PROBLEM.hcp WORKDIR R ROUNDS SECONDS [TOUR]
//!
//! Multilevel repair of a tour of an HCP problem. Each round, every node
//! within R tour positions of a defect is set free; each maximal run of the
//! other nodes is contracted to a link between its two ends. LKH
//! ($POSUM_LKH) searches the contracted problem, started from the current
//! tour restricted to its nodes, for at most SECONDS; its tour is expanded
//! back. Runs can be relinked anywhere, not only near their old place. The
//! start tour is TOUR or the Warnsdorff greedy. Writes WORKDIR/tour-<round>.txt
//! and, with no defect left, WORKDIR/final.tour.
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
    let (r, rounds, secs): (usize, usize, u64) = (a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap());
    let work = std::path::PathBuf::from(&a[2]);
    std::fs::create_dir_all(&work).unwrap();
    let lkh = std::env::var("POSUM_LKH").unwrap_or_else(|_| "LKH".into());
    let t0 = std::time::Instant::now();
    let (dim, g) = read_hcp(&a[1]);
    let mut tour: Vec<u32> = if a.len() > 6 {
        let text = std::fs::read_to_string(&a[6]).unwrap();
        let body = text.split("TOUR_SECTION").last().unwrap();
        body.split_whitespace().map(|x| x.parse::<i64>().unwrap()).take_while(|&x| x != -1).map(|x| x as u32).collect()
    } else {
        greedy(dim, &g)
    };
    assert_eq!(tour.len(), dim);
    eprintln!("start: {} defects, {:.0} s", defects(&g, &tour), t0.elapsed().as_secs_f64());
    let mut sub_of = vec![0u32; dim + 1];
    for round in 0..rounds {
        let n = tour.len();
        let bad: Vec<bool> = (0..n).map(|i| !g.has(tour[i], tour[(i + 1) % n])).collect();
        // free[i]: position i is within r of a defect edge (i, i+1).
        let mut free = vec![false; n];
        for i in 0..n {
            if bad[i] {
                for d in 0..=2 * r + 1 {
                    free[(i + n + d - r) % n] = true;
                }
            }
        }
        // Rotate so that position 0 starts a run boundary: a free position, if any.
        let Some(z) = (0..n).find(|&i| free[i]) else { break };
        tour.rotate_left(z);
        free.rotate_left(z);
        // Sub nodes: free positions and both ends of each maximal non-free run.
        let mut nodes: Vec<u32> = Vec::new();
        let mut runs: Vec<(usize, usize)> = Vec::new(); // tour positions lo..=hi, hi > lo
        let mut i = 0;
        while i < n {
            if free[i] {
                nodes.push(tour[i]);
                i += 1;
            } else {
                let lo = i;
                while i < n && !free[i] { i += 1; }
                let hi = i - 1;
                nodes.push(tour[lo]);
                if hi > lo {
                    nodes.push(tour[hi]);
                    runs.push((lo, hi));
                }
            }
        }
        for v in 1..=dim { sub_of[v] = 0; }
        for (j, &v) in nodes.iter().enumerate() { sub_of[v as usize] = j as u32 + 1; }
        let m = nodes.len();
        let mut link_of = std::collections::HashMap::new();
        let mut fixed: Vec<(u32, u32)> = Vec::new();
        for &(lo, hi) in &runs {
            let (x, y) = (sub_of[tour[lo] as usize], sub_of[tour[hi] as usize]);
            fixed.push((x.min(y), x.max(y)));
            link_of.insert((x.min(y), x.max(y)), (lo, hi));
        }
        for (j, &v) in nodes.iter().enumerate() {
            let p = g.partner[v as usize];
            let x = j as u32 + 1;
            if p != 0 && sub_of[p as usize] > x {
                let e = (x, sub_of[p as usize]);
                if !link_of.contains_key(&e) { fixed.push(e); }
            }
        }
        let dir = work.join(format!("r{round}"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = BufWriter::with_capacity(1 << 22, File::create(dir.join("sub.hcp")).unwrap());
        writeln!(f, "NAME : relink\nTYPE : HCP\nDIMENSION : {m}\nEDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION").unwrap();
        for (j, &v) in nodes.iter().enumerate() {
            let x = j as u32 + 1;
            for &u in g.nbrs(v) {
                let y = sub_of[u as usize];
                if y > x && !link_of.contains_key(&(x, y)) { writeln!(f, "{x} {y}").unwrap(); }
            }
        }
        for &(x, y) in link_of.keys() { writeln!(f, "{x} {y}").unwrap(); }
        writeln!(f, "-1\nFIXED_EDGES_SECTION").unwrap();
        for (x, y) in &fixed { writeln!(f, "{x} {y}").unwrap(); }
        writeln!(f, "-1\nEOF").unwrap();
        f.flush().unwrap();
        let mut it = BufWriter::new(File::create(dir.join("init.tour")).unwrap());
        writeln!(it, "TOUR_SECTION").unwrap();
        for j in 1..=m { writeln!(it, "{j}").unwrap(); }
        writeln!(it, "-1\nEOF").unwrap();
        it.flush().unwrap();
        std::fs::write(dir.join("sub.par"), format!("PROBLEM_FILE = sub.hcp\nINITIAL_TOUR_FILE = init.tour\nTOUR_FILE = out.tour\nSUBGRADIENT = NO\nOPTIMUM = 0\nSTOP_AT_OPTIMUM = YES\nRUNS = 1\nSEED = {}\nTOTAL_TIME_LIMIT = {secs}\nTRACE_LEVEL = 1\n", round + 1)).unwrap();
        let log = File::create(dir.join("lkh.log")).unwrap();
        Command::new(&lkh).arg("sub.par").current_dir(&dir).stdout(log).status().unwrap();
        let text = std::fs::read_to_string(dir.join("out.tour")).unwrap();
        let body = text.split("TOUR_SECTION").last().unwrap();
        let sub: Vec<u32> = body.split_whitespace().map(|x| x.parse::<i64>().unwrap()).take_while(|&x| x != -1).map(|x| x as u32).collect();
        assert_eq!(sub.len(), m);
        let mut next = Vec::with_capacity(n);
        for (j, &x) in sub.iter().enumerate() {
            let y = sub[(j + 1) % m];
            let prev = sub[(j + m - 1) % m];
            if link_of.contains_key(&(x.min(y), x.max(y))) {
                let (lo, hi) = link_of[&(x.min(y), x.max(y))];
                if nodes[x as usize - 1] == tour[lo] {
                    next.extend_from_slice(&tour[lo..=hi]);
                } else {
                    next.extend(tour[lo..=hi].iter().rev());
                }
            } else if link_of.contains_key(&(x.min(prev), x.max(prev))) {
                // x ends a run already written from its other end
            } else {
                next.push(nodes[x as usize - 1]);
            }
        }
        assert_eq!(next.len(), n, "expansion length");
        tour = next;
        let d = defects(&g, &tour);
        eprintln!("round {round}: {m} nodes ({} runs), {d} defects, {:.0} s", runs.len(), t0.elapsed().as_secs_f64());
        let mut f = BufWriter::new(File::create(work.join(format!("tour-{round}.txt"))).unwrap());
        for &v in &tour { writeln!(f, "{v}").unwrap(); }
        f.flush().unwrap();
        if d == 0 {
            let mut f = BufWriter::new(File::create(work.join("final.tour")).unwrap());
            writeln!(f, "TOUR_SECTION").unwrap();
            for &v in &tour { writeln!(f, "{v}").unwrap(); }
            writeln!(f, "-1\nEOF").unwrap();
            f.flush().unwrap();
            eprintln!("Hamiltonian cycle of the problem: {}", work.join("final.tour").display());
            return;
        }
    }
}
