//! impose K N C RULE [OUT.hcp]
//! impose K N C RULE --expand TOUR OUT
//!
//! Degree-two propagation on G_K(N) with imposed edges, then contraction.
//! The island parameter C (even) defines the zones:
//!   Z4 = [(C-1)^K - (C-2)^K + 1, C^K - (C-1)^K]   low chain ends
//!   Z5 = [C^K - (C-1)^K + 1, (C-2)^K - 1]          free vertices of degree 3
//! RULE is a comma-separated list of
//!   none  nothing imposed
//!   c     m - (C^K - m) for every m in Z5 (the matching of Z5 with Z8)
//!   c1    m - ((C-1)^K - m) for every m in Z5 with (C-1)^K - m in Z4
//! An imposed edge already forced is counted; one already forbidden is
//! skipped and counted. Propagation runs to its fixed point after each.
//! Writes the contracted problem in TSPLIB HCP form if OUT.hcp is given; with
//! --expand, expands an LKH tour of that problem to a Hamiltonian cycle of
//! G_K(N), checks it, and writes OUT.txt and OUT.bases, canonical.
use posum_core::{Graph, bases, cycle, format};
use std::fs::File;
use std::io::{BufWriter, Write};

const FREE: u8 = 0;
const FORCED: u8 = 1;
const FORBIDDEN: u8 = 2;

struct P {
    n: u32,
    off: Vec<u64>,
    adj: Vec<u32>,
    st: Vec<u8>,
    av: Vec<u8>,
    fd: Vec<u8>,
    other: Vec<u32>,
    forced_edges: u64,
    queue: Vec<u32>,
    cycle: bool,
}

