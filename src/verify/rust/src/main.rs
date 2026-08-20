use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("powercycle-verify: INVALID: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let options = parse_arguments(&arguments)?;

    let cycle = read_cycle(
        &options.path,
        options.n,
    )?;

    validate_permutation(
        &cycle,
        options.n,
    )?;

    let powers = perfect_powers(
        options.k,
        2 * options.n - 1,
    )?;

    validate_adjacencies(
        &cycle,
        &powers,
    )?;

    let canonical = is_canonical(
        &cycle,
    );

    println!("VALID HAMILTONIAN CYCLE");
    println!("k              : {}", options.k);
    println!("n              : {}", options.n);
    println!("vertices       : {}", cycle.len());
    println!("closing edge   : {} + {}", cycle[cycle.len() - 1], cycle[0]);
    println!("canonical      : {canonical}");

    Ok(())
}

fn read_cycle(
    path: &PathBuf,
    expected_n: usize,
) -> Result<Vec<usize>, VerifyError> {
    let file = File::open(
        path,
    )
    .map_err(
        |error| VerifyError::Io {
            context: format!("cannot open {}", path.display()),
            error,
        },
    )?;

    let reader = BufReader::new(file);
    let mut cycle = Vec::with_capacity(expected_n);

    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(
            |error| VerifyError::Io {
                context: format!(
                    "cannot read line {} from {}",
                    line_index + 1,
                    path.display()
                ),
                error,
            },
        )?;

        if line.trim().is_empty() {
            return Err(
                VerifyError::BlankLine {
                    line: line_index + 1,
                },
            );
        }

        if line.trim() != line {
            return Err(
                VerifyError::Whitespace {
                    line: line_index + 1,
                },
            );
        }

        let vertex = line
            .parse::<usize>()
            .map_err(
                |_| VerifyError::InvalidInteger {
                    line: line_index + 1,
                    value: line,
                },
            )?;

        cycle.push(vertex);
    }

    if cycle.len() != expected_n {
        return Err(
            VerifyError::WrongLength {
                expected: expected_n,
                found: cycle.len(),
            },
        );
    }

    Ok(cycle)
}

fn validate_permutation(
    cycle: &[usize],
    n: usize,
) -> Result<(), VerifyError> {
    let mut seen = vec![false; n + 1];

    for (position, &vertex) in cycle.iter().enumerate() {
        if vertex == 0 || vertex > n {
            return Err(
                VerifyError::VertexOutOfRange {
                    position: position + 1,
                    vertex,
                    n,
                },
            );
        }

        if seen[vertex] {
            return Err(
                VerifyError::DuplicateVertex {
                    position: position + 1,
                    vertex,
                },
            );
        }

        seen[vertex] = true;
    }

    for vertex in 1..=n {
        if !seen[vertex] {
            return Err(
                VerifyError::MissingVertex {
                    vertex,
                },
            );
        }
    }

    Ok(())
}

fn perfect_powers(
    k: u32,
    maximum: usize,
) -> Result<HashSet<usize>, VerifyError> {
    if k < 2 {
        return Err(
            VerifyError::ExponentTooSmall {
                k,
            },
        );
    }

    let maximum = maximum as u128;
    let mut powers = HashSet::new();
    let mut base = 1u128;

    loop {
        let Some(power) = base.checked_pow(k) else {
            break;
        };

        if power > maximum {
            break;
        }

        powers.insert(
            power as usize,
        );

        base += 1;
    }

    Ok(powers)
}

fn validate_adjacencies(
    cycle: &[usize],
    powers: &HashSet<usize>,
) -> Result<(), VerifyError> {
    for index in 0..cycle.len() {
        let next = (index + 1) % cycle.len();

        let first = cycle[index];
        let second = cycle[next];
        let sum = first + second;

        if !powers.contains(
            &sum,
        ) {
            return Err(
                VerifyError::InvalidAdjacency {
                    position: index + 1,
                    first,
                    second,
                    sum,
                },
            );
        }
    }

    Ok(())
}

fn is_canonical(
    cycle: &[usize],
) -> bool {
    cycle.len() >= 3
        && cycle[0] == 1
        && cycle[1] < cycle[cycle.len() - 1]
}

