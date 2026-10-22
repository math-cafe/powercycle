//! expand K N TOUR OUT: expand an LKH tour of the contracted G_K(N) to a
//! Hamiltonian cycle, check it, and write it canonical as OUT.txt (one vertex
//! per line) and OUT.bases (bases encoding).
use posum_core::Graph;
use posum_core::contraction::Contraction;
use posum_core::propagation::Propagation;
use posum_core::{bases, cycle, format};
use std::fs::File;
use std::io::BufWriter;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (k, n): (u32, u128) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let graph = Graph::new(k, n).unwrap();
    let propagation = Propagation::run(&graph).unwrap();
    let c = Contraction::new(&propagation).unwrap();
    let text = std::fs::read_to_string(&a[3]).unwrap();
    let tour = posum_lkh::problem::parse_tour(&text).unwrap();
    let mut cyc = c.expand(&tour).unwrap();
    cycle::check(&graph, &cyc).unwrap();
    cycle::canonicalize(&mut cyc);
    let b = bases::encode(&graph, &cyc).unwrap();
    format::write_plain(BufWriter::new(File::create(format!("{}.txt", a[4])).unwrap()), &cyc).unwrap();
    format::write_bases(BufWriter::new(File::create(format!("{}.bases", a[4])).unwrap()), &b).unwrap();
    eprintln!("Hamiltonian cycle of G_{k}({n}): {} vertices, checked", cyc.len());
}
