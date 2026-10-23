//! posa K N TOUR OUT [SECONDS] [SEED]
//!
//! Rotation-extension (Posa) on the contracted G_K(N), from an LKH tour that
//! uses a few non-edges. The tour, cut at its non-edges, gives paths covering
//! every node. Repeatedly, at an end e of a path: if e is adjacent to an end
//! of another path, the two paths are joined; if a single path is left and e
//! is adjacent to its other end, the cycle closes; otherwise e is adjacent to
//! some node u inside a path, and the path is rotated (u in the same path) or
//! re-cut (u in another path) so that a new node becomes the end. Links,
//! the contracted forced chains, are never removed. On success the cycle is
//! expanded, checked, and written to OUT as a plain list.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use std::time::Instant;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, m: usize) -> usize {
        (self.next() % m as u64) as usize
    }
}

struct State {
    paths: Vec<Vec<u32>>,
    pid: Vec<u32>,
    idx: Vec<u32>,
}

impl State {
    fn reindex(&mut self, p: usize) {
        for (i, &x) in self.paths[p].iter().enumerate() {
            self.pid[x as usize] = p as u32;
            self.idx[x as usize] = i as u32;
        }
    }
    fn reindex_from(&mut self, p: usize, from: usize) {
        for i in from..self.paths[p].len() {
            let x = self.paths[p][i];
            self.pid[x as usize] = p as u32;
            self.idx[x as usize] = i as u32;
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: u32 = a[1].parse().unwrap();
    let n: u128 = a[2].parse().unwrap();
    let seconds: u64 = a.get(5).map_or(3600, |s| s.parse().unwrap());
    let mut rng = Rng(a.get(6).map_or(0x9E3779B97F4A7C15, |s| s.parse::<u64>().unwrap() | 1));
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).expect("no obstruction");
    let c = Contraction::new(&propagation).unwrap();
    let m = c.node_count();
    // adjacency of nodes: free edges and links
    let mut deg = vec![0u32; m + 2];
    let free: Vec<(u32, u32)> = c.free_edges(&propagation).collect();
    for &(x, y) in &free {
        deg[x as usize] += 1;
        deg[y as usize] += 1;
    }
    for &(x, y) in c.links() {
        deg[x as usize] += 1;
        deg[y as usize] += 1;
    }
    let mut start = vec![0usize; m + 2];
    for x in 1..=m + 1 {
        start[x] = start[x - 1] + deg[x - 1] as usize;
    }
    let mut fill = start.clone();
    let mut adj = vec![0u32; start[m + 1]];
    for &(x, y) in free.iter().chain(c.links().iter()) {
        adj[fill[x as usize]] = y;
        fill[x as usize] += 1;
        adj[fill[y as usize]] = x;
        fill[y as usize] += 1;
    }
    drop(free);
    let nb = |x: u32| &adj[start[x as usize]..start[x as usize + 1]];
    let is_link = |x: u32, y: u32| c.partner(x) == Some(y);
    let adjacent = |x: u32, y: u32| nb(x).contains(&y);

    // cut the tour at its non-edges
    let mut tour = posum_lkh::problem::parse_tour(&std::fs::read_to_string(&a[3]).unwrap()).unwrap();
    let len = tour.len();
    // test mode: PERTURB=r applies r random segment reversals that keep the links
    if let Ok(r) = std::env::var("PERTURB") {
        let r: usize = r.parse().unwrap();
        let mut done = 0;
        while done < r {
            let (mut i, mut j) = (rng.below(len - 1), rng.below(len - 1));
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            if j - i < 2 || is_link(tour[i], tour[i + 1]) || is_link(tour[j], tour[j + 1]) {
                continue;
            }
            tour[i + 1..=j].reverse();
            done += 1;
        }
    }
    let cuts: Vec<usize> = (0..len).filter(|&i| !adjacent(tour[i], tour[(i + 1) % len])).collect();
    eprintln!("{m} nodes, {} non-edges in the tour", cuts.len());
    let mut st = State { paths: Vec::new(), pid: vec![0; m + 1], idx: vec![0; m + 1] };
    if cuts.is_empty() {
        st.paths.push(tour.clone());
    } else {
        for w in 0..cuts.len() {
            let (s, e) = (cuts[w] + 1, cuts[(w + 1) % cuts.len()]);
            let mut p = Vec::new();
            let mut i = s % len;
            loop {
                p.push(tour[i]);
                if i == e {
                    break;
                }
                i = (i + 1) % len;
            }
            st.paths.push(p);
        }
    }
    for p in 0..st.paths.len() {
        st.reindex(p);
    }

    let t0 = Instant::now();
    let mut steps: u64 = 0;
    let closed: Vec<u32> = loop {
        if t0.elapsed().as_secs() > seconds {
            eprintln!("time limit: {} paths left after {steps} steps", st.paths.len());
            std::process::exit(2);
        }
        steps += 1;
        if steps % 100_000 == 0 {
            eprintln!("{steps} steps, {} paths, {:.0} s", st.paths.len(), t0.elapsed().as_secs_f64());
        }
        let p = rng.below(st.paths.len());
        // work at the last end; flip half of the time
        if rng.below(2) == 0 {
            st.paths[p].reverse();
            st.reindex(p);
        }
        let e = *st.paths[p].last().unwrap();
        let lenp = st.paths[p].len();
        let pred = if lenp >= 2 { Some(st.paths[p][lenp - 2]) } else { None };
        // 1. close or join if possible
        let single = st.paths.len() == 1;
        let first = st.paths[p][0];
        if single && lenp == m && adjacent(e, first) && lenp > 2 {
            break st.paths[p].clone();
        }
        let mut joined = false;
        for &u in nb(e) {
            let q = st.pid[u as usize] as usize;
            if q == p {
                continue;
            }
            let qi = st.idx[u as usize] as usize;
            let ql = st.paths[q].len();
            if qi == 0 || qi == ql - 1 {
                let mut qpath = std::mem::take(&mut st.paths[q]);
                if qi == ql - 1 {
                    qpath.reverse();
                }
                let from = st.paths[p].len();
                st.paths[p].extend(qpath);
                st.paths.swap_remove(q);
                let p2 = if p == st.paths.len() { q } else { p };
                // reindex the moved path and the joined part
                if q < st.paths.len() {
                    st.reindex(q);
                }
                st.reindex_from(p2, if p2 == p { from } else { 0 });
                if p2 != p {
                    st.reindex(p2);
                }
                eprintln!("joined: {} paths left, {steps} steps, {:.0} s", st.paths.len(), t0.elapsed().as_secs_f64());
                joined = true;
                break;
            }
        }
        if joined {
            continue;
        }
        // 2. rotate or re-cut through a random neighbour
        let ns = nb(e);
        let u = ns[rng.below(ns.len())];
        if Some(u) == pred {
            continue;
        }
        let q = st.pid[u as usize] as usize;
        let qi = st.idx[u as usize] as usize;
        if q == p {
            // P = .. u, w .. e  ->  .. u, e .. w ; remove u-w, which must not be a link
            let w = st.paths[p][qi + 1];
            if is_link(u, w) {
                continue;
            }
            st.paths[p][qi + 1..].reverse();
            st.reindex_from(p, qi + 1);
        } else {
            // Q = .. v, u ..  ->  P gets u .. end of Q; Q keeps .. v ; remove v-u
            let ql = st.paths[q].len();
            if qi == 0 || qi == ql - 1 {
                continue; // handled by joining
            }
            let v = st.paths[q][qi - 1];
            if is_link(v, u) {
                continue;
            }
            let tail = st.paths[q].split_off(qi);
            let from = st.paths[p].len();
            st.paths[p].extend(tail);
            st.reindex_from(p, from);
        }
    };
    eprintln!("cycle closed after {steps} steps, {:.0} s", t0.elapsed().as_secs_f64());
    let cycle = c.expand(&closed).unwrap();
    // check against the graph
    let nn = cycle.len();
    for i in 0..nn {
        assert!(graph.has_edge(cycle[i], cycle[(i + 1) % nn]), "not an edge at {i}");
    }
    let mut seen = vec![false; nn + 1];
    for &v in &cycle {
        assert!(!seen[v as usize]);
        seen[v as usize] = true;
    }
    // canonical order: start at 1, second vertex smaller than the last
    let mut cycle = cycle;
    let at = cycle.iter().position(|&v| v == 1).unwrap();
    cycle.rotate_left(at);
    if cycle[1] > cycle[nn - 1] {
        cycle[1..].reverse();
    }
    let text: String = cycle.iter().map(|v| format!("{v}\n")).collect();
    std::fs::write(&a[4], text).unwrap();
    eprintln!("Hamiltonian cycle written to {}", a[4]);
}
