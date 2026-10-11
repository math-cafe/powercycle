//! The plain-list witness format shared by every tool that writes `cycle.txt`.
//!
//! A witness lists the vertices of a Hamiltonian cycle one per line. Its
//! canonical form is rotated so that vertex `1` comes first and oriented so
//! that the smaller neighbour of `1` comes second, so the same cycle always
//! yields the same bytes whichever tool found it.

use std::error::Error;
use std::fmt;

/// Rotates and orients a cycle into canonical witness order.
pub fn canonical_order(cycle: &[usize]) -> Result<Vec<usize>, WitnessError> {
    if cycle.len() < 3 {
        return Err(WitnessError::TooShort {
            length: cycle.len(),
        });
    }

    let Some(start) = cycle.iter().position(|&vertex| vertex == 1) else {
        return Err(WitnessError::MissingVertexOne);
    };

    let mut forward = Vec::with_capacity(cycle.len());

    for offset in 0..cycle.len() {
        forward.push(cycle[(start + offset) % cycle.len()]);
    }

    if forward[1] < forward[forward.len() - 1] {
        return Ok(forward);
    }

    let mut reverse = Vec::with_capacity(forward.len());
    reverse.push(1);

    for &vertex in forward[1..].iter().rev() {
        reverse.push(vertex);
    }

    Ok(reverse)
}

/// Checks that `cycle` is a Hamiltonian cycle of `G_k(n)` without building
/// the graph: a permutation of `1..=n` whose consecutive sums, closure
/// included, are perfect `k`-th powers.
pub fn check_hamiltonian_cycle(k: u32, n: usize, cycle: &[usize]) -> Result<(), WitnessError> {
    if cycle.len() != n {
        return Err(WitnessError::WrongLength {
            expected: n,
            found: cycle.len(),
        });
    }

    let mut seen = vec![false; n + 1];

    for (index, &vertex) in cycle.iter().enumerate() {
        if vertex == 0 || vertex > n {
            return Err(WitnessError::VertexOutOfRange {
                position: index + 1,
                vertex,
                n,
            });
        }

        if seen[vertex] {
            return Err(WitnessError::DuplicateVertex {
                position: index + 1,
                vertex,
            });
        }

        seen[vertex] = true;
    }

    for (index, &first) in cycle.iter().enumerate() {
        let second = cycle[(index + 1) % cycle.len()];

        if !is_kth_power(first as u128 + second as u128, k) {
            return Err(WitnessError::NotAdjacent {
                position: index + 1,
                first,
                second,
            });
        }
    }

    Ok(())
}

/// Whether `value` is a perfect `k`-th power, in exact integers.
fn is_kth_power(value: u128, k: u32) -> bool {
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

    low.checked_pow(k) == Some(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessError {
    TooShort {
        length: usize,
    },

    MissingVertexOne,

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

    NotAdjacent {
        position: usize,
        first: usize,
        second: usize,
    },
}

impl fmt::Display for WitnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { length } => {
                write!(f, "a cycle of length {length} has no canonical form")
            }

            Self::MissingVertexOne => write!(f, "cycle does not contain vertex 1"),

            Self::WrongLength { expected, found } => {
                write!(f, "cycle has {found} vertices, expected {expected}")
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

            Self::DuplicateVertex { position, vertex } => {
                write!(f, "duplicate vertex {vertex} at position {position}")
            }

            Self::NotAdjacent {
                position,
                first,
                second,
            } => {
                write!(
                    f,
                    "{first} + {second} at position {position} is not a perfect power"
                )
            }
        }
    }
}

impl Error for WitnessError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_order_rotates_to_one() {
        let cycle = [4usize, 7, 3, 1, 5, 9];

        assert_eq!(canonical_order(&cycle).unwrap(), vec![1, 3, 7, 4, 9, 5]);
    }

    #[test]
    fn canonical_order_chooses_smaller_neighbor_second() {
        let cycle = [1usize, 9, 8, 7, 3];

        assert_eq!(canonical_order(&cycle).unwrap(), vec![1, 3, 7, 8, 9]);
    }

    #[test]
    fn canonical_order_rejects_degenerate_cycles() {
        assert_eq!(
            canonical_order(&[1, 2]),
            Err(WitnessError::TooShort { length: 2 })
        );

        assert_eq!(
            canonical_order(&[2, 3, 4]),
            Err(WitnessError::MissingVertexOne)
        );
    }

    /// The unique Hamiltonian cycle of `G_2(32)`.
    const G2_32: [usize; 32] = [
        1, 8, 28, 21, 4, 32, 17, 19, 30, 6, 3, 13, 12, 24, 25, 11, 5, 31, 18, 7, 29, 20, 16, 9, 27,
        22, 14, 2, 23, 26, 10, 15,
    ];

    #[test]
    fn accepts_a_hamiltonian_cycle() {
        assert_eq!(check_hamiltonian_cycle(2, 32, &G2_32), Ok(()));
    }

    #[test]
    fn rejects_broken_cycles() {
        assert!(matches!(
            check_hamiltonian_cycle(2, 33, &G2_32),
            Err(WitnessError::WrongLength { .. })
        ));

        let mut duplicate = G2_32;
        duplicate[5] = 1;
        assert!(matches!(
            check_hamiltonian_cycle(2, 32, &duplicate),
            Err(WitnessError::DuplicateVertex { .. })
        ));

        let mut swapped = G2_32;
        swapped.swap(1, 2);
        assert!(matches!(
            check_hamiltonian_cycle(2, 32, &swapped),
            Err(WitnessError::NotAdjacent { position: 1, .. })
        ));
    }

    #[test]
    fn recognises_powers_exactly() {
        assert!(is_kth_power(893_871_739, 7));
        assert!(!is_kth_power(893_871_740, 7));
        assert!(is_kth_power(0, 3));
        assert!(is_kth_power(1, 9));
    }
}
