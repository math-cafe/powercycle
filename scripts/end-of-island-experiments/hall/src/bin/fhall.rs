//! fhall K N [OUT]: the f-factor test on the contracted G_K(N).
//!
//! After degree-two propagation, every Hamiltonian cycle of G_K(N) contains
//! every forced chain. In the contracted graph, it uses each link (a chain,
//! end to end) and, besides, exactly f(x) free edges at each node x: one at a
//! chain end, two at a node outside every chain. Necessary condition: the
//! graph of free edges has an f-factor, hence a perfect f-matching of its
//! bipartite double cover (x on the left with capacity f(x), y on the right
//! with capacity f(y), x_L ~ y_R for every free edge xy). This is a perfect
//! matching once every node of capacity 2 is split into two copies. If there
//! is none, the set S of left nodes reachable from an unsaturated copy by
//! alternating paths satisfies f(S) > f(N(S)), N over free edges: then no
//! Hamiltonian cycle exists, since the f(x) free cycle edges at each x in S
//! all end in N(S), where each y takes at most f(y) of them.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use std::collections::VecDeque;

const NIL: u32 = u32::MAX;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: u32 = a[1].parse().unwrap();
    let n: u128 = a[2].parse().unwrap();
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).expect("no obstruction");
    let c = Contraction::new(&propagation).unwrap();
    let m = c.node_count();
    let f = |x: u32| if c.partner(x).is_some() { 1u32 } else { 2u32 };
    // copies: node x has copies base[x] .. base[x] + f(x)
    let mut base = vec![0u32; m + 2];
    for x in 1..=m as u32 {
        base[x as usize + 1] = base[x as usize] + f(x);
    }
    let copies = base[m + 1] as usize;
    let need: u64 = (1..=m as u32).map(|x| u64::from(f(x))).sum();
    // free adjacency of nodes
    let free: Vec<(u32, u32)> = c.free_edges(&propagation).collect();
    let mut deg = vec![0u32; m + 2];
    for &(x, y) in &free {
        deg[x as usize] += 1;
        deg[y as usize] += 1;
    }
    let mut start = vec![0usize; m + 2];
    for x in 1..=m + 1 {
        start[x] = start[x - 1] + deg[x - 1] as usize;
    }
    let mut fill = start.clone();
    let mut adj = vec![0u32; start[m + 1]];
    for &(x, y) in &free {
        adj[fill[x as usize]] = y;
        fill[x as usize] += 1;
        adj[fill[y as usize]] = x;
        fill[y as usize] += 1;
    }
    drop(free);
    eprintln!("{m} nodes, {copies} copies, f-sum {need}, {} free edges", adj.len() / 2);
    let owner: Vec<u32> = (1..=m as u32).flat_map(|x| std::iter::repeat_n(x, f(x) as usize)).collect();
    // left copy i (node owner[i]) is adjacent to every right copy of every free neighbour
    let nbr = |i: usize| {
        let x = owner[i] as usize;
        adj[start[x]..start[x + 1]].iter().flat_map(|&y| base[y as usize]..base[y as usize + 1])
    };
    let mut ml = vec![NIL; copies];
    let mut mr = vec![NIL; copies];
    for i in 0..copies {
        for j in nbr(i) {
            if mr[j as usize] == NIL {
                ml[i] = j;
                mr[j as usize] = i as u32;
                break;
            }
        }
    }
    let mut size = ml.iter().filter(|&&x| x != NIL).count();
    eprintln!("greedy: {size} of {copies}");
    let mut dist = vec![0u32; copies];
    loop {
        let mut q = VecDeque::new();
        for i in 0..copies {
            if ml[i] == NIL {
                dist[i] = 0;
                q.push_back(i as u32);
            } else {
                dist[i] = u32::MAX;
            }
        }
        let mut found = false;
        while let Some(i) = q.pop_front() {
            for j in nbr(i as usize) {
                let v = mr[j as usize];
                if v == NIL {
                    found = true;
                } else if dist[v as usize] == u32::MAX {
                    dist[v as usize] = dist[i as usize] + 1;
                    q.push_back(v);
                }
            }
        }
        if !found {
            break;
        }
        let mut augmented = 0usize;
        let mut it = vec![0u32; copies];
        for root in 0..copies {
            if ml[root] != NIL {
                continue;
            }
            let mut stack = vec![root as u32];
            while let Some(&i) = stack.last() {
                let ns: Vec<u32> = nbr(i as usize).collect();
                if it[i as usize] as usize >= ns.len() {
                    dist[i as usize] = u32::MAX;
                    stack.pop();
                    continue;
                }
                let j = ns[it[i as usize] as usize];
                it[i as usize] += 1;
                let v = mr[j as usize];
                if v == NIL {
                    let mut j = j;
                    for &x in stack.iter().rev() {
                        let prev = ml[x as usize];
                        ml[x as usize] = j;
                        mr[j as usize] = x;
                        j = prev;
                    }
                    augmented += 1;
                    break;
                } else if dist[v as usize] == dist[i as usize] + 1 {
                    stack.push(v);
                }
            }
        }
        size += augmented;
        eprintln!("phase: {size} of {copies}");
        if augmented == 0 {
            break;
        }
    }
    println!("maximum f-matching of the double cover: {size} of {copies}");
    if size == copies {
        println!("perfect f-matching exists: no f-Hall obstruction");
        return;
    }
    let mut in_s = vec![false; copies];
    let mut in_ns = vec![false; copies];
    let mut q = VecDeque::new();
    for i in 0..copies {
        if ml[i] == NIL {
            in_s[i] = true;
            q.push_back(i as u32);
        }
    }
    while let Some(i) = q.pop_front() {
        for j in nbr(i as usize) {
            if !in_ns[j as usize] {
                in_ns[j as usize] = true;
                let v = mr[j as usize];
                if v != NIL && !in_s[v as usize] {
                    in_s[v as usize] = true;
                    q.push_back(v);
                }
            }
        }
    }
    let mut s_nodes: Vec<u32> = (0..copies).filter(|&i| in_s[i]).map(|i| owner[i]).collect();
    s_nodes.dedup();
    let mut n_nodes: Vec<u32> = (0..copies).filter(|&j| in_ns[j]).map(|j| owner[j]).collect();
    n_nodes.dedup();
    let fs: u64 = s_nodes.iter().map(|&x| u64::from(f(x))).sum();
    let fns: u64 = n_nodes.iter().map(|&x| u64::from(f(x))).sum();
    println!("f-Hall violator: |S| = {}, f(S) = {fs}, |N(S)| = {}, f(N(S)) = {fns}", s_nodes.len(), n_nodes.len());
    if let Some(out) = a.get(3) {
        let text: String = s_nodes.iter().map(|&x| format!("{}\n", c.vertex(x))).collect();
        std::fs::write(out, text).unwrap();
        println!("S (as vertices of G) written to {out}");
    }
}
