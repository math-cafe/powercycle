//! defects K N TOUR...: the steps of LKH tours of the contracted G_K(N) that
//! are not edges, reported as original vertices, with their degrees.
use posum_core::contraction::Contraction;
use posum_core::propagation::{EdgeState, Propagation};
use posum_core::Graph;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let k: u32 = args[1].parse().unwrap();
    let n: u128 = args[2].parse().unwrap();
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).expect("no obstruction");
    let contraction = Contraction::new(&propagation).unwrap();
    eprintln!("{} nodes", contraction.node_count());
    for path in &args[3..] {
        let text = std::fs::read_to_string(path).unwrap();
        let tour = posum_lkh::problem::parse_tour(&text).unwrap();
        let m = tour.len();
        let mut defects = Vec::new();
        for i in 0..m {
            let (x, y) = (tour[i], tour[(i + 1) % m]);
            if contraction.partner(x) == Some(y) {
                continue;
            }
            let (a, b) = (contraction.vertex(x), contraction.vertex(y));
            let ok = graph.has_edge(a, b) && propagation.state(a, b) == Some(EdgeState::Free);
            if !ok {
                defects.push((i, a.min(b), a.max(b)));
            }
        }
        println!("{path}: {} defects", defects.len());
        for (i, a, b) in defects {
            println!(
                "  pos {i:>9}  {a:>9} - {b:<9} deg {} {}  sum {}  edge {}",
                graph.degree(a),
                graph.degree(b),
                u64::from(a) + u64::from(b),
                graph.has_edge(a, b)
            );
        }
    }
}
