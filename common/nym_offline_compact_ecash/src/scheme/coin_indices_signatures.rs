// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{Signature, SignerIndex};
use crate::constants;
use crate::error::{CompactEcashError, Result};
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::scheme::setup::Parameters;
use crate::utils::generate_lagrangian_coefficients_at_origin;
use crate::utils::{batch_verify_signatures, hash_g1};
use bls12_381::{G1Projective, Scalar};
use itertools::Itertools;

pub type CoinIndexSignature = Signature;
pub type PartialCoinIndexSignature = CoinIndexSignature;

pub struct CoinIndexSignatureShare {
    pub index: SignerIndex,
    pub key: VerificationKeyAuth,
    pub signatures: Vec<PartialCoinIndexSignature>,
}

/// Signs coin indices.
///
/// This function takes cryptographic parameters, a global verification key, and a secret key of the signing authority,
/// and generates partial coin index signatures for a specified number of indices using a parallel fold operation.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `vk` - The global verification key.
/// * `sk_auth` - The secret key associated with the individual signing authority.
///
/// # Returns
///
/// A vector containing partial coin index signatures.
pub fn sign_coin_indices(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    sk_auth: &SecretKeyAuth,
) -> Result<Vec<PartialCoinIndexSignature>> {
    if sk_auth.ys.len() < 3 {
        return Err(CompactEcashError::KeyTooShort);
    }
    let m1: Scalar = constants::TYPE_IDX;
    let m2: Scalar = constants::TYPE_IDX;

    let vk_bytes = vk.to_bytes();
    let partial_s_exponent = sk_auth.x + sk_auth.ys[1] * m1 + sk_auth.ys[2] * m2;

    let sign_index = |l: u64| {
        let m0: Scalar = Scalar::from(l);
        // Compute the hash h
        let mut concatenated_bytes = Vec::with_capacity(vk_bytes.len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk_bytes);
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());
        let h = hash_g1(concatenated_bytes);

        // Sign the attributes
        let s_exponent = partial_s_exponent + sk_auth.ys[0] * m0;

        // Create the signature struct
        PartialCoinIndexSignature {
            h,
            s: h * s_exponent,
        }
    };

    cfg_if::cfg_if! {
        if #[cfg(feature = "par_signing")] {
            use rayon::prelude::*;

            Ok((0..params.get_total_coins())
                .into_par_iter()
                .map(sign_index)
                .collect())
        } else {
           Ok((0..params.get_total_coins()).map(sign_index).collect())
        }
    }
}

/// Verifies coin index signatures using parallel iterators.
///
/// This function takes cryptographic parameters, verification keys, and a list of coin index
/// signatures. It verifies each signature's commitment hash and performs a bilinear pairing check.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the verification process.
/// * `vk` - The global verification key.
/// * `vk_auth` - The verification key associated with the authority which issued the partial signatures.
/// * `signatures` - A slice containing coin index signatures to be verified.
///
/// # Returns
///
/// Returns `Ok(())` if all signatures are valid, otherwise returns an error with a description
/// of the verification failure.
pub fn verify_coin_indices_signatures(
    vk: &VerificationKeyAuth,
    vk_auth: &VerificationKeyAuth,
    signatures: &[CoinIndexSignature],
) -> Result<()> {
    if vk_auth.beta_g2.len() < 3 {
        return Err(CompactEcashError::KeyTooShort);
    }
    let m1: Scalar = constants::TYPE_IDX;
    let m2: Scalar = constants::TYPE_IDX;
    let partially_signed = vk_auth.alpha + vk_auth.beta_g2[1] * m1 + vk_auth.beta_g2[2] * m2;
    let vk_bytes = vk.to_bytes();

    let mut pairing_terms = Vec::with_capacity(signatures.len());

    for (i, sig) in signatures.iter().enumerate() {
        let l = i as u64;
        let mut concatenated_bytes = Vec::with_capacity(vk_bytes.len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk_bytes);
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());

        // Compute the hash h
        let h = hash_g1(concatenated_bytes.clone());

        // Check if the hash is matching
        if sig.h != h {
            return Err(CompactEcashError::CoinIndicesSignatureVerification);
        }

        let m0 = Scalar::from(l);
        // push elements for computing
        // e(h1, X1) * e(s1, g2^-1) * ... * e(hi, Xi) * e(si, g2^-1)
        // where
        // h: H(vk, l)
        // si: h^{xi + yi[0] * mi0 + yi[1] * m1 + yi[2] * m2}
        // X: g2^{x + y[0] * mi0 + yi[1] * m1 + yi[2] * m2}
        pairing_terms.push((sig, vk_auth.beta_g2[0] * m0 + partially_signed));
    }

    // computing all pairings in parallel using rayon makes it go from ~45ms to ~30ms,
    // but given this function is called very infrequently, the possible interference up the stack is not worth it
    if !batch_verify_signatures(pairing_terms.iter()) {
        return Err(CompactEcashError::CoinIndicesSignatureVerification);
    }

    Ok(())
}

