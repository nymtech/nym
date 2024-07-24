// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{Signature, SignerIndex};
use crate::constants;
use crate::error::{CompactEcashError, Result};
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::utils::generate_lagrangian_coefficients_at_origin;
use crate::utils::{batch_verify_signatures, hash_g1};
use bls12_381::{G1Projective, Scalar};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;

/// A structure representing an expiration date signature.
pub type ExpirationDateSignature = Signature;
pub type PartialExpirationDateSignature = ExpirationDateSignature;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnnotatedExpirationDateSignature {
    pub signature: ExpirationDateSignature,
    pub expiration_timestamp: u64,
    pub spending_timestamp: u64,
}

impl Borrow<ExpirationDateSignature> for AnnotatedExpirationDateSignature {
    fn borrow(&self) -> &ExpirationDateSignature {
        &self.signature
    }
}

impl From<AnnotatedExpirationDateSignature> for ExpirationDateSignature {
    fn from(value: AnnotatedExpirationDateSignature) -> Self {
        value.signature
    }
}

pub struct ExpirationDateSignatureShare<B = PartialExpirationDateSignature>
where
    B: Borrow<PartialExpirationDateSignature>,
{
    pub index: SignerIndex,
    pub key: VerificationKeyAuth,
    pub signatures: Vec<B>,
}

/// Signs given expiration date for a specified validity period using the given secret key of a single authority.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `sk_auth` - The secret key of the signing authority.
/// * `expiration_unix_timestamp` - The expiration date for which signatures will be generated (as unix timestamp).
///
/// # Returns
///
/// A vector containing partial signatures for each date within the validity period (i.e.,
/// from expiration_date - CRED_VALIDITY_PERIOD till expiration_date.
///
/// # Note
///
/// This function is executed by a single singing authority and generates partial expiration date
/// signatures for a specified validity period. Each signature is created by combining cryptographic
/// attributes derived from the expiration date, and the resulting vector contains signatures for
/// each date within the defined validity period till expiration date.
/// The validity period is determined by the constant `CRED_VALIDITY_PERIOD` in the `constants` module.
pub fn sign_expiration_date(
    sk_auth: &SecretKeyAuth,
    expiration_unix_timestamp: u64,
) -> Result<Vec<AnnotatedExpirationDateSignature>> {
    if sk_auth.ys.len() < 3 {
        return Err(CompactEcashError::KeyTooShort);
    }
    let m0: Scalar = Scalar::from(expiration_unix_timestamp);
    let m2: Scalar = constants::TYPE_EXP;

    let partial_s_exponent = sk_auth.x + sk_auth.ys[0] * m0 + sk_auth.ys[2] * m2;

    let sign_expiration = |offset: u64| {
        // we produce tuples of (assuming CRED_VALIDITY_PERIOD_DAYS = 30):
        // (expiration, expiration - 29)
        // (expiration, expiration - 28)
        // ...
        // (expiration, expiration)
        let spending_unix_timestamp = expiration_unix_timestamp
            - ((constants::CRED_VALIDITY_PERIOD_DAYS - offset - 1) * constants::SECONDS_PER_DAY);
        let m1: Scalar = Scalar::from(spending_unix_timestamp);
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes()].concat());
        // Sign the attributes by performing scalar-point multiplications and accumulating the result
        let s_exponent = partial_s_exponent + sk_auth.ys[1] * m1;

        // Create the signature struct on the expiration date
        let signature = PartialExpirationDateSignature {
            h,
            s: h * s_exponent,
        };

        AnnotatedExpirationDateSignature {
            signature,
            expiration_timestamp: expiration_unix_timestamp,
            spending_timestamp: spending_unix_timestamp,
        }
    };

    cfg_if::cfg_if! {
        if #[cfg(feature = "par_signing")] {
            use rayon::prelude::*;

            Ok((0..constants::CRED_VALIDITY_PERIOD_DAYS)
                .into_par_iter()
                .map(sign_expiration)
                .collect())
        } else {
           Ok((0..constants::CRED_VALIDITY_PERIOD_DAYS).map(sign_expiration).collect())
        }
    }
}

