//! classes K N [TOUR...]: the contracted nodes of G_K(N) by degree and by the
//! offset of their forced chain (partner vertex minus vertex), and where the
//! non-edges of the given LKH tours fall in this classification.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use std::collections::BTreeMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: u32 = a[1].parse().unwrap();
    let n: u128 = a[2].parse().unwrap();
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).unwrap();
    let c = Contraction::new(&propagation).unwrap();
    let m = c.node_count() as u32;
    let mut cdeg = vec![0u32; m as usize + 1];
    for (x, y) in c.free_edges(&propagation) {
        cdeg[x as usize] += 1;
        cdeg[y as usize] += 1;
    }
    for &(x, y) in c.links() {
        cdeg[x as usize] += 1;
        cdeg[y as usize] += 1;
    }
    let offset = |x: u32| c.partner(x).map(|y| i64::from(c.vertex(y)) - i64::from(c.vertex(x)));
    // classes (contracted degree, offset) with counts
    let mut classes: BTreeMap<(u32, i64), (u64, u32, u32)> = BTreeMap::new();
    for x in 1..=m {
        let key = (cdeg[x as usize], offset(x).unwrap_or(0));
        let v = c.vertex(x);
        let e = classes.entry(key).or_insert((0, u32::MAX, 0));
        e.0 += 1;
        e.1 = e.1.min(v);
        e.2 = e.2.max(v);
    }
    let mut top: Vec<_> = classes.iter().collect();
    top.sort_by_key(|(_, v)| std::cmp::Reverse(v.0));
    println!("largest classes (contracted degree, chain offset): count, vertex range");
    for ((d, o), (cnt, lo, hi)) in top.iter().take(25) {
        println!("  deg {d:>2} offset {o:>10}: {cnt:>9}  [{lo}, {hi}]");
    }
    let d3: u64 = classes.iter().filter(|((d, _), _)| *d == 3).map(|(_, v)| v.0).sum();
    println!("contracted degree 3: {d3} of {m} nodes");
    for path in &a[3..] {
        let tour = posum_lkh::problem::parse_tour(&std::fs::read_to_string(path).unwrap()).unwrap();
        let l = tour.len();
        println!("{path}:");
        for i in 0..l {
            let (x, y) = (tour[i], tour[(i + 1) % l]);
            if c.partner(x) == Some(y) {
                continue;
            }
            let (u, v) = (c.vertex(x), c.vertex(y));
            if graph.has_edge(u, v) {
                continue;
            }
            println!(
                "  {u:>9} (deg {}, offset {:?})  -  {v:>9} (deg {}, offset {:?})",
                cdeg[x as usize], offset(x), cdeg[y as usize], offset(y)
            );
        }
    }
}
