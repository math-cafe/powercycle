use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use powercycle_core::{CycleDecomposition, canonical_order};
use powercycle_format::Dump;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-export: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let input = parse_arguments(&arguments)?;

    let dump = Dump::read(&input)?;
    let graph = dump.rebuild_graph()?;
    let factor = dump.build_current_factor(&graph)?;
    let decomposition = CycleDecomposition::new(&graph, &factor)?;

    if !decomposition.is_hamiltonian() {
        return Err(ExportError::NotHamiltonian {
            cycles: decomposition.cycle_count(),
            giant: decomposition.largest_cycle().len(),
            residual: decomposition.residual_vertex_count(),
        }
        .into());
    }

    let cycle = decomposition
        .largest_cycle()
        .vertices()
        .iter()
        .map(|vertex| vertex.number())
        .collect::<Vec<_>>();

    let canonical = canonical_order(&cycle)?;

    eprintln!("pc-export: dump {}", dump.dump_id);
    eprintln!("pc-export: G_{}({})", graph.k(), graph.vertex_count());
    eprintln!(
        "pc-export: canonical start {}, second {}, last {}",
        canonical[0],
        canonical[1],
        canonical[canonical.len() - 1]
    );

    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());

    for vertex in canonical {
        writeln!(output, "{vertex}")?;
    }

    output.flush()?;

    Ok(())
}

fn parse_arguments(arguments: &[String]) -> Result<PathBuf, ExportError> {
    if arguments.len() != 1 {
        return Err(ExportError::Usage);
    }

    Ok(PathBuf::from(&arguments[0]))
}

#[derive(Debug)]
enum ExportError {
    Usage,

    NotHamiltonian {
        cycles: usize,
        giant: usize,
        residual: usize,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(f, "usage: pc-export <dump.pcdump.json>")
            }

            Self::NotHamiltonian {
                cycles,
                giant,
                residual,
            } => {
                write!(
                    f,
                    "dump current factor is not Hamiltonian: \
                     {cycles} cycles, giant {giant}, residual {residual}"
                )
            }
        }
    }
}

impl Error for ExportError {}
