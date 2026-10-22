//! structure K N: what is left after propagation and contraction of G_K(N).
//! Nodes by kind (chain end or free vertex) and free degree, free edges by
//! power, and node positions in 32 bins of [1, N].
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::power::root;
use posum_core::propagation::{EdgeState, Propagation};
use std::collections::BTreeMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n): (u32, u128) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let t = std::time::Instant::now();
    let graph = Graph::new(k, n).unwrap();
    let p = Propagation::run(&graph).unwrap();
    eprintln!("propagation {:.0} s", t.elapsed().as_secs_f64());
    let c = Contraction::new(&p).unwrap();
    let m = c.node_count();
    println!("G_{k}({n}): {m} nodes ({:.1} % of n), {} chains, {} free edges",
        100.0 * m as f64 / n as f64, c.chain_count(), p.free_edges());
    let mut fdeg = vec![0u8; m + 1];
    let mut by_base: BTreeMap<u128, u64> = BTreeMap::new();
    for (x, y, s) in p.edges() {
        if s != EdgeState::Free { continue; }
        *by_base.entry(root(u128::from(x) + u128::from(y), k)).or_default() += 1;
        for v in [x, y] {
            let i = c.node(v).unwrap() as usize;
            fdeg[i] = fdeg[i].saturating_add(1);
        }
    }
    let mut kinds: BTreeMap<(&str, u8), u64> = BTreeMap::new();
    let bins = 32u128;
    let mut pos = vec![[0u64; 3]; bins as usize];
    for x in 1..=m as u32 {
        let end = c.partner(x).is_some();
        let d = fdeg[x as usize];
        *kinds.entry((if end { "chain end" } else { "free vertex" }, d.min(9))).or_default() += 1;
        let b = ((u128::from(c.vertex(x)) - 1) * bins / n) as usize;
        let col = if !end { 2 } else if d == 2 { 0 } else { 1 };
        pos[b][col] += 1;
    }
    println!("\nnodes by kind and free degree (9 = 9 or more):");
    for ((kind, d), cnt) in &kinds {
        println!("  {kind:<12} {d}  {cnt:>11}  {:5.1} %", 100.0 * *cnt as f64 / m as f64);
    }
    println!("\nfree edges by base:");
    for (b, cnt) in &by_base { println!("  {b:>3}  {cnt:>11}"); }
    println!("\nnode positions: bin start, chain ends of free degree 2, other chain ends, free vertices");
    for (i, r) in pos.iter().enumerate() {
        println!("  {:>11}  {:>10} {:>10} {:>10}", 1 + i as u128 * n / bins, r[0], r[1], r[2]);
    }
}
