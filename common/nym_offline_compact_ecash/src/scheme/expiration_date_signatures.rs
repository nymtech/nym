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

// This function is execute by a single issuing authority
pub fn sign_expiration_date(params: &Parameters, sk_auth: SecretKeyAuth, expiration_date: u64) -> Vec<PartialExpirationDateSignature>{

    // Initialize a vector to collect exp_sign values
    let mut exp_signs: Vec<PartialExpirationDateSignature> = Vec::new();

    for l in 0..constants::VALIDITY_PERIOD {
        let m0: Scalar = Scalar::from(expiration_date);
        let m1: Scalar = Scalar::from(expiration_date - constants::VALIDITY_PERIOD + l);
        let m2: Scalar = Scalar::from_bytes(&constants::TYPE_EXP).unwrap();

        // Convert u64 values to bytes
        let m0_bytes: [u8; 32] = m0.to_bytes();
        let m1_bytes: [u8; 32] = m1.to_bytes();
        let m2_bytes: [u8; 32] = m2.to_bytes();
        let mut concatenated_attributes = Vec::new();

        concatenated_attributes.extend_from_slice(&m0_bytes);
        concatenated_attributes.extend_from_slice(&m1_bytes);
        concatenated_attributes.extend_from_slice(&m2_bytes);

        // Compute the hash
        let h = hash_g1(concatenated_attributes);

        // Compute s = x + y[0]*m0 + y[1]*m1 + y[2]*m2
        let mut s_exponent = sk_auth.x;

        // Perform scalar-point multiplications and accumulate the result
        s_exponent += &sk_auth.ys[0] * m0;
        s_exponent += &sk_auth.ys[1] * m1;
        s_exponent += &sk_auth.ys[2] * m2;

        // Create the signature on the expiration date
        let exp_sign = PartialExpirationDateSignature {
            h,
            s: h * s_exponent,
        };

        // Collect the exp_sign value into the vector
        exp_signs.push(exp_sign);
    }

    exp_signs

}

pub fn aggregate_expiration_signatures(
    params: &Parameters,
    vk_auth: &VerificationKeyAuth,
    expiration_date: u64,
    indices: &[u64],
    vkeys: &[VerificationKeyAuth],
    signatures: &[PartialExpirationDateSignature]) -> Result<Vec<ExpirationDateSignature>>{
    // check if we have enough unique partial signatures to meet the required threshold
    if !(indices.iter().unique_by(|&index| index).count() == indices.len()){
        return Err(CompactEcashError::ExpirationDate(
            "Not enough unique indices shares".to_string(),
        ));
    }

    // evaluate at 0 the Lagrange basis polynomials k_i
    let coefficients = generate_lagrangian_coefficients_at_origin(indices);

    let mut exp_date_signs: Vec<ExpirationDateSignature> = Vec::new();

    for l in 0..params.L() {
        let m0: u64 = expiration_date;
        let m1: u64 = expiration_date - constants::VALIDITY_PERIOD + l;
        let m2 = constants::TYPE_EXP;

        let m0_bytes: [u8; 8] = m0.to_le_bytes();
        let m1_bytes: [u8; 8] = m1.to_le_bytes();

        let m0_scalar = Scalar::from(m0);
        let m1_scalar = Scalar::from(m1);
        let m2_scalar = Scalar::from_bytes(&m2).unwrap();

        let mut concatenated_attributes = Vec::new();

        concatenated_attributes.extend_from_slice(&m0_bytes);
        concatenated_attributes.extend_from_slice(&m1_bytes);
        // Compute the hash
        let h = hash_g1(concatenated_attributes);

        for (vkey, sig) in vkeys.iter().zip(signatures.iter()) {
            if sig.h != h {
                // return Err(CompactEcashError::ExpirationDate(
                //     "Failed to verify the commitment hash".to_string(),
                // ));
            }
            // Verify the signature correctness
            let partially_signed_attributes = [m0_scalar, m1_scalar, m2_scalar]
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
                // return Err(CompactEcashError::ExpirationDate(
                //     "Verification of the partial expiration date signature failed".to_string(),
                // ));
            }
        }
        // compute the aggregated signature of the expiration date
        let aggr_s: G1Projective = coefficients.clone()
            .into_iter()
            .zip(signatures.iter())
            .map(|(coeff, sig)| sig.s * coeff)
            .sum();

        let aggr_sig = ExpirationDateSignature{
            h : signatures[0].h,
            s : aggr_s,
        };

        let signed_attributes  = [m0_scalar, m1_scalar, m2_scalar]
            .iter()
            .zip(vk_auth.beta_g2.iter())
            .map(|(m, beta_i)| beta_i * Scalar::from(*m))
            .sum::<G2Projective>();

        // check validity of the aggregated signature
        if !check_bilinear_pairing(
            &aggr_sig.h.to_affine(),
            &G2Prepared::from((vk_auth.alpha + signed_attributes).to_affine()),
            &aggr_sig.s.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            // return Err(CompactEcashError::ExpirationDate(
            //     "Verification of the aggregated expiration date signature failed".to_string(),
            // ));
        }
        exp_date_signs.push(aggr_sig);
    }
    Ok(exp_date_signs)

}