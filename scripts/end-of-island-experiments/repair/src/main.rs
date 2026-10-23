//! repair K N TOUR RADIUS OUT.pcdump.json
//!
//! From an LKH tour of the contracted G_K(N) that uses a few non-edges, build
//! a 2-factor of G_K(N) close to it: keep every tour edge, except within
//! graph distance RADIUS of a defect's ends, where CaDiCaL picks edges of the graph so
//! that every vertex gets degree two. The result, a giant cycle and small
//! satellites, is written as a powercycle dump for pc-patch.
use std::collections::{HashMap, HashSet};

use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use powercycle_core::{CycleDecomposition, EdgeId, Factor, PowerGraph, Vertex};
use powercycle_format::{Dump, Provenance};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n): (u32, usize) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let radius: usize = a[4].parse().unwrap();
    let graph = Graph::new(k, n as u128).unwrap();
    let propagation = Propagation::run(&graph).expect("no obstruction");
    let contraction = Contraction::new(&propagation).unwrap();
    let tour = posum_lkh::problem::parse_tour(&std::fs::read_to_string(&a[3]).unwrap()).unwrap();
    // expand: the vertex sequence, the chains included
    let cycle = contraction.expand(&tour).unwrap();
    let len = cycle.len();
    assert_eq!(len, n);
    let pg = PowerGraph::new(k, n).unwrap();
    let vx = |v: u32| Vertex::from_number(v as usize).unwrap();
    let defects: Vec<usize> = (0..len)
        .filter(|&i| pg.edge_between(vx(cycle[i]), vx(cycle[(i + 1) % len])).is_none())
        .collect();
    eprintln!("{} defects at positions {:?}", defects.len(), defects);

    // the free zone: the vertices within RADIUS of a defect's ends in the graph
    let mut zone: HashSet<u32> = HashSet::new();
    let mut frontier: Vec<u32> = defects
        .iter()
        .flat_map(|&d| [cycle[d], cycle[(d + 1) % len]])
        .collect();
    zone.extend(frontier.iter().copied());
    for _ in 0..radius {
        let mut next = Vec::new();
        for &v in &frontier {
            for &e in pg.incident(vx(v)).unwrap() {
                let w = pg.other(e, vx(v)).unwrap().number() as u32;
                if zone.insert(w) {
                    next.push(w);
                }
            }
        }
        frontier = next;
    }
    // kept edges: tour steps not inside the zone (both ends in the zone means freed)
    let mut kept: Vec<EdgeId> = Vec::new();
    let mut kept_deg: HashMap<u32, usize> = HashMap::new();
    for i in 0..len {
        let (u, v) = (cycle[i], cycle[(i + 1) % len]);
        if zone.contains(&u) && zone.contains(&v) {
            continue;
        }
        let e = pg.edge_between(vx(u), vx(v)).expect("a defect lies inside the zone");
        kept.push(e);
        *kept_deg.entry(u).or_default() += 1;
        *kept_deg.entry(v).or_default() += 1;
    }
    // SAT on the edges with both ends in the zone
    let mut var_of: HashMap<usize, Lit> = HashMap::new();
    let mut edges: Vec<EdgeId> = Vec::new();
    let mut solver = CaDiCaL::default();
    let mut next = 0u32;
    for &v in &zone {
        for &e in pg.incident(vx(v)).unwrap() {
            let w = pg.other(e, vx(v)).unwrap().number() as u32;
            if zone.contains(&w) && !var_of.contains_key(&e.index()) {
                var_of.insert(e.index(), Lit::positive(next));
                edges.push(e);
                next += 1;
            }
        }
    }
    let mut clauses = 0usize;
    for &v in &zone {
        let need = 2 - kept_deg.get(&v).copied().unwrap_or(0);
        let lits: Vec<Lit> = pg
            .incident(vx(v))
            .unwrap()
            .iter()
            .filter_map(|e| var_of.get(&e.index()).copied())
            .collect();
        let m = lits.len();
        let mut add = |c: Vec<Lit>| {
            solver.add_clause(Clause::from_iter(c)).unwrap();
            clauses += 1;
        };
        // at least `need`
        if need >= 1 {
            if need == 1 {
                add(lits.clone());
            } else {
                for skip in 0..m {
                    add(lits.iter().enumerate().filter(|&(i, _)| i != skip).map(|(_, &l)| l).collect());
                }
            }
        }
        // at most `need`
        let subsets: Vec<Vec<usize>> = match need {
            0 => (0..m).map(|i| vec![i]).collect(),
            1 => (0..m).flat_map(|i| (i + 1..m).map(move |j| vec![i, j])).collect(),
            _ => (0..m)
                .flat_map(|i| (i + 1..m).flat_map(move |j| (j + 1..m).map(move |l| vec![i, j, l])))
                .collect(),
        };
        for s in subsets {
            add(s.iter().map(|&i| !lits[i]).collect());
        }
    }
    eprintln!("zone {} vertices, {} edge variables, {} clauses", zone.len(), edges.len(), clauses);
    match solver.solve().unwrap() {
        SolverResult::Sat => {}
        r => {
            eprintln!("no degree-two completion in this zone: {r:?}; try a larger radius");
            std::process::exit(2);
        }
    }
    let mut selected = kept;
    for e in &edges {
        if solver.lit_val(var_of[&e.index()]).unwrap() == TernaryVal::True {
            selected.push(*e);
        }
    }
    selected.sort_unstable();
    let factor = Factor::new(&pg, selected).unwrap();
    factor.validate_two_factor(&pg).unwrap();
    let dec = CycleDecomposition::new(&pg, &factor).unwrap();
    eprintln!(
        "2-factor: {} cycles, giant {}, residual {}",
        dec.cycle_count(),
        dec.largest_cycle().len(),
        dec.residual_vertex_count()
    );
    let dump = Dump::new(&pg, &factor, &factor, Provenance::new("repair-from-lkh", "0.1"), Vec::new()).unwrap();
    dump.write(&a[5]).unwrap();
    eprintln!("wrote {}", a[5]);
}