/// Verifies the expiration date signatures against the given verification key.
///
/// This function iterates over the provided valid date signatures and verifies each one
/// against the provided verification key. It computes the hash and checks the correctness of the
/// signature using bilinear pairings.
///
/// # Arguments
///
/// * `vkey` - The verification key of the signing authority.
/// * `signatures` - The list of date signatures to be verified.
/// * `expiration_date` - The expiration date for which signatures are being issued (as unix timestamp).
///
/// # Returns
///
/// Returns `Ok(true)` if all signatures are verified successfully, otherwise returns an error
///
pub fn verify_valid_dates_signatures<B>(
    vk: &VerificationKeyAuth,
    signatures: &[B],
    expiration_date: u64,
) -> Result<()>
where
    B: Borrow<ExpirationDateSignature>,
{
    let m0: Scalar = Scalar::from(expiration_date);
    let m2: Scalar = constants::TYPE_EXP;

    let partially_signed = vk.alpha + vk.beta_g2[0] * m0 + vk.beta_g2[2] * m2;
    let mut pairing_terms = Vec::with_capacity(signatures.len());

    for (i, sig) in signatures.iter().enumerate() {
        let l = i as u64;
        let valid_date = expiration_date
            - ((constants::CRED_VALIDITY_PERIOD_DAYS - l - 1) * constants::SECONDS_PER_DAY);
        let m1: Scalar = Scalar::from(valid_date);

        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes()].concat());

        let sig = *sig.borrow();
        // Check if the hash is matching
        if sig.h != h {
            return Err(CompactEcashError::ExpirationDateSignatureVerification);
        }

        // let partially_signed_attributes = partially_signed + vk.beta_g2[1] * m1;
        pairing_terms.push((sig, partially_signed + vk.beta_g2[1] * m1));
    }

    if !batch_verify_signatures(pairing_terms.iter()) {
        return Err(CompactEcashError::ExpirationDateSignatureVerification);
    }
    Ok(())
}

/// Aggregates partial expiration date signatures into a list of aggregated expiration date signatures.
///
/// # Arguments
///
/// * `vk_auth` - The global verification key.
/// * `expiration_date` - The expiration date for which the signatures are being aggregated (as unix timestamp).
/// * `signatures_shares` - A list of tuples containing unique indices, verification keys, and partial expiration date signatures corresponding to the signing authorities.
///
/// # Returns
///
/// A `Result` containing a vector of `ExpirationDateSignature` if the aggregation is successful,
/// or an `Err` variant with a description of the encountered error.
///
/// # Errors
///
/// This function returns an error if there is a mismatch in the lengths of `signatures`. This occurs
/// when the number of tuples in `signatures` is not equal to the expected number of signing authorities.
/// Each tuple should contain a unique index, a verification key, and a list of partial signatures.
///
/// It also returns an error if there are not enough unique indices. This happens when the number
/// of unique indices in the tuples is less than the total number of signing authorities.
///
/// Additionally, an error is returned if the verification of the partial or aggregated signatures fails.
/// This can occur if the cryptographic verification process fails for any of the provided signatures.
///
fn _aggregate_expiration_signatures<B>(
    vk: &VerificationKeyAuth,
    expiration_date: u64,
    signatures_shares: &[ExpirationDateSignatureShare<B>],
    validate_shares: bool,
) -> Result<Vec<ExpirationDateSignature>>
where
    B: Borrow<ExpirationDateSignature>,
{
    // Check if all indices are unique
    if signatures_shares
        .iter()
        .map(|share| share.index)
        .unique()
        .count()
        != signatures_shares.len()
    {
        return Err(CompactEcashError::AggregationDuplicateIndices);
    }

    // Evaluate at 0 the Lagrange basis polynomials k_i
    let coefficients = generate_lagrangian_coefficients_at_origin(
        &signatures_shares
            .iter()
            .map(|share| share.index)
            .collect::<Vec<_>>(),
    );

    // Verify that all signatures are valid
    if validate_shares {
        cfg_if::cfg_if! {
            if #[cfg(feature = "par_verify")] {
                use rayon::prelude::*;

                signatures_shares.par_iter().try_for_each(|share| {
                    verify_valid_dates_signatures(&share.key, &share.signatures, expiration_date)
                })?;
            } else {
                signatures_shares.iter().try_for_each(|share| verify_valid_dates_signatures(&share.key, &share.signatures, expiration_date))?;
            }
        }
    }

    // Pre-allocate vectors
    let mut aggregated_date_signatures: Vec<ExpirationDateSignature> =
        Vec::with_capacity(constants::CRED_VALIDITY_PERIOD_DAYS as usize);

    let m0: Scalar = Scalar::from(expiration_date);

    for l in 0..constants::CRED_VALIDITY_PERIOD_DAYS {
        let valid_date = expiration_date
            - ((constants::CRED_VALIDITY_PERIOD_DAYS - l - 1) * constants::SECONDS_PER_DAY);
        let m1: Scalar = Scalar::from(valid_date);
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes()].concat());

        // Collect the partial signatures for the same valid date
        let collected_at_l: Vec<_> = signatures_shares
            .iter()
            .filter_map(|share| share.signatures.get(l as usize))
            .collect();

        // Aggregate partial signatures for each validity date
        let aggr_s: G1Projective = coefficients
            .iter()
            .zip(collected_at_l.iter())
            .map(|(coeff, &sig)| sig.borrow().s * coeff)
            .sum();
        let aggr_sig = ExpirationDateSignature { h, s: aggr_s };
        aggregated_date_signatures.push(aggr_sig);
    }
    verify_valid_dates_signatures(vk, &aggregated_date_signatures, expiration_date)?;
    Ok(aggregated_date_signatures)
}

