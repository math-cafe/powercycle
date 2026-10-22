//! motif SRC.hcp OUT.hcp: impose on G_6(16777215) the motif of
//! doc/en/end-of-island.md, section 5: for every x <= 2056509 but the fixed
//! point 1028255, the high ends h_x = x + D and h_{2056510 - x} are joined
//! (their sum is 16^6). Each such edge must be a free edge of the graph after
//! propagation, and no node may receive two imposed edges. The edges are
//! appended to the fixed edges of SRC.hcp, written as OUT.hcp.
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::{EdgeState, Propagation};
use std::io::{BufRead, BufReader, BufWriter, Write};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n) = (6u32, 16u128.pow(6) - 1);
    let d: u32 = 17u32.pow(6) - 16u32.pow(6);
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).unwrap();
    let c = Contraction::new(&propagation).unwrap();
    let mut used = vec![false; c.node_count() + 1];
    let mut pairs = Vec::new();
    let (mut forced, mut forbidden) = (0u32, 0u32);
    for x in 1..1_028_255u32 {
        let (h1, h2) = (x + d, 2_056_510 - x + d);
        assert!(graph.has_edge(h1, h2), "not an edge: {h1} {h2}");
        match propagation.state(h1, h2) {
            Some(EdgeState::Free) => {}
            Some(EdgeState::Forced) => {
                forced += 1;
                continue;
            }
            other => {
                forbidden += 1;
                eprintln!("motif edge {h1} {h2} is {other:?}");
                continue;
            }
        }
        let (y1, y2) = (c.node(h1).expect("a node"), c.node(h2).expect("a node"));
        assert!(c.partner(y1).is_some() && c.partner(y2).is_some(), "chain ends expected");
        assert!(!used[y1 as usize] && !used[y2 as usize], "a node gets two imposed edges");
        used[y1 as usize] = true;
        used[y2 as usize] = true;
        pairs.push((y1, y2));
    }
    eprintln!("{} imposed edges; {forced} already forced, {forbidden} forbidden", pairs.len());
    let src = BufReader::with_capacity(1 << 22, std::fs::File::open(&a[1]).unwrap());
    let mut out = BufWriter::with_capacity(1 << 22, std::fs::File::create(&a[2]).unwrap());
    let mut in_fixed = false;
    for line in src.lines() {
        let line = line.unwrap();
        if line == "FIXED_EDGES_SECTION" {
            in_fixed = true;
        } else if in_fixed && line == "-1" {
            for &(y1, y2) in &pairs {
                writeln!(out, "{y1} {y2}").unwrap();
            }
            in_fixed = false;
        }
        if line.starts_with("COMMENT") {
            writeln!(out, "{line}; motif (b) for x <= 2056509 imposed").unwrap();
        } else {
            writeln!(out, "{line}").unwrap();
        }
    }
    out.flush().unwrap();
    eprintln!("wrote {}", a[2]);
}
