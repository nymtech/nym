// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::DkgError;
use crate::utils::deserialize_g2;
use bls12_381::{G2Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand_core::RngCore;
use std::ops::{Add, Index, IndexMut};
use zeroize::Zeroize;

#[derive(Clone, Debug, PartialEq)]
pub struct PublicCoefficients {
    pub(crate) coefficients: Vec<G2Projective>,
}

impl PublicCoefficients {
    pub(crate) fn size(&self) -> usize {
        self.coefficients.len()
    }

    pub(crate) fn nth(&self, n: usize) -> &G2Projective {
        &self.coefficients[n]
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.coefficients.is_empty()
    }

    pub(crate) fn inner(&self) -> &[G2Projective] {
        &self.coefficients
    }

    pub(crate) fn evaluate_at(&self, x: &Scalar) -> G2Projective {
        if self.coefficients.is_empty() {
            G2Projective::identity()
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

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let coeffs = self.coefficients.len();
        let mut bytes = Vec::with_capacity(4 + 96 * coeffs);
        bytes.extend_from_slice(&((coeffs as u32).to_be_bytes()));
        for coeff in &self.coefficients {
            bytes.extend_from_slice(coeff.to_bytes().as_ref())
        }

        bytes
    }

    pub(crate) fn try_from_bytes(b: &[u8]) -> Result<Self, DkgError> {
        if b.len() < 4 {
            return Err(DkgError::new_deserialization_failure(
                "PublicCoefficients",
                "insufficient number of bytes provided",
            ));
        }

        let coeffs = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
        let mut coefficients = Vec::with_capacity(coeffs);

        if b.len() != 4 + coeffs * 96 {
            return Err(DkgError::new_deserialization_failure(
                "PublicCoefficients",
                "insufficient number of bytes provided",
            ));
        }

        let mut i = 4;
        for _ in 0..coeffs {
            let coefficient = deserialize_g2(&b[i..i + 96]).ok_or_else(|| {
                DkgError::new_deserialization_failure(
                    "PublicCoefficients.coefficient",
                    "invalid curve point",
                )
            })?;

            coefficients.push(coefficient);
            i += 96;
        }

        Ok(PublicCoefficients { coefficients })
    }
}

impl Index<usize> for PublicCoefficients {
    type Output = G2Projective;

    fn index(&self, index: usize) -> &Self::Output {
        self.coefficients.index(index)
    }
}

impl IndexMut<usize> for PublicCoefficients {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.coefficients.index_mut(index)
    }
}

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

    pub fn set_constant_coefficient(&mut self, value: Scalar) {
        if self.coefficients.is_empty() {
            self.coefficients = vec![value]
        } else {
            self.coefficients[0] = value
        }
    }

    /// Creates a zero-polynomial, i.e. p(x) = 0
    pub const fn zero() -> Self {
        Polynomial {
            coefficients: Vec::new(),
        }
    }

    /// Returns public coefficients associated with this polynomial.
    pub fn public_coefficients(&self) -> PublicCoefficients {
        let g2 = G2Projective::generator();
        let coefficients = self.coefficients.iter().map(|a_i| g2 * a_i).collect();

        PublicCoefficients { coefficients }
    }

    /// Evaluates the polynomial at point x.
    pub fn evaluate_at(&self, x: &Scalar) -> Scalar {
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
    use rand_core::SeedableRng;

    #[test]
    fn polynomial_evaluation() {
        // y = 42 (it should be 42 regardless of x)
        let poly = Polynomial {
            coefficients: vec![Scalar::from(42)],
        };

        assert_eq!(Scalar::from(42), poly.evaluate_at(&Scalar::from(1)));
        assert_eq!(Scalar::from(42), poly.evaluate_at(&Scalar::from(0)));
        assert_eq!(Scalar::from(42), poly.evaluate_at(&Scalar::from(10)));

        // y = x + 10, at x = 2 (exp: 12)
        let poly = Polynomial {
            coefficients: vec![Scalar::from(10), Scalar::from(1)],
        };

        assert_eq!(Scalar::from(12), poly.evaluate_at(&Scalar::from(2)));

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

        assert_eq!(Scalar::from(39), poly.evaluate_at(&Scalar::from(3)));

        // empty polynomial
        let poly = Polynomial::zero();

        // should always be 0
        assert_eq!(Scalar::from(0), poly.evaluate_at(&Scalar::from(1)));
        assert_eq!(Scalar::from(0), poly.evaluate_at(&Scalar::from(0)));
        assert_eq!(Scalar::from(0), poly.evaluate_at(&Scalar::from(10)));
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

    #[test]
    fn public_coefficients_evaluation() {
        // we use the same values as in polynomial evaluation test

        let g2 = G2Projective::generator();

        // y = 42 (it should be 42 regardless of x)
        let coeffs = PublicCoefficients {
            coefficients: vec![g2 * Scalar::from(42)],
        };

        assert_eq!(g2 * Scalar::from(42), coeffs.evaluate_at(&Scalar::from(1)));
        assert_eq!(g2 * Scalar::from(42), coeffs.evaluate_at(&Scalar::from(0)));
        assert_eq!(g2 * Scalar::from(42), coeffs.evaluate_at(&Scalar::from(10)));

        // y = x + 10, at x = 2 (exp: 12)
        let poly = PublicCoefficients {
            coefficients: vec![g2 * Scalar::from(10), g2 * Scalar::from(1)],
        };

        assert_eq!(g2 * Scalar::from(12), poly.evaluate_at(&Scalar::from(2)));

        // y = x^4 - 5x^2 + 2x - 3, at x = 3 (exp: 39)
        let coeffs = PublicCoefficients {
            coefficients: vec![
                (-g2 * Scalar::from(3)),
                g2 * Scalar::from(2),
                (-g2 * Scalar::from(5)),
                G2Projective::identity(),
                g2 * Scalar::from(1),
            ],
        };

        assert_eq!(g2 * Scalar::from(39), coeffs.evaluate_at(&Scalar::from(3)));

        // empty coefficients
        let coeffs = PublicCoefficients {
            coefficients: Vec::new(),
        };

        // should always be 0
        assert_eq!(
            G2Projective::identity(),
            coeffs.evaluate_at(&Scalar::from(1))
        );
        assert_eq!(
            G2Projective::identity(),
            coeffs.evaluate_at(&Scalar::from(0))
        );
        assert_eq!(
            G2Projective::identity(),
            coeffs.evaluate_at(&Scalar::from(10))
        );
    }

    #[test]
    fn public_coefficients_roundtrip() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let good = vec![
            Polynomial::zero().public_coefficients(),
            Polynomial::new_random(&mut rng, 0).public_coefficients(),
            Polynomial::new_random(&mut rng, 1).public_coefficients(),
            Polynomial::new_random(&mut rng, 4).public_coefficients(),
            Polynomial::new_random(&mut rng, 15).public_coefficients(),
        ];

        for coefficient in good {
            let bytes = coefficient.to_bytes();
            let recovered = PublicCoefficients::try_from_bytes(&bytes).unwrap();
            assert_eq!(coefficient, recovered);
        }

        assert!(PublicCoefficients::try_from_bytes(&[]).is_err());
        assert!(PublicCoefficients::try_from_bytes(&[1]).is_err());
        assert!(PublicCoefficients::try_from_bytes(&[1, 2, 3, 4]).is_err());

        let g2 = G2Projective::generator().to_bytes();
        let mut bad_length = Vec::new();
        bad_length.extend_from_slice(&2u32.to_be_bytes());
        bad_length.extend_from_slice(g2.as_ref());
        assert!(PublicCoefficients::try_from_bytes(&bad_length).is_err());

        let mut incomplete = Vec::new();
        incomplete.extend_from_slice(&1u32.to_be_bytes());
        incomplete.extend_from_slice(&g2.as_ref()[..95]);
        assert!(PublicCoefficients::try_from_bytes(&incomplete).is_err());
    }
}