/// Aggregates partial expiration date signatures into a list of aggregated expiration date signatures.
///
/// # Arguments
///
/// * `vk_auth` - The global verification key.
/// * `expiration_date` - The expiration date for which the signatures are being aggregated (as unix timestamp).
/// * `signatures_shares` - A list of tuples containing unique indices, verification keys, and partial expiration date signatures corresponding to the signing authorities.
///
/// # Returns
///
/// A `Result` containing a vector of `ExpirationDateSignature` if the aggregation is successful,
/// or an `Err` variant with a description of the encountered error.
///
/// # Errors
///
/// This function returns an error if there is a mismatch in the lengths of `signatures`. This occurs
/// when the number of tuples in `signatures` is not equal to the expected number of signing authorities.
/// Each tuple should contain a unique index, a verification key, and a list of partial signatures.
///
/// It also returns an error if there are not enough unique indices. This happens when the number
/// of unique indices in the tuples is less than the total number of signing authorities.
///
/// Additionally, an error is returned if the verification of the partial or aggregated signatures fails.
/// This can occur if the cryptographic verification process fails for any of the provided signatures.
///
pub fn aggregate_expiration_signatures<B>(
    vk: &VerificationKeyAuth,
    expiration_date: u64,
    signatures_shares: &[ExpirationDateSignatureShare<B>],
) -> Result<Vec<ExpirationDateSignature>>
where
    B: Borrow<PartialExpirationDateSignature>,
{
    _aggregate_expiration_signatures(vk, expiration_date, signatures_shares, true)
}

/// Perform aggregation and verification of partial expiration date signatures with
/// an additional check ensuring correct ordering of provided shares
///
/// It further annotates the result with timestamp information
pub fn aggregate_annotated_expiration_signatures(
    vk: &VerificationKeyAuth,
    expiration_date: u64,
    signatures_shares: &[ExpirationDateSignatureShare<AnnotatedExpirationDateSignature>],
) -> Result<Vec<AnnotatedExpirationDateSignature>> {
    // it's sufficient to just verify the first share as if the rest of them don't match,
    // the aggregation will fail anyway
    let Some(share) = signatures_shares.first() else {
        return Ok(Vec::new());
    };

    if share.signatures.len() != constants::CRED_VALIDITY_PERIOD_DAYS as usize {
        return Err(CompactEcashError::ExpirationDateSignatureVerification);
    }

    for (i, sig) in share.signatures.iter().enumerate() {
        if sig.expiration_timestamp != expiration_date {
            return Err(CompactEcashError::ExpirationDateSignatureVerification);
        }

        let l = i as u64;
        let expected_spending = sig.expiration_timestamp
            - ((constants::CRED_VALIDITY_PERIOD_DAYS - l - 1) * constants::SECONDS_PER_DAY);

        if sig.spending_timestamp != expected_spending {
            return Err(CompactEcashError::ExpirationDateSignatureVerification);
        }
    }

    let aggregated = aggregate_expiration_signatures(vk, expiration_date, signatures_shares)?;
    assert_eq!(aggregated.len(), share.signatures.len());

    Ok(aggregated
        .into_iter()
        .zip(share.signatures.iter())
        .map(|(signature, sh)| AnnotatedExpirationDateSignature {
            signature,
            expiration_timestamp: sh.expiration_timestamp,
            spending_timestamp: sh.spending_timestamp,
        })
        .collect())
}

/// An unchecked variant of `aggregate_expiration_signatures` that does not perform
/// validation of intermediate signatures.
///
/// It is expected the caller has already pre-validated them via manual calls to `verify_valid_dates_signatures`
pub fn unchecked_aggregate_expiration_signatures(
    vk: &VerificationKeyAuth,
    expiration_date: u64,
    signatures_shares: &[ExpirationDateSignatureShare],
) -> Result<Vec<ExpirationDateSignature>> {
    _aggregate_expiration_signatures(vk, expiration_date, signatures_shares, false)
}

