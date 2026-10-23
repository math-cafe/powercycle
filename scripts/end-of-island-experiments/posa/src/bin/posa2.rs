//! posa2 K N TOUR OUT [DEPTH] [CAP]
//!
//! Posa's rotation sets on the contracted G_K(N). The LKH tour, cut at its
//! non-edges, gives paths covering every node. For each end of each path, a
//! breadth-first search over rotations (fixing the other end) collects every
//! node that can be made that end: rotating at end e through a neighbour u
//! inside the path, the node after u becomes the end. A rotation removes the
//! path edge after u, which must not be a link (a contracted forced chain).
//! When a node of one path's set is adjacent to a node of another path's set,
//! both paths are rotated accordingly and joined. With one path left, the
//! cycle closes when an end set meets the other end. Rotation sequences are
//! kept as lists of pivots and replayed only when used.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use std::collections::HashMap;
use std::time::Instant;

struct Search {
    /// (parent state, pivot): state 0 is the path as it is
    states: Vec<(u32, u32)>,
    /// endpoint node of each state
    ends: Vec<u32>,
}

/// Original index of the element at current position `pos`, after the
/// reversals of `chain` (pivots, first applied first), on a path of `len`.
fn orig_of(chain: &[u32], len: usize, mut pos: usize) -> usize {
    for &c in chain.iter().rev() {
        let c = c as usize;
        if pos > c {
            pos = c + 1 + (len - 1 - pos);
        }
    }
    pos
}

fn cur_of(chain: &[u32], len: usize, mut pos: usize) -> usize {
    for &c in chain {
        let c = c as usize;
        if pos > c {
            pos = c + 1 + (len - 1 - pos);
        }
    }
    pos
}

