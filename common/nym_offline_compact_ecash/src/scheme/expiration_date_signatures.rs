use crate::scheme::setup::Parameters;
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::constants;
use crate::utils::hash_g1;
use bls12_381::{Scalar, G1Projective, G2Projective, G2Prepared};
use group::{Curve, GroupEncoding};
use crate::utils::{check_bilinear_pairing, generate_lagrangian_coefficients_at_origin};
use crate::error::{CompactEcashError, Result};
use itertools::Itertools;

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
/// * `expiration_date` - The expiration date for which signatures will be generated.
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
/// * `expiration_date` - The expiration date for which signatures are being issued.
///
/// # Returns
///
/// Returns `Ok(true)` if all signatures are verified successfully, otherwise returns an
/// `Err(CompactEcashError::ExpirationDate)` with an error message.
///
pub fn verify_sign_expiration_date(
    params: &Parameters,
    vkey: &VerificationKeyAuth,
    signatures: &[ExpirationDateSignature],
    expiration_date: u64,
) -> Result<bool>{
    for (l , sig) in signatures.iter().enumerate() {
        let m0: Scalar = Scalar::from(expiration_date);
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l as u64);
        let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes(), m2.to_bytes()].concat());
        // Verify the signature correctness
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
    Ok(true)
}

/// Aggregates partial expiration date signatures into a list of aggregated expiration date signatures.
///
/// # Arguments
///
/// * `params` - The cryptographic parameters used in the signing process.
/// * `vk_auth` - The global verification key.
/// * `expiration_date` - The expiration date for which the signatures are being aggregated.
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
    signatures: &[PartialExpirationDateSignature]) -> Result<Vec<ExpirationDateSignature>>{
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

    let mut exp_date_signs: Vec<ExpirationDateSignature> = Vec::new();

    for l in 0..constants::VALIDITY_PERIOD {
        let m0: Scalar = Scalar::from(expiration_date);
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l);
        let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();
        // Compute the hash
        let h = hash_g1([m0.to_bytes(), m1.to_bytes(), m2.to_bytes()].concat());

        // Verify each partial signature
        for (vkey, sig) in vkeys.iter().zip(signatures.iter()) {
            if sig.h != h {
                return Err(CompactEcashError::ExpirationDate(
                    "Failed to verify the commitment hash".to_string(),
                ));
            }
            // Verify the signature correctness
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
                    "Verification of the partial expiration date signature failed".to_string(),
                ));
            }
        }
        // Compute the aggregated signature of the expiration date
        let aggr_s: G1Projective = coefficients
            .iter()
            .zip(signatures.iter())
            .map(|(coeff, sig)| sig.s * coeff)
            .sum();

        let aggr_sig = ExpirationDateSignature { h, s: aggr_s };

        let signed_attributes = [m0, m1, m2]
            .iter()
            .zip(vk_auth.beta_g2.iter())
            .map(|(m, beta_i)| beta_i * Scalar::from(*m))
            .sum::<G2Projective>();

        // Check the validity of the aggregated signature
        if !check_bilinear_pairing(
            &aggr_sig.h.to_affine(),
            &G2Prepared::from((vk_auth.alpha + signed_attributes).to_affine()),
            &aggr_sig.s.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::ExpirationDate(
                "Verification of the aggregated expiration date signature failed".to_string(),
            ));
        }
        exp_date_signs.push(aggr_sig);
    }
    Ok(exp_date_signs)

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::keygen::{ttp_keygen};
    use crate::scheme::setup::setup;

    #[test]
    fn test_sign_expiration_date(){
        let L = 32;
        let params = setup(L);
        let expiration_date = 1703183958;

        let authorities_keys = ttp_keygen(&params.grp(), 2, 3).unwrap();
        let sk_auth = authorities_keys[0].secret_key();
        let vk_auth = authorities_keys[0].verification_key();
        let partial_exp_sig = sign_expiration_date(&params, &sk_auth, expiration_date);

        assert!(verify_sign_expiration_date(&params, &vk_auth, &partial_exp_sig, expiration_date).unwrap());

    }
}