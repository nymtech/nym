// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use group::{Curve, GroupEncoding};
use rand::thread_rng;

use crate::error::{CompactEcashError, Result};
use crate::{constants, Base58};

use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::traits::Bytable;
use crate::utils::{check_bilinear_pairing, generate_lagrangian_coefficients_at_origin};
use crate::utils::{hash_g1, try_deserialize_g1_projective};
use itertools::Itertools;
use rayon::prelude::*;

#[derive(Debug)]
pub struct GroupParameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Projective>,
    // Additional generator of the G1 group
    delta: G1Projective,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl GroupParameters {
    pub fn new() -> GroupParameters {
        let gammas = (1..=constants::ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();

        let delta = hash_g1("delta");

        GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            delta,
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        }
    }

    pub(crate) fn gen1(&self) -> &G1Affine {
        &self.g1
    }

    pub(crate) fn gen2(&self) -> &G2Affine {
        &self.g2
    }

    pub(crate) fn gammas(&self) -> &Vec<G1Projective> {
        &self.gammas
    }

    pub(crate) fn gammas_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.gammas.len() * 48);
        for g in &self.gammas {
            bytes.extend_from_slice(g.to_bytes().as_ref());
        }
        bytes
    }

    pub(crate) fn gamma_idx(&self, i: usize) -> Option<&G1Projective> {
        self.gammas.get(i)
    }

    pub(crate) fn delta(&self) -> &G1Projective {
        &self.delta
    }

    pub fn random_scalar(&self) -> Scalar {
        // lazily-initialized thread-local random number generator, seeded by the system
        let mut rng = thread_rng();
        Scalar::random(&mut rng)
    }

    pub fn n_random_scalars(&self, n: usize) -> Vec<Scalar> {
        (0..n).map(|_| self.random_scalar()).collect()
    }

    pub(crate) fn prepared_miller_g2(&self) -> &G2Prepared {
        &self._g2_prepared_miller
    }
}

impl Default for GroupParameters {
    fn default() -> Self {
        GroupParameters::new()
    }
}

#[derive(Debug)]
pub struct Parameters {
    /// group parameters
    grp: GroupParameters,
    /// Number of coins of fixed denomination in the credential wallet; L in construction
    total_coins: u64,
}

impl Parameters {
    pub fn grp(&self) -> &GroupParameters {
        &self.grp
    }

