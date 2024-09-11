// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::iter::Sum;
use core::ops::Mul;

use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve, HashToField};
use bls12_381::{G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use ff::Field;

use crate::error::{CoconutError, Result};
use crate::scheme::setup::Parameters;
use crate::scheme::SignerIndex;

pub struct Polynomial {
    coefficients: Vec<Scalar>,
}

impl Polynomial {
    // for polynomial of degree n, we generate n+1 values
    // (for example for degree 1, like y = x + 2, we need [2,1])
    pub fn new_random(params: &Parameters, degree: u64) -> Self {
        Polynomial {
            coefficients: params.n_random_scalars((degree + 1) as usize),
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
            #[allow(clippy::unwrap_used)]
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

#[inline]
fn generate_lagrangian_coefficients_at_origin(points: &[u64]) -> Vec<Scalar> {
    let x = Scalar::zero();

    points
        .iter()
        .enumerate()
        .map(|(i, point_i)| {
            let mut numerator = Scalar::one();
            let mut denominator = Scalar::one();
            let xi = Scalar::from(*point_i);

            for (j, point_j) in points.iter().enumerate() {
                if j != i {
                    let xj = Scalar::from(*point_j);

                    // numerator = (x - xs[0]) * ... * (x - xs[j]), j != i
                    numerator *= x - xj;

                    // denominator = (xs[i] - x[0]) * ... * (xs[i] - x[j]), j != i
                    denominator *= xi - xj;
                }
            }
            // numerator / denominator
            numerator * denominator.invert().unwrap()
        })
        .collect()
}

/// Performs a Lagrange interpolation at the origin for a polynomial defined by `points` and `values`.
/// It can be used for Scalars, G1 and G2 points.
pub(crate) fn perform_lagrangian_interpolation_at_origin<T>(
    points: &[SignerIndex],
    values: &[T],
) -> Result<T>
where
    T: Sum,
    for<'a> &'a T: Mul<Scalar, Output = T>,
{
    if points.is_empty() || values.is_empty() {
        return Err(CoconutError::Interpolation(
            "Tried to perform lagrangian interpolation for an empty set of coordinates".to_string(),
        ));
    }

    if points.len() != values.len() {
        return Err(CoconutError::Interpolation(
            "Tried to perform lagrangian interpolation for an incomplete set of coordinates"
                .to_string(),
        ));
    }

    let coefficients = generate_lagrangian_coefficients_at_origin(points);

    Ok(coefficients
        .into_iter()
        .zip(values.iter())
        .map(|(coeff, val)| val * coeff)
        .sum())
}

// A temporary way of hashing particular message into G1.
// Implementation idea was taken from `threshold_crypto`:
// https://github.com/poanetwork/threshold_crypto/blob/7709462f2df487ada3bb3243060504b5881f2628/src/lib.rs#L691
// Eventually it should get replaced by, most likely, the osswu map
// method once ideally it's implemented inside the pairing crate.

// note: I have absolutely no idea what are the correct domains for those. I just used whatever
// was given in the test vectors of `Hashing to Elliptic Curves draft-irtf-cfrg-hash-to-curve-11`

// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#appendix-J.9.1
const G1_HASH_DOMAIN: &[u8] = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";

// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#appendix-K.1
const SCALAR_HASH_DOMAIN: &[u8] = b"QUUX-V01-CS02-with-expander";

pub fn hash_g1<M: AsRef<[u8]>>(msg: M) -> G1Projective {
    <G1Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(msg, G1_HASH_DOMAIN)
}

pub fn hash_to_scalar<M: AsRef<[u8]>>(msg: M) -> Scalar {
    let mut output = vec![Scalar::zero()];

    Scalar::hash_to_field::<ExpandMsgXmd<sha2::Sha256>>(
        msg.as_ref(),
        SCALAR_HASH_DOMAIN,
        &mut output,
    );
    output[0]
}

pub fn try_deserialize_scalar_vec(
    expected_len: u64,
    bytes: &[u8],
    err: CoconutError,
) -> Result<Vec<Scalar>> {
    if bytes.len() != expected_len as usize * 32 {
        return Err(err);
    }

    let mut out = Vec::with_capacity(expected_len as usize);
    for i in 0..expected_len as usize {
        // we just checked we have exactly the amount of bytes we need and thus the unwrap is fine
        #[allow(clippy::unwrap_used)]
        let s_bytes = bytes[i * 32..(i + 1) * 32].try_into().unwrap();
        let s = match Into::<Option<Scalar>>::into(Scalar::from_bytes(&s_bytes)) {
            None => return Err(err),
            Some(scalar) => scalar,
        };
        out.push(s)
    }

    Ok(out)
}

pub fn try_deserialize_scalar(bytes: &[u8; 32], err: CoconutError) -> Result<Scalar> {
    Into::<Option<Scalar>>::into(Scalar::from_bytes(bytes)).ok_or(err)
}

pub fn try_deserialize_g1_projective(bytes: &[u8; 48], err: CoconutError) -> Result<G1Projective> {
    Into::<Option<G1Affine>>::into(G1Affine::from_compressed(bytes))
        .ok_or(err)
        .map(G1Projective::from)
}

pub fn try_deserialize_g2_projective(bytes: &[u8; 96], err: CoconutError) -> Result<G2Projective> {
    Into::<Option<G2Affine>>::into(G2Affine::from_compressed(bytes))
        .ok_or(err)
        .map(G2Projective::from)
}

// use core::fmt;
// #[cfg(feature = "serde")]
// use serde::de::Visitor;
// #[cfg(feature = "serde")]
// use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
//
// // #[cfg(feature = "serde")]
// #[serde(remote = "Scalar")]
// pub(crate) struct ScalarDef(pub Scalar);
//
// // #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
//
// impl Serialize for ScalarDef {
//     fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use serde::ser::SerializeTuple;
//         let mut tup = serializer.serialize_tuple(32)?;
//         for byte in self.0.to_bytes().iter() {
//             tup.serialize_element(byte)?;
//         }
//         tup.end()
//     }
// }
//
// impl<'de> Deserialize<'de> for ScalarDef {
//     fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         struct ScalarVisitor;
//
//         impl<'de> Visitor<'de> for ScalarVisitor {
//             type Value = ScalarDef;
//
//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("a 32-byte canonical bls12_381 scalar")
//             }
//
//             fn visit_seq<A>(self, mut seq: A) -> core::result::Result<ScalarDef, A::Error>
//             where
//                 A: serde::de::SeqAccess<'de>,
//             {
//                 let mut bytes = [0u8; 32];
//                 for i in 0..32 {
//                     bytes[i] = seq
//                         .next_element()?
//                         .ok_or_else(|| serde::de::Error::invalid_length(i, &"expected 32 bytes"))?;
//                 }
//
//                 let res = Scalar::from_bytes(&bytes);
//                 if res.is_some().into() {
//                     Ok(ScalarDef(res.unwrap()))
//                 } else {
//                     Err(serde::de::Error::custom(
//                         &"scalar was not canonically encoded",
//                     ))
//                 }
//             }
//         }
//
//         deserializer.deserialize_tuple(32, ScalarVisitor)
//     }
// }
//
// #[cfg(feature = "serde")]
// pub(crate) struct G1ProjectiveSerdeHelper(Scalar);
//
// #[cfg(feature = "serde")]
// pub(crate) struct G2ProjectiveSerdeHelper(Scalar);

#[cfg(test)]
mod tests {
    use rand::RngCore;

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
        let poly = Polynomial {
            coefficients: vec![],
        };

        // should always be 0
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(1)));
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(0)));
        assert_eq!(Scalar::from(0), poly.evaluate(&Scalar::from(10)));
    }

    #[test]
    fn performing_lagrangian_scalar_interpolation_at_origin() {
        // x^2 + 3
        // x, f(x):
        // 1, 4,
        // 2, 7,
        // 3, 12,
        let points = vec![1, 2, 3];
        let values = vec![Scalar::from(4), Scalar::from(7), Scalar::from(12)];

        assert_eq!(
            Scalar::from(3),
            perform_lagrangian_interpolation_at_origin(&points, &values).unwrap()
        );

        // x^3 + 3x^2 - 5x + 11
        // x, f(x):
        // 1, 10
        // 2, 21
        // 3, 50
        // 4, 103
        let points = vec![1, 2, 3, 4];
        let values = vec![
            Scalar::from(10),
            Scalar::from(21),
            Scalar::from(50),
            Scalar::from(103),
        ];

        assert_eq!(
            Scalar::from(11),
            perform_lagrangian_interpolation_at_origin(&points, &values).unwrap()
        );

        // more points than it is required
        // x^2 + x + 10
        // x, f(x)
        // 1, 12
        // 2, 16
        // 3, 22
        // 4, 30
        // 5, 40
        let points = vec![1, 2, 3, 4, 5];
        let values = vec![
            Scalar::from(12),
            Scalar::from(16),
            Scalar::from(22),
            Scalar::from(30),
            Scalar::from(40),
        ];

        assert_eq!(
            Scalar::from(10),
            perform_lagrangian_interpolation_at_origin(&points, &values).unwrap()
        );
    }

    #[test]
    fn hash_g1_sanity_check() {
        let mut rng = rand::thread_rng();
        let mut msg1 = [0u8; 1024];
        rng.fill_bytes(&mut msg1);
        let mut msg2 = [0u8; 1024];
        rng.fill_bytes(&mut msg2);

        assert_eq!(hash_g1(msg1), hash_g1(msg1));
        assert_eq!(hash_g1(msg2), hash_g1(msg2));
        assert_ne!(hash_g1(msg1), hash_g1(msg2));
    }

    #[test]
    fn hash_scalar_sanity_check() {
        let mut rng = rand::thread_rng();
        let mut msg1 = [0u8; 1024];
        rng.fill_bytes(&mut msg1);
        let mut msg2 = [0u8; 1024];
        rng.fill_bytes(&mut msg2);

        assert_eq!(hash_to_scalar(msg1), hash_to_scalar(msg1));
        assert_eq!(hash_to_scalar(msg2), hash_to_scalar(msg2));
        assert_ne!(hash_to_scalar(msg1), hash_to_scalar(msg2));
    }
}
