// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use ff::Field;
use rand_core::RngCore;
use std::borrow::Borrow;
use std::ops;
use std::ops::{Add, AddAssign, Index, IndexMut, MulAssign, SubAssign};
use zeroize::Zeroize;

#[derive(Clone, Debug, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct Polynomial {
    coefficients: Vec<Scalar>,
}

impl Polynomial {
    // for polynomial of degree n, we generate n+1 values
    // (for example for degree 1, like y = x + 2, we need [2,1])
    /// Creates new pseudorandom polynomial of specified degree.
    pub fn new_random(mut rng: impl RngCore, degree: u64) -> Self {
        Polynomial {
            coefficients: (0..=degree).map(|_| Scalar::random(&mut rng)).collect(),
        }
    }

    /// Creates new polynomial with provided coefficients.
    pub fn new(coefficients: Vec<Scalar>) -> Self {
        Polynomial { coefficients }
    }

    /// Creates a zero-polynomial, i.e. p(x) = 0
    pub const fn zero() -> Self {
        Polynomial {
            coefficients: Vec::new(),
        }
    }

    /// Evaluates the polynomial at point x.
    pub fn evaluate(&self, x: &Scalar) -> Scalar {
        if self.coefficients.is_empty() {
            Scalar::zero()
            // if x is zero then we can ignore most of the expensive computation and
            // just return the last term of the polynomial
        } else if x.is_zero().into() {
            // we checked that coefficients are not empty so unwrap here is fine
            *self.coefficients.first().unwrap()
        } else {
            self.coefficients
                .iter()
                .enumerate()
                // coefficient[n] * x ^ n
                .map(|(i, coefficient)| coefficient * x.pow(&[i as u64, 0, 0, 0]))
                .sum()
        }
    }
}

// PURGE ENDS HERE

impl Index<usize> for Polynomial {
    type Output = Scalar;

    fn index(&self, index: usize) -> &Self::Output {
        self.coefficients.index(index)
    }
}

impl IndexMut<usize> for Polynomial {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.coefficients.index_mut(index)
    }
}

impl<'b> Add<&'b Polynomial> for Polynomial {
    type Output = Polynomial;

    fn add(self, rhs: &'b Polynomial) -> Polynomial {
        &self + rhs
    }
}

impl<'a> Add<Polynomial> for &'a Polynomial {
    type Output = Polynomial;

    fn add(self, rhs: Polynomial) -> Polynomial {
        self + &rhs
    }
}

impl Add<Polynomial> for Polynomial {
    type Output = Polynomial;

    fn add(self, rhs: Polynomial) -> Polynomial {
        &self + &rhs
    }
}

impl<'a, 'b> Add<&'b Polynomial> for &'a Polynomial {
    type Output = Polynomial;

    fn add(self, rhs: &'b Polynomial) -> Self::Output {
        let len = self.coefficients.len();
        let rhs_len = rhs.coefficients.len();

        // to have easier bound checks
        if rhs_len > len {
            return rhs + self;
        }

        // we know len >= rhs_len and hence the output will also be of size len
        let mut res = Vec::with_capacity(len);

        for i in 0..len {
            if let Some(rhs_coeff) = rhs.coefficients.get(i) {
                res.push(self[i] + rhs_coeff)
            } else {
                res.push(self[i])
            }
        }

        Polynomial { coefficients: res }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polynomial_evaluation() {
        // y = 42 (it should be 42 regardless of x)
        let poly = Polynomial {
            coefficients: vec![Scalar::from(42)],
        };

        assert_eq!(Scalar::from(42), poly.evaluate(&Scalar::from(1)));
        assert_eq!(Scalar::from(42), poly.evaluate(&Scalar::from(0)));
        assert_eq!(Scalar::from(42), poly.evaluate(&Scalar::from(10)));

        // y = x + 10, at x = 2 (exp: 12)
        let poly = Polynomial {
            coefficients: vec![Scalar::from(10), Scalar::from(1)],
        };

        assert_eq!(Scalar::from(12), poly.evaluate(&Scalar::from(2)));

        // y = x^4 - 5x^2 + 2x - 3, at x = 3 (exp: 39)
        let poly = Polynomial {
            coefficients: vec![
                (-Scalar::from(3)),
                Scalar::from(2),
                (-Scalar::from(5)),
                Scalar::zero(),
                Scalar::from(1),
            ],
        };

        assert_eq!(Scalar::from(39), poly.evaluate(&Scalar::from(3)));

        // empty polynomial
        let poly = Polynomial::zero();

        // should always be 0
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(1)));
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(0)));
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(10)));
    }

    #[test]
    fn polynomial_addition() {
        let empty = Polynomial::zero();
        let p1 = Polynomial {
            coefficients: vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)],
        };
        let p2 = Polynomial {
            coefficients: vec![Scalar::from(4), Scalar::from(5)],
        };
        let expected_sum = Polynomial {
            coefficients: vec![Scalar::from(5), Scalar::from(7), Scalar::from(3)],
        };

        assert_eq!(p1, &p1 + &empty);
        assert_eq!(p1, empty + &p1);
        assert_eq!(expected_sum, &p1 + &p2);
        assert_eq!(expected_sum, &p2 + &p1);
    }
}