    pub fn get_total_coins(&self) -> u64 {
        self.total_coins
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoinIndexSignature {
    pub(crate) h: G1Projective,
    pub(crate) s: G1Projective,
}

pub type PartialCoinIndexSignature = CoinIndexSignature;

impl CoinIndexSignature {
    pub fn randomise(&self, params: &GroupParameters) -> (CoinIndexSignature, Scalar) {
        let r = params.random_scalar();
        let r_prime = params.random_scalar();
        let h_prime = self.h * r_prime;
        let s_prime = (self.s * r_prime) + (h_prime * r);
        (
            CoinIndexSignature {
                h: h_prime,
                s: s_prime,
            },
            r,
        )
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(48 + 48);
        bytes.extend(self.h.to_affine().to_compressed());
        bytes.extend(self.s.to_affine().to_compressed());
        bytes
    }
}

impl TryFrom<&[u8]> for CoinIndexSignature {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<CoinIndexSignature> {
        if bytes.len() != 96 {
            return Err(CompactEcashError::Deserialization(format!(
                "CoinIndexSignature must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        let h_bytes: &[u8; 48] = &bytes[..48].try_into().expect("Slice size != 48");
        let s_bytes: &[u8; 48] = &bytes[48..].try_into().expect("Slice size != 48");

        let h = try_deserialize_g1_projective(
            h_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize compressed h of the CoinIndexSignature".to_string(),
            ),
        )?;

        let s = try_deserialize_g1_projective(
            s_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize compressed s of the CoinIndexSignature".to_string(),
            ),
        )?;

        Ok(CoinIndexSignature { h, s })
    }
}

impl Bytable for CoinIndexSignature {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Self::try_from(slice)
    }
}
impl Base58 for CoinIndexSignature {}

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
) -> Vec<PartialCoinIndexSignature> {
    let m1: Scalar = constants::TYPE_IDX;
    let m2: Scalar = constants::TYPE_IDX;
    (0..params.get_total_coins())
        .into_par_iter()
        .fold(
            || Vec::with_capacity(params.get_total_coins() as usize),
            |mut partial_coins_signatures, l| {
                let m0: Scalar = Scalar::from(l);
                // Compute the hash h
                let mut concatenated_bytes =
                    Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
                concatenated_bytes.extend_from_slice(&vk.to_bytes());
                concatenated_bytes.extend_from_slice(&l.to_le_bytes());
                let h = hash_g1(concatenated_bytes);

                // Sign the attributes
                let mut s_exponent = sk_auth.x;
                s_exponent += sk_auth.ys[0] * m0;
                s_exponent += sk_auth.ys[1] * m1;
                s_exponent += sk_auth.ys[2] * m2;

                // Create the signature struct
                let coin_idx_sign = PartialCoinIndexSignature {
                    h,
                    s: h * s_exponent,
                };
                partial_coins_signatures.push(coin_idx_sign);

                partial_coins_signatures
            },
        )
        .reduce(Vec::new, |mut v1, mut v2| {
            v1.append(&mut v2);
            v1
        })
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
    params: &Parameters,
    vk: &VerificationKeyAuth,
    vk_auth: &VerificationKeyAuth,
    signatures: &[CoinIndexSignature],
) -> Result<()> {
    let m1: Scalar = constants::TYPE_IDX;
    let m2: Scalar = constants::TYPE_IDX;

    // Precompute concatenated_bytes for each l
    let concatenated_bytes_list: Vec<Vec<u8>> = signatures
        .iter()
        .enumerate()
        .map(|(l, _)| {
            let mut concatenated_bytes =
                Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
            concatenated_bytes.extend_from_slice(&vk.to_bytes());
            concatenated_bytes.extend_from_slice(&(l as u64).to_le_bytes());
            concatenated_bytes
        })
        .collect();
    // Create a vector of m0 values
    let m0_values: Vec<Scalar> = (0..signatures.len() as u64).map(Scalar::from).collect();

    // Verify signatures using precomputed concatenated_bytes and m0 values
    m0_values
        .par_iter()
        .zip(
            signatures
                .par_iter()
                .zip(concatenated_bytes_list.par_iter()),
        )
        .enumerate()
        .try_for_each(|(_, (m0, (sig, concatenated_bytes)))| {
            // Compute the hash h
            let h = hash_g1(concatenated_bytes.clone());
            // Check if the hash is matching
            if sig.h != h {
                return Err(CompactEcashError::CoinIndices(
                    "Failed to verify the commitment hash".to_string(),
                ));
            }
            let partially_signed_attributes = [*m0, m1, m2]
                .iter()
                .zip(vk_auth.beta_g2.iter())
                .map(|(m, beta_i)| beta_i * m)
                .sum::<G2Projective>();

            if !check_bilinear_pairing(
                &sig.h.to_affine(),
                &G2Prepared::from((vk_auth.alpha + partially_signed_attributes).to_affine()),
                &sig.s.to_affine(),
                params.grp().prepared_miller_g2(),
            ) {
                return Err(CompactEcashError::CoinIndices(
                    "Verification of the coin signature failed".to_string(),
                ));
            }
            Ok(())
        })?;

    Ok(())
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
    signatures: &[(u64, VerificationKeyAuth, Vec<PartialCoinIndexSignature>)],
) -> Result<Vec<CoinIndexSignature>> {
    // Check if all indices are unique
    if signatures
        .iter()
        .map(|(index, _, _)| index)
        .unique()
        .count()
        != signatures.len()
    {
        return Err(CompactEcashError::CoinIndices(
            "Not enough unique indices shares".to_string(),
        ));
    }

    // Evaluate at 0 the Lagrange basis polynomials k_i
    let coefficients = generate_lagrangian_coefficients_at_origin(
        &signatures
            .iter()
            .map(|(index, _, _)| *index)
            .collect::<Vec<_>>(),
    );

    // Verify that all signatures are valid
    signatures
        .par_iter()
        .try_for_each(|(_, vk_auth, partial_signatures)| {
            verify_coin_indices_signatures(params, vk, vk_auth, partial_signatures)
        })?;

    // Pre-allocate vectors
    let mut aggregated_coin_signatures: Vec<CoinIndexSignature> =
        Vec::with_capacity(params.get_total_coins() as usize);

    for l in 0..params.get_total_coins() {
        // Compute the hash h
        let mut concatenated_bytes =
            Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk.to_bytes());
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());
        let h = hash_g1(concatenated_bytes);

        // Collect the partial signatures for the same coin index
        let collected_at_l: Vec<_> = signatures
            .iter()
            .filter_map(|(_, _, inner_vec)| inner_vec.get(l as usize))
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
    verify_coin_indices_signatures(params, vk, vk, &aggregated_coin_signatures)?;
    Ok(aggregated_coin_signatures)
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
pub fn setup(total_coins: u64) -> Parameters {
    assert!(total_coins > 0);
    let grp = GroupParameters::new();
    Parameters { grp, total_coins }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::ttp_keygen;

    #[test]
    fn test_sign_coins() {
        let total_coins = 32;
        let params = setup(total_coins);
        let authorities_keypairs = ttp_keygen(params.grp(), 2, 3).unwrap();
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

        let partial_signatures = sign_coin_indices(&params, &verification_key, &sk_i_auth);
        assert!(verify_coin_indices_signatures(
            &params,
            &verification_key,
            &vk_i_auth,
            &partial_signatures
        )
        .is_ok());
    }

    #[test]
    fn test_sign_coins_fail() {
        let total_coins = 32;
        let params = setup(total_coins);
        let authorities_keypairs = ttp_keygen(params.grp(), 2, 3).unwrap();
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

        let partial_signatures = sign_coin_indices(&params, &verification_key, &sk_0_auth);
        // Since we used a non matching verification key to verify the signature, the verification should fail
        assert!(verify_coin_indices_signatures(
            &params,
            &verification_key,
            &vk_1_auth,
            &partial_signatures
        )
        .is_err());
    }

    #[test]
    fn test_aggregate_coin_indices_signatures() {
        let total_coins = 32;
        let params = setup(total_coins);
        let authorities_keypairs = ttp_keygen(params.grp(), 2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];

        // list of secret keys of each authority
        let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
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
            .map(|sk_auth| sign_coin_indices(&params, &verification_key, sk_auth))
            .collect();

        let combined_data: Vec<(u64, VerificationKeyAuth, Vec<PartialCoinIndexSignature>)> =
            indices
                .iter()
                .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
                .map(|(i, (vk, sigs))| (*i, vk.clone(), sigs.clone()))
                .collect();

        assert!(aggregate_indices_signatures(&params, &verification_key, &combined_data).is_ok());
    }
}