fn _aggregate_indices_signatures(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    signatures_shares: &[CoinIndexSignatureShare],
    validate_shares: bool,
) -> Result<Vec<CoinIndexSignature>> {
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
                    verify_coin_indices_signatures(vk, &share.key, &share.signatures)
                })?;
            } else {

                signatures_shares.iter().try_for_each(|share| verify_coin_indices_signatures(vk, &share.key, &share.signatures))?;
            }
        }
    }

    // Pre-allocate vectors
    let mut aggregated_coin_signatures: Vec<CoinIndexSignature> =
        Vec::with_capacity(params.get_total_coins() as usize);

    let vk_bytes = vk.to_bytes();
    for l in 0..params.get_total_coins() {
        // Compute the hash h
        let mut concatenated_bytes = Vec::with_capacity(vk_bytes.len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk_bytes);
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());
        let h = hash_g1(concatenated_bytes);

        // Collect the partial signatures for the same coin index
        let collected_at_l: Vec<_> = signatures_shares
            .iter()
            .filter_map(|share| share.signatures.get(l as usize))
            .collect();

        // Aggregate partial signatures for each coin index
        let aggr_s: G1Projective = coefficients
            .iter()
            .zip(collected_at_l.iter())
            .map(|(coeff, sig)| sig.s * coeff)
            .sum();
        let aggr_sig = CoinIndexSignature { h, s: aggr_s };
        aggregated_coin_signatures.push(aggr_sig);
    }
    verify_coin_indices_signatures(vk, vk, &aggregated_coin_signatures)?;
    Ok(aggregated_coin_signatures)
}

/// Aggregates and verifies partial coin index signatures.
///
/// This function takes cryptographic parameters, a master verification key, and a list of tuples
/// containing indices, verification keys, and partial coin index signatures from different authorities.
/// It aggregates these partial signatures into a final set of coin index signatures, and verifying the
/// final aggregated signatures.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the aggregation process.
/// * `vk` - The master verification key against which the partial signatures are verified.
/// * `signatures` - A slice of tuples, where each tuple contains an index, a verification key, and
///   a vector of partial coin index signatures from a specific authority.
///
/// # Returns
///
/// Returns a vector of aggregated coin index signatures if the aggregation is successful.
/// Otherwise, returns an error describing the nature of the failure.
pub fn aggregate_indices_signatures(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    signatures_shares: &[CoinIndexSignatureShare],
) -> Result<Vec<CoinIndexSignature>> {
    _aggregate_indices_signatures(params, vk, signatures_shares, true)
}

/// An unchecked variant of `aggregate_indices_signatures` that does not perform
/// validation of intermediate signatures.
///
/// It is expected the caller has already pre-validated them via manual calls to `verify_coin_indices_signatures`
pub fn unchecked_aggregate_indices_signatures(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    signatures_shares: &[CoinIndexSignatureShare],
) -> Result<Vec<CoinIndexSignature>> {
    _aggregate_indices_signatures(params, vk, signatures_shares, false)
}

/// Generates parameters for the scheme setup.
///
/// # Arguments
///
/// * `total_coins` - it is the number of coins in a freshly generated wallet. It is the public parameter of the scheme.
///
/// # Returns
///
/// A `Parameters` struct containing group parameters, public key, the number of signatures (`total_coins`),
/// and a map of signatures for each index `l`.
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::ttp_keygen;

    #[test]
    fn test_sign_coins() {
        let total_coins = 32;
        let params = Parameters::new(total_coins);
        let authorities_keypairs = ttp_keygen(2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];

        // Pick one authority to do the signing
        let sk_i_auth = authorities_keypairs[0].secret_key();
        let vk_i_auth = authorities_keypairs[0].verification_key();

        // list of verification keys of each authority
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();
        // the global master verification key
        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

        let partial_signatures = sign_coin_indices(&params, &verification_key, sk_i_auth).unwrap();
        assert!(
            verify_coin_indices_signatures(&verification_key, &vk_i_auth, &partial_signatures)
                .is_ok()
        );
    }

    #[test]
    fn test_sign_coins_fail() {
        let total_coins = 32;
        let params = Parameters::new(total_coins);
        let authorities_keypairs = ttp_keygen(2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];

        // Pick one authority to do the signing
        let sk_0_auth = authorities_keypairs[0].secret_key();
        let vk_1_auth = authorities_keypairs[1].verification_key();

        // list of verification keys of each authority
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();
        // the global master verification key
        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

        let partial_signatures = sign_coin_indices(&params, &verification_key, sk_0_auth).unwrap();
        // Since we used a non matching verification key to verify the signature, the verification should fail
        assert!(
            verify_coin_indices_signatures(&verification_key, &vk_1_auth, &partial_signatures)
                .is_err()
        );
    }

    #[test]
    fn test_aggregate_coin_indices_signatures() {
        let total_coins = 32;
        let params = Parameters::new(total_coins);
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

        // create the partial signatures from each authority
        let partial_signatures: Vec<Vec<PartialCoinIndexSignature>> = secret_keys_authorities
            .iter()
            .map(|sk_auth| sign_coin_indices(&params, &verification_key, sk_auth).unwrap())
            .collect();

        let combined_data = indices
            .iter()
            .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
            .map(|(i, (vk, sigs))| CoinIndexSignatureShare {
                index: *i,
                key: vk.clone(),
                signatures: sigs.clone(),
            })
            .collect::<Vec<_>>();

        assert!(aggregate_indices_signatures(&params, &verification_key, &combined_data).is_ok());
    }
}
