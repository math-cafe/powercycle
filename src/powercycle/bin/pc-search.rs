use std::error::Error;
use std::fmt;
use std::process::ExitCode;

use powercycle_config::Config;
use powercycle_search::{Invocation, ProgramInfo, RunContext};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("pc-search: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let invocation = Invocation::capture()?;

    let problem = parse_problem(&invocation.argv)?;

    let config = Config::load()?;

    let program = ProgramInfo::new("pc-search", env!("CARGO_PKG_VERSION"));

    eprintln!("pc-search: creating run for G_{}({})", problem.k, problem.n);

    let mut run = RunContext::create(&config, problem.k, problem.n, program, invocation)?;

    eprintln!("pc-search: run {}", run.id());

    eprintln!("pc-search: directory {}", run.paths().root.display());

    // No search engine is connected yet.
    //
    // This executable currently exercises the complete run lifecycle:
    //
    // configuration
    //     ↓
    // invocation capture
    //     ↓
    // run creation
    //     ↓
    // persistent metadata/log/status
    //     ↓
    // clean completion
    run.log_info("no search engine configured yet")?;

    run.mark_completed()?;

    eprintln!("pc-search: completed");

    // stdout is deliberately machine-consumable:
    // only the resulting run path is written here.
    println!("{}", run.paths().root.display());

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Problem {
    k: u32,
    n: usize,
}

fn parse_problem(arguments: &[String]) -> Result<Problem, CliError> {
    if arguments.len() != 2 {
        return Err(CliError::Usage);
    }

    let k = arguments[0]
        .parse::<u32>()
        .map_err(|_| CliError::InvalidExponent {
            value: arguments[0].clone(),
        })?;

    let n = arguments[1]
        .parse::<usize>()
        .map_err(|_| CliError::InvalidVertexCount {
            value: arguments[1].clone(),
        })?;

    if k < 2 {
        return Err(CliError::ExponentTooSmall { k });
    }

    if n == 0 {
        return Err(CliError::VertexCountZero);
    }

    Ok(Problem { k, n })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CliError {
    Usage,

    InvalidExponent { value: String },

    InvalidVertexCount { value: String },

    ExponentTooSmall { k: u32 },

    VertexCountZero,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => {
                write!(f, "usage: pc-search <k> <n>")
            }

            Self::InvalidExponent { value } => {
                write!(f, "invalid exponent {value:?}: expected an integer k >= 2")
            }

            Self::InvalidVertexCount { value } => {
                write!(
                    f,
                    "invalid vertex count {value:?}: expected an integer n >= 1"
                )
            }

            Self::ExponentTooSmall { k } => {
                write!(f, "invalid exponent k={k}: expected k >= 2")
            }

            Self::VertexCountZero => {
                write!(f, "invalid vertex count n=0: expected n >= 1")
            }
        }
    }
}

impl Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_valid_problem() {
        assert_eq!(
            parse_problem(&args(&["5", "490463"])),
            Ok(Problem { k: 5, n: 490_463 })
        );
    }

    #[test]
    fn rejects_missing_arguments() {
        assert_eq!(parse_problem(&args(&["5"])), Err(CliError::Usage));
    }

    #[test]
    fn rejects_extra_arguments() {
        assert_eq!(
            parse_problem(&args(&["5", "490463", "extra",])),
            Err(CliError::Usage)
        );
    }

    #[test]
    fn rejects_invalid_exponent() {
        assert_eq!(
            parse_problem(&args(&["five", "490463",])),
            Err(CliError::InvalidExponent {
                value: "five".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["1", "490463"])),
            Err(CliError::ExponentTooSmall { k: 1 })
        );
    }

    #[test]
    fn rejects_invalid_vertex_count() {
        assert_eq!(
            parse_problem(&args(&["5", "many"])),
            Err(CliError::InvalidVertexCount {
                value: "many".to_owned(),
            })
        );

        assert_eq!(
            parse_problem(&args(&["5", "0"])),
            Err(CliError::VertexCountZero)
        );
    }
}
