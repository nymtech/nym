// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DkgError;
use bls12_381::Scalar;
use core::iter::Sum;
use core::ops::Mul;
use std::collections::HashSet;

pub mod polynomial;

fn contains_duplicates(vals: &[Scalar]) -> bool {
    let mut set = HashSet::new();

    for x in vals {
        if !set.insert(x.to_bytes()) {
            return true;
        }
    }

    false
}

#[inline]
fn generate_lagrangian_coefficients_at_x(
    x: &Scalar,
    points: &[Scalar],
) -> Result<Vec<Scalar>, DkgError> {
    let num_points = points.len();
    if num_points == 0 {
        return Ok(Vec::new());
    } else if num_points == 1 {
        return Ok(vec![Scalar::one()]);
    }

    if contains_duplicates(points) {
        return Err(DkgError::DuplicateCoordinate);
    }

    let mut res = Vec::with_capacity(points.len());

    for (i, xi) in points.iter().enumerate() {
        let mut numerator = Scalar::one();
        let mut denominator = Scalar::one();

        for (j, xj) in points.iter().enumerate() {
            if j != i {
                // numerator = (x - xs[0]) * ... * (x - xs[j]), j != i
                numerator *= x - xj;

                // denominator = (xs[i] - x[0]) * ... * (xs[i] - x[j]), j != i
                denominator *= xi - xj;
            }
        }

        // 1 / denominator
        let inv: Scalar =
            Option::from(denominator.invert()).ok_or(DkgError::DuplicateCoordinate)?;

        // numerator / denominator
        res.push(numerator * inv)
    }

    Ok(res)
}

/// Performs a Lagrange interpolation at specified x for a polynomial defined by set of coordinates
/// (x, f(x)), where x is a `Scalar` and f(x) is a generic type that can be obtained by evaluating `f` at `x`.
/// It can be used for Scalars, G1 and G2 points.
pub fn perform_lagrangian_interpolation_at_x<T>(
    x: &Scalar,
    points: &[(Scalar, T)],
) -> Result<T, DkgError>
where
    T: Sum,
    for<'a> &'a T: Mul<Scalar, Output = T>,
{
    let xs = points.iter().map(|p| p.0).collect::<Vec<_>>();
    let coefficients = generate_lagrangian_coefficients_at_x(x, &xs)?;

    Ok(coefficients
        .into_iter()
        .zip(points.iter().map(|p| &p.1))
        .map(|(coeff, y)| y * coeff)
        .sum())
}

/// Performs a Lagrange interpolation at the origin for a polynomial defined by set of coordinates
/// (x, f(x)), where x is a `Scalar` and f(x) is a generic type that can be obtained by evaluating `f` at `x`.
/// It can be used for Scalars, G1 and G2 points.
pub fn perform_lagrangian_interpolation_at_origin<T>(points: &[(Scalar, T)]) -> Result<T, DkgError>
where
    T: Sum,
    for<'a> &'a T: Mul<Scalar, Output = T>,
{
    perform_lagrangian_interpolation_at_x(&Scalar::zero(), points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performing_lagrangian_scalar_interpolation_at_origin() {
        // x^2 + 3
        // x, f(x):
        // 1, 4,
        // 2, 7,
        // 3, 12,
        let points = vec![
            (Scalar::from(1), Scalar::from(4)),
            (Scalar::from(2), Scalar::from(7)),
            (Scalar::from(3), Scalar::from(12)),
        ];
        assert_eq!(
            Scalar::from(3),
            perform_lagrangian_interpolation_at_origin(&points).unwrap()
        );

        // x^3 + 3x^2 - 5x + 11
        // x, f(x):
        // 1, 10
        // 2, 21
        // 3, 50
        // 4, 103
        let points = vec![
            (Scalar::from(1), Scalar::from(10)),
            (Scalar::from(2), Scalar::from(21)),
            (Scalar::from(3), Scalar::from(50)),
            (Scalar::from(4), Scalar::from(103)),
        ];
        assert_eq!(
            Scalar::from(11),
            perform_lagrangian_interpolation_at_origin(&points).unwrap()
        );

        // more points than it is required
        // x^2 + x + 10
        // x, f(x)
        // 1, 12
        // 2, 16
        // 3, 22
        // 4, 30
        // 5, 40
        let points = vec![
            (Scalar::from(1), Scalar::from(12)),
            (Scalar::from(2), Scalar::from(16)),
            (Scalar::from(3), Scalar::from(22)),
            (Scalar::from(4), Scalar::from(30)),
            (Scalar::from(5), Scalar::from(40)),
        ];
        assert_eq!(
            Scalar::from(10),
            perform_lagrangian_interpolation_at_origin(&points).unwrap()
        );
    }
}