fn parse_arguments(
    arguments: &[String],
) -> Result<Options, VerifyError> {
    if arguments.len() != 3 {
        return Err(
            VerifyError::Usage,
        );
    }

    let k = arguments[0]
        .parse::<u32>()
        .map_err(
            |_| VerifyError::InvalidExponent {
                value: arguments[0].clone(),
            },
        )?;

    if k < 2 {
        return Err(
            VerifyError::ExponentTooSmall {
                k,
            },
        );
    }

    let n = arguments[1]
        .parse::<usize>()
        .map_err(
            |_| VerifyError::InvalidVertexCount {
                value: arguments[1].clone(),
            },
        )?;

    if n < 3 {
        return Err(
            VerifyError::VertexCountTooSmall {
                n,
            },
        );
    }

    Ok(
        Options {
            k,
            n,
            path: PathBuf::from(
                &arguments[2],
            ),
        },
    )
}

#[derive(Debug)]
struct Options {
    k: u32,
    n: usize,
    path: PathBuf,
}

#[derive(Debug)]
enum VerifyError {
    Usage,

    Io {
        context: String,
        error: std::io::Error,
    },

    InvalidExponent {
        value: String,
    },

    ExponentTooSmall {
        k: u32,
    },

    InvalidVertexCount {
        value: String,
    },

    VertexCountTooSmall {
        n: usize,
    },

    BlankLine {
        line: usize,
    },

    Whitespace {
        line: usize,
    },

    InvalidInteger {
        line: usize,
        value: String,
    },

    WrongLength {
        expected: usize,
        found: usize,
    },

    VertexOutOfRange {
        position: usize,
        vertex: usize,
        n: usize,
    },

    DuplicateVertex {
        position: usize,
        vertex: usize,
    },

    MissingVertex {
        vertex: usize,
    },

    InvalidAdjacency {
        position: usize,
        first: usize,
        second: usize,
        sum: usize,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(
                    f,
                    "usage: powercycle-verify <k> <n> <cycle.txt>"
                )
            }

            Self::Io {
                context,
                error,
            } => {
                write!(
                    f,
                    "{context}: {error}"
                )
            }

            Self::InvalidExponent {
                value,
            } => {
                write!(
                    f,
                    "invalid exponent {value:?}"
                )
            }

            Self::ExponentTooSmall {
                k,
            } => {
                write!(
                    f,
                    "invalid exponent k={k}: expected k >= 2"
                )
            }

            Self::InvalidVertexCount {
                value,
            } => {
                write!(
                    f,
                    "invalid vertex count {value:?}"
                )
            }

            Self::VertexCountTooSmall {
                n,
            } => {
                write!(
                    f,
                    "invalid vertex count n={n}: expected n >= 3"
                )
            }

            Self::BlankLine {
                line,
            } => {
                write!(
                    f,
                    "blank line at line {line}"
                )
            }

            Self::Whitespace {
                line,
            } => {
                write!(
                    f,
                    "unexpected leading or trailing whitespace at line {line}"
                )
            }

            Self::InvalidInteger {
                line,
                value,
            } => {
                write!(
                    f,
                    "invalid integer {value:?} at line {line}"
                )
            }

            Self::WrongLength {
                expected,
                found,
            } => {
                write!(
                    f,
                    "wrong witness length: expected {expected}, found {found}"
                )
            }

            Self::VertexOutOfRange {
                position,
                vertex,
                n,
            } => {
                write!(
                    f,
                    "vertex {vertex} at position {position} is outside 1..={n}"
                )
            }

            Self::DuplicateVertex {
                position,
                vertex,
            } => {
                write!(
                    f,
                    "duplicate vertex {vertex} at position {position}"
                )
            }

            Self::MissingVertex {
                vertex,
            } => {
                write!(
                    f,
                    "missing vertex {vertex}"
                )
            }

            Self::InvalidAdjacency {
                position,
                first,
                second,
                sum,
            } => {
                write!(
                    f,
                    "invalid adjacency at position {position}: \
                     {first} + {second} = {sum} is not a perfect kth power"
                )
            }
        }
    }
}

impl Error for VerifyError {
    fn source(
        &self,
    ) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io {
                error,
                ..
            } => Some(error),

            _ => None,
        }
    }
}