impl P {
    fn new(g: &Graph) -> P {
        let n = g.n() as u32;
        let mut off = Vec::with_capacity(n as usize + 2);
        off.push(0u64);
        off.push(0u64);
        let mut total = 0u64;
        for v in 1..=n {
            total += u64::from(g.degree(v));
            off.push(total);
        }
        let mut adj = Vec::with_capacity(total as usize);
        for v in 1..=n {
            let start = adj.len();
            adj.extend(g.neighbours(v));
            adj[start..].sort_unstable();
        }
        let av: Vec<u8> = (0..=n as usize)
            .map(|v| if v == 0 { 0 } else { (off[v + 1] - off[v]).min(255) as u8 })
            .collect();
        P {
            n,
            st: vec![FREE; total as usize],
            off,
            adj,
            av,
            fd: vec![0; n as usize + 1],
            other: (0..=n).collect(),
            forced_edges: 0,
            queue: (1..=n).collect(),
            cycle: false,
        }
    }
    fn range(&self, v: u32) -> std::ops::Range<usize> {
        self.off[v as usize] as usize..self.off[v as usize + 1] as usize
    }
    fn slot(&self, v: u32, w: u32) -> Option<usize> {
        let r = self.range(v);
        self.adj[r.clone()].binary_search(&w).ok().map(|i| r.start + i)
    }
    fn state(&self, v: u32, w: u32) -> Option<u8> {
        self.slot(v, w).map(|s| self.st[s])
    }
    fn set(&mut self, a: u32, b: u32, s: u8) {
        let (i, j) = (self.slot(a, b).unwrap(), self.slot(b, a).unwrap());
        self.st[i] = s;
        self.st[j] = s;
    }
    fn forbid(&mut self, a: u32, b: u32) {
        self.set(a, b, FORBIDDEN);
        self.av[a as usize] -= 1;
        self.av[b as usize] -= 1;
        self.queue.push(a);
        self.queue.push(b);
    }
    fn force(&mut self, a: u32, b: u32) -> Result<(), String> {
        let (ea, eb) = (self.other[a as usize], self.other[b as usize]);
        self.set(a, b, FORCED);
        self.fd[a as usize] += 1;
        self.fd[b as usize] += 1;
        self.forced_edges += 1;
        if ea == b {
            if self.forced_edges == u64::from(self.n) {
                self.cycle = true;
                return Ok(());
            }
            return Err(format!("edge {a}-{b} closes a short cycle"));
        }
        self.other[ea as usize] = eb;
        self.other[eb as usize] = ea;
        self.queue.extend([a, b, ea, eb]);
        if self.forced_edges + 1 < u64::from(self.n) && self.state(ea, eb) == Some(FREE) {
            self.forbid(ea, eb);
        }
        Ok(())
    }
    fn run(&mut self) -> Result<(), String> {
        while let Some(v) = self.queue.pop() {
            let (fd, av) = (self.fd[v as usize], self.av[v as usize]);
            if fd > 2 || av < 2 {
                return Err(format!("vertex {v}: {fd} forced, {av} available"));
            }
            if fd == 2 && av > 2 {
                for s in self.range(v) {
                    if self.st[s] == FREE {
                        self.forbid(v, self.adj[s]);
                    }
                }
            } else if av == 2 && fd < 2 {
                for s in self.range(v) {
                    if self.st[s] == FREE {
                        self.force(v, self.adj[s])?;
                    }
                }
            }
        }
        Ok(())
    }
    /// Imposes a-b: (imposed, already forced, forbidden).
    fn impose(&mut self, a: u32, b: u32, tally: &mut [u64; 3]) -> Result<(), String> {
        match self.state(a, b) {
            Some(FREE) => {
                tally[0] += 1;
                self.force(a, b)?;
                self.run()
            }
            Some(FORCED) => {
                tally[1] += 1;
                Ok(())
            }
            _ => {
                tally[2] += 1;
                Ok(())
            }
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n, c): (u32, u128, u64) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap());
    let rules: Vec<&str> = a[4].split(',').collect();
    let pw = |j: u64| j.pow(k);
    let (z4, z5) = (
        (pw(c - 1) - pw(c - 2) + 1, pw(c) - pw(c - 1)),
        (pw(c) - pw(c - 1) + 1, pw(c - 2) - 1),
    );
    let t = std::time::Instant::now();
    let g = Graph::new(k, n).unwrap();
    let mut p = P::new(&g);
    p.run().expect("no obstruction without a motif");
    let nodes = |p: &P| (1..=p.n).filter(|&v| p.fd[v as usize] < 2).count();
    eprintln!("propagation {:.0} s: {} nodes", t.elapsed().as_secs_f64(), nodes(&p));
    for rule in &rules {
        let mut tally = [0u64; 3];
        let res: Result<(), String> = (|| {
            match *rule {
                "none" => {}
                "c" => {
                    for m in z5.0..=z5.1 {
                        p.impose(m as u32, (pw(c) - m) as u32, &mut tally)?;
                    }
                }
                "c1" => {
                    for m in z5.0..=z5.1 {
                        let w = pw(c - 1) - m;
                        if (z4.0..=z4.1).contains(&w) {
                            p.impose(m as u32, w as u32, &mut tally)?;
                        }
                    }
                }
                other if other.starts_with("c1mod=") => {
                    let (x, y) = other[6..].split_once('/').unwrap();
                    let (x, y): (u64, u64) = (x.parse().unwrap(), y.parse().unwrap());
                    for m in z5.0..=z5.1 {
                        let w = pw(c - 1) - m;
                        if (z4.0..=z4.1).contains(&w) && m % y < x {
                            p.impose(m as u32, w as u32, &mut tally)?;
                        }
                    }
                }
                other => panic!("unknown rule {other}"),
            }
            Ok(())
        })();
        eprintln!("rule {rule}: {} imposed, {} already forced, {} forbidden; {} nodes", tally[0], tally[1], tally[2], nodes(&p));
        if let Err(e) = res {
            println!("OBSTRUCTION after rule {rule}: {e}");
            std::process::exit(1);
        }
    }
    // Contraction.
    let mut node_of = vec![0u32; p.n as usize + 1];
    let mut vertex_of = vec![0u32];
    for v in 1..=p.n {
        if p.fd[v as usize] < 2 {
            vertex_of.push(v);
            node_of[v as usize] = vertex_of.len() as u32 - 1;
        }
    }
    let m = vertex_of.len() - 1;
    let chains = (1..=p.n).filter(|&v| p.fd[v as usize] == 1 && v < p.other[v as usize]).count();
    let free: u64 = p.st.iter().filter(|&&s| s == FREE).count() as u64 / 2;
    println!(
        "G_{k}({n}) rules {}: {m} nodes ({:.1} % of n), {chains} chains, {free} free edges",
        a[4],
        100.0 * m as f64 / n as f64
    );
    if a.len() > 5 && a[5] != "--expand" {
        let mut f = BufWriter::with_capacity(1 << 22, File::create(&a[5]).unwrap());
        writeln!(f, "NAME : G{k}_{n}\nCOMMENT : rules {}\nTYPE : HCP\nDIMENSION : {m}", a[4]).unwrap();
        writeln!(f, "EDGE_DATA_FORMAT : EDGE_LIST\nEDGE_DATA_SECTION").unwrap();
        let mut links = Vec::new();
        for v in 1..=p.n {
            if p.fd[v as usize] == 1 && v < p.other[v as usize] {
                links.push((node_of[v as usize], node_of[p.other[v as usize] as usize]));
            }
            for s in p.range(v) {
                let w = p.adj[s];
                if v < w && p.st[s] == FREE {
                    let (x, y) = (node_of[v as usize], node_of[w as usize]);
                    assert!(x > 0 && y > 0, "free edge at an interior vertex");
                    writeln!(f, "{x} {y}").unwrap();
                }
            }
        }
        for (x, y) in &links {
            writeln!(f, "{x} {y}").unwrap();
        }
        writeln!(f, "-1\nFIXED_EDGES_SECTION").unwrap();
        for (x, y) in &links {
            writeln!(f, "{x} {y}").unwrap();
        }
        writeln!(f, "-1\nEOF").unwrap();
        f.flush().unwrap();
        eprintln!("wrote {}", a[5]);
    }
    if a.len() > 7 && a[5] == "--expand" {
        let text = std::fs::read_to_string(&a[6]).unwrap();
        let tour = posum_lkh::problem::parse_tour(&text).unwrap();
        assert_eq!(tour.len(), m, "tour length");
        let forced_nbrs = |v: u32| p.range(v).filter(|&s| p.st[s] == FORCED).map(|s| p.adj[s]);
        let mut cyc: Vec<u32> = Vec::with_capacity(p.n as usize);
        for (i, &x) in tour.iter().enumerate() {
            let after = tour[(i + 1) % m];
            let (vx, va) = (vertex_of[x as usize], vertex_of[after as usize]);
            cyc.push(vx);
            if p.fd[vx as usize] == 1 && p.other[vx as usize] == va {
                let (mut prev, mut cur) = (vx, forced_nbrs(vx).next().unwrap());
                while cur != va {
                    cyc.push(cur);
                    let next = forced_nbrs(cur).find(|&w| w != prev).unwrap();
                    (prev, cur) = (cur, next);
                }
            }
        }
        cycle::check(&g, &cyc).unwrap();
        cycle::canonicalize(&mut cyc);
        let b = bases::encode(&g, &cyc).unwrap();
        format::write_plain(BufWriter::new(File::create(format!("{}.txt", a[7])).unwrap()), &cyc).unwrap();
        format::write_bases(BufWriter::new(File::create(format!("{}.bases", a[7])).unwrap()), &b).unwrap();
        eprintln!("Hamiltonian cycle of G_{k}({n}): {} vertices, checked", cyc.len());
    }
}
