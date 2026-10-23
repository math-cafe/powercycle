//! hall K N [OUT]: does G_K(N) have a perfect 2-matching?
//!
//! A Hamiltonian cycle is a 2-factor, hence a perfect 2-matching, which
//! exists if and only if the bipartite double cover (left and right copies of
//! every vertex, u_L ~ v_R for every edge uv) has a perfect matching. Found
//! with Hopcroft-Karp. If there is none, the left vertices reachable from an
//! unmatched one by alternating paths form a set S with |N(S)| < |S|, a
//! certificate that G_K(N) is not Hamiltonian; it is written to OUT.
use posum_core::Graph;
use std::collections::VecDeque;

const NIL: u32 = u32::MAX;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: u32 = a[1].parse().unwrap();
    let n: usize = a[2].parse().unwrap();
    let graph = Graph::new(k, n as u128).unwrap();
    // CSR adjacency, vertices 0..n
    let mut start = vec![0u64; n + 1];
    for v in 1..=n as u32 {
        start[v as usize] = start[v as usize - 1] + u64::from(graph.degree(v));
    }
    let mut adj = vec![0u32; start[n] as usize];
    for v in 1..=n as u32 {
        let s = start[v as usize - 1] as usize;
        for (i, w) in graph.neighbours(v).enumerate() {
            adj[s + i] = w - 1;
        }
    }
    eprintln!("{} vertices, {} arcs", n, adj.len());
    let nb = |v: usize| &adj[start[v] as usize..start[v + 1] as usize];
    let mut ml = vec![NIL; n]; // left -> right
    let mut mr = vec![NIL; n]; // right -> left
    // greedy, lowest degree first
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_by_key(|&v| nb(v as usize).len());
    for &u in &order {
        for &w in nb(u as usize) {
            if mr[w as usize] == NIL {
                ml[u as usize] = w;
                mr[w as usize] = u;
                break;
            }
        }
    }
    let mut size = ml.iter().filter(|&&x| x != NIL).count();
    eprintln!("greedy: {size}");
    let mut dist = vec![0u32; n];
    loop {
        // BFS from free left vertices
        let mut q = VecDeque::new();
        for u in 0..n {
            if ml[u] == NIL {
                dist[u] = 0;
                q.push_back(u as u32);
            } else {
                dist[u] = u32::MAX;
            }
        }
        let mut found = false;
        while let Some(u) = q.pop_front() {
            for &w in nb(u as usize) {
                let v = mr[w as usize];
                if v == NIL {
                    found = true;
                } else if dist[v as usize] == u32::MAX {
                    dist[v as usize] = dist[u as usize] + 1;
                    q.push_back(v);
                }
            }
        }
        if !found {
            break;
        }
        // DFS augment, iterative
        let mut it = vec![0u32; n];
        let mut augmented = 0usize;
        for root in 0..n {
            if ml[root] != NIL {
                continue;
            }
            let mut stack: Vec<u32> = vec![root as u32];
            let mut done = false;
            while let Some(&u) = stack.last() {
                let ns = nb(u as usize);
                if (it[u as usize] as usize) >= ns.len() {
                    dist[u as usize] = u32::MAX;
                    stack.pop();
                    continue;
                }
                let w = ns[it[u as usize] as usize];
                it[u as usize] += 1;
                let v = mr[w as usize];
                if v == NIL {
                    // augment along the stack
                    let mut w = w;
                    for &x in stack.iter().rev() {
                        let prev = ml[x as usize];
                        ml[x as usize] = w;
                        mr[w as usize] = x;
                        w = prev;
                    }
                    done = true;
                    break;
                } else if dist[v as usize] == dist[u as usize] + 1 {
                    stack.push(v);
                }
            }
            if done {
                augmented += 1;
            }
        }
        size += augmented;
        eprintln!("phase: {size} matched");
        if augmented == 0 {
            break;
        }
    }
    println!("maximum matching of the double cover: {size} of {n}");
    if size == n {
        println!("perfect 2-matching exists: no Hall obstruction");
        return;
    }
    // Hall violator: left vertices reachable from free left ones by alternating paths
    let mut in_s = vec![false; n];
    let mut in_ns = vec![false; n];
    let mut q = VecDeque::new();
    for u in 0..n {
        if ml[u] == NIL {
            in_s[u] = true;
            q.push_back(u as u32);
        }
    }
    while let Some(u) = q.pop_front() {
        for &w in nb(u as usize) {
            if !in_ns[w as usize] {
                in_ns[w as usize] = true;
                let v = mr[w as usize];
                if v != NIL && !in_s[v as usize] {
                    in_s[v as usize] = true;
                    q.push_back(v);
                }
            }
        }
    }
    let s: Vec<usize> = (0..n).filter(|&u| in_s[u]).map(|u| u + 1).collect();
    let ns: Vec<usize> = (0..n).filter(|&w| in_ns[w]).map(|w| w + 1).collect();
    println!("Hall violator: |S| = {}, |N(S)| = {}", s.len(), ns.len());
    if let Some(out) = a.get(3) {
        let text: String = s.iter().map(|v| format!("{v}\n")).collect();
        std::fs::write(out, text).unwrap();
        println!("S written to {out}");
    }
}
