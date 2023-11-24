use std::collections::HashMap;
use std::ops::Index;

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use group::{Curve, GroupEncoding};
use rand::thread_rng;

use crate::constants;
use crate::error::{CompactEcashError, Result};
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::utils::{check_bilinear_pairing, generate_lagrangian_coefficients_at_origin};
use crate::utils::{hash_g1, Signature};
use itertools::Itertools;
use rayon::prelude::*;

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
    pub fn new() -> Result<GroupParameters> {
        let gammas = (1..=constants::ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();

        let delta = hash_g1("delta");

        Ok(GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            delta,
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        })
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

#[derive(Debug, PartialEq, Clone)]
pub struct SecretKeyRP {
    pub(crate) x: Scalar,
    pub(crate) y: Scalar,
}

impl SecretKeyRP {
    pub fn public_key(&self, params: &GroupParameters) -> PublicKeyRP {
        PublicKeyRP {
            alpha: params.gen2() * self.x,
            beta: params.gen2() * self.y,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PublicKeyRP {
    pub(crate) alpha: G2Projective,
    pub(crate) beta: G2Projective,
}

pub struct Parameters {
    /// group parameters
    grp: GroupParameters,
    /// Public Key for range proof verification
    pk_rp: PublicKeyRP,
    /// Max value of wallet
    L: u64,
    /// list of signatures for values l in [0, L]
    signs: HashMap<u64, Signature>,
}

impl Parameters {
    pub fn grp(&self) -> &GroupParameters {
        &self.grp
    }
    pub fn pk_rp(&self) -> &PublicKeyRP {
        &self.pk_rp
    }
    pub fn L(&self) -> u64 {
        self.L
    }
    pub fn signs(&self) -> &HashMap<u64, Signature> {
        &self.signs
    }
    pub fn get_sign_by_idx(&self, idx: u64) -> Result<&Signature> {
        match self.signs.get(&idx) {
            Some(val) => return Ok(val),
            None => {
                return Err(CompactEcashError::RangeProofOutOfBound(
                    "Cannot find the range proof signature for the given value. \
                        Check if the requested value is within the bound 0..L"
                        .to_string(),
                ));
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CoinIndexSignature {
    pub(crate) h: G1Projective,
    pub(crate) s: G1Projective,
}

pub type PartialCoinIndexSignature = CoinIndexSignature;

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
///
/// # Panics
///
/// The function may panic if there is an issue with converting bytes to Scalar during initialization.
///
pub fn sign_coin_indices(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    sk_auth: &SecretKeyAuth,
) -> Vec<PartialCoinIndexSignature> {
    let m1: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    let m2: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    (0..params.L())
        .into_par_iter()
        .fold(
            || Vec::with_capacity(params.L() as usize),
            |mut partial_coins_signatures, l| {
                let m0: Scalar = Scalar::from(l as u64);
                // Compute the hash h
                let mut concatenated_bytes =
                    Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
                concatenated_bytes.extend_from_slice(&vk.to_bytes());
                concatenated_bytes.extend_from_slice(&l.to_le_bytes());
                let h = hash_g1(concatenated_bytes);

                // Sign the attributes
                let mut s_exponent = sk_auth.x;
                s_exponent += &sk_auth.ys[0] * m0;
                s_exponent += &sk_auth.ys[1] * m1;
                s_exponent += &sk_auth.ys[2] * m2;

                // Create the signature struct
                let coin_idx_sign = PartialCoinIndexSignature { h, s: h * s_exponent };
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
///
/// # Panics
///
/// The function may panic if there is an issue with converting bytes to Scalar during initialization.
pub fn verify_coin_indices_signatures(
    params: &Parameters,
    vk: &VerificationKeyAuth,
    vk_auth: &VerificationKeyAuth,
    signatures: &[CoinIndexSignature],
) -> Result<()> {
    let m1: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    let m2: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();

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
        .zip(signatures.par_iter().zip(concatenated_bytes_list.par_iter()))
        .enumerate()
        .try_for_each(|(l, (m0, (sig, concatenated_bytes)))| {
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
                .map(|(m, beta_i)| beta_i * Scalar::from(*m))
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
///
/// # Panics
///
/// The function may panic if there is an issue with converting bytes to Scalar during initialization.
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
    let m1: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    let m2: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();

    // Pre-allocate vectors
    let mut aggregated_coin_signatures: Vec<CoinIndexSignature> =
        Vec::with_capacity(params.L() as usize);

    for l in 0..params.L() {
        let m0: Scalar = Scalar::from(l);
        // Compute the hash h
        let mut concatenated_bytes =
            Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk.to_bytes());
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());
        let h = hash_g1(concatenated_bytes);

        signatures
            .par_iter()
            .try_for_each(|(_, vk_auth, partial_signatures)| {
                verify_coin_indices_signatures(&params, &vk, &vk_auth, &partial_signatures)
            })?;

        // Collect the partial signatures for the same coin index
        let collected_at_l: Vec<_> = signatures
            .iter()
            .filter_map(|(_, _, inner_vec)| inner_vec.get(l as usize))
            .cloned()
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
    verify_coin_indices_signatures(&params, &vk, &vk, &aggregated_coin_signatures)?;
    Ok(aggregated_coin_signatures)
}

pub fn setup(L: u64) -> Parameters {
    let grp = GroupParameters::new().unwrap();
    let x = grp.random_scalar();
    let y = grp.random_scalar();
    let sk_rp = SecretKeyRP { x, y };
    let pk_rp = sk_rp.public_key(&grp);
    let mut signs = HashMap::new();
    for l in 0..L {
        let r = grp.random_scalar();
        let h = grp.gen1() * r;
        signs.insert(
            l,
            Signature {
                0: h,
                1: h * (x + y * Scalar::from(l)),
            },
        );
    }
    Parameters {
        grp,
        pk_rp,
        L,
        signs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::ttp_keygen;

    #[test]
    fn test_sign_coins() {
        let L = 32;
        let params = setup(L);
        let authorities_keypairs = ttp_keygen(&params.grp(), 2, 3).unwrap();
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
    fn test_aggregate_coin_signatures() {
        let L = 32;
        let params = setup(L);
        let authorities_keypairs = ttp_keygen(&params.grp(), 2, 3).unwrap();
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

        let combined_data: Vec<(
            u64,
            VerificationKeyAuth,
            Vec<PartialCoinIndexSignature>,
        )> = indices
            .iter()
            .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
            .map(|(i, (vk, sigs))| (i.clone(), vk.clone(), sigs.clone()))
            .collect();

        assert!(aggregate_indices_signatures(&params, &verification_key, &combined_data).is_ok());
    }

}
