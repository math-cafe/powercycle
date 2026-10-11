//! Independent verifier for a lower-bound certificate.
//!
//! A certificate is `pc-scan --bands` output covering `1..n-1`, one band per
//! line:
//!
//! ```text
//! <start>..<end>\tIMPOSSIBLE\tdegree<2 vertex=<v> degree=<d>
//! ```
//!
//! with `<start>` alone when the band holds a single value. A band claims that
//! vertex `v` has degree `d <= 1` in `G_k(m)` for every `m` in the range.
//!
//! The degree of `v <= m` counts the `k`-th powers `p` with `v < p <= v + m`,
//! other than the loop position `p = 2v`. As `m` grows the window only widens,
//! so the degree never decreases: equal at both ends means equal throughout.
//! Checking the two endpoints of each band therefore settles every `m` in it.
//!
//! Uses only the standard library, and none of the powercycle crates.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("powercycle-verify-bands: INVALID: {error}");
            ExitCode::FAILURE
        }
    }
}

struct Band {
    start: u128,
    end: u128,
    vertex: u128,
    degree: u128,
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();

    if arguments.len() != 3 {
        return Err("usage: powercycle-verify-bands <k> <n> <bands.txt>".into());
    }

    let k = arguments[0]
        .parse::<u32>()
        .map_err(|_| format!("invalid exponent {:?}", arguments[0]))?;

    if k < 2 {
        return Err(format!("invalid exponent k={k}: expected k >= 2").into());
    }

    let n = arguments[1]
        .parse::<u128>()
        .map_err(|_| format!("invalid threshold {:?}", arguments[1]))?;

    if n < 2 {
        return Err(format!("invalid threshold n={n}: expected n >= 2").into());
    }

    let path = PathBuf::from(&arguments[2]);

    let bytes =
        fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let text = String::from_utf8(bytes)
        .ok()
        .filter(|text| text.is_ascii())
        .ok_or("certificate is not ASCII")?;

    let mut bands = Vec::new();

    for (index, line) in text.lines().enumerate() {
        bands.push(
            parse_band(line)
                .ok_or_else(|| format!("malformed band at line {}: {line:?}", index + 1))?,
        );
    }

    validate(&bands, k, n)?;

    println!("VALID LOWER BOUND");
    println!("k              : {k}");
    println!("n              : {n}");
    println!("bands          : {}", bands.len());
    println!("covers         : 1..{}", n - 1);
    println!("conclusion     : no G_{k}(m) with m < {n} is Hamiltonian");

    Ok(())
}

/// Parses one line strictly: any deviation from the format is rejected.
fn parse_band(line: &str) -> Option<Band> {
    let mut fields = line.split('\t');

    let range = fields.next()?;
    let status = fields.next()?;
    let reason = fields.next()?;

    if fields.next().is_some() || status != "IMPOSSIBLE" {
        return None;
    }

    let (start, end) = match range.split_once("..") {
        Some((start, end)) => (parse_integer(start)?, parse_integer(end)?),

        None => {
            let value = parse_integer(range)?;
            (value, value)
        }
    };

    let rest = reason.strip_prefix("degree<2 vertex=")?;
    let (vertex, degree) = rest.split_once(" degree=")?;

    Some(Band {
        start,
        end,
        vertex: parse_integer(vertex)?,
        degree: parse_integer(degree)?,
    })
}

/// Decimal digits only, so no sign, space or empty field slips through.
fn parse_integer(text: &str) -> Option<u128> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    text.parse().ok()
}

fn validate(bands: &[Band], k: u32, n: u128) -> Result<(), String> {
    if bands.is_empty() {
        return Err("empty certificate".to_owned());
    }

    let mut expected_start = 1u128;

    for (index, band) in bands.iter().enumerate() {
        let line = index + 1;

        if band.start != expected_start {
            return Err(format!(
                "line {line} starts at {}, expected {expected_start}",
                band.start
            ));
        }

        if band.end < band.start {
            return Err(format!(
                "line {line} has end {} < start {}",
                band.end, band.start
            ));
        }

        if band.vertex < 1 || band.vertex > band.start {
            return Err(format!(
                "line {line}: vertex {} is not in G_{k}({})",
                band.vertex, band.start
            ));
        }

        if band.degree > 1 {
            return Err(format!(
                "line {line}: degree {} is not below two",
                band.degree
            ));
        }

        for m in [band.start, band.end] {
            let actual = degree(k, band.vertex, m)?;

            if actual != band.degree {
                return Err(format!(
                    "line {line}: vertex {} has degree {actual} in G_{k}({m}), certificate claims {}",
                    band.vertex, band.degree
                ));
            }
        }

        expected_start = band.end + 1;
    }

    if expected_start != n {
        return Err(format!(
            "bands end at {}, expected {}",
            expected_start - 1,
            n - 1
        ));
    }

    Ok(())
}

/// Degree of `vertex` in `G_k(m)`, for `1 <= vertex <= m`.
fn degree(k: u32, vertex: u128, m: u128) -> Result<u128, String> {
    let overflow = || format!("G_{k}({m}) is beyond u128 arithmetic");

    let window_end = vertex.checked_add(m).ok_or_else(overflow)?;
    let twice = vertex.checked_mul(2).ok_or_else(overflow)?;

    let mut count = kth_root(window_end, k) - kth_root(vertex, k);

    // vertex <= m puts the loop position 2v inside the window when it is a power.
    let root = kth_root(twice, k);

    if root.checked_pow(k) == Some(twice) {
        count -= 1;
    }

    Ok(count)
}

/// `floor(value^(1/k))`, in exact integers.
fn kth_root(value: u128, k: u32) -> u128 {
    let fits = |base: u128| base.checked_pow(k).is_some_and(|power| power <= value);

    let mut low = 0u128;
    let mut high = 1u128;

    while fits(high) {
        high *= 2;
    }

    while high - low > 1 {
        let middle = low + (high - low) / 2;

        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }

    low
}
