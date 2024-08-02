// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::SignerIndex;
use crate::error::{CompactEcashError, Result};
use crate::scheme::setup::GroupParameters;
use crate::{ecash_group_parameters, Signature, VerificationKeyAuth};
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve, HashToField};
use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar,
};
use core::iter::Sum;
use core::ops::Mul;
use ff::Field;
use group::{Curve, Group};
use itertools::Itertools;
use std::borrow::Borrow;
use std::ops::Neg;

pub struct Polynomial {
    coefficients: Vec<Scalar>,
}

impl Polynomial {
    // for polynomial of degree n, we generate n+1 values
    // (for example for degree 1, like y = x + 2, we need [2,1])
    pub fn new_random(params: &GroupParameters, degree: u64) -> Self {
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
        } else if x.is_zero().unwrap_u8() == 1 {
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
pub fn generate_lagrangian_coefficients_at_origin(points: &[u64]) -> Vec<Scalar> {
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
            //SAFETY: denominator start as one, and (xi-xj) is guaranteed to be non zero, as we force i != j
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
        return Err(CompactEcashError::InterpolationSetSize);
    }

    if points.len() != values.len() {
        return Err(CompactEcashError::InterpolationSetSize);
    }

    let coefficients = generate_lagrangian_coefficients_at_origin(points);

    Ok(coefficients
        .into_iter()
        .zip(values.iter())
        .map(|(coeff, val)| val * coeff)
        .sum())
}

//domain name following https://www.rfc-editor.org/rfc/rfc9380.html#name-domain-separation-requireme recommendation
const G1_HASH_DOMAIN: &[u8] = b"NYMECASH-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";
const SCALAR_HASH_DOMAIN: &[u8] = b"NYMECASH-V01-CS02-with-expander-SHA256";

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

pub fn try_deserialize_scalar_vec(expected_len: u64, bytes: &[u8]) -> Result<Vec<Scalar>> {
    if bytes.len() != expected_len as usize * 32 {
        return Err(CompactEcashError::DeserializationLengthMismatch {
            type_name: "Scalar vector".into(),
            expected: expected_len as usize * 32,
            actual: bytes.len(),
        });
    }

    let mut out = Vec::with_capacity(expected_len as usize);
    for i in 0..expected_len as usize {
        //SAFETY : casting 32 len slice into 32 len array
        #[allow(clippy::unwrap_used)]
        let s_bytes = bytes[i * 32..(i + 1) * 32].try_into().unwrap();
        let s = match Scalar::from_bytes(&s_bytes).into() {
            None => return Err(CompactEcashError::ScalarDeserializationFailure),
            Some(scalar) => scalar,
        };
        out.push(s)
    }

    Ok(out)
}

pub fn try_deserialize_scalar(bytes: &[u8; 32]) -> Result<Scalar> {
    Into::<Option<Scalar>>::into(Scalar::from_bytes(bytes))
        .ok_or(CompactEcashError::ScalarDeserializationFailure)
}

pub fn try_deserialize_g1_projective(bytes: &[u8; 48]) -> Result<G1Projective> {
    Into::<Option<G1Affine>>::into(G1Affine::from_compressed(bytes))
        .ok_or(CompactEcashError::G1ProjectiveDeserializationFailure)
        .map(G1Projective::from)
}

pub fn try_deserialize_g2_projective(bytes: &[u8; 96]) -> Result<G2Projective> {
    Into::<Option<G2Affine>>::into(G2Affine::from_compressed(bytes))
        .ok_or(CompactEcashError::G2ProjectiveDeserializationFailure)
        .map(G2Projective::from)
}

/// Checks whether e(P, Q) * e(-R, S) == id
pub fn check_bilinear_pairing(p: &G1Affine, q: &G2Prepared, r: &G1Affine, s: &G2Prepared) -> bool {
    // checking e(P, Q) * e(-R, S) == id
    // is equivalent to checking e(P, Q) == e(R, S)
    // but requires only a single final exponentiation rather than two of them
    // and therefore, as seen via benchmarks.rs, is almost 50% faster
    // (1.47ms vs 2.45ms, tested on R9 5900X)

    let multi_miller = multi_miller_loop(&[(p, q), (&r.neg(), s)]);
    multi_miller.final_exponentiation().is_identity().into()
}

// compute e(h1, X1) * e(s1, g2^-1) * ... == id
// pub fn batch_verify_signatures(iter: Vec<(&Signature, G2Projective)>) -> bool {
pub fn batch_verify_signatures<S, G2, T>(iter: impl Iterator<Item = T>) -> bool
where
    T: Borrow<(S, G2)>,
    S: Borrow<Signature>,
    G2: Borrow<G2Projective>,
{
    let mut miller_terms_owned = Vec::new();
    for t in iter {
        let (sig, q) = t.borrow();
        let sig = sig.borrow();
        miller_terms_owned.push((
            sig.h.to_affine(),
            G2Prepared::from(q.borrow().to_affine()),
            sig.s.to_affine(),
        ));
    }

    let params = ecash_group_parameters();
    let g2_prep_neg = G2Prepared::from(params.gen2().neg());

    let mut miller_terms = Vec::with_capacity(miller_terms_owned.len() * 2);
    for (h, q, s) in &miller_terms_owned {
        miller_terms.push((h, q));
        miller_terms.push((s, &g2_prep_neg));
    }

    multi_miller_loop(&miller_terms)
        .final_exponentiation()
        .is_identity()
        .into()
}

pub fn check_vk_pairing(
    params: &GroupParameters,
    dkg_values: &[G2Projective],
    vk: &VerificationKeyAuth,
) -> bool {
    let values_len = dkg_values.len();
    if values_len == 0 || values_len - 1 != vk.beta_g1.len() || values_len - 1 != vk.beta_g2.len() {
        return false;
    }

    // safety: we made an explicit check for if the length of the slice is 0, thus unwrap here is fine
    #[allow(clippy::unwrap_used)]
    if &vk.alpha != *dkg_values.first().as_ref().unwrap() {
        return false;
    }
    let dkg_betas = &dkg_values[1..];
    if dkg_betas
        .iter()
        .zip(vk.beta_g2.iter())
        .any(|(dkg_beta, vk_beta)| dkg_beta != vk_beta)
    {
        return false;
    }

    let mut owned_miller_terms = vec![];
    for (i, (g1, g2)) in vk.beta_g1.iter().zip(vk.beta_g2.iter()).enumerate() {
        if i % 2 == 0 {
            owned_miller_terms.push((g1.to_affine(), G2Prepared::from(g2.to_affine())));
        } else {
            // negate every other g1 element
            owned_miller_terms.push((g1.neg().to_affine(), G2Prepared::from(g2.to_affine())));
        }
    }

    // if our key has odd length, make sure to include the generators to correctly compute the final exponentiation
    if owned_miller_terms.len() % 2 == 1 {
        let neg_g1 = params.gen1().neg();
        let g2_prep = params.prepared_miller_g2();
        owned_miller_terms.push((neg_g1, g2_prep.to_owned()))
    }

    let mut miller_terms = Vec::new();
    for ((p, s), (r, q)) in owned_miller_terms.iter().tuples::<(_, _)>() {
        miller_terms.push((p, q));
        miller_terms.push((r, s));
    }

    // check if e(g1^x, g2^y) * e(g1^-y, g2^x) * ... == id
    // in case of odd-length key check:
    // check if e(g1^x, g2^y) * e(g1^-y, g2^x) * ... * e(g1^-z, g2) * e(g1^1, g2^z) == id
    // (this is more than 2x as fast as checking each pairing individually for key of size 5)
    multi_miller_loop(&miller_terms)
        .final_exponentiation()
        .is_identity()
        .into()
}

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
