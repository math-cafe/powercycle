//! zones K N CUT...: chains and free edges of the contracted G_K(N), counted
//! by pair of zones. Zone i is [CUT_i, CUT_{i+1} - 1], with CUT_0 = 1.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::power::root;
use posum_core::propagation::{EdgeState, Propagation};
use std::collections::BTreeMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n): (u32, u128) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let mut cuts: Vec<u32> = vec![1];
    cuts.extend(a[3..].iter().map(|s| s.parse::<u32>().unwrap()));
    let zone = |v: u32| cuts.partition_point(|&c| c <= v) - 1;
    let graph = Graph::new(k, n).unwrap();
    let p = Propagation::run(&graph).unwrap();
    let c = Contraction::new(&p).unwrap();
    println!("zones:");
    for (i, w) in cuts.iter().enumerate() {
        let hi = cuts.get(i + 1).map_or(n as u32, |h| h - 1);
        println!("  Z{i:<2} [{w}, {hi}]");
    }
    let mut chains: BTreeMap<(usize, usize), (u64, u64)> = BTreeMap::new();
    for &(x, y) in c.links() {
        let (zx, zy) = (zone(c.vertex(x)), zone(c.vertex(y)));
        let e = chains.entry((zx.min(zy), zx.max(zy))).or_default();
        e.0 += 1;
    }
    for ch in c.chains() {
        let (zx, zy) = (zone(ch[0]), zone(ch[ch.len() - 1]));
        chains.entry((zx.min(zy), zx.max(zy))).or_default().1 += ch.len() as u64 - 2;
    }
    println!("\nchains by zones of their ends: count, interior vertices");
    for ((i, j), (cnt, int)) in &chains { println!("  Z{i:<2} - Z{j:<2} {cnt:>11} {int:>11}"); }
    let mut free: BTreeMap<(usize, usize, u128), u64> = BTreeMap::new();
    for (x, y, s) in p.edges() {
        if s != EdgeState::Free { continue; }
        let (zx, zy) = (zone(x), zone(y));
        *free.entry((zx.min(zy), zx.max(zy), root(u128::from(x) + u128::from(y), k))).or_default() += 1;
    }
    println!("\nfree edges by zones and base (counts above 0.1 % of all):");
    let total: u64 = free.values().sum();
    for ((i, j, b), cnt) in &free {
        if *cnt * 1000 > total { println!("  Z{i:<2} - Z{j:<2} base {b:>2} {cnt:>11}"); }
    }
}
