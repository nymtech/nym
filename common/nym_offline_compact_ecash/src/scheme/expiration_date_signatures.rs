use crate::scheme::setup::Parameters;
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::constants;
use crate::utils::hash_g1;
use bls12_381::{Scalar, G1Projective, G2Projective, G2Prepared};
use group::{Curve, GroupEncoding};
use crate::utils::{check_bilinear_pairing, generate_lagrangian_coefficients_at_origin};
use crate::error::{CompactEcashError, Result};
use itertools::Itertools;
use rayon::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub struct ExpirationDateSignature{
    pub(crate) h : G1Projective,
    pub(crate) s : G1Projective,
}

pub type PartialExpirationDateSignature = ExpirationDateSignature;

/// Signs given expiration date for a specified validity period using the given secret key of a single authority.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `sk_auth` - The secret key of the signing authority.
/// * `expiration_date` - The expiration date for which signatures will be generated (as unix timestamp).
///
/// # Returns
///
/// A vector containing partial signatures for each date within the validity period (i.e.,
/// from expiration_date - VALIDITY_PERIOD till expiration_date.
///
/// # Note
///
/// This function is executed by a single singing authority and generates partial expiration date
/// signatures for a specified validity period. Each signature is created by combining cryptographic
/// attributes derived from the expiration date, and the resulting vector contains signatures for
/// each date within the defined validity period till expiration date.
/// The validity period is determined by the constant `VALIDITY_PERIOD` in the `constants` module.
pub fn sign_expiration_date(
    params: &Parameters,
    sk_auth: &SecretKeyAuth,
    expiration_date: u64
) -> Vec<PartialExpirationDateSignature>{

    // Initialize a vector to collect exp_sign values
    let mut exp_signs = Vec::with_capacity(constants::VALIDITY_PERIOD as usize);

    for l in 0..constants::VALIDITY_PERIOD {
        let m0: Scalar = Scalar::from(expiration_date);
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l);
        let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes(), m2.to_bytes()].concat());
        // Sign the attributes by performing scalar-point multiplications and accumulating the result
        let mut s_exponent = sk_auth.x;
        s_exponent += &sk_auth.ys[0] * m0;
        s_exponent += &sk_auth.ys[1] * m1;
        s_exponent += &sk_auth.ys[2] * m2;
        // Create the signature struct on the expiration date
        let exp_sign = PartialExpirationDateSignature {
            h,
            s: h * s_exponent,
        };
        // Collect the exp_sign value into the vector
        exp_signs.push(exp_sign);
    }
    exp_signs
}

/// Verifies the expiration date signatures against the given verification key.
///
/// This function iterates over the provided valid date signatures and verifies each one
/// against the provided verification key. It computes the hash and checks the correctness of the
/// signature using bilinear pairings.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `vkey` - The verification key of the signing authority.
/// * `signatures` - The list of date signatures to be verified.
/// * `expiration_date` - The expiration date for which signatures are being issued (as unix timestamp).
///
/// # Returns
///
/// Returns `Ok(true)` if all signatures are verified successfully, otherwise returns an
/// `Err(CompactEcashError::ExpirationDate)` with an error message.
///
pub fn verify_valid_dates_signatures(
    params: &Parameters,
    vkey: &VerificationKeyAuth,
    signatures: &[ExpirationDateSignature],
    expiration_date: u64,
) -> Result<()>{
    for (l , sig) in signatures.iter().enumerate() {
        let m0: Scalar = Scalar::from(expiration_date);
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l as u64);
        let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes(), m2.to_bytes()].concat());
        // Verify the signature correctness
        if sig.h != h {
            return Err(CompactEcashError::ExpirationDate(
                "Failed to verify the commitment hash".to_string(),
            ));
        }
        let partially_signed_attributes = [m0, m1, m2]
            .iter()
            .zip(vkey.beta_g2.iter())
            .map(|(m, beta_i)| beta_i * Scalar::from(*m))
            .sum::<G2Projective>();

        if !check_bilinear_pairing(
            &sig.h.to_affine(),
            &G2Prepared::from((vkey.alpha + partially_signed_attributes).to_affine()),
            &sig.s.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::ExpirationDate(
                "Verification of the date signature failed".to_string(),
            ));
        }
    }
    Ok(())
}