/// Finds the index corresponding to the given spend date based on the expiration date.
///
/// This function calculates the index such that the following equality holds:
/// `spend_date = expiration_date - 30 + index`
/// This index is used to retrieve a corresponding signature.
///
/// # Arguments
///
/// * `spend_date` - The spend date for which to find the index.
/// * `expiration_date` - The expiration date used in the calculation.
///
/// # Returns
///
/// If a valid index is found, returns `Ok(index)`. If no valid index is found
/// (i.e., `spend_date` is earlier than `expiration_date - 30`), returns `Err(InvalidDateError)`.
///
pub fn find_index(spend_date: u64, expiration_date: u64) -> Result<usize> {
    let start_date =
        expiration_date - ((constants::CRED_VALIDITY_PERIOD_DAYS - 1) * constants::SECONDS_PER_DAY);

    if spend_date >= start_date {
        let index_a = ((spend_date - start_date) / constants::SECONDS_PER_DAY) as usize;
        if index_a as u64 >= constants::CRED_VALIDITY_PERIOD_DAYS {
            Err(CompactEcashError::SpendDateTooLate)
        } else {
            Ok(index_a)
        }
    } else {
        Err(CompactEcashError::SpendDateTooEarly)
    }
}

pub fn date_scalar(date: u64) -> Scalar {
    Scalar::from(date)
}

// TODO: this will not work for **all** scalars,
// but timestamps have extremely (relatively speaking) limited range,
// so this should be fine
pub fn scalar_date(scalar: &Scalar) -> u64 {
    let b = scalar.to_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

pub fn type_scalar(t_type: u64) -> Scalar {
    Scalar::from(t_type)
}

pub fn scalar_type(scalar: &Scalar) -> u64 {
    let b = scalar.to_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::ttp_keygen;

    #[test]
    fn test_find_index() {
        let expiration_date = 1701993600; // Dec 8 2023
        for i in 0..constants::CRED_VALIDITY_PERIOD_DAYS {
            let current_spend_date = expiration_date - i * 86400;
            assert_eq!(
                find_index(current_spend_date, expiration_date).unwrap(),
                (constants::CRED_VALIDITY_PERIOD_DAYS - 1 - i) as usize
            )
        }

        let late_spend_date = expiration_date + 86400;
        assert!(find_index(late_spend_date, expiration_date).is_err());

        let early_spend_date = expiration_date - (constants::CRED_VALIDITY_PERIOD_DAYS) * 86400;
        assert!(find_index(early_spend_date, expiration_date).is_err());
    }

    #[test]
    fn test_sign_expiration_date() {
        let expiration_date = 1702050209; // Dec 8 2023

        let authorities_keys = ttp_keygen(2, 3).unwrap();
        let sk_i_auth = authorities_keys[0].secret_key();
        let vk_i_auth = authorities_keys[0].verification_key();
        let partial_exp_sig = sign_expiration_date(sk_i_auth, expiration_date).unwrap();

        assert!(
            verify_valid_dates_signatures(&vk_i_auth, &partial_exp_sig, expiration_date).is_ok()
        );
    }

    #[test]
    fn test_aggregate_expiration_signatures() {
        let expiration_date = 1702050209; // Dec 8 2023

        let authorities_keypairs = ttp_keygen(2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        // list of secret keys of each authority
        let secret_keys_authorities: Vec<&SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();
        // list of verification keys of each authority
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();
        // the global master verification key
        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

        let mut edt_partial_signatures: Vec<Vec<_>> =
            Vec::with_capacity(constants::CRED_VALIDITY_PERIOD_DAYS as usize);
        for sk_auth in secret_keys_authorities.iter() {
            let sign = sign_expiration_date(sk_auth, expiration_date).unwrap();
            edt_partial_signatures.push(sign);
        }

        let combined_data = indices
            .iter()
            .zip(
                verification_keys_auth
                    .iter()
                    .zip(edt_partial_signatures.iter()),
            )
            .map(|(i, (vk, sigs))| ExpirationDateSignatureShare {
                index: *i,
                key: vk.clone(),
                signatures: sigs.clone(),
            })
            .collect::<Vec<_>>();

        assert!(aggregate_expiration_signatures(
            &verification_key,
            expiration_date,
            &combined_data,
        )
        .is_ok());
    }
}