fn chain_of(s: &Search, mut id: u32) -> Vec<u32> {
    let mut out = Vec::new();
    while id != 0 {
        let (p, c) = s.states[id as usize];
        out.push(c);
        id = p;
    }
    out.reverse();
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: u32 = a[1].parse().unwrap();
    let n: u128 = a[2].parse().unwrap();
    let depth: usize = a.get(5).map_or(40, |s| s.parse().unwrap());
    let cap: usize = a.get(6).map_or(400_000, |s| s.parse().unwrap());
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).expect("no obstruction");
    let c = Contraction::new(&propagation).unwrap();
    let m = c.node_count();
    let mut deg = vec![0u32; m + 2];
    let free: Vec<(u32, u32)> = c.free_edges(&propagation).collect();
    for &(x, y) in free.iter().chain(c.links().iter()) {
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

    let tour = posum_lkh::problem::parse_tour(&std::fs::read_to_string(&a[3]).unwrap()).unwrap();
    let len = tour.len();
    let cuts: Vec<usize> = (0..len).filter(|&i| !adjacent(tour[i], tour[(i + 1) % len])).collect();
    eprintln!("{m} nodes, {} non-edges in the tour, depth {depth}, cap {cap}", cuts.len());
    let mut paths: Vec<Vec<u32>> = Vec::new();
    if cuts.is_empty() {
        paths.push(tour.clone());
    }
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
        paths.push(p);
    }
    let mut pid = vec![u32::MAX; m + 1];
    let mut idx = vec![0u32; m + 1];
    let reindex = |paths: &Vec<Vec<u32>>, pid: &mut Vec<u32>, idx: &mut Vec<u32>| {
        for (p, path) in paths.iter().enumerate() {
            for (i, &x) in path.iter().enumerate() {
                pid[x as usize] = p as u32;
                idx[x as usize] = i as u32;
            }
        }
    };
    reindex(&paths, &mut pid, &mut idx);
    let t0 = Instant::now();
    if std::env::var("DIAG").is_ok() {
        for (p, path) in paths.iter().enumerate() {
            for side in 0..2 {
                let l = path.len();
                let (e, pred) = if side == 0 { (path[l - 1], path[l - 2]) } else { (path[0], path[1]) };
                let mut line = format!("path {p} len {l} side {side}: end {} (vertex {}), partner {:?}, deg {}:",
                    e, c.vertex(e), c.partner(e).map(|x| c.vertex(x)), nb(e).len());
                for &u in nb(e) {
                    if u == pred { line += " [pred]"; continue; }
                    let q = pid[u as usize] as usize;
                    let i = idx[u as usize] as usize;
                    let ql = paths[q].len();
                    let tag = if q != p {
                        if i == 0 || i == ql - 1 { "END-OF-OTHER".to_string() } else { format!("other path {q}") }
                    } else {
                        let nexti = if side == 0 { i + 1 } else { i.wrapping_sub(1) };
                        let w = paths[p][nexti];
                        if is_link(u, w) { "same path, blocked by link".into() } else { "same path, ROTATABLE".into() }
                    };
                    line += &format!(" [{} {tag}]", c.vertex(u));
                }
                eprintln!("{line}");
            }
        }
        std::process::exit(0);
    }

    // rotation set at the last end of path p (the first end fixed)
    let rotations = |path: &Vec<u32>, p: u32, pid: &Vec<u32>, idx: &Vec<u32>| -> Search {
        let l = path.len();
        let mut s = Search { states: vec![(0, 0)], ends: vec![path[l - 1]] };
        let mut seen: HashMap<u32, ()> = HashMap::new();
        seen.insert(path[l - 1], ());
        let mut level = vec![0u32];
        for _ in 0..depth {
            let mut next = Vec::new();
            for &st in &level {
                let chain = chain_of(&s, st);
                let e = s.ends[st as usize];
                for &u in nb(e) {
                    if pid[u as usize] != p {
                        continue;
                    }
                    let cu = cur_of(&chain, l, idx[u as usize] as usize);
                    if cu + 2 > l - 1 {
                        continue; // u is the end or its predecessor
                    }
                    let w = path[orig_of(&chain, l, cu + 1)];
                    if is_link(u, w) || seen.contains_key(&w) {
                        continue;
                    }
                    seen.insert(w, ());
                    s.states.push((st, cu as u32));
                    s.ends.push(w);
                    next.push((s.states.len() - 1) as u32);
                    if s.states.len() >= cap {
                        return s;
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            level = next;
        }
        s
    };
    let replay = |path: &mut Vec<u32>, chain: &[u32]| {
        for &cv in chain {
            path[cv as usize + 1..].reverse();
        }
    };

    loop {
        let np = paths.len();
        eprintln!("{np} paths, {:.0} s", t0.elapsed().as_secs_f64());
        if np == 1 {
            // close: rotate the last end, look for a neighbour of the first
            let mut done = false;
            for _side in 0..2 {
                let first = paths[0][0];
                let s = rotations(&paths[0], 0, &pid, &idx);
                eprintln!("closing: {} ends reachable", s.ends.len());
                if let Some(st) = (0..s.ends.len()).find(|&i| adjacent(s.ends[i], first)) {
                    let chain = chain_of(&s, st as u32);
                    replay(&mut paths[0], &chain);
                    done = true;
                    break;
                }
                paths[0].reverse();
                reindex(&paths, &mut pid, &mut idx);
            }
            if done {
                break;
            }
            eprintln!("cannot close at this depth");
            std::process::exit(2);
        }
        // rotation sets of every end of every path
        let mut owner: HashMap<u32, (usize, usize, u32)> = HashMap::new(); // node -> (path, side, state)
        let mut sets: Vec<Vec<Search>> = Vec::new();
        let mut found = None;
        'outer: for p in 0..np {
            let mut both = Vec::new();
            for side in 0..2 {
                let mut path = paths[p].clone();
                if side == 1 {
                    path.reverse();
                    for (i, &x) in path.iter().enumerate() {
                        idx[x as usize] = i as u32;
                    }
                }
                let s = rotations(&path, p as u32, &pid, &idx);
                if side == 1 {
                    for (i, &x) in paths[p].iter().enumerate() {
                        idx[x as usize] = i as u32;
                    }
                }
                eprintln!("path {p} side {side}: {} ends reachable", s.ends.len());
                for (st, &x) in s.ends.iter().enumerate() {
                    for &y in nb(x) {
                        if let Some(&(q, qs, qst)) = owner.get(&y)
                            && q != p
                        {
                            found = Some((p, side, st as u32, s.states.len(), q, qs, qst));
                        }
                    }
                    if found.is_some() {
                        both.push(s);
                        sets.push(both);
                        break 'outer;
                    }
                }
                for (st, &x) in s.ends.iter().enumerate() {
                    owner.entry(x).or_insert((p, side, st as u32));
                }
                both.push(s);
            }
            sets.push(both);
        }
        let Some((p, side, st, _, q, qs, qst)) = found else {
            eprintln!("no join at this depth: increase DEPTH or CAP");
            std::process::exit(2);
        };
        // rotate P on its side so that the found node is its last element
        let chain_p = chain_of(&sets[p][side], st);
        let chain_q = chain_of(&sets[q][qs], qst);
        let mut pp = std::mem::take(&mut paths[p]);
        if side == 1 {
            pp.reverse();
        }
        replay(&mut pp, &chain_p);
        let mut qq = std::mem::take(&mut paths[q]);
        if qs == 1 {
            qq.reverse();
        }
        replay(&mut qq, &chain_q);
        assert!(adjacent(*pp.last().unwrap(), *qq.last().unwrap()));
        qq.reverse();
        pp.extend(qq);
        let (lo, hi) = (p.min(q), p.max(q));
        paths.remove(hi);
        paths.remove(lo);
        paths.push(pp);
        reindex(&paths, &mut pid, &mut idx);
        eprintln!("joined paths {p} and {q}");
    }

    eprintln!("cycle closed, {:.0} s", t0.elapsed().as_secs_f64());
    let closed = paths.pop().unwrap();
    for (i, &x) in closed.iter().enumerate() {
        let y = closed[(i + 1) % closed.len()];
        assert!(adjacent(x, y), "not an edge");
    }
    let cycle = c.expand(&closed).expect("every link kept");
    let nn = cycle.len();
    for i in 0..nn {
        assert!(graph.has_edge(cycle[i], cycle[(i + 1) % nn]));
    }
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