/// Aggregates partial expiration date signatures into a list of aggregated expiration date signatures.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `vk_auth` - The global verification key.
/// * `expiration_date` - The expiration date for which the signatures are being aggregated (as unix timestamp).
/// * `indices` - A list of unique indices corresponding to the signing authorities.
/// * `vkeys` - A list of verification keys associated with the signing authorities.
/// * `signatures` - A list of partial expiration date signatures to be aggregated.
///
/// # Returns
///
/// A `Result` containing a vector of `ExpirationDateSignature` if the aggregation is successful,
/// or an `Err` variant with a description of the encountered error.
///
/// # Errors
///
/// This function returns an error if there is a mismatch in the lengths of `vkeys` and `signatures`,
/// or if there are not enough unique indices. It also returns an error the verification of
/// the partial or aggregated signatures fails.
pub fn aggregate_expiration_signatures(
    params: &Parameters,
    vk_auth: &VerificationKeyAuth,
    expiration_date: u64,
    indices: &[u64],
    vkeys: &[VerificationKeyAuth],
    signatures: &[Vec<PartialExpirationDateSignature>]) -> Result<Vec<ExpirationDateSignature>>{
    // Check if vkeys and signatures have the same length
    if vkeys.len() != signatures.len() {
        return Err(CompactEcashError::ExpirationDate(
            "Mismatched lengths of vkeys and signatures".to_string(),
        ));
    }

    // Check if we have enough unique partial signatures to meet the required threshold
    if indices.iter().unique_by(|&index| index).count() != indices.len() {
        return Err(CompactEcashError::ExpirationDate(
            "Not enough unique indices shares".to_string(),
        ));
    }

    // Evaluate at 0 the Lagrange basis polynomials k_i
    let coefficients = generate_lagrangian_coefficients_at_origin(indices);

    // Pre-allocate vectors
    let mut collected_per_date: Vec<Vec<PartialExpirationDateSignature>> =
        Vec::with_capacity(constants::VALIDITY_PERIOD as usize);
    let mut aggregated_date_signatures: Vec<ExpirationDateSignature> =
        Vec::with_capacity(constants::VALIDITY_PERIOD as usize);

    let m0: Scalar = Scalar::from(expiration_date);
    let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();
    for l in 0..constants::VALIDITY_PERIOD {
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l);
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes(), m2.to_bytes()].concat());

        // Verify each partial signature
        signatures
            .par_iter()
            .zip(vkeys.par_iter())
            .try_for_each(|(partial_signatures, vkey)| {
                verify_valid_dates_signatures(params, vkey, partial_signatures, expiration_date)
            })?;

        // Collect the partial signatures for the same valid date
        let collected_at_l: Vec<_> = signatures
            .iter()
            .filter_map(|inner_vec| inner_vec.get(l as usize))
            .cloned()
            .collect();

        // Aggregate partial signatures for each validity date
        let aggr_s: G1Projective = coefficients
            .iter()
            .zip(collected_at_l.iter())
            .map(|(coeff, sig)| sig.s * coeff)
            .sum();
        let aggr_sig = ExpirationDateSignature { h, s: aggr_s };
        aggregated_date_signatures.push(aggr_sig);
    }
    verify_valid_dates_signatures(&params, &vk_auth, &aggregated_date_signatures, expiration_date)?;
    Ok(aggregated_date_signatures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::keygen::{ttp_keygen};
    use crate::scheme::setup::setup;
    use crate::scheme::aggregation::aggregate_verification_keys;

    #[test]
    fn test_sign_expiration_date(){
        let L = 32;
        let params = setup(L);
        let expiration_date = 1703183958;

        let authorities_keys = ttp_keygen(&params.grp(), 2, 3).unwrap();
        let sk_i_auth = authorities_keys[0].secret_key();
        let vk_i_auth = authorities_keys[0].verification_key();
        let partial_exp_sig = sign_expiration_date(&params, &sk_i_auth, expiration_date);

        assert!(verify_valid_dates_signatures(&params, &vk_i_auth, &partial_exp_sig, expiration_date).is_ok());

    }

    #[test]
    fn test_aggregate_expiration_signatures(){
        let L = 32;
        let params = setup(L);
        let expiration_date = 1703183958;

        let authorities_keypairs = ttp_keygen(&params.grp(), 2, 3).unwrap();
        let indices = [1, 2, 3];
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
        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

        let mut partial_signatures: Vec<Vec<PartialExpirationDateSignature>> = Vec::with_capacity(constants::VALIDITY_PERIOD as usize);
        for sk in secret_keys_authorities.iter(){
            let sign = sign_expiration_date(&params,
                                 &sk,
                                 expiration_date);
            partial_signatures.push(sign);
        }
        aggregate_expiration_signatures(&params, &verification_key, expiration_date, &indices, &verification_keys_auth, &partial_signatures);
    }

}